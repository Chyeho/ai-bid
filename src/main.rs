use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

use ai_bid::domain::raw_document::RawDocument;
use ai_bid::services::docx_convert_service::convert_docx_to_pdf;
use ai_bid::services::pdf_extract_service::{extract_pdf_to_raw_json, extract_with_python};
use ai_bid::services::sectionize_service::sectionize;

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

    let raw_doc: RawDocument = match extract_pdf_to_raw_json(&pdf_path) {
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

    Ok(())
}
