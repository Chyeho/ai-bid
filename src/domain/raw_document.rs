//! 原始文档数据模型
//!
//! 本模块定义了从投标文件（PDF、Word 等）解析后得到的中间数据结构。
//! 这些结构体是纯数据载体（DTO），不含业务逻辑，通过 serde 支持 JSON 序列化与反序列化，
//! 用于在解析引擎与后续的语义分析、结构化提取等模块之间传递数据。

use serde::{Deserialize, Serialize};

/// 一份完整的原始文档。
///
/// 对应一个投标文件解析后的全部内容，由多个页面组成。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawDocument {
    /// 文档唯一标识符（如文件名哈希或 UUID）
    pub document_id: String,
    /// 源文件在磁盘上的路径（如 `./bids/xxx.pdf`）
    pub source_path: String,
    /// 文档包含的所有页面，按页码顺序排列
    pub pages: Vec<RawPage>,
}

/// 文档中的单个页面。
///
/// 包含页面尺寸、全文文本以及从该页提取的各类排版元素：
/// 单词、表格、线段、矩形区域等。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawPage {
    /// 页码索引，从 0 开始
    pub page_index: usize,
    /// 页面宽度（单位：磅 pt）
    pub width: f64,
    /// 页面高度（单位：磅 pt）
    pub height: f64,
    /// 本页的纯文本内容（按阅读顺序拼接）
    pub text: String,
    /// 本页所有单词及其包围盒（用于定位和高亮）
    pub words: Vec<RawWord>,
    /// 本页解析出的表格
    pub tables: Vec<RawTable>,
    /// 本页的线条元素（如下划线、分隔线等）
    pub lines: Vec<RawLine>,
    /// 本页的矩形区域（如图片占位框、色块、文本框边界等）
    pub rects: Vec<RawRect>,
}

/// 一个单词及其在页面上的位置。
///
/// 保留位置信息可用于：
/// - 关键词搜索高亮
/// - 坐标敏感的内容提取（如"右上角的公司名称"）
#[derive(Debug, Serialize, Deserialize)]
pub struct RawWord {
    /// 单词文本
    pub text: String,
    /// 单词的包围盒，定位其在页面上的矩形区域
    pub bbox: BBox,
}

/// 页面中解析出的表格。
///
/// 表格以二维网格表示，外层 Vec 为行，内层 Vec 为单元格。
/// 单元格类型为 `Option<String>`，`None` 表示该单元格为空或不存在（合并单元格场景）。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawTable {
    /// 表格行集合，`rows[row_index][col_index]` 定位单元格
    pub rows: Vec<Vec<Option<String>>>,
}

/// 页面中的线段元素。
///
/// 常用于识别下划线、删除线、表格边框线、分隔线等排版线索。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawLine {
    /// 线段的包围盒（通常宽度或高度极小，呈线状）
    pub bbox: BBox,
}

/// 页面中的矩形区域。
///
/// 常用于识别图片占位框、色块填充区、文本框边界等闭合矩形元素。
#[derive(Debug, Serialize, Deserialize)]
pub struct RawRect {
    /// 矩形的包围盒
    pub bbox: BBox,
}

/// 包围盒（Bounding Box）—— 描述一个轴对齐的矩形区域。
///
/// 坐标系原点为页面左上角，X 轴向右，Y 轴向下（与 PDF 坐标系一致）。
#[derive(Debug, Serialize, Deserialize)]
pub struct BBox {
    /// 矩形左上角的 X 坐标
    pub x0: f64,
    /// 矩形上边界的 Y 坐标（距页面顶部的距离）
    pub top: f64,
    /// 矩形右下角的 X 坐标
    pub x1: f64,
    /// 矩形下边界的 Y 坐标（距页面顶部的距离）
    pub bottom: f64,
}
