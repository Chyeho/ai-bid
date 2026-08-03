use crate::agents::types::{RiskFinding, RiskSeverity};
use crate::knowledge::types::Candidate;

/// 从审核结果中挑出值得收藏的精华。
///
/// 规则：`severity == RiskSeverity::High` 或 `legal_basis` 非空，且排除 `no_risk`。
pub fn collect_candidates(findings: &[RiskFinding]) -> Vec<Candidate> {
    findings
        .iter()
        .filter(|item| {
            // 1. 排除 no_risk 或 Info 级别
            if item.risk_type == "no_risk" || item.severity == RiskSeverity::Info {
                return false;
            }

            // 2. 挑精华：高风险 OR 有法律依据
            item.severity == RiskSeverity::High || !item.legal_basis.is_empty()
        })
        .map(|item| {
            // 3. 映射到 Candidate 类型，severity 转为 String
            Candidate {
                candidate_id: item.risk_id.clone(),
                risk_id: item.risk_id.clone(),
                severity: item.severity.to_string(), // 使用 .to_string() 转换成 String
                risk_type: item.risk_type.clone(),
                legal_basis: item.legal_basis.clone(),
                case_refs: item.case_refs.clone(),
                source_quote: item.source_quote.clone(),
                reason: item.reason.clone(),
                suggestion: item.suggestion.clone(),
                confidence: item.confidence,
            }
        })
        .collect()
}