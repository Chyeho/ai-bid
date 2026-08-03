//! 组长：写库调试入口 — 读 entities_decisions.jsonl → 写入 Neo4j。
//!
//! 用法: cargo run --bin graph_write <entities_decisions.jsonl>
//!
//! ⚠️ 这只是独立调试用。8/4 整合后改走 `knowledge::run::run`，输入变为内存 Vec。

use std::fs;

use anyhow::{Context, Result};

use ai_bid::knowledge::graph::Neo4jClient;
use ai_bid::knowledge::types::{Decision, EntityDecision};

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("用法: cargo run --bin graph_write <entities_decisions.jsonl>")?;

    let decisions = read_jsonl(&path)?;
    let new_count = decisions
        .iter()
        .filter(|d| d.decision == Decision::New)
        .count();
    let exists_count = decisions.len() - new_count;

    let client = Neo4jClient::connect().await?;
    client.write(decisions).await?;

    println!("写入 Neo4j 完成：new={}, exists(跳过)={}", new_count, exists_count);
    Ok(())
}

fn read_jsonl(path: &str) -> Result<Vec<EntityDecision>> {
    let text = fs::read_to_string(path).with_context(|| format!("读取 {}", path))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).with_context(|| format!("解析 {} 失败", path)))
        .collect()
}
