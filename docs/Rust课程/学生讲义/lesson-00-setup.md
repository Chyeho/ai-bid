# 第0课：Rust 环境搭建

> 第一件事：让你的电脑能编译 Rust 代码。这节课没有作业，只有检查清单。

---

## 你需要安装什么

### Windows

**第一步：装 Rust**

去 [rustup.rs](https://rustup.rs/) 下载 `rustup-init.exe`，双击运行。全部默认选项，一路回车。

**第二步：装 VS Build Tools（关键！）**

Rust 在 Windows 上编译需要 Microsoft C++ 链接器。如果你没装过 Visual Studio，去 [visualstudio.microsoft.com/zh-hans/downloads/](https://visualstudio.microsoft.com/zh-hans/downloads/) 下载 **Visual Studio Build Tools**。

安装时勾选 **"使用 C++ 的桌面开发"** 工作负载。右侧只保留 **MSVC v143** 和 **Windows 11 SDK**，其余取消勾选。

> 这一步最容易出问题。如果你运行 `cargo build` 时看到 `error: linking with link.exe failed`，说明 VS Build Tools 没装或没勾选 C++ 负载。

### macOS

```bash
# 先装 Xcode Command Line Tools（提供 cc 链接器）
xcode-select --install

# 再装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux (Ubuntu/Debian)

```bash
# 先装编译依赖
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# 再装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## 验证安装

打开 **PowerShell**（Windows）或 **终端**（macOS/Linux），逐条运行：

```powershell
# 1. Rust 版本
rustc --version
# 期望输出：rustc 1.96.0 或更高

# 2. Cargo 版本
cargo --version
# 期望输出：cargo 1.96.0 或更高

# 3. 创建第一个项目
cargo new hello-rust
cd hello-rust
cargo run
# 期望输出：Hello, world!

# 4. 代码检查工具
cargo clippy --version
# 如果没有：rustup component add clippy

# 5. （Windows 专属）确认链接器可用
where link.exe
# 期望输出：C:\Program Files\Microsoft Visual Studio\...\link.exe
```

---

## 配置 VSCode

1. 安装 [VSCode](https://code.visualstudio.com/)
2. 装两个插件：
   - **rust-analyzer**（代码补全、跳转、报错提示）
   - **Even Better TOML**（Cargo.toml 语法高亮）
3. 打开你的 `hello-rust` 项目，打开 `src/main.rs`
4. 你会看到：
   - `main()` 上方有 "Run | Debug" 灰色按钮
   - 变量有类型提示
   - 写错代码时红色波浪线

> rust-analyzer 第一次加载会比较慢（下载 server），等右下角的加载提示消失就行。

---

## 常见问题

| 问题 | 解决 |
|---|---|
| `error: linking with link.exe failed` | 安装 VS Build Tools，勾选"使用 C++ 的桌面开发" |
| `link.exe not found` | 同上 |
| `rustup` 不是命令 | 重启终端，或手动把 `%USERPROFILE%\.cargo\bin` 加到 PATH |
| rust-analyzer 不工作 | 用 `cargo check` 确认项目能编译；重启 VSCode |
| 下载慢 | rustup 默认用官方源，不用换镜像（有 CDN） |
| 不知道 `cargo add` | `cargo add anyhow` 自动加依赖到 Cargo.toml；也可以手动编辑 `[dependencies]` 加 `anyhow = "1"` |

---

## 检查清单

全部通过才算环境就绪：

- [ ] `rustc --version` ≥ 1.96
- [ ] `cargo new test && cd test && cargo run` → 输出 "Hello, world!"
- [ ] `cargo clippy --version` 能运行
- [ ] VSCode 中 `main.rs` 有语法高亮和代码补全
- [ ] 知道 `cargo check`（只检查不编译）和 `cargo build`（完整构建）的区别

---

