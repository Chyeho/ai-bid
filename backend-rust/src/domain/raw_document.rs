//! 原始文档数据模型
//!
//! 本模块定义了从投标文件（PDF、Word 等）解析后得到的中间数据结构。
//! 这些结构体是纯数据载体（DTO），不含业务逻辑，通过 serde 支持 JSON 序列化与反序列化，
//! 用于在解析引擎与后续的语义分析、结构化提取等模块之间传递数据。
//!
//! ## 回溯高亮设计
//!
//! 每个可定位元素（单词、表格、段落块）都带有唯一 `id` 和 `bbox` 坐标。
//! 下游 LLM 审核时引用 ID，渲染层通过 ID 查表获取坐标进行高亮：
//!
//! ```text
//! LLM 输出: "段落 b_2_5 中的'信用中国'引用过时"
//!                          │
//!              RawBlock.id = "b_2_5"
//!              RawBlock.bbox → {x0, top, x1, bottom} → PDF 高亮
//! ```
//!
//! ID 命名规则:
//! - 单词: `"w_{页码}_{序号}"`    如 `"w_3_15"`
//! - 表格: `"t_{页码}_{序号}"`    如 `"t_2_0"`
//! - 段落: `"b_{页码}_{序号}"`    如 `"b_5_3"`

use serde::{Deserialize, Serialize};

/// 一份完整的原始文档。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawDocument {
    /// 文档唯一标识符（UUID）
    pub document_id: String,
    /// 源文件在磁盘上的路径
    pub source_path: String,
    /// 文档包含的所有页面
    pub pages: Vec<RawPage>,
}

/// 文档中的单个页面。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawPage {
    /// 页码索引，从 0 开始
    pub page_index: usize,
    /// 页面宽度（单位：磅 pt）
    pub width: f64,
    /// 页面高度（单位：磅 pt）
    pub height: f64,
    /// 本页的纯文本内容（按阅读顺序拼接，已清洗）
    pub text: String,
    /// 本页所有单词及其包围盒（用于关键词定位和高亮）
    pub words: Vec<RawWord>,
    /// 本页的语义段落块（用于段落级回溯高亮）
    pub blocks: Vec<RawBlock>,
    /// 本页解析出的表格
    pub tables: Vec<RawTable>,
    /// 本页的线条元素（下划线、分隔线等）
    pub lines: Vec<RawLine>,
    /// 本页的矩形区域（图片占位框、色块、文本框边界等）
    pub rects: Vec<RawRect>,
}

// ─── 文本元素 ───────────────────────────────────────────────

/// 一个单词及其在页面上的位置。
///
/// 每个单词都有唯一 ID，可供 LLM 引用后回溯高亮。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawWord {
    /// 唯一标识，格式 `"w_{页码}_{序号}"`，如 `"w_3_15"`
    pub id: String,
    /// 单词文本
    pub text: String,
    /// 单词的包围盒
    pub bbox: BBox,
}

/// 语义段落块。
///
/// 从 words 按行间距自动聚合而成。每个块代表一个逻辑段落、
/// 标题或独立文本行，附带完整包围盒，用于段落级回溯高亮。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawBlock {
    /// 唯一标识，格式 `"b_{页码}_{序号}"`，如 `"b_5_3"`
    pub id: String,
    /// 块类型
    #[serde(rename = "type")]
    pub block_type: BlockType,
    /// 块的完整文本
    pub text: String,
    /// 块的包围盒（覆盖块内所有单词的最小矩形）
    pub bbox: BBox,
}

/// 块类型枚举。
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    /// 标题（字号大 / 加粗 / 居中）
    Heading,
    /// 正文段落
    Paragraph,
    /// 列表项
    ListItem,
    /// 页眉页脚 / 页码
    HeaderFooter,
}

// ─── 表格 ───────────────────────────────────────────────────

/// 页面中解析出的表格。
///
/// 每个表格都有唯一 ID 和整体包围盒，用于表格级回溯高亮。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawTable {
    /// 唯一标识，格式 `"t_{页码}_{序号}"`，如 `"t_2_0"`
    pub id: String,
    /// 表格整体包围盒（覆盖表格全部区域）
    pub bbox: Option<BBox>,
    /// 表格数据，rows[row][col] 定位单元格
    pub rows: Vec<Vec<Option<String>>>,
}

// ─── 图形元素 ───────────────────────────────────────────────

/// 线段元素。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawLine {
    pub bbox: BBox,
}

/// 矩形区域。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawRect {
    pub bbox: BBox,
}

// ─── 基础类型 ───────────────────────────────────────────────

/// 包围盒（Bounding Box）—— 描述一个轴对齐的矩形区域。
///
/// 坐标系原点为页面左上角，X 轴向右，Y 轴向下（与 PDF 坐标系一致）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BBox {
    /// 矩形左上角的 X 坐标
    pub x0: f64,
    /// 矩形上边界的 Y 坐标（距页面顶部的距离）
    pub top: f64,
    /// 矩形右下角的 X 坐标
    pub x1: f64,
    /// 矩形下边界的 Y 坐标
    pub bottom: f64,
}
