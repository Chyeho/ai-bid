//! 组长：Neo4j 访问层。
//!
//! 环境变量：`NEO4J_URI` / `NEO4J_USER` / `NEO4J_PASSWORD`。

use std::collections::HashSet;

use anyhow::Result;
use neo4rs::Graph;

use crate::knowledge::types::{EntityDecision, SearchHit};

/// Neo4j 连接封装。
pub struct Neo4jClient {
    graph: Graph,
}

impl Neo4jClient {
    /// 连接 Neo4j，参数从环境变量读取。
    pub async fn connect() -> Result<Self> {
        todo!("组长实现")
    }

    /// 库中已有的所有 law_id 集合（查重数据源）。
    pub async fn all_law_ids(&self) -> Result<HashSet<String>> {
        todo!("组长实现")
    }

    /// 写入"新"实体（`decision == Exists` 跳过）。
    pub async fn write(&self, decisions: Vec<EntityDecision>) -> Result<()> {
        todo!("组长实现")
    }

    /// 关键词查询风险及关联的法律/条款。
    pub async fn search(&self, query: &str) -> Result<Vec<SearchHit>> {
        todo!("组长实现")
    }
}
