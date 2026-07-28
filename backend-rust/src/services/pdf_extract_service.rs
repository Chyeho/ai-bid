//! PDF 原始内容提取服务
//!
//! 本模块负责将 PDF 投标文件解析为结构化中间数据 [`RawDocument`]。
//! 提取内容包括：文本、单词坐标、表格、线段、矩形等排版元素，
//! 供下游语义分析模块（如章节切分、关键词定位、表格结构化）使用。
//!
//! ## 双引擎策略
//!
//! 1. **Rust 主路径** — 底层依赖 `pdfplumber` (lopdf)，速度快，对标准 PDF 效果好
//! 2. **Python 兜底路径** — 当 Rust 解析失败时，通过子进程调用 Python pdfplumber
//!    (pdfminer.six)，对畸形 content stream 的容错性远高于 Rust 版
//!
//! ## 文本清洗与段落分块
//!
//! 政府标书等 PDF 常用绝对定位渲染每个字符，导致 layout 模式的 text
//! 包含大量空格用于对齐。本模块在提取后自动执行清洗，并根据行间距
//! 将单词聚合为语义段落块，每块带有唯一 ID 和包围盒，用于下游回溯高亮。

use anyhow::{Context, Result};
use pdfplumber::{Pdf, TableSettings, TextOptions, WordOptions};
use regex::Regex;
use std::process::Command;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::domain::raw_document::{
    BBox, BlockType, RawBlock, RawDocument, RawLine, RawPage, RawRect, RawTable, RawWord,
};

// ---------- 文本清洗工具 ----------

/// 匹配"汉字后跟空白再跟汉字"的模式，用于合并被空格拆散的中文词组。
static CJK_SPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([一-鿿])\s+([一-鿿])").expect("CJK regex 编译失败"));

/// 匹配 2 个及以上的连续空格
static MULTI_SPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" {2,}").expect("multi-space regex 编译失败"));

/// 清洗 layout 文本：去除排版空格噪音，保留逻辑结构。
fn clean_layout_text(text: &str) -> String {
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    for line in &mut lines {
        for _ in 0..5 {
            let new_s = CJK_SPACE_RE.replace_all(line, "$1$2").to_string();
            if new_s == *line {
                break;
            }
            *line = new_s;
        }
        *line = MULTI_SPACE_RE.replace_all(line, "  ").to_string();
    }

    lines.join("\n")
}

/// 从单词坐标重建干净文本（当 layout text 不可用时兜底）。
fn reconstruct_text_from_words(words: &[RawWord]) -> String {
    if words.is_empty() {
        return String::new();
    }

    let mut heights: Vec<f64> = words.iter().map(|w| w.bbox.bottom - w.bbox.top).collect();
    heights.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let line_height = heights[heights.len() / 2];

    let mut sorted: Vec<&RawWord> = words.iter().collect();
    sorted.sort_by(|a, b| {
        a.bbox
            .top
            .partial_cmp(&b.bbox.top)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.bbox
                    .x0
                    .partial_cmp(&b.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut rows: Vec<Vec<&RawWord>> = Vec::new();
    let mut current_row: Vec<&RawWord> = vec![sorted[0]];
    let mut current_top = sorted[0].bbox.top;

    for w in sorted.iter().skip(1) {
        if w.bbox.top - current_top < line_height * 1.2 {
            current_row.push(w);
        } else {
            rows.push(std::mem::take(&mut current_row));
            current_row.push(w);
            current_top = w.bbox.top;
        }
    }
    rows.push(current_row);

    let mut lines: Vec<String> = Vec::new();
    for row in &mut rows {
        row.sort_by(|a, b| {
            a.bbox
                .x0
                .partial_cmp(&b.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let first_text = &row[0].text;
        let avg_w = if first_text.is_empty() {
            10.0
        } else {
            (row[0].bbox.x1 - row[0].bbox.x0) / first_text.len() as f64
        };
        let col_gap = avg_w * 8.0;

        let mut parts: Vec<String> = Vec::new();
        let mut current: Vec<&RawWord> = vec![row[0]];

        for w in row.iter().skip(1) {
            let gap = w.bbox.x0 - current.last().unwrap().bbox.x1;
            if gap < col_gap {
                current.push(w);
            } else {
                parts.push(current.iter().map(|w| w.text.as_str()).collect());
                current = vec![w];
            }
        }
        parts.push(current.iter().map(|w| w.text.as_str()).collect());

        let line = parts.join("  ");
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }

    lines.join("\n")
}

// ---------- 段落块计算 ----------

/// 行间距大于此倍率视为段落边界
const HEADING_GAP_RATIO: f64 = 1.8;

/// 从单词列表计算出语义段落块。
///
/// 分两步：先按 y 坐标分组为行，再按行间距合并为段落。
/// 每块有唯一 ID、文本和 bbox，用于下游 LLM 引用后回溯高亮。
fn compute_blocks(words: &[RawWord], page_index: usize) -> Vec<RawBlock> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut heights: Vec<f64> = words.iter().map(|w| w.bbox.bottom - w.bbox.top).collect();
    heights.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let line_height = heights[heights.len() / 2];

    let mut sorted: Vec<&RawWord> = words.iter().collect();
    sorted.sort_by(|a, b| {
        a.bbox
            .top
            .partial_cmp(&b.bbox.top)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.bbox
                    .x0
                    .partial_cmp(&b.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Step 1: 分组为行
    let mut text_rows: Vec<Vec<&RawWord>> = Vec::new();
    let mut current_row: Vec<&RawWord> = vec![sorted[0]];
    let mut current_top = sorted[0].bbox.top;

    for w in sorted.iter().skip(1) {
        if w.bbox.top - current_top < line_height * 1.2 {
            current_row.push(w);
        } else {
            text_rows.push(std::mem::take(&mut current_row));
            current_row.push(w);
            current_top = w.bbox.top;
        }
    }
    text_rows.push(current_row);

    // Step 2: 按行间距合并为段落块
    let mut blocks: Vec<RawBlock> = Vec::new();
    let mut block_rows: Vec<Vec<&RawWord>> = Vec::new();
    let mut prev_bottom: Option<f64> = None;

    for (i, row) in text_rows.iter().enumerate() {
        let row_top = row.iter().map(|w| w.bbox.top).fold(f64::INFINITY, f64::min);
        let row_bottom = row.iter().map(|w| w.bbox.bottom).fold(0.0, f64::max);

        let start_new =
            prev_bottom.is_some_and(|pb| (row_top - pb) > line_height * HEADING_GAP_RATIO);

        if start_new && !block_rows.is_empty() {
            blocks.push(build_block(&block_rows, page_index, blocks.len()));
            block_rows.clear();
        }

        block_rows.push(row.clone());
        prev_bottom = Some(row_bottom);

        if i == text_rows.len() - 1 {
            blocks.push(build_block(&block_rows, page_index, blocks.len()));
        }
    }

    blocks
}

/// 将一组行构建为一个 RawBlock。
fn build_block(rows: &[Vec<&RawWord>], page_index: usize, block_index: usize) -> RawBlock {
    let all_words: Vec<&&RawWord> = rows.iter().flat_map(|r| r.iter()).collect();

    let x0 = all_words
        .iter()
        .map(|w| w.bbox.x0)
        .fold(f64::INFINITY, f64::min);
    let top = all_words
        .iter()
        .map(|w| w.bbox.top)
        .fold(f64::INFINITY, f64::min);
    let x1 = all_words.iter().map(|w| w.bbox.x1).fold(0.0, f64::max);
    let bottom = all_words.iter().map(|w| w.bbox.bottom).fold(0.0, f64::max);

    let mut row_texts: Vec<String> = Vec::new();
    for row in rows {
        let mut sorted_row: Vec<&&RawWord> = row.iter().collect();
        sorted_row.sort_by(|a, b| {
            a.bbox
                .x0
                .partial_cmp(&b.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let text: String = sorted_row.iter().map(|w| w.text.as_str()).collect();
        if !text.trim().is_empty() {
            row_texts.push(text);
        }
    }

    let block_type = if rows.len() == 1 && all_words.len() <= 10 {
        BlockType::Heading
    } else {
        BlockType::Paragraph
    };

    RawBlock {
        id: format!("b_{}_{}", page_index, block_index),
        block_type,
        text: row_texts.join("\n"),
        bbox: BBox {
            x0,
            top,
            x1,
            bottom,
        },
    }
}

// ---------- PDF 提取主函数 ----------

/// 将 PDF 文件解析为 [`RawDocument`]。
pub fn extract_pdf_to_raw_json(path: &str) -> Result<RawDocument> {
    let pdf = Pdf::open_file(path, None)?;

    let mut pages = Vec::new();

    for page_result in pdf.pages_iter() {
        let page = page_result?;

        let page_index = page.page_number();
        let width = page.width();
        let height = page.height();

        // 1. 页面文本
        let raw_text = page.extract_text(&TextOptions {
            layout: true,
            ..Default::default()
        });

        // 2. 单词（只提取一次，后续复用）
        let words: Vec<RawWord> = page
            .extract_words(&WordOptions::default())
            .into_iter()
            .enumerate()
            .map(|(i, w)| RawWord {
                id: format!("w_{}_{}", page_index, i),
                text: w.text,
                bbox: BBox {
                    x0: w.bbox.x0,
                    top: w.bbox.top,
                    x1: w.bbox.x1,
                    bottom: w.bbox.bottom,
                },
            })
            .collect();

        // 文本清洗，必要时从单词重建
        let cleaned = clean_layout_text(&raw_text);
        let text = if cleaned.len() < raw_text.len() * 20 / 100 {
            eprintln!(
                "  [优化] 第{}页: 高空白占比 ({}→{} 字符)，用单词坐标重建文本...",
                page_index + 1,
                raw_text.len(),
                cleaned.len(),
            );
            reconstruct_text_from_words(&words)
        } else {
            cleaned
        };

        // 3. 段落块
        let blocks = compute_blocks(&words, page_index);

        // 4. 表格（带 ID，bbox 暂缺 — Rust 版 lopdf 不提供表格坐标）
        let tables: Vec<RawTable> = page
            .extract_tables(&TableSettings::default())
            .into_iter()
            .enumerate()
            .map(|(i, rows)| RawTable {
                id: format!("t_{}_{}", page_index, i),
                bbox: None,
                rows,
            })
            .collect();

        // 5. 线条
        let lines: Vec<RawLine> = page
            .lines()
            .iter()
            .map(|line| RawLine {
                bbox: BBox {
                    x0: line.x0,
                    top: line.top,
                    x1: line.x1,
                    bottom: line.bottom,
                },
            })
            .collect();

        // 6. 矩形
        let rects: Vec<RawRect> = page
            .rects()
            .iter()
            .map(|rect| RawRect {
                bbox: BBox {
                    x0: rect.x0,
                    top: rect.top,
                    x1: rect.x1,
                    bottom: rect.bottom,
                },
            })
            .collect();

        pages.push(RawPage {
            page_index,
            width,
            height,
            text,
            words,
            blocks,
            tables,
            lines,
            rects,
        });
    }

    Ok(RawDocument {
        document_id: Uuid::new_v4().to_string(),
        source_path: path.to_string(),
        pages,
    })
}

// ---------- Python 兜底提取 ----------

/// 用 Python pdfplumber 兜底提取 PDF 内容。
pub fn extract_with_python(input_path: &str, output_path: &str) -> Result<()> {
    // 编译期嵌入脚本的绝对路径（位于 backend-rust/scripts/）
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/pdf_extract.py");
    let python = std::env::var("AI_BID_PYTHON_EXECUTABLE").unwrap_or_else(|_| "python".to_string());

    let output = Command::new(&python)
        .args([script, input_path, output_path])
        .output()
        .with_context(|| format!("无法使用 {} 执行 Python 脚本: {}", python, script))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Python 脚本执行失败: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("{}", stdout.trim());
    }

    let meta = std::fs::metadata(output_path)
        .with_context(|| format!("Python 兜底提取未生成文件: {}", output_path))?;
    anyhow::ensure!(meta.len() > 0, "Python 兜底提取的 JSON 文件为空");

    Ok(())
}

// ─── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── clean_layout_text ──────────────────────────────────────────

    #[test]
    fn test_clean_layout_text_merges_cjk_spaces() {
        // 中文绝对定位渲染：每个字之间被空格填充
        let input = "投  标  人  须  在  东  莞  地  区  设  有  常  驻  服  务  机  构";
        let result = clean_layout_text(input);
        assert_eq!(result, "投标人须在东莞地区设有常驻服务机构");
    }

    #[test]
    fn test_clean_layout_text_preserves_cjk_between_lines() {
        // CJK 之间空格消除，但换行保留
        let input = "第一章  总  则\n第二条  合  同  标  的";
        let result = clean_layout_text(input);
        // CJK_SPACE_RE 合并所有汉-空-汉模式，包括跨词边界
        assert_eq!(result, "第一章总则\n第二条合同标的");
    }

    #[test]
    fn test_clean_layout_text_compresses_multiple_spaces() {
        // CJK_RE 先合并汉字间的所有空格，MULTI_SPACE_RE 处理剩余
        // "符合" 和 "以下" 都是 CJK，所以被 CJK_SPACE_RE 完全合并
        let input = "符合    以下    条件";
        let result = clean_layout_text(input);
        assert_eq!(result, "符合以下条件");
    }

    #[test]
    fn test_clean_layout_text_empty_input() {
        assert_eq!(clean_layout_text(""), "");
        assert_eq!(clean_layout_text("   \n   \n  "), "");
    }

    #[test]
    fn test_clean_layout_text_pure_ascii() {
        let input = "The bidder shall comply with the requirements.";
        let result = clean_layout_text(input);
        assert_eq!(result, "The bidder shall comply with the requirements.");
    }

    #[test]
    fn test_clean_layout_text_mixed_cjk_and_ascii() {
        // 中文用 CJK 规则，英文空格保留
        let input = "项目编号  ABC-2024  投标  人";
        let result = clean_layout_text(input);
        assert_eq!(result, "项目编号  ABC-2024  投标人");
    }

    // ── reconstruct_text_from_words ────────────────────────────────

    fn make_word(text: &str, x0: f64, top: f64, x1: f64, bottom: f64) -> RawWord {
        RawWord {
            id: String::new(),
            text: text.to_string(),
            bbox: BBox {
                x0,
                top,
                x1,
                bottom,
            },
        }
    }

    #[test]
    fn test_reconstruct_text_empty_input() {
        let words: Vec<RawWord> = vec![];
        let result = reconstruct_text_from_words(&words);
        assert_eq!(result, "");
    }

    #[test]
    fn test_reconstruct_text_single_word() {
        let words = vec![make_word("投标人", 100.0, 200.0, 140.0, 210.0)];
        let result = reconstruct_text_from_words(&words);
        assert_eq!(result, "投标人");
    }

    #[test]
    fn test_reconstruct_text_single_line() {
        // 同一行内按 X 坐标排序拼接
        let words = vec![
            make_word("投标人", 100.0, 200.0, 140.0, 210.0),
            make_word("须", 145.0, 200.0, 160.0, 210.0),
            make_word("在", 165.0, 200.0, 180.0, 210.0),
            make_word("东莞", 185.0, 200.0, 215.0, 210.0),
        ];
        let result = reconstruct_text_from_words(&words);
        assert_eq!(result, "投标人须在东莞");
    }

    #[test]
    fn test_reconstruct_text_multi_line() {
        // 跨行：Y 坐标差超过 1.2 倍行高视为新行
        let words = vec![
            make_word("第一章", 100.0, 100.0, 140.0, 110.0),
            make_word("总则", 145.0, 100.0, 170.0, 110.0),
            make_word("第一条", 100.0, 130.0, 140.0, 140.0),
            make_word("合同标的", 145.0, 130.0, 190.0, 140.0),
        ];
        let result = reconstruct_text_from_words(&words);
        assert!(result.contains('\n'), "跨行文本应包含换行符");
        assert_eq!(result, "第一章总则\n第一条合同标的");
    }

    #[test]
    fn test_reconstruct_text_column_separation() {
        // 大列间距（> 8 倍平均字宽）→ 用双空格分隔
        let words = vec![
            make_word("条款", 50.0, 100.0, 80.0, 110.0),
            // 间隙 > 8x avg_w ≈ 80pt → 列分隔
            make_word("说明", 200.0, 100.0, 230.0, 110.0),
        ];
        let result = reconstruct_text_from_words(&words);
        assert_eq!(result, "条款  说明");
    }

    // ── compute_blocks ─────────────────────────────────────────────

    #[test]
    fn test_compute_blocks_empty_input() {
        let words: Vec<RawWord> = vec![];
        let blocks = compute_blocks(&words, 0);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_compute_blocks_single_line_heading() {
        // 单行单词 ≤ 10 → heading
        let words: Vec<RawWord> = (0..5)
            .map(|i| {
                make_word(
                    &format!("w{}", i),
                    50.0 + i as f64 * 30.0,
                    100.0,
                    75.0 + i as f64 * 30.0,
                    110.0,
                )
            })
            .collect();
        let blocks = compute_blocks(&words, 0);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Heading);
        assert!(blocks[0].id.starts_with("b_0_"));
    }

    #[test]
    fn test_compute_blocks_multi_line_paragraph() {
        // 多行多个单词 → paragraph
        let mut words = Vec::new();
        // Line 1: 10 words
        for i in 0..10 {
            words.push(make_word(
                &format!("L1W{}", i),
                50.0 + i as f64 * 30.0,
                100.0,
                75.0 + i as f64 * 30.0,
                110.0,
            ));
        }
        // Line 2: 5 words (same paragraph, gap < 1.8x line_height)
        for i in 0..5 {
            words.push(make_word(
                &format!("L2W{}", i),
                50.0 + i as f64 * 30.0,
                118.0,
                75.0 + i as f64 * 30.0,
                128.0,
            ));
        }
        let blocks = compute_blocks(&words, 2);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
        assert!(blocks[0].id.starts_with("b_2_"));
    }

    #[test]
    fn test_compute_blocks_paragraph_boundary() {
        // 行间距 > 1.8x line_height → 新段落
        let mut words = Vec::new();
        // Paragraph 1: line at y=100, height=10
        for i in 0..3 {
            words.push(make_word(
                &format!("P1W{}", i),
                50.0 + i as f64 * 30.0,
                100.0,
                75.0,
                110.0,
            ));
        }
        // Paragraph 2: line at y=140, gap=30 > 1.8*10=18 → new block
        for i in 0..3 {
            words.push(make_word(
                &format!("P2W{}", i),
                50.0 + i as f64 * 30.0,
                140.0,
                75.0,
                150.0,
            ));
        }
        let blocks = compute_blocks(&words, 1);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].id.starts_with("b_1_"));
        assert!(blocks[1].id.starts_with("b_1_"));
    }

    #[test]
    fn test_compute_blocks_id_uniqueness() {
        // 每个 block 的 ID 在同一页内唯一
        let mut words = Vec::new();
        for p in 0..3 {
            let y = 100.0 + p as f64 * 40.0;
            for i in 0..5 {
                words.push(make_word(
                    &format!("W{}", i),
                    50.0 + i as f64 * 30.0,
                    y,
                    75.0,
                    y + 10.0,
                ));
            }
        }
        let blocks = compute_blocks(&words, 5);
        assert_eq!(blocks.len(), 3);
        let ids: Vec<&str> = blocks.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, vec!["b_5_0", "b_5_1", "b_5_2"]);
    }
}
