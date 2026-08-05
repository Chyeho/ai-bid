//! 统一路径解析模块
//!
//! 所有文件系统路径通过此模块解析，便于 monorepo 结构下通过环境变量
//! `AIBID_DATA_DIR` 统一控制数据目录位置。
//!
//! ## 使用方式
//!
//! ```ignore
//! use ai_bid::paths::data_path;
//!
//! let raw_dir = data_path("output/raw_json");
//! let agents_file = data_path("agents/dynamic_agents.json");
//! ```
//!
//! ## 环境变量
//!
//! - `AIBID_DATA_DIR` — 数据根目录，默认为当前工作目录 `.`
//!
//! monorepo 结构中，若将 Rust 代码放在 `backend-rust/` 子目录，
//! 设置 `AIBID_DATA_DIR=..` 即可将所有路径指向项目根目录。

use std::path::PathBuf;

/// 返回数据根目录。
///
/// 由环境变量 `AIBID_DATA_DIR` 控制，默认值为 `"."`（当前工作目录）。
pub fn data_dir() -> PathBuf {
    std::env::var("AIBID_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// 将相对路径拼接到数据根目录。
///
/// 等价于 `data_dir().join(relative)`。
pub fn data_path<P: AsRef<std::path::Path>>(relative: P) -> PathBuf {
    data_dir().join(relative)
}

/// 将相对路径拼接到数据根目录，返回字符串。
///
/// 用于需要 `String` 的场景（如 `format!` 宏参数）。
pub fn data_path_str(relative: &str) -> String {
    data_dir().join(relative).to_string_lossy().to_string()
}
