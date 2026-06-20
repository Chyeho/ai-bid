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
//! ## 文本清洗
//!
//! 政府标书等 PDF 常用绝对定位渲染每个字符，导致 layout 模式的 text
//! 包含大量空格用于对齐。本模块在提取后自动执行清洗和重建。

use anyhow::{Context, Result};
use pdfplumber::{Pdf, TableSettings, TextOptions, WordOptions};
use regex::Regex;
use std::process::Command;
use std::sync::LazyLock;
use uuid::Uuid;

use crate::domain::raw_document::{
    BBox, RawDocument, RawLine, RawPage, RawRect, RawTable, RawWord,
};

// ---------- 文本清洗工具 ----------

// ---------- 文本清洗工具 ----------

/// 匹配"汉字后跟空白再跟汉字"的模式，用于合并被空格拆散的中文词组。
/// 例如："竞 争 性 磋 商" → "竞争性磋商"
static CJK_SPACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([一-鿿])\s+(?=[一-鿿])").expect("CJK regex 编译失败")
});

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

    // 合并汉字之间被空格拆散的情况
    // "竞 争 性 磋 商 文 件" → "竞争性磋商文件"
    for line in &mut lines {
        // 多次替换直到无法再合并（处理多次空格穿插）
        for _ in 0..5 {
            let new_s = CJK_SPACE_RE.replace_all(line, "$1").to_string();
            if new_s == *line {
                break;
            }
            *line = new_s;
        }
        // 压缩连续空格
        *line = MULTI_SPACE_RE.replace_all(line, "  ").to_string();
    }

    lines.join("\n")
}

/// 从单词坐标重建干净文本（当 layout text 不可用时兜底）。
///
/// 按 y 坐标分行，行内按 x 坐标排序，自动检测列/块边界。
fn reconstruct_text_from_words(words: &[RawWord]) -> String {
    if words.is_empty() {
        return String::new();
    }

    // 估算行高（取中位数词高）
    let mut heights: Vec<f64> = words
        .iter()
        .map(|w| w.bbox.bottom - w.bbox.top)
        .collect();
    heights.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let line_height = heights[heights.len() / 2];

    // 按 y 坐标排序，分组为行
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

    // 行内排序 + 合并
    let mut lines: Vec<String> = Vec::new();
    for row in &mut rows {
        row.sort_by(|a, b| {
            a.bbox
                .x0
                .partial_cmp(&b.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 估算字符宽
        let first_text = &row[0].text;
        let avg_w = if first_text.is_empty() {
            10.0
        } else {
            (row[0].bbox.x1 - row[0].bbox.x0) / first_text.len() as f64
        };
        let col_gap = avg_w * 8.0; // 列间距阈值

        let mut parts: Vec<String> = Vec::new();
        let mut current: Vec<&RawWord> = vec![row[0]];

        for w in row.iter().skip(1) {
            let gap = w.bbox.x0 - current.last().unwrap().bbox.x1;
            if gap < col_gap {
                current.push(w);
            } else {
                parts.push(current.iter().map(|w| w.text.as_str()).collect::<String>());
                current = vec![w];
            }
        }
        parts.push(current.iter().map(|w| w.text.as_str()).collect::<String>());

        let line = parts.join("  ");
        if !line.trim().is_empty() {
            lines.push(line);
        }
    }

    lines.join("\n")
}

/// 将 PDF 文件解析为 [`RawDocument`]。
///
/// # 提取内容
///
/// 对 PDF 的每一页依次提取以下 5 类信息：
///
/// 1. **页面文本** — 启用 `layout: true`，尽可能保留原始版面结构，适合标书这种
///    章节标题、正文段落与表格混排的文档。
/// 2. **单词 + 包围盒** — 每个单词的文本和坐标，用于关键词搜索高亮、坐标敏感提取。
/// 3. **表格** — 以 `Vec<Vec<Option<String>>>` 二维网格表示，兼容合并单元格。
/// 4. **线段** — 线条元素（下划线、分隔线、表格边框等）。
/// 5. **矩形** — 闭合矩形区域（图片占位框、色块、文本框边界等）。
///
/// # 参数
///
/// * `path` - PDF 文件的磁盘路径，如 `"./bids/投标文件.pdf"`
///
/// # 返回
///
/// 成功时返回 [`RawDocument`]，包含文档级别的元信息和所有页面的提取结果。
/// 失败时（如文件不存在、PDF 损坏、解析错误）返回 `anyhow::Error`。
pub fn extract_pdf_to_raw_json(path: &str) -> Result<RawDocument> {
    let pdf = Pdf::open_file(path, None)?;

    let mut pages = Vec::new();

    for page_result in pdf.pages_iter() {
        let page = page_result?;

        let page_index = page.page_number();
        let width = page.width();
        let height = page.height();

        // 1. 页面文本
        // layout=true 更接近原始版面，适合标书这种章节/表格混排文档
        let raw_text = page.extract_text(&TextOptions {
            layout: true,
            ..Default::default()
        });

        // 清洗文本：处理绝对定位 PDF 的排版空格噪音
        let text = clean_layout_text(&raw_text);

        // 2. 单词 + 包围盒（BBox）
        // 每个单词附带其坐标信息，后续可用于：
        // - 关键词搜索 + 页面高亮定位
        // - 按坐标区域提取内容（如"页眉的公司名称"）
        let words: Vec<RawWord> = page
            .extract_words(&WordOptions::default())
            .into_iter()
            .map(|w| RawWord {
                text: w.text,
                bbox: BBox {
                    x0: w.bbox.x0,
                    top: w.bbox.top,
                    x1: w.bbox.x1,
                    bottom: w.bbox.bottom,
                },
            })
            .collect();

        // 如果清洗后仍然太空洞（空白占比 > 80%），用单词坐标重建
        let text = if text.len() < raw_text.len() * 20 / 100 {
            eprintln!(
                "  [优化] 第{}页: 高空白占比 ({}→{} 字符)，用单词坐标重建文本...",
                page_index + 1,
                raw_text.len(),
                text.len(),
            );
            reconstruct_text_from_words(&words)
        } else {
            text
        };

        // 3. 表格
        // 二维网格结构：外层 Vec 为行，内层 Vec<Option<String>> 为单元格，
        // None 表示合并单元格导致的内容缺失。
        let tables = page
            .extract_tables(&TableSettings::default())
            .into_iter()
            .map(|rows| RawTable { rows })
            .collect();

        // 4. 线条
        // 包含下划线、删除线、表格边框、分隔线等排版元素，
        // 可用于后续的版面结构分析（如识别表单填写区域）。
        let lines = page
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

        // 5. 矩形
        // 闭合矩形区域，通常对应图片占位框、色块填充区、
        // 文本框边界等，有助于识别非文本内容区域。
        let rects = page
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
///
/// 调用 `scripts/pdf_extract.py`，输出与 Rust `RawDocument` 结构对齐的 JSON。
/// Python 版 pdfplumber 底层为 pdfminer.six，对流语法错误有更好的容错处理。
///
/// # 参数
///
/// * `input_path` - PDF 文件路径
/// * `output_path` - 输出 JSON 文件路径
pub fn extract_with_python(input_path: &str, output_path: &str) -> Result<()> {
    let script = "scripts/pdf_extract.py";

    let output = Command::new("python")
        .args([script, input_path, output_path])
        .output()
        .with_context(|| format!("无法执行 Python 脚本: {}", script))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Python 脚本执行失败: {}", stderr);
    }

    // 打印 Python 脚本的标准输出（含进度信息）
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("{}", stdout.trim());
    }

    // 验证输出文件存在且非空
    let meta = std::fs::metadata(output_path)
        .with_context(|| format!("Python 兜底提取未生成文件: {}", output_path))?;
    anyhow::ensure!(meta.len() > 0, "Python 兜底提取的 JSON 文件为空");

    Ok(())
}