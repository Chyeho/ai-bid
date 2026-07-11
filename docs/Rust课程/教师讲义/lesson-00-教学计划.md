# 第0课教学计划：环境搭建

> 不占用正式课时。W1 之前 3 天发布，学员自行完成。只设一个答疑时间。

---

## 教学目标

每个人的电脑能 `cargo run` 出 Hello World。Windows 用户搞定 MSVC 链接器。

---

## 时间安排

```
W1 前 3 天   发布 lesson-00-setup.md + 环境检查脚本
W1 前 2 天   Windows 用户集中踩坑（link.exe 问题）
W1 前 1 天   学员在群里发验证截图
W1 当天      讲师检查全员环境 + 10min 答疑
```

> 不给正式课时。环境搭建是自学内容。只设定截止时间。

---

## 你需要准备的

### 环境检查脚本（PowerShell）

发给学员，让他们跑完贴输出到群里：

```powershell
Write-Host "=== Rust 环境检查 ===" -ForegroundColor Cyan

# 1. rustc
try {
    $v = rustc --version
    Write-Host "[OK] $v" -ForegroundColor Green
} catch {
    Write-Host "[FAIL] rustc 未安装或不在 PATH 中" -ForegroundColor Red
}

# 2. cargo
try {
    $v = cargo --version
    Write-Host "[OK] $v" -ForegroundColor Green
} catch {
    Write-Host "[FAIL] cargo 未安装" -ForegroundColor Red
}

# 3. clippy
try {
    $v = cargo clippy --version
    Write-Host "[OK] $v" -ForegroundColor Green
} catch {
    Write-Host "[WARN] clippy 未安装，运行: rustup component add clippy" -ForegroundColor Yellow
}

# 4. 试编译
$tmp = "rust_check_temp"
try {
    cargo new $tmp 2>$null | Out-Null
    Set-Location $tmp
    cargo check 2>$null | Out-Null
    Write-Host "[OK] cargo check 通过" -ForegroundColor Green
    Set-Location ..
    Remove-Item -Recurse -Force $tmp
} catch {
    Write-Host "[FAIL] cargo check 失败，检查 VS Build Tools" -ForegroundColor Red
}

# 5. link.exe (Windows)
if ($env:OS -match "Windows") {
    $link = Get-Command link.exe -ErrorAction SilentlyContinue
    if ($link) {
        Write-Host "[OK] link.exe: $($link.Source)" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] link.exe 找不到，需要安装 VS Build Tools (C++ 桌面开发)" -ForegroundColor Red
    }
}

Write-Host "`n如果全部 [OK]，你的环境就绪了。" -ForegroundColor Cyan
```

### 常见坑预案

| 坑 | 判断 | 解决 |
|---|---|---|
| link.exe 找不到 | `where link.exe` 无结果 | 装 VS Build Tools，勾选"使用 C++ 的桌面开发" |
| 装了 VS 但还是找不到 link.exe | 只装了 VS 没装 C++ 工作负载 | 打开 Visual Studio Installer → 修改 → 勾选"使用 C++ 的桌面开发" |
| rustup 下载慢 | 看进度条 | 耐心等，有 CDN |
| rust-analyzer 不启动 | VSCode 右下角没有 rust-analyzer 图标 | `cargo check` 先确认项目能编译；重载 VSCode 窗口 |

---

## 开课前的检查

W1 上课前 1 小时，扫一遍群里的验证截图。如果有人还没通过，1v1 私聊解决。**绝不让任何一个学员因为环境问题在第一课就掉队。**
