//! 组长：Neo4j 访问层。
//!
//! 环境变量：
//!   - `NEO4J_URI`       默认 `bolt://localhost:7687`
//!   - `NEO4J_USER`      默认 `neo4j`
//!   - `NEO4J_PASSWORD`  默认 `b1234567`（演示环境）

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use neo4rs::{query, Graph};

use crate::knowledge::types::{Decision, EntityDecision, LawArticleEntity, RiskEntity, SearchHit};

/// Neo4j 连接封装。
pub struct Neo4jClient {
    graph: Graph,
}

impl Neo4jClient {
    /// 连接 Neo4j，参数从环境变量读取。
    pub async fn connect() -> Result<Self> {
        let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".into());
        let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into());
        let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "b1234567".into());
        let graph = Graph::new(uri.as_str(), user.as_str(), password.as_str())
            .await
            .with_context(|| {
                format!(
                    "无法连接 Neo4j: {}（请先启动容器，并检查 NEO4J_URI / NEO4J_USER / NEO4J_PASSWORD）",
                    uri
                )
            })?;
        Ok(Self { graph })
    }

    /// 库中已有的所有 law_id 集合（查重数据源）。
    pub async fn all_law_ids(&self) -> Result<HashSet<String>> {
        let mut result = self
            .graph
            .execute(query("MATCH (l:Law) RETURN l.law_id AS law_id"))
            .await?;
        let mut ids = HashSet::new();
        while let Some(row) = result.next().await? {
            let id: String = row.get("law_id")?;
            ids.insert(id);
        }
        Ok(ids)
    }

    /// 写入"新"实体（`decision == Exists` 跳过）。全部 MERGE，幂等，可重复执行。
    pub async fn write(&self, decisions: Vec<EntityDecision>) -> Result<()> {
        for d in decisions {
            if d.decision == Decision::Exists {
                continue;
            }
            self.upsert_risk(&d).await?;
            for law in &d.laws {
                self.upsert_law(&d, law).await?;
            }
        }
        Ok(())
    }

    /// upsert Risk 节点，并累加 candidate_id（去重）、更新 snippet。
    async fn upsert_risk(&self, d: &EntityDecision) -> Result<()> {
        let cql = r"
MERGE (r:Risk {risk_id: $risk_id})
ON CREATE SET r.name = $risk_name, r.severity = $severity,
              r.candidate_ids = [$candidate_id], r.snippet = $snippet
ON MATCH  SET r.candidate_ids = [x IN coalesce(r.candidate_ids, []) WHERE x <> $candidate_id] + $candidate_id,
              r.snippet = $snippet";
        self.graph
            .run(
                query(cql)
                    .param("risk_id", d.risk.id.as_str())
                    .param("risk_name", d.risk.name.as_str())
                    .param("severity", d.risk.severity.as_str())
                    .param("candidate_id", d.candidate_id.as_str())
                    .param("snippet", d.snippet.as_str()),
            )
            .await
            .context("写入 Risk 节点失败")?;
        Ok(())
    }

    /// upsert Law 节点及关系：Risk 永远直接 cites Law；有条款号时 Law 再 has_article Article。
    async fn upsert_law(&self, d: &EntityDecision, law: &LawArticleEntity) -> Result<()> {
        // 注意：节点分开 MERGE、关系单独 MERGE。
        // 一次性路径 MERGE（(r)-[:cites]->(l)）在节点已存在时也会新建重复节点（已验证）。
        let cql = r"
MATCH (r:Risk {risk_id: $risk_id})
MERGE (l:Law {law_id: $law_id})
ON CREATE SET l.name = $law_name
MERGE (r)-[:cites]->(l)";
        self.graph
            .run(
                query(cql)
                    .param("risk_id", d.risk.id.as_str())
                    .param("law_id", law.law_id.as_str())
                    .param("law_name", law.law_name.as_str()),
            )
            .await
            .context("写入 Law 节点失败")?;

        if let Some(article_id) = &law.article_id {
            let cql = r"
MATCH (l:Law {law_id: $law_id})
MERGE (a:Article {article_id: $article_id})
ON CREATE SET a.law_id = $law_id, a.article_no = $article_no
MERGE (l)-[:has_article]->(a)";
            self.graph
                .run(
                    query(cql)
                        .param("law_id", law.law_id.as_str())
                        .param("article_id", article_id.as_str())
                        .param("article_no", law.article_no.as_deref().unwrap_or("")),
                )
                .await
                .context("写入 Article 节点失败")?;
        }
        Ok(())
    }

    /// 关键词查询风险及关联的法律/条款（按风险名匹配）。
    pub async fn search(&self, q: &str) -> Result<Vec<SearchHit>> {
        let cql = r"
MATCH (r:Risk)
WHERE r.name CONTAINS $q
OPTIONAL MATCH (r)-[:cites]->(l:Law)
OPTIONAL MATCH (l)-[:has_article]->(a:Article)
RETURN r.risk_id AS risk_id, r.name AS risk_name, r.severity AS severity,
       coalesce(r.snippet, '') AS snippet,
       coalesce(r.candidate_ids, []) AS candidate_ids,
       coalesce(l.law_id, '') AS law_id, coalesce(l.law_name, '') AS law_name,
       coalesce(a.article_id, '') AS article_id, coalesce(a.article_no, '') AS article_no";
        let mut result = self.graph.execute(query(cql).param("q", q)).await?;

        let mut hits: Vec<SearchHit> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        while let Some(row) = result.next().await? {
            let risk_id: String = row.get("risk_id")?;
            let risk_name: String = row.get("risk_name")?;
            let severity: String = row.get("severity")?;
            let snippet: String = row.get("snippet")?;
            let candidate_ids: Vec<String> = row.get("candidate_ids")?;
            let law_id: String = row.get("law_id")?;
            let law_name: String = row.get("law_name")?;
            let article_id: String = row.get("article_id")?;
            let article_no: String = row.get("article_no")?;

            let idx = match index.get(&risk_id) {
                Some(&i) => i,
                None => {
                    hits.push(SearchHit {
                        risk: RiskEntity {
                            id: risk_id.clone(),
                            name: risk_name,
                            severity,
                        },
                        laws: Vec::new(),
                        candidate_ids,
                        snippet,
                    });
                    index.insert(risk_id.clone(), hits.len() - 1);
                    hits.len() - 1
                }
            };

            if law_id.is_empty() {
                continue;
            }
            let key = format!("{law_id}:{article_id}");
            if hits[idx]
                .laws
                .iter()
                .any(|l| format!("{}:{}", l.law_id, l.article_id.as_deref().unwrap_or("")) == key)
            {
                continue;
            }
            hits[idx].laws.push(LawArticleEntity {
                law_id,
                law_name,
                article_id: (!article_id.is_empty()).then_some(article_id),
                article_no: (!article_no.is_empty()).then_some(article_no),
            });
        }
        Ok(hits)
    }
}
