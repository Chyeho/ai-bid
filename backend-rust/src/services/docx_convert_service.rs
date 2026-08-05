//! DOCX → PDF 转换服务
//!
//! 通过调用 LibreOffice headless 模式将 .docx 文件转换为 PDF，
//! 转换后的 PDF 可直接输入现有的 PDF 提取管线。
//!
//! ## 依赖
//!
//! 需要系统已安装 LibreOffice（Windows / Linux / macOS 均支持）。
//! 默认搜索路径：
//!   - Windows: `C:\Program Files\LibreOffice\program\soffice.exe`
//!   - Linux/macOS: `soffice` (PATH 中)

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 搜索 LibreOffice 可执行文件路径。
///
/// 按以下顺序查找：
/// 1. 环境变量 `LIBREOFFICE_PATH`
/// 2. Windows 默认安装路径
/// 3. PATH 中的 `soffice`
fn find_soffice() -> Result<PathBuf> {
    // 1. 环境变量
    if let Ok(path) = std::env::var("LIBREOFFICE_PATH") {
        let p = Path::new(&path);
        if p.exists() {
            return Ok(p.to_path_buf());
        }
    }

    // 2. Windows 默认路径
    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ];
        for candidate in &candidates {
            let p = Path::new(candidate);
            if p.exists() {
                return Ok(p.to_path_buf());
            }
        }
    }

    // 3. PATH 中的 soffice (Linux/macOS)
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(output) = Command::new(which_cmd).arg("soffice").output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let p = Path::new(&path);
        if p.exists() {
            return Ok(p.to_path_buf());
        }
    }

    anyhow::bail!(
        "找不到 LibreOffice (soffice)。请安装 LibreOffice 或设置 LIBREOFFICE_PATH 环境变量。\n\
         下载: https://www.libreoffice.org/download/"
    )
}

/// 将 DOCX 文件转换为 PDF。
///
/// # 参数
///
/// * `input_path` - 输入的 .docx 文件路径
/// * `output_dir` - 输出目录（生成的 PDF 文件名与输入相同，仅扩展名改为 .pdf）
///
/// # 返回
///
/// 成功时返回生成的 PDF 文件路径。
///
/// # 示例
///
/// ```ignore
/// let pdf_path = convert_docx_to_pdf("tests/投标文件.docx", "tests/")?;
/// // → "tests/投标文件.pdf"
/// ```
pub fn convert_docx_to_pdf(input_path: &str, output_dir: &str) -> Result<PathBuf> {
    let input = Path::new(input_path);

    // 验证输入文件存在
    anyhow::ensure!(input.exists(), "输入文件不存在: {}", input.display());

    // 检查扩展名
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    anyhow::ensure!(
        ext == "docx" || ext == "doc",
        "不支持的文件格式: .{}，仅支持 .docx / .doc",
        ext
    );

    let soffice = find_soffice()?;
    let output_dir_abs = Path::new(output_dir)
        .canonicalize()
        .with_context(|| format!("输出目录不存在或无法访问: {}", output_dir))?;

    println!(
        "  [转换] {} → PDF (LibreOffice)",
        input.file_name().unwrap().to_string_lossy()
    );

    let output = Command::new(&soffice)
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(&output_dir_abs)
        .arg(input.canonicalize()?)
        .output()
        .with_context(|| {
            format!(
                "无法执行 LibreOffice: {}。请确认已安装。",
                soffice.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "LibreOffice 转换失败:\nstdout: {}\nstderr: {}",
            stdout.trim(),
            stderr.trim()
        );
    }

    // 推断输出 PDF 路径
    let stem = input.file_stem().unwrap().to_string_lossy();
    let pdf_path = output_dir_abs.join(format!("{}.pdf", stem));

    anyhow::ensure!(
        pdf_path.exists(),
        "LibreOffice 未生成 PDF 文件: {}",
        pdf_path.display()
    );

    // 打印转换日志
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        println!("  {}", stdout.trim());
    }

    Ok(pdf_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_soffice() {
        let result = find_soffice();
        assert!(result.is_ok(), "LibreOffice 未安装: {:?}", result.err());
    }
}
