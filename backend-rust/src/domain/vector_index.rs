//! 单文档向量索引数据模型
//!
//! 本模块定义了 [`DocumentVectorIndex`] — 一份标书所有 Chunk 的
//! 内存级向量索引。用于 Agent 的 `search_document` 工具进行语义搜索。
//!
//! ## 设计原则
//!
//! - **内存暴力 KNN**：200 chunks × 1024d ≈ 0.8 MB，< 0.5ms
//! - **L2 归一化**：存储时归一化 → 搜索时内积 = 余弦相似度
//! - **可序列化**：支持 JSON 元数据 + 二进制向量文件，避免重复 embed
//!
//! ## 与 search_knowledge 的区别
//!
//! `search_document` 只搜当前这一份标书（~100-200 chunks），内存足够。
//! `search_knowledge` 搜的是跨文档外部知识库（数万条法规+案例），那才需要 Qdrant。

use serde::{Deserialize, Serialize};

/// 单文档向量索引 — 内存级，暴力 KNN。
///
/// 规模: ~100-200 chunks × 1024d (BGE-M3) ≈ 0.8 MB
/// 搜索: 暴力点积 < 0.5 ms
#[derive(Debug, Clone)]
pub struct DocumentVectorIndex {
    /// 文档标识（与 RawDocument.document_id 一致）
    pub document_id: String,
    /// Chunk 元数据列表
    pub chunks: Vec<ChunkMeta>,
    /// N × D 矩阵，每行已 L2 归一化 (单位向量)
    pub embeddings: Vec<Vec<f32>>,
}

/// 存入向量索引的 Chunk 元数据（轻量版，不含完整 text）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// Chunk 唯一标识，格式 `"ch_042"`
    pub chunk_id: String,
    /// 从根章节到当前节点的标题链
    pub section_path: Vec<String>,
    /// embed_text() 的输出（已携带层级前缀），用于搜索时返回摘要
    pub embed_text: String,
    /// chunk 文本长度（字符数），用于统计
    pub text_len: usize,
    /// 起始页码 (0-based)
    pub page_start: usize,
    /// 结束页码 (0-based)
    pub page_end: usize,
}

/// 单次搜索结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// 匹配到的 Chunk ID
    pub chunk_id: String,
    /// 最后一级章节标题（便于展示上下文）
    pub title: String,
    /// 余弦相似度 [0, 1]，值越大越相似
    pub score: f32,
    /// 前 200 字摘要，避免直接返回大段原文
    pub snippet: String,
    /// 起始页码
    pub page_start: usize,
}

impl DocumentVectorIndex {
    /// 从已 L2 归一化的 dense embeddings 构建索引。
    ///
    /// `embeddings` 应已通过 embedding API 生成（BGE-M3 1024d）。
    /// 如果未归一化，内部会执行 L2 归一化。
    pub fn new(chunks: Vec<ChunkMeta>, embeddings: Vec<Vec<f32>>) -> Self {
        let embeddings: Vec<Vec<f32>> = embeddings
            .into_iter()
            .map(|v| {
                let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    v.into_iter().map(|x| x / norm).collect()
                } else {
                    v
                }
            })
            .collect();

        Self {
            document_id: String::new(),
            chunks,
            embeddings,
        }
    }

    /// 语义搜索，返回 Top-K 最相似的 chunk。
    ///
    /// `query_embedding` 必须来自同一 embedding 模型（BGE-M3 1024d），
    /// 且**已 L2 归一化**（调用方负责）。
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<SearchHit> {
        // L2 归一化 query（防御性，如果调用方已归一化则几乎无开销）
        let norm: f32 = query_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let query: Vec<f32> = if norm > 0.0 {
            query_embedding.iter().map(|x| x / norm).collect()
        } else {
            query_embedding.to_vec()
        };

        // 暴力余弦相似度 — 对于 200 条数据 < 0.5ms
        let mut scored: Vec<(usize, f32)> = self
            .embeddings
            .iter()
            .enumerate()
            .map(|(i, emb)| {
                let score: f32 = emb.iter().zip(&query).map(|(a, b)| a * b).sum();
                (i, score)
            })
            .collect();

        // Top-K by partial sort
        scored.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        scored
            .into_iter()
            .map(|(i, score)| {
                let chunk = &self.chunks[i];
                SearchHit {
                    chunk_id: chunk.chunk_id.clone(),
                    title: chunk.section_path.last().cloned().unwrap_or_default(),
                    score,
                    snippet: chunk.embed_text.chars().take(500).collect(),
                    page_start: chunk.page_start,
                }
            })
            .collect()
    }

    /// 返回索引中的 chunk 数量。
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// 索引是否为空。
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

// ─── 序列化 ──────────────────────────────────────────────────

/// 序列化元数据（JSON 格式）。
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexMeta {
    pub document_id: String,
    pub chunk_count: usize,
    pub dimension: usize,
    pub chunks: Vec<ChunkMeta>,
}

impl DocumentVectorIndex {
    /// 导出元数据（供序列化）。
    pub fn to_meta(&self) -> IndexMeta {
        IndexMeta {
            document_id: self.document_id.clone(),
            chunk_count: self.chunks.len(),
            dimension: self.embeddings.first().map(|v| v.len()).unwrap_or(0),
            chunks: self.chunks.clone(),
        }
    }
}
