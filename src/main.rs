use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

use ai_bid::domain::chunk::{Chunk, ChunkType, ChunkingConfig};
use ai_bid::domain::raw_document::RawDocument;
use ai_bid::services::chunking_service::chunk_sections;
use ai_bid::services::docx_convert_service::convert_docx_to_pdf;
use ai_bid::services::pdf_extract_service::{extract_pdf_to_raw_json, extract_with_python};
use ai_bid::services::sectionize_service::sectionize;
use serde::Serialize;

/// Chunk 切分完整输出（对应验证.md V4.8 格式）。
#[derive(Debug, Serialize)]
struct ChunkingOutput {
    document_id: String,
    source_path: String,
    config: ChunkingConfigInfo,
    stats: ChunkingStats,
    chunks: Vec<ChunkOutputItem>,
}

#[derive(Debug, Serialize)]
struct ChunkingConfigInfo {
    merge_min_len: usize,
    split_max_len: usize,
    split_overlap: usize,
    embed_ctx_depth: usize,
    min_chunk_size: usize,
    embed_path_max_len: usize,
}

#[derive(Debug, Serialize)]
struct ChunkingStats {
    total_chunks: usize,
    #[serde(rename = "type_counts")]
    type_counts: TypeCounts,
    total_chars: usize,
    avg_chunk_size: f64,
    max_chunk_size: usize,
    min_chunk_size: usize,
}

#[derive(Debug, Serialize)]
struct TypeCounts {
    #[serde(rename = "Leaf")]
    leaf: usize,
    #[serde(rename = "Merged")]
    merged: usize,
    #[serde(rename = "Split")]
    split: usize,
}

/// 单个 chunk 的输出格式（含 embed_text）。
#[derive(Debug, Serialize)]
struct ChunkOutputItem {
    chunk_id: String,
    chunk_type: serde_json::Value,
    section_path: Vec<String>,
    text: String,
    page_start: usize,
    page_end: usize,
    source_block_ids: Vec<String>,
    embed_text: String,
}

/// 递归收集 Section 树中所有的 block_id。
fn collect_all_block_ids(section: &ai_bid::services::sectionize_service::Section) -> Vec<&str> {
    let mut ids: Vec<&str> = section.block_ids.iter().map(|s| s.as_str()).collect();
    for child in &section.children {
        ids.extend(collect_all_block_ids(child));
    }
    ids
}

impl ChunkingOutput {
    fn new(
        document_id: String,
        source_path: String,
        config: &ChunkingConfig,
        chunks: &[Chunk],
    ) -> Self {
        let leaf_count = chunks.iter().filter(|c| matches!(c.chunk_type, ChunkType::Leaf)).count();
        let merged_count = chunks
            .iter()
            .filter(|c| matches!(c.chunk_type, ChunkType::Merged { .. }))
            .count();
        let split_count = chunks
            .iter()
            .filter(|c| matches!(c.chunk_type, ChunkType::Split { .. }))
            .count();

        let sizes: Vec<usize> = chunks.iter().map(|c| c.text.chars().count()).collect();
        let total_chars: usize = sizes.iter().sum();
        let max_size = sizes.iter().copied().max().unwrap_or(0);
        let min_size = sizes.iter().copied().min().unwrap_or(0);
        let avg_size = if chunks.is_empty() {
            0.0
        } else {
            total_chars as f64 / chunks.len() as f64
        };

        let chunk_items: Vec<ChunkOutputItem> = chunks
            .iter()
            .map(|c| {
                let chunk_type_value = match &c.chunk_type {
                    ChunkType::Leaf => {
                        serde_json::json!({ "type": "Leaf" })
                    }
                    ChunkType::Merged { rule, child_count } => {
                        serde_json::json!({
                            "type": "Merged",
                            "rule": rule,
                            "child_count": child_count
                        })
                    }
                    ChunkType::Split { part, total } => {
                        serde_json::json!({
                            "type": "Split",
                            "part": part,
                            "total": total
                        })
                    }
                };
                ChunkOutputItem {
                    chunk_id: c.chunk_id.clone(),
                    chunk_type: chunk_type_value,
                    section_path: c.section_path.clone(),
                    text: c.text.clone(),
                    page_start: c.page_start,
                    page_end: c.page_end,
                    source_block_ids: c.source_block_ids.clone(),
                    embed_text: c.embed_text(config.embed_ctx_depth, config.embed_path_max_len),
                }
            })
            .collect();

        ChunkingOutput {
            document_id,
            source_path,
            config: ChunkingConfigInfo {
                merge_min_len: config.merge_min_len,
                split_max_len: config.split_max_len,
                split_overlap: config.split_overlap,
                embed_ctx_depth: config.embed_ctx_depth,
                min_chunk_size: config.min_chunk_size,
                embed_path_max_len: config.embed_path_max_len,
            },
            stats: ChunkingStats {
                total_chunks: chunks.len(),
                type_counts: TypeCounts {
                    leaf: leaf_count,
                    merged: merged_count,
                    split: split_count,
                },
                total_chars,
                avg_chunk_size: (avg_size * 10.0).round() / 10.0,
                max_chunk_size: max_size,
                min_chunk_size: min_size,
            },
            chunks: chunk_items,
        }
    }
}

fn main() -> Result<()> {
    // 从命令行参数或默认值获取输入路径
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/智慧教室环境改造工程.pdf".to_string());

    let input = Path::new(&input_path);
    anyhow::ensure!(input.exists(), "文件不存在: {}", input.display());

    // 根据扩展名确定处理路径
    let pdf_path: String;
    let stem: String;

    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "docx" | "doc" => {
            let dir = input
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".")
                .to_string();
            let converted = convert_docx_to_pdf(&input_path, &dir)?;
            pdf_path = converted.to_string_lossy().to_string();
            stem = input.file_stem().unwrap().to_string_lossy().to_string();
        }
        "pdf" => {
            pdf_path = input_path.clone();
            stem = input.file_stem().unwrap().to_string_lossy().to_string();
        }
        other => anyhow::bail!("不支持的文件格式: .{}，仅支持 .pdf / .docx / .doc", other),
    }

    println!("输入文件: {}", input_path);
    println!("PDF 路径: {}", pdf_path);

    // ─── 阶段 1: PDF → RawDocument ───────────────────────────

    let raw_json_dir = "output/raw_json".to_string();
    fs::create_dir_all(&raw_json_dir)
        .with_context(|| format!("无法创建输出目录: {}", raw_json_dir))?;
    let raw_json_path = format!("{}/{}_raw.json", raw_json_dir, stem);

    let mut raw_doc: RawDocument = match extract_pdf_to_raw_json(&pdf_path) {
        Ok(doc) => {
            println!("Rust pdfplumber 解析成功");
            let json = serde_json::to_string_pretty(&doc)?;
            fs::write(&raw_json_path, json)?;
            println!("Raw JSON 已生成: {}", raw_json_path);
            doc
        }
        Err(e) => {
            println!("Rust pdfplumber 失败: {}", e);
            println!("切换到 Python pdfplumber 兜底提取...");
            extract_with_python(&pdf_path, &raw_json_path)?;
            println!("Raw JSON 已生成: {}", raw_json_path);
            // Python 兜底后，读回 RawDocument
            let json_str = fs::read_to_string(&raw_json_path)
                .with_context(|| "无法读取 Python 兜底输出的 JSON")?;
            serde_json::from_str(&json_str)
                .with_context(|| "Python 兜底输出的 JSON 解析失败")?
        }
    };

    // ─── 阶段 2: RawDocument → Sections ──────────────────────

    println!("正在进行章节结构识别 (sectionize)...");
    let sections_output = sectionize(&raw_doc);

    let sections_dir = "output/sections".to_string();
    fs::create_dir_all(&sections_dir)
        .with_context(|| format!("无法创建输出目录: {}", sections_dir))?;

    let sections_path = format!("{}/{}_sections.json", sections_dir, stem);
    let sections_json = serde_json::to_string_pretty(&sections_output)?;
    fs::write(&sections_path, sections_json)?;

    println!("Sections JSON 已生成: {}", sections_path);
    println!(
        "  总章节数: {} (orphan blocks: {})",
        sections_output.stats.total_sections, sections_output.stats.orphan_blocks
    );
    for (level, count) in sections_output.stats.level_counts.iter() {
        println!("    Level {}: {} 个", level, count);
    }

    // ─── 阶段 2.4: 启发式表格检测（`|` 分隔符）─────────────────
    // 在 sectionize 之后、表格注入之前执行，确保后续 inject_tables_into_sections
    // 能消费这些补充表格。对于 pdfplumber 已检测到表格的标书，此步骤通常为无操作。

    println!("正在启发式检测纯文本表格...");
    let pipe_table_count = ai_bid::services::sectionize_service::detect_pipe_tables(&mut raw_doc);
    println!("  已启发式检测到 {} 张纯文本表格", pipe_table_count);

    // ─── 阶段 2.5: Orphan blocks → Section（在表格注入之前）──────
    // 将孤儿块按连续页码分组构造临时 Section，追加到 sections 列表，
    // 使其走完整的 chunking 管线（含 split_long_chunk + merge_tiny_chunks）。

    let chunking_config = ChunkingConfig::default();

    // 收集所有已分配的 block_id
    let assigned: std::collections::HashSet<&str> = sections_output
        .sections
        .iter()
        .flat_map(|s| collect_all_block_ids(s))
        .collect();

    // 找出未分配的 orphan blocks
    let orphan_blocks: Vec<&ai_bid::domain::raw_document::RawBlock> = raw_doc
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| !assigned.contains(b.id.as_str()))
        .collect();

    let mut all_sections = sections_output.sections.clone();

    if !orphan_blocks.is_empty() {
        // 构建 block_id → page_index 索引（避免 O(P×B×O) 嵌套扫描）
        let block_page: std::collections::HashMap<&str, usize> = raw_doc
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(move |b| (b.id.as_str(), p.page_index as usize)))
            .collect();

        // 按页码分组 orphan blocks
        let mut page_to_blocks: std::collections::BTreeMap<
            usize,
            Vec<&ai_bid::domain::raw_document::RawBlock>,
        > = std::collections::BTreeMap::new();
        for block in &orphan_blocks {
            if let Some(&page_idx) = block_page.get(block.id.as_str()) {
                page_to_blocks.entry(page_idx).or_default().push(*block);
            }
        }

        // 将连续页码合并为组（相邻页码 → 同一组）
        let sorted_pages: Vec<usize> = page_to_blocks.keys().copied().collect();
        let mut page_groups: Vec<Vec<usize>> = Vec::new();
        let mut current_group: Vec<usize> = Vec::new();
        for &p in &sorted_pages {
            if current_group.is_empty() || p == current_group.last().unwrap() + 1 {
                current_group.push(p);
            } else {
                page_groups.push(std::mem::take(&mut current_group));
                current_group.push(p);
            }
        }
        if !current_group.is_empty() {
            page_groups.push(current_group);
        }

        // 每组构造一个临时 Section
        for group in &page_groups {
            let group_start = *group.first().unwrap();
            let group_end = *group.last().unwrap();
            let group_blocks: Vec<&&ai_bid::domain::raw_document::RawBlock> = group
                .iter()
                .flat_map(|p| page_to_blocks[p].iter())
                .collect();
            let orphan_ids: Vec<String> = group_blocks.iter().map(|b| b.id.clone()).collect();
            let orphan_text = group_blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            let orphan_section = ai_bid::services::sectionize_service::Section {
                level: 0, // 特殊层级：未归类内容
                title: format!("未归类内容 (第{}-{}页)", group_start + 1, group_end + 1),
                pattern: "orphan".to_string(),
                page_start: group_start,
                page_end: group_end,
                block_ids: orphan_ids,
                body_text: orphan_text,
                children: Vec::new(),
                body_page_start: group_start,
                body_page_end: group_end,
            };

            all_sections.push(orphan_section);
        }

        println!(
            "  已补充 {} 个 orphan block（{} 个连续页码组）兜底 Section",
            orphan_blocks.len(),
            page_groups.len()
        );
    }

    // ─── 阶段 2.6: 表格内容注入 Sections ───────────────────────
    // 注意：此处使用 all_sections（含孤儿 Section），而非 sections_output.sections，
    // 确保孤儿 Section 也能接收表格内容注入。

    println!("正在注入表格内容到章节结构...");
    ai_bid::services::sectionize_service::inject_tables_into_sections(
        &mut all_sections,
        &raw_doc,
    );
    // 递归统计所有 section（含子节点）中 t_ 前缀的 block_id 数量
    fn count_table_ids(sections: &[ai_bid::services::sectionize_service::Section]) -> usize {
        sections
            .iter()
            .map(|s| {
                s.block_ids.iter().filter(|id| id.starts_with("t_")).count()
                    + count_table_ids(&s.children)
            })
            .sum()
    }
    let injected_table_count = count_table_ids(&all_sections);
    println!("  已注入 {} 张表格到对应章节", injected_table_count);

    // ─── 阶段 3: Sections → Chunks ────────────────────────────

    println!("正在进行条款级 Chunk 切分 (chunking)...");
    let chunks = chunk_sections(&all_sections, &chunking_config);

    let chunks_dir = "output/chunks".to_string();
    fs::create_dir_all(&chunks_dir)
        .with_context(|| format!("无法创建输出目录: {}", chunks_dir))?;

    let chunks_path = format!("{}/{}_chunks.json", chunks_dir, stem);

    let chunking_output = ChunkingOutput::new(
        sections_output.document_id.clone(),
        sections_output.source_path.clone(),
        &chunking_config,
        &chunks,
    );
    let chunks_json = serde_json::to_string_pretty(&chunking_output)?;
    fs::write(&chunks_path, chunks_json)?;

    println!("Chunks JSON 已生成: {}", chunks_path);
    let stats = &chunking_output.stats;
    println!("  总 Chunk 数: {}", stats.total_chunks);
    println!(
        "  类型分布 — Leaf: {}, Merged: {}, Split: {}",
        stats.type_counts.leaf, stats.type_counts.merged, stats.type_counts.split
    );
    println!(
        "  大小 — 总计 {} 字符, 平均 {:.1}, 最小 {}, 最大 {}",
        stats.total_chars, stats.avg_chunk_size, stats.min_chunk_size, stats.max_chunk_size
    );

    Ok(())
}
