@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0run_acceptance.ps1" -Split test -Scope injected
echo.
echo 验收已结束，报告位于 benchmark\results 目录。
pause
