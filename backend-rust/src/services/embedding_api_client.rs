//! 远程 Embedding API 客户端 — 通义千问 text-embedding-v4 (DashScope)
//!
//! 使用 `ureq`（纯同步 HTTP，无 async runtime）封装 text-embedding-v4 的 HTTP 调用，
//! 包含自动分批、进度提示。调用侧负责先脱敏再传入。
//!
//! 选择 ureq 而非 reqwest::blocking 的原因：后者内部携带 tokio runtime，
//! 在 `#[tokio::main]` 上下文中 drop 会 panic（嵌套 runtime 冲突）。

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::{Duration, Instant};

/// text-embedding API 客户端（默认 DashScope，通过环境变量切换）。
pub struct EmbeddingApiClient {
    api_key: String,
    model: String,
    api_base: String,
    api_style: String, // "dashscope" 或 "openai"
    agent: ureq::Agent,
}

/// DashScope embedding 响应结构。
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    output: Option<EmbeddingOutput>,
    message: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingOutput {
    embeddings: Vec<EmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingItem {
    text_index: usize,
    embedding: Vec<f32>,
}

/// OpenAI 兼容格式的响应结构（`EMBED_API_STYLE=openai` 时使用）。
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingItem>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingItem {
    index: usize,
    embedding: Vec<f32>,
}

impl EmbeddingApiClient {
    /// 从环境变量创建客户端。
    ///
    /// API 密钥读取顺序：`DASHSCOPE_API_KEY` → `OPENAI_API_KEY`（回退）
    /// 这样 Chat 和 Embedding 共用一个 API 密钥时无需重复设置。
    ///
    /// 可选变量：
    ///   `EMBED_MODEL`     — 模型名（默认 `text-embedding-v4`）
    ///   `EMBED_API_BASE`  — API 地址（默认 DashScope）
    ///   `EMBED_API_STYLE` — 请求格式：`dashscope`（默认）或 `openai`
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("DASHSCOPE_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .context(
                "远程 Embedding 模式需要 API 密钥。请设置 DASHSCOPE_API_KEY 或 OPENAI_API_KEY",
            )?;
        let model =
            std::env::var("EMBED_MODEL").unwrap_or_else(|_| "text-embedding-v4".to_string());
        let api_base = std::env::var("EMBED_API_BASE").unwrap_or_else(|_| {
            "https://dashscope.aliyuncs.com/api/v1/services/embeddings/text-embedding/text-embedding"
                .to_string()
        });
        let api_style =
            std::env::var("EMBED_API_STYLE").unwrap_or_else(|_| "dashscope".to_string());
        // http_status_as_error(false): 保留 4xx/5xx 响应体以便读取错误详情
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(120)))
                .http_status_as_error(false)
                .build(),
        );
        Ok(Self {
            api_key,
            model,
            api_base,
            api_style,
            agent,
        })
    }

    /// 批量编码文本（自适应分批：从 10 条起，遇到 batch size 超限自动减半重试）。
    ///
    /// DashScope text-embedding-v4 的批量上限不固定，取决于每段文本的 token 数总和；
    /// 单次请求上限约 2048 tokens，所以长文本会导致单批条数限制进一步降低。
    ///
    /// 返回的向量**未**做 L2 归一化（由 `DocumentVectorIndex::new` 统一处理）。
    pub fn encode_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let total = texts.len();
        let mut all_embs: Vec<Vec<f32>> = Vec::with_capacity(total);
        let start = Instant::now();
        let mut processed: usize = 0;

        // 从 10 条/批开始，分批处理；每批内部遇到 batch size 超限会自动递归减半
        const INITIAL_BATCH: usize = 10;
        for chunk in texts.chunks(INITIAL_BATCH) {
            let batch_embs = self.encode_chunk_adaptive(chunk)?;
            all_embs.extend(batch_embs);
            processed += chunk.len();

            let elapsed = start.elapsed().as_secs_f64();
            let progress = processed as f64 / total as f64;
            let eta = if progress > 0.0 {
                elapsed / progress * (1.0 - progress)
            } else {
                0.0
            };
            println!(
                "  API 编码: {}/{} chunks, 已耗时 {:.1}s, 预计剩余 {:.1}s",
                processed, total, elapsed, eta
            );
        }

        println!(
            "  API 编码完成: {} 条向量, 维度 {}, 总耗时 {:.1}s",
            all_embs.len(),
            all_embs.first().map(|v| v.len()).unwrap_or(0),
            start.elapsed().as_secs_f64()
        );
        Ok(all_embs)
    }

    /// 自适应编码一块文本：先尝试整块调用 API，若报 batch size 超限则二分递归。
    fn encode_chunk_adaptive(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let batch_texts: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        match self.call_api(&batch_texts) {
            Ok(response) => {
                let mut items: Vec<(usize, Vec<f32>)> = response
                    .into_iter()
                    .map(|item| (item.text_index, item.embedding))
                    .collect();
                items.sort_by_key(|(idx, _)| *idx);
                Ok(items.into_iter().map(|(_, emb)| emb).collect())
            }
            Err(e) => {
                let msg = format!("{}", e);
                // 400 且提到 batch size 超限 → 二分重试
                if msg.contains("batch size") || msg.contains("larger than") {
                    if texts.len() == 1 {
                        // 单条文本仍然超限（token 数超出模型上限），无法继续拆分
                        return Err(e);
                    }
                    let mid = texts.len() / 2;
                    let mut left = self.encode_chunk_adaptive(&texts[..mid])?;
                    let right = self.encode_chunk_adaptive(&texts[mid..])?;
                    left.extend(right);
                    Ok(left)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// 编码单条文本（查询用）。返回 L2 归一化向量。
    pub fn encode_single(&self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.call_api(&[text])?;
        let emb = results.pop().context("API 返回空结果")?.embedding;
        Ok(l2_normalize(emb))
    }

    /// 调用 Embedding API（底层 HTTP POST，根据 api_style 切换请求/响应格式）。
    fn call_api(&self, texts: &[&str]) -> Result<Vec<EmbeddingItem>> {
        let url = &self.api_base;

        // 构建请求体（DashScope vs OpenAI 格式）
        let body = match self.api_style.as_str() {
            "openai" => serde_json::json!({
                "model": self.model,
                "input": texts,
            }),
            _ => serde_json::json!({
                "model": self.model,
                "input": {
                    "texts": texts,
                },
                "parameters": {
                    "dimension": 1024
                }
            }),
        };

        let resp = self
            .agent
            .post(url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| map_ureq_error(e, url))?;

        // 手动检查 HTTP 状态码（已关闭 http_status_as_error 自动转换）
        let status = resp.status();
        if !status.is_success() {
            let body = resp.into_body().read_to_string().unwrap_or_default();
            anyhow::bail!(
                "Embedding API 返回错误 (HTTP {}): {}",
                status.as_u16(),
                body.chars().take(500).collect::<String>()
            );
        }

        // 解析响应（DashScope vs OpenAI 格式）
        match self.api_style.as_str() {
            "openai" => {
                let parsed: OpenAIEmbeddingResponse = resp
                    .into_body()
                    .read_json()
                    .context("解析 OpenAI 格式响应失败")?;
                Ok(parsed
                    .data
                    .into_iter()
                    .map(|item| EmbeddingItem {
                        text_index: item.index,
                        embedding: item.embedding,
                    })
                    .collect())
            }
            _ => {
                let emb_resp: EmbeddingResponse = resp
                    .into_body()
                    .read_json()
                    .context("解析 DashScope 格式响应失败")?;
                if let Some(output) = emb_resp.output {
                    Ok(output.embeddings)
                } else {
                    anyhow::bail!(
                        "API 业务错误: {} (code: {:?})",
                        emb_resp.message.unwrap_or_default(),
                        emb_resp.code
                    );
                }
            }
        }
    }
}

/// 将 ureq 错误转为 anyhow 错误，HTTP 错误时截取响应体前 500 字符。
fn map_ureq_error(e: ureq::Error, url: &str) -> anyhow::Error {
    use ureq::Error;
    match e {
        Error::StatusCode(code) => {
            anyhow::anyhow!("Embedding API 返回 HTTP {}", code)
        }
        _ => {
            anyhow::anyhow!("HTTP 请求失败: {} (url: {})", e, url)
        }
    }
}

/// L2 归一化（原地修改）。
fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}
