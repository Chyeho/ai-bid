

use std::collections::HashSet;

use crate::knowledge::types::{Candidate, EntityDecision};

/// 确定性 ID：SHA256(input) 取前 8 位十六进制。
pub fn deterministic_id(input: &str) -> String {
    todo!("待实现")
}

/// 条款号归一化："第二十二条" → "第22条"。
pub fn normalize_article_no(no: &str) -> String {
    todo!("待实现")
}

/// 从一句法条引用拆出 (法律名, 条款号)。
/// 例："《政府采购法实施条例》第二十条" → ("政府采购法实施条例", Some("第20条"))
pub fn parse_legal_basis(quote: &str) -> (String, Option<String>) {
    todo!("待实现")
}

/// 拆实体 + 查重。
///
/// `existing_law_ids` 为库中已有的 law_id 集合（组长从 Neo4j 查一次）。
pub fn extract_and_dedup(
    candidates: Vec<Candidate>,
    existing_law_ids: &HashSet<String>,
) -> Vec<EntityDecision> {
    todo!("待实现")
}
