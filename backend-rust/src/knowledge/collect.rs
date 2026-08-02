

use crate::agents::types::RiskFinding;
use crate::knowledge::types::Candidate;

/// 从审核结果中挑出值得收藏的精华。
///
/// 规则：`severity == "high"` 或 `legal_basis` 非空，且排除 `no_risk`。
pub fn collect_candidates(findings: &[RiskFinding]) -> Vec<Candidate> {
    todo!("待实现")
}
