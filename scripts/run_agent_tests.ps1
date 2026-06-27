# Agent 框架集成测试运行脚本
# 使用真实 LLM API 验证 §8-§13 各模块行为
#
# 前置条件:
#   - .env 中已配置 API 密钥（OPENAI_API_KEY 或 DASHSCOPE_API_KEY）
#   - LibreOffice 已安装（可选，仅 PDF 测试需要）
#
# 用法:
#   ./scripts/run_agent_tests.ps1                    # 全部测试
#   ./scripts/run_agent_tests.ps1 -Test bus          # 单个测试
#   ./scripts/run_agent_tests.ps1 -Test bus,legal    # 多个测试
#   ./scripts/run_agent_tests.ps1 -SkipFault         # 跳过故障测试

param(
    [string]$Test = "all",
    [switch]$SkipFault = $false,
    [string]$OutputDir = "output/test_results"
)

$ErrorActionPreference = "Continue"
$script:StartTime = Get-Date

# ── 颜色辅助 ─────────────────────────────────────────────────
function Write-Header {
    param([string]$Text)
    Write-Host "`n$('═' * 60)" -ForegroundColor Cyan
    Write-Host "  $Text" -ForegroundColor Cyan
    Write-Host "$('═' * 60)" -ForegroundColor Cyan
}

function Write-Pass {
    param([string]$Text)
    Write-Host "  [PASS] $Text" -ForegroundColor Green
}

function Write-Fail {
    param([string]$Text)
    Write-Host "  [FAIL] $Text" -ForegroundColor Red
}

function Write-Skip {
    param([string]$Text)
    Write-Host "  [SKIP] $Text" -ForegroundColor Yellow
}

function Write-Info {
    param([string]$Text)
    Write-Host "  [INFO] $Text" -ForegroundColor Gray
}

# ── 环境检查 ─────────────────────────────────────────────────
Write-Header "Agent 框架集成测试"

Write-Info "检查前置条件..."

# 检查 .env 文件
if (-not (Test-Path ".env")) {
    Write-Fail ".env 文件不存在。请创建 .env 并设置 API 密钥"
    Write-Info "示例: OPENAI_API_KEY=sk-xxx"
    exit 1
}

# 检查 Rust 工具链
$cargoVersion = cargo --version 2>$null
if (-not $cargoVersion) {
    Write-Fail "cargo 未找到。请安装 Rust 工具链。"
    exit 1
}
Write-Info "cargo: $cargoVersion"

# 确保输出目录存在
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# 设置环境变量
$env:AIBID_AGENT = "1"

# 检查 LLM 连接
Write-Info "验证 LLM 连接..."
$llmCheck = cargo test --test test_llm_connection test_llm_factory -- --nocapture 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Fail "LLM 连接失败。请检查 .env 中的 API 密钥配置。"
    Write-Info "输出: $($llmCheck | Select-Object -Last 5)"
    exit 1
}
Write-Pass "LLM 连接正常"

# ── 测试执行函数 ─────────────────────────────────────────────
$script:TotalPassed = 0
$script:TotalFailed = 0
$script:TotalSkipped = 0
$script:AllResults = @()

function Invoke-AgentTest {
    param(
        [string]$TestName,
        [string]$DisplayName
    )

    Write-Header "§ $DisplayName"

    $outputFile = "$OutputDir/test_${TestName}_$(Get-Date -Format 'yyyyMMdd_HHmmss').jsonl"
    $logFile = "$OutputDir/test_${TestName}.log"

    Write-Info "运行: cargo run --bin test_agents -- $TestName"
    Write-Info "结果: $outputFile"
    Write-Info "日志: $logFile"

    # 运行测试，捕获 stderr 和 stdout
    $result = cargo run --bin test_agents -- $TestName 2>&1

    # 分离 stdout (NDJSON) 和 stderr (日志)
    $ndjsonLines = $result | Where-Object { $_ -match '^\{"test":' }
    $logLines = $result | Where-Object { $_ -notmatch '^\{"test":' }

    # 保存日志
    $logLines | Out-File -FilePath $logFile -Encoding utf8

    # 保存 NDJSON 结果
    $ndjsonLines | Out-File -FilePath $outputFile -Encoding utf8

    # 解析结果
    $passCount = 0
    $failCount = 0
    $skipCount = 0

    foreach ($line in $ndjsonLines) {
        try {
            $check = $line | ConvertFrom-Json
            $script:AllResults += $check

            switch ($check.status) {
                "PASS" {
                    Write-Pass "$($check.check): $($check.detail)"
                    $passCount++
                }
                "FAIL" {
                    Write-Fail "$($check.check): $($check.detail)"
                    $failCount++
                }
                "SKIP" {
                    Write-Skip "$($check.check): $($check.detail)"
                    $skipCount++
                }
            }
        } catch {
            Write-Fail "无法解析 NDJSON 行: $_"
        }
    }

    # 如果 cargo 本身失败了
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "测试进程退出码 = $LASTEXITCODE"
        Write-Info "最后 10 行日志:"
        $logLines | Select-Object -Last 10 | ForEach-Object { Write-Info $_ }
    }

    $total = $passCount + $failCount + $skipCount
    $passRate = if ($total -gt 0) { [math]::Round($passCount / $total * 100) } else { 0 }

    Write-Host "  ─────────────────────────────────" -ForegroundColor DarkGray
    Write-Host "  $DisplayName : $total checks | $passCount PASS | $failCount FAIL | $skipCount SKIP | ${passRate}% pass" -ForegroundColor $(if ($failCount -eq 0) { "Green" } else { "Yellow" })

    $script:TotalPassed += $passCount
    $script:TotalFailed += $failCount
    $script:TotalSkipped += $skipCount

    return @{
        Passed = $passCount
        Failed = $failCount
        Skipped = $skipCount
    }
}

# ── 运行测试 ─────────────────────────────────────────────────

$testsToRun = @()

if ($Test -eq "all") {
    $testsToRun = @(
        @{Name="bus";       Display="§8  双通道协同"},
        @{Name="memory";    Display="§9  分层记忆"},
        @{Name="execute";   Display="§10.3 并行Agent隔离"},
        @{Name="legal";     Display="§10.5 LEGAL VERIFY"},
        @{Name="blindspot"; Display="§10.6 BLINDSPOT"},
        @{Name="debate";    Display="§10.7 DEBATE"},
        @{Name="dynamic";   Display="§11  动态Agent闭环"}
    )
    if (-not $SkipFault) {
        $testsToRun += @{Name="fault"; Display="§12/13 故障边界"}
    }
} else {
    $names = $Test -split ',' | ForEach-Object { $_.Trim() }
    $displayMap = @{
        "bus"       = "§8  双通道协同"
        "memory"    = "§9  分层记忆"
        "execute"   = "§10.3 并行Agent隔离"
        "legal"     = "§10.5 LEGAL VERIFY"
        "blindspot" = "§10.6 BLINDSPOT"
        "debate"    = "§10.7 DEBATE"
        "dynamic"   = "§11  动态Agent闭环"
        "fault"     = "§12/13 故障边界"
    }
    foreach ($name in $names) {
        if ($displayMap.ContainsKey($name)) {
            $testsToRun += @{Name=$name; Display=$displayMap[$name]}
        } else {
            Write-Fail "未知测试: $name"
            Write-Info "可用测试: $($displayMap.Keys -join ', ')"
            exit 1
        }
    }
}

$results = @{}
foreach ($test in $testsToRun) {
    $results[$test.Name] = Invoke-AgentTest -TestName $test.Name -DisplayName $test.Display
}

# ── 汇总报告 ─────────────────────────────────────────────────
$elapsed = (Get-Date) - $script:StartTime
$total = $script:TotalPassed + $script:TotalFailed + $script:TotalSkipped
$overallRate = if ($total -gt 0) { [math]::Round($script:TotalPassed / $total * 100) } else { 0 }

Write-Header "汇总报告"
Write-Host "  耗时: $($elapsed.ToString('mm\:ss'))"
Write-Host "  总计: $total checks"
Write-Host "  PASS: $script:TotalPassed" -ForegroundColor Green
Write-Host "  FAIL: $script:TotalFailed" -ForegroundColor $(if ($script:TotalFailed -gt 0) { "Red" } else { "Gray" })
Write-Host "  SKIP: $script:TotalSkipped" -ForegroundColor $(if ($script:TotalSkipped -gt 0) { "Yellow" } else { "Gray" })
Write-Host "  通过率: ${overallRate}%" -ForegroundColor $(if ($overallRate -ge 80) { "Green" } elseif ($overallRate -ge 50) { "Yellow" } else { "Red" })
Write-Host ""

# 输出按测试分组的 FAIL 详情
if ($script:TotalFailed -gt 0) {
    Write-Host "FAIL 详情:" -ForegroundColor Red
    foreach ($check in $script:AllResults) {
        if ($check.status -eq "FAIL") {
            Write-Host "  [$($check.test)] $($check.check): $($check.detail)" -ForegroundColor Red
        }
    }
}

# 保存汇总 JSON
$summaryFile = "$OutputDir/summary_$(Get-Date -Format 'yyyyMMdd_HHmmss').json"
$summary = @{
    timestamp = (Get-Date -Format 'yyyy-MM-ddTHH:mm:sszzz')
    elapsed = $elapsed.ToString()
    total = $total
    passed = $script:TotalPassed
    failed = $script:TotalFailed
    skipped = $script:TotalSkipped
    pass_rate = $overallRate
    tests = foreach ($test in $testsToRun) {
        $r = $results[$test.Name]
        @{
            name = $test.Name
            display = $test.Display
            passed = $r.Passed
            failed = $r.Failed
            skipped = $r.Skipped
        }
    }
}
$summary | ConvertTo-Json -Depth 3 | Out-File -FilePath $summaryFile -Encoding utf8
Write-Info "汇总已保存: $summaryFile"

# 退出码
if ($script:TotalFailed -gt 0) {
    exit 1
} else {
    Write-Host "✅ 全部测试通过！" -ForegroundColor Green
    exit 0
}
