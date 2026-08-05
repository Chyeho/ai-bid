//! 统一风险分类、证据准入与重大问题策略。
//!
//! LLM 可以自由描述 `risk_type`，但进入合并与展示前必须收敛到稳定分类。
//! Critical 不是模型自由选择的第五级 severity，而是由分类和原文证据共同决定。

use crate::agents::types::{RiskFinding, RiskSeverity};

const CATEGORIES: &[(&str, &str)] = &[
    ("LOCAL_REGISTRATION", "地域注册限制"),
    ("BRAND_LOCK", "指定品牌且不接受同等产品"),
    ("UNRELATED_CERT", "设置与履约无关的资格条件"),
    ("REGIONAL_PERFORMANCE", "特定区域业绩限制"),
    ("SCALE_THRESHOLD", "以经营规模设置资格门槛"),
    ("SHORT_DEADLINE", "投标准备期不足"),
    ("EXCESSIVE_DEPOSIT", "投标保证金比例过高"),
    ("OEM_AUTHORIZATION", "将厂家授权作为资格条件"),
    ("SUBJECTIVE_SCORING", "主观评分未细化量化"),
    ("LOCAL_AWARD", "本地奖项加分"),
    ("VAGUE_ACCEPTANCE", "验收标准模糊"),
    ("UNBOUNDED_IP", "知识产权责任无限扩大"),
    ("UNILATERAL_CHANGE", "采购人可单方无限变更需求"),
    ("CONFLICTING_DATES", "关键日期相互矛盾"),
    ("UNCLEAR_PENALTY", "违约责任口径不清"),
];

fn contains_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

fn normalized_code(value: &str) -> String {
    let upper: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .flat_map(char::to_uppercase)
        .collect();
    if let Some((prefix, rest)) = upper.split_once('_')
        && prefix.len() >= 2
        && prefix.starts_with(|c: char| c.is_ascii_alphabetic())
        && prefix[1..].chars().all(|c| c.is_ascii_digit())
    {
        return rest.to_string();
    }
    upper
}

/// 优先依据逐字证据分类，避免模型填了合法但语义错误的自由编码。
fn category_from_evidence(text: &str) -> Option<&'static str> {
    let regional = contains_any(
        text,
        &[
            "本市",
            "本区",
            "本县",
            "本省",
            "当地",
            "所在地",
            "所在区县",
            "采购人所在地",
        ],
    );
    if regional
        && contains_any(text, &["注册", "分公司", "分支机构", "经营满"])
        && contains_any(text, &["须", "必须", "仅限", "不接受", "资格"])
    {
        return Some("LOCAL_REGISTRATION");
    }
    if regional
        && contains_any(text, &["业绩", "案例", "合同"])
        && contains_any(text, &["须", "必须", "仅限", "不认可", "不予认可", "不得"])
    {
        return Some("REGIONAL_PERFORMANCE");
    }
    if regional
        && contains_any(text, &["奖项", "荣誉", "获奖", "证书", "诚信企业"])
        && contains_any(text, &["加分", "得分", "评分", "分"])
    {
        return Some("LOCAL_AWARD");
    }
    if contains_any(text, &["注册资本", "营业收入", "资产总额", "净资产"])
        && contains_any(text, &["不得低于", "不少于", "以上", "门槛", "资格"])
    {
        return Some("SCALE_THRESHOLD");
    }
    if contains_any(text, &["品牌", "商标", "型号"])
        && contains_any(text, &["仅", "只能", "唯一", "指定", "不接受", "不得偏离"])
    {
        return Some("BRAND_LOCK");
    }
    if contains_any(text, &["原厂", "厂家", "制造商"])
        && contains_any(text, &["授权", "承诺函", "证明"])
        && contains_any(text, &["资格", "无效", "废标", "必须", "须"])
    {
        return Some("OEM_AUTHORIZATION");
    }
    if contains_any(text, &["认证", "证书", "荣誉", "示范企业"])
        && contains_any(
            text,
            &["资格", "无效", "废标", "不通过", "必须", "须提供", "无关"],
        )
    {
        return Some("UNRELATED_CERT");
    }
    if text.contains("投标保证金")
        && contains_any(
            text,
            &[
                "3%",
                "4%",
                "5%",
                "百分之三",
                "百分之四",
                "百分之五",
                "比例过高",
            ],
        )
    {
        return Some("EXCESSIVE_DEPOSIT");
    }
    if contains_any(text, &["投标截止", "开标", "投标准备", "获取招标文件"])
        && contains_any(text, &["3日", "5日", "不足", "少于", "仅有", "仅"])
    {
        return Some("SHORT_DEADLINE");
    }
    if contains_any(text, &["评分", "得分", "评委"])
        && contains_any(
            text,
            &["酌情", "自行掌握", "主观", "优良", "满意程度", "综合判断"],
        )
    {
        return Some("SUBJECTIVE_SCORING");
    }
    if text.contains("验收")
        && contains_any(
            text,
            &["满意", "自行判断", "无异议", "未明确", "不明确", "无需说明"],
        )
    {
        return Some("VAGUE_ACCEPTANCE");
    }
    if contains_any(text, &["知识产权", "侵权", "专利", "既有软件", "权利"])
        && contains_any(
            text,
            &["全部责任", "一切责任", "无限", "无上限", "既有", "永久归"],
        )
    {
        return Some("UNBOUNDED_IP");
    }
    if contains_any(text, &["单方", "新增需求", "任意变更", "采购人有权变更"])
        && contains_any(
            text,
            &["不得调整", "不调整", "无条件", "原合同范围", "费用", "工期"],
        )
    {
        return Some("UNILATERAL_CHANGE");
    }
    if contains_any(text, &["日期", "截止", "开标时间"])
        && contains_any(text, &["矛盾", "不一致", "另一处", "分别为", "同时规定"])
    {
        return Some("CONFLICTING_DATES");
    }
    if contains_any(text, &["违约金", "违约责任", "处罚"])
        && contains_any(
            text,
            &["重复", "累计", "无上限", "自行决定", "不明确", "不清"],
        )
    {
        return Some("UNCLEAR_PENALTY");
    }
    None
}

fn category_from_alias(value: &str) -> Option<&'static str> {
    let code = normalized_code(value);
    CATEGORIES
        .iter()
        .find_map(|(canonical, _)| (code == *canonical).then_some(*canonical))
        .or_else(|| match code.as_str() {
            "UNRELATED_CERTIFICATE"
            | "UNRELATED_CERTIFICATION"
            | "UNRELATED_QUALIFICATION"
            | "IRRELEVANT_CERTIFICATE" => Some("UNRELATED_CERT"),
            "SHORT_PREPARATION_PERIOD"
            | "UNREASONABLE_TIME_LIMIT"
            | "UNREASONABLE_PREPARATION_TIME"
            | "SHORT_BIDDING_PERIOD"
            | "TIME_LIMIT" => Some("SHORT_DEADLINE"),
            "ASSET_THRESHOLD" | "ASSET_REQUIREMENT" | "CAPITAL_THRESHOLD" => {
                Some("SCALE_THRESHOLD")
            }
            "MANUFACTURER_AUTHORIZATION" | "FACTORY_AUTHORIZATION" => Some("OEM_AUTHORIZATION"),
            "SCORING_DISCRETION" | "UNSPECIFIED_SCORING" | "UNQUANTIFIED_ASSESSMENT" => {
                Some("SUBJECTIVE_SCORING")
            }
            "LOCAL_CERTIFICATE_BONUS" | "LOCAL_HONOR_BONUS" => Some("LOCAL_AWARD"),
            "UNCLEAR_ACCEPTANCE_CRITERIA"
            | "ACCEPTANCE_CRITERIA"
            | "AMBIGUOUS_ACCEPTANCE"
            | "UNCLEAR_ACCEPTANCE" => Some("VAGUE_ACCEPTANCE"),
            "UNLIMITED_IP_LIABILITY" | "IP_LIABILITY" => Some("UNBOUNDED_IP"),
            "UNDEFINED_PENALTY" | "UNLIMITED_PENALTY" | "UNCLEAR_CONTRACTUAL_RESPONSIBILITY" => {
                Some("UNCLEAR_PENALTY")
            }
            "DATE_CONFLICT" | "关键日期矛盾" => Some("CONFLICTING_DATES"),
            _ => None,
        })
}

pub fn canonical_category(finding: &RiskFinding) -> String {
    category_from_evidence(finding.source_quote.trim())
        .or_else(|| category_from_alias(&finding.category_code))
        .or_else(|| category_from_alias(&finding.risk_type))
        .map(str::to_string)
        .unwrap_or_else(|| {
            let fallback = if finding.category_code.trim().is_empty() {
                &finding.risk_type
            } else {
                &finding.category_code
            };
            normalized_code(fallback)
        })
}

pub fn display_name(code: &str) -> Option<&'static str> {
    CATEGORIES
        .iter()
        .find_map(|(candidate, name)| (*candidate == code).then_some(*name))
}

/// 零成本预检：从一个 chunk 的各行中找出必须复核的高信号风险候选。
/// 返回候选而非最终结论，最终仍须由责任 Agent 输出逐字证据和理由。
pub fn candidate_categories(text: &str) -> Vec<&'static str> {
    let mut result = Vec::new();
    for segment in text.split(['\n', '。', '；']) {
        if let Some(category) = category_from_evidence(segment)
            && !result.contains(&category)
        {
            result.push(category);
        }
    }
    // 日期冲突常跨两行表达，额外用完整 chunk 检查。
    if let Some(category) = category_from_evidence(text)
        && !result.contains(&category)
    {
        result.push(category);
    }
    result
}

pub fn owner_agent(code: &str) -> &'static str {
    match code {
        "SHORT_DEADLINE" | "EXCESSIVE_DEPOSIT" | "OEM_AUTHORIZATION" => "ProcedureAgent",
        "SUBJECTIVE_SCORING" | "LOCAL_AWARD" => "ScoringAgent",
        "BRAND_LOCK" | "UNRELATED_CERT" | "SCALE_THRESHOLD" => "DemandAgent",
        "VAGUE_ACCEPTANCE" | "UNBOUNDED_IP" | "UNILATERAL_CHANGE" | "UNCLEAR_PENALTY" => {
            "ContractAgent"
        }
        "LOCAL_REGISTRATION" | "REGIONAL_PERFORMANCE" => "SemanticRiskAgent",
        "CONFLICTING_DATES" => "FactCheckAgent",
        _ => "RuleEngineAgent",
    }
}

pub fn review_candidates_for_agent(text: &str, agent: &str) -> Vec<&'static str> {
    candidate_categories(text)
        .into_iter()
        .filter(|category| owner_agent(category) == agent || agent == "RuleEngineAgent")
        .collect()
}

fn critical_evidence(code: &str, quote: &str) -> bool {
    match code {
        "LOCAL_REGISTRATION" => {
            contains_any(quote, &["注册", "分公司", "分支机构"])
                && contains_any(quote, &["本市", "本区", "本县", "所在地", "外地"])
        }
        "BRAND_LOCK" => {
            contains_any(quote, &["品牌", "商标", "型号"])
                && contains_any(quote, &["仅", "只能", "唯一", "不接受", "指定"])
        }
        "UNRELATED_CERT" => {
            contains_any(quote, &["认证", "证书", "荣誉", "示范企业"])
                && contains_any(
                    quote,
                    &["资格", "无效", "废标", "不通过", "必须", "须", "无关"],
                )
        }
        "REGIONAL_PERFORMANCE" => {
            contains_any(quote, &["业绩", "案例", "合同"])
                && contains_any(quote, &["本市", "本区", "本县", "本省", "当地", "所在区县"])
        }
        "SCALE_THRESHOLD" => {
            contains_any(quote, &["注册资本", "营业收入", "资产总额", "净资产"])
                && contains_any(quote, &["不得低于", "不少于", "以上", "资格"])
        }
        _ => false,
    }
}

pub fn is_actionable(finding: &RiskFinding) -> bool {
    if finding.no_risk {
        return true;
    }
    let quote = finding.source_quote.trim();
    if quote.is_empty() {
        return false;
    }
    let looks_like_heading = quote.chars().count() < 20
        && !contains_any(
            quote,
            &[
                "须",
                "必须",
                "不得",
                "不接受",
                "不予",
                "否则",
                "仅限",
                "无上限",
                "永久归",
                "承担",
                "得分",
                "加分",
            ],
        );
    if looks_like_heading {
        return false;
    }
    let negative = contains_any(
        quote,
        &[
            "未提及",
            "未发现",
            "未说明",
            "需要进一步确认",
            "建议进一步审查",
        ],
    );
    !(finding.severity == RiskSeverity::Info && negative)
}

/// 在所有 Agent 输出汇合后执行，保证下游只面对一种分类和 Critical 语义。
pub fn normalize_finding(finding: &mut RiskFinding) {
    let category = canonical_category(finding);
    finding.category_code = category.clone();
    if let Some(name) = display_name(&category) {
        finding.risk_type = name.to_string();
    }

    let is_critical = !finding.no_risk
        && !finding.source_quote.trim().is_empty()
        && critical_evidence(&category, finding.source_quote.trim());
    finding.is_critical = is_critical;
    if is_critical {
        finding.severity = RiskSeverity::High;
        finding.critical_reason = format!(
            "命中重大问题分类 {}，且原文证据满足红线判定条件。",
            category
        );
    } else {
        finding.critical_reason.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::{FindingRole, RiskTier};

    fn finding(code: &str, quote: &str) -> RiskFinding {
        RiskFinding {
            risk_id: "R_001".into(),
            clause_ids: vec!["ch_1".into()],
            block_ids: vec![],
            agent: "test".into(),
            no_risk: false,
            severity: RiskSeverity::High,
            is_critical: true,
            critical_reason: "model decision".into(),
            risk_type: code.into(),
            category_code: code.into(),
            source_quote: quote.into(),
            legal_basis: vec![],
            case_refs: vec![],
            reason: String::new(),
            suggestion: String::new(),
            confidence: 0.9,
            initial_tier: RiskTier::Medium,
            final_tier: RiskTier::Medium,
            tier_escalated: false,
            truncated: false,
            suggested_agent: None,
            citations: vec![],
            finding_role: FindingRole::Verified,
            knowledge_source: String::new(),
            verification_required: vec![],
            hypothesized_by: vec![],
            verified_by: vec![],
            page_number: None,
            section_path: None,
            context: None,
        }
    }

    #[test]
    fn alias_and_evidence_are_canonicalized() {
        let mut f = finding(
            "UNRELATED_CERTIFICATE",
            "供应商必须提供诚信示范企业荣誉证书，否则资格审查不通过。",
        );
        normalize_finding(&mut f);
        assert_eq!(f.category_code, "UNRELATED_CERT");
        assert_eq!(f.risk_type, "设置与履约无关的资格条件");
        assert!(f.is_critical);
    }

    #[test]
    fn ordinary_high_is_not_critical() {
        let mut f = finding(
            "EXCESSIVE_DEPOSIT",
            "供应商须缴纳相当于采购预算5%的投标保证金。",
        );
        normalize_finding(&mut f);
        assert!(!f.is_critical);
        assert!(f.critical_reason.is_empty());
    }

    #[test]
    fn empty_evidence_is_not_actionable() {
        let f = finding("OTHER", "");
        assert!(!is_actionable(&f));
    }

    #[test]
    fn heading_only_is_not_actionable_evidence() {
        let f = finding("OEM_AUTHORIZATION", "将厂家授权作为资格条件");
        assert!(!is_actionable(&f));
    }

    #[test]
    fn multi_issue_chunk_routes_candidates_to_owners() {
        let text = "供应商须提供采购人所在区县的同类服务案例，跨区域案例不作为有效业绩。\n\
                    投标人必须提交生产厂家针对本项目出具的授权函，否则投标无效。";
        assert_eq!(
            review_candidates_for_agent(text, "SemanticRiskAgent"),
            vec!["REGIONAL_PERFORMANCE"]
        );
        assert_eq!(
            review_candidates_for_agent(text, "ProcedureAgent"),
            vec!["OEM_AUTHORIZATION"]
        );
    }

    #[test]
    fn date_and_time_aliases_use_canonical_codes() {
        let mut date = finding(
            "DATE_CONFLICT",
            "投标截止时间为[日期]9时，同时规定[日期]17时后提交的文件一律拒收。",
        );
        normalize_finding(&mut date);
        assert_eq!(date.category_code, "CONFLICTING_DATES");

        let mut deadline = finding(
            "TIME_LIMIT",
            "供应商须在获取本条款后10日内递交投标文件，该期限不作顺延。",
        );
        normalize_finding(&mut deadline);
        assert_eq!(deadline.category_code, "SHORT_DEADLINE");
    }
}
