#!/usr/bin/env pwsh
# E2E 全链路验证脚本
#
# 验证文档解析管线的端到端正确性（DOCX -> PDF -> Raw -> Sections -> Chunks -> Embeddings）。
# 用法:
#   cd backend-rust
#   .\tests\verify_e2e.ps1                    # 使用默认测试 PDF
#   .\tests\verify_e2e.ps1 -PdfPath "file.pdf" # 指定测试 PDF

param(
    [string]$PdfPath = "",
    [switch]$SkipDocx = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# 数据根目录
$DataDir = if ($env:AIBID_DATA_DIR) { $env:AIBID_DATA_DIR } else { $ProjectRoot }

# 默认测试 PDF
if (-not $PdfPath) {
    $candidates = @(
        "$ProjectRoot\tests\file\研究生院智慧校园项目招标文件.pdf",
        "$ProjectRoot\tests\file\清华大学深圳国际研究生院智慧校园项目公开招标文件.pdf",
        "$ProjectRoot\tests\file\智慧教室环境改造工程.pdf",
        "$ProjectRoot\tests\file\清华大学智慧校园项目招标文件.pdf"
    )
    $PdfPath = ($candidates | Where-Object { Test-Path $_ } | Select-Object -First 1)
}

if (-not $PdfPath -or -not (Test-Path $PdfPath)) {
    Write-Host "[ERROR] 找不到测试 PDF 文件" -ForegroundColor Red
    Write-Host "  请将测试 PDF 放到 tests/file/ 目录或使用 -PdfPath 指定"
    exit 1
}

Write-Host "============================================================"
Write-Host "  AI-Bid E2E 全链路验证"
Write-Host "============================================================"
Write-Host "  PDF: $PdfPath"
Write-Host "  数据目录: $DataDir"
Write-Host ""

$env:AIBID_DATA_DIR = $DataDir

$totalTests = 5
$passed = 0
$failed = 0

function Test-Step {
    param([string]$Name, [string]$Expected, [scriptblock]$Check)
    Write-Host "  [$Name] " -NoNewline
    try {
        $result = & $Check
        if ($result -eq $true -or $result -like "*$Expected*") {
            Write-Host "PASS" -ForegroundColor Green
            $script:passed++
        } else {
            Write-Host "FAIL (expected: $Expected, got: $result)" -ForegroundColor Red
            $script:failed++
        }
    } catch {
        Write-Host "FAIL ($_)" -ForegroundColor Red
        $script:failed++
    }
}

# Step 1: 检查依赖
Write-Host "--- 依赖检查 ---"
Test-Step "Rust 编译" "Finished" {
    cargo check --lib 2>&1 | Select-String "Finished" | Out-String
}
if ($Verbose) {
    Write-Host "    Python: $(python --version 2>&1)"
    Write-Host "    LibreOffice: $(soffice --version 2>&1 | Select-Object -First 1)"
}

# Step 2: cargo test --lib
Write-Host "`n--- 单元测试 ---"
Test-Step "cargo test" "0 failed" {
    $output = cargo test --lib 2>&1 | Out-String
    if ($output -match "(\d+) passed.*(\d+) failed") {
        "$($matches[2]) failed"
    } else {
        "unknown"
    }
}

# Step 3: 运行 CLI 管线
Write-Host "`n--- CLI 管线 ---"
$outputDir = "$DataDir\output"
$tempOutput = "$DataDir\tmp\e2e_test_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
New-Item -ItemType Directory -Force -Path $tempOutput | Out-Null

Test-Step "CLI 运行" "Agent 审查完成" {
    $output = cargo run -- "$PdfPath" 2>&1 | Out-String
    if ($output -match "审查完成") { "审查完成" } else { $output.Substring(0, [Math]::Min(200, $output.Length)) }
}

# Step 4: 检查输出文件
Write-Host "`n--- 输出文件 ---"
$pdfStem = [System.IO.Path]::GetFileNameWithoutExtension($PdfPath)

Test-Step "raw_json 存在" "True" {
    $files = Get-ChildItem "$outputDir\raw_json\*_raw.json" | Where-Object { $_.Name -like "*$pdfStem*" }
    ($files.Count -gt 0).ToString()
}

Test-Step "sections 存在" "True" {
    $files = Get-ChildItem "$outputDir\sections\*_sections.json" | Where-Object { $_.Name -like "*$pdfStem*" }
    ($files.Count -gt 0).ToString()
}

Test-Step "chunks 存在" "True" {
    $files = Get-ChildItem "$outputDir\chunks\*_chunks.json" | Where-Object { $_.Name -like "*$pdfStem*" }
    ($files.Count -gt 0).ToString()
}

Test-Step "embeddings 存在" "True" {
    $files = Get-ChildItem "$outputDir\embeddings\*_embedding_index\chunk_meta.json" -Recurse | Where-Object { $_.FullName -like "*$pdfStem*" }
    ($files.Count -gt 0).ToString()
}

# Step 5: 验证脚本
Write-Host "`n--- 质量验证脚本 ---"
if (Get-Command python -ErrorAction SilentlyContinue) {
    $chunkFile = Get-ChildItem "$outputDir\chunks\*_chunks.json" | Where-Object { $_.Name -like "*$pdfStem*" } | Select-Object -First 1
    if ($chunkFile) {
        Test-Step "validate_chunks" "PASS" {
            $out = python "$ScriptDir\..\scripts\validate_chunks.py" $chunkFile.FullName 2>&1 | Out-String
            if ($out -match "PASS") { "PASS" } else { "FAIL" }
        }
    } else {
        Write-Host "  [validate_chunks] SKIP — 无匹配 chunk 文件"
    }

    $embedDir = Get-ChildItem "$outputDir\embeddings\*_embedding_index" -Directory | Where-Object { $_.Name -like "*$pdfStem*" } | Select-Object -First 1
    if ($embedDir) {
        Test-Step "validate_embeddings" "PASS" {
            $out = python "$ScriptDir\..\scripts\validate_embeddings.py" $embedDir.FullName 2>&1 | Out-String
            if ($out -match "All automated checks passed") { "PASS" } else { "FAIL" }
        }
    } else {
        Write-Host "  [validate_embeddings] SKIP — 无匹配 embedding 目录"
    }
} else {
    Write-Host "  Python 未安装，跳过质量验证脚本"
}

# Summary
Write-Host ""
Write-Host "============================================================"
Write-Host "  E2E 验证结果: $passed/$totalTests PASSED"
if ($failed -gt 0) {
    Write-Host "  $failed tests FAILED" -ForegroundColor Red
    exit 1
} else {
    Write-Host "  全链路验证通过" -ForegroundColor Green
    exit 0
}
