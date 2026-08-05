# SearXNG Docker 一键启动（Windows PowerShell）
# 自动挂载 searxng_settings.yml 配置

$settingsPath = Join-Path $PSScriptRoot "searxng_settings.yml"

if (-not (Test-Path $settingsPath)) {
    Write-Host "❌ 找不到 $settingsPath" -ForegroundColor Red
    exit 1
}

Write-Host "🚀 启动 SearXNG..." -ForegroundColor Cyan
Write-Host "   配置: $settingsPath" -ForegroundColor Gray
Write-Host "   端口: 8080" -ForegroundColor Gray

# 如果已有同名容器在运行，先停止
$existing = docker ps -q -f "name=ai-bid-searxng"
if ($existing) {
    Write-Host "⏳ 停止已有容器..." -ForegroundColor Yellow
    docker stop ai-bid-searxng | Out-Null
    docker rm ai-bid-searxng | Out-Null
}

docker run -d `
  --name ai-bid-searxng `
  -p 8080:8080 `
  -v "${settingsPath}:/etc/searxng/settings.yml:ro" `
  searxng/searxng

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ SearXNG 已启动: http://localhost:8080" -ForegroundColor Green
    Write-Host "   验证: curl http://localhost:8080/search?q=test&format=json" -ForegroundColor Gray
} else {
    Write-Host "❌ 启动失败" -ForegroundColor Red
}
