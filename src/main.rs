use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;

use ai_bid::services::docx_convert_service::convert_docx_to_pdf;
use ai_bid::services::pdf_extract_service::{extract_pdf_to_raw_json, extract_with_python};

fn main() -> Result<()> {
    // 从命令行参数或默认值获取输入路径
    let input_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/智慧教室环境改造工程.pdf".to_string());

    let input = Path::new(&input_path);
    anyhow::ensure!(input.exists(), "文件不存在: {}", input.display());

    // 根据扩展名确定处理路径
    let pdf_path: String;
    let output_path: String;

    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "docx" | "doc" => {
            // 路径1: DOCX → LibreOffice → PDF → 提取管线
            let output_dir = input
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".");
            let converted = convert_docx_to_pdf(&input_path, output_dir)?;
            pdf_path = converted.to_string_lossy().to_string();
            // 生成对应的 JSON 输出路径
            let stem = input.file_stem().unwrap().to_string_lossy();
            output_path = format!("{}/{}raw.json", output_dir, stem);
        }
        "pdf" => {
            // 路径2: PDF → 直接提取
            pdf_path = input_path.clone();
            let stem = input.file_stem().unwrap().to_string_lossy();
            let parent = input.parent().and_then(|p| p.to_str()).unwrap_or(".");
            output_path = format!("{}/{}raw.json", parent, stem);
        }
        other => anyhow::bail!("不支持的文件格式: .{}，仅支持 .pdf / .docx / .doc", other),
    }

    println!("输入文件: {}", input_path);
    println!("PDF 路径: {}", pdf_path);

    // 1. 先尝试 Rust 直接解析
    match extract_pdf_to_raw_json(&pdf_path) {
        Ok(doc) => {
            println!("Rust pdfplumber 解析成功");
            let json = serde_json::to_string_pretty(&doc)?;
            fs::write(&output_path, json)?;
        }
        Err(e) => {
            // 2. 失败则用 Python pdfplumber 兜底
            println!("Rust pdfplumber 失败: {}", e);
            println!("切换到 Python pdfplumber 兜底提取...");
            extract_with_python(&pdf_path, &output_path)?;
        }
    }

    println!("Raw JSON 已生成: {}", output_path);
    Ok(())
}
