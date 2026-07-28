param(
    [ValidateSet("train", "dev", "test")]
    [string]$Split = "test",

    [ValidateSet("injected", "full")]
    [string]$Scope = "injected",

    [string]$RunId = "",

    [int]$Limit = 0,

    [string]$Documents = "",

    [ValidateSet("off", "low")]
    [string]$DesensitizeMode = "low",

    [string]$DatasetRoot = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$rustRoot = Join-Path $repoRoot "backend-rust"
$python = "C:\Users\Administrator\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe"
$env:AI_BID_PYTHON_EXECUTABLE = $python
$server = Join-Path $rustRoot "target\debug\server.exe"
$serverJob = $null

function Test-EngineHealth {
    try {
        $health = Invoke-RestMethod -Uri "http://127.0.0.1:3001/health" -TimeoutSec 3
        return $health.status -eq "ok"
    }
    catch {
        return $false
    }
}

try {
    if (-not (Test-Path -LiteralPath $server)) {
        Push-Location $rustRoot
        try {
            cargo build --bin server
        }
        finally {
            Pop-Location
        }
    }

    if (-not (Test-EngineHealth)) {
        $logDir = Join-Path $PSScriptRoot "results\server"
        New-Item -ItemType Directory -Force -Path $logDir | Out-Null
        $serverJob = Start-Job -ScriptBlock {
            param($ServerPath, $WorkingDirectory)
            Set-Location -LiteralPath $WorkingDirectory
            & $ServerPath
        } -ArgumentList $server, $rustRoot

        $deadline = (Get-Date).AddMinutes(3)
        while (-not (Test-EngineHealth)) {
            if ($serverJob.State -in @("Failed", "Completed", "Stopped")) {
                throw "审核引擎启动失败，请查看 benchmark/results/server/stderr.log"
            }
            if ((Get-Date) -gt $deadline) {
                throw "审核引擎启动超时"
            }
            Start-Sleep -Seconds 2
        }
    }

    $arguments = @(
        (Join-Path $PSScriptRoot "run_benchmark.py"),
        "--split", $Split,
        "--scope", $Scope,
        "--desensitize-mode", $DesensitizeMode
    )
    if ($DatasetRoot) {
        $arguments += @("--dataset-root", $DatasetRoot)
    }
    if ($RunId) {
        $arguments += @("--run-id", $RunId)
    }
    if ($Limit -gt 0) {
        $arguments += @("--limit", "$Limit")
    }
    if ($Documents) {
        $arguments += @("--documents", $Documents)
    }

    & $python @arguments
    exit $LASTEXITCODE
}
finally {
    if ($null -ne $serverJob) {
        $jobOutput = Receive-Job -Job $serverJob -ErrorAction SilentlyContinue 2>&1
        $jobOutput | Out-File -FilePath (Join-Path $PSScriptRoot "results\server\combined.log") -Encoding utf8
        Stop-Job -Job $serverJob -ErrorAction SilentlyContinue
        Remove-Job -Job $serverJob -Force -ErrorAction SilentlyContinue
    }
}
