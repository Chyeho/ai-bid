use super::types::*;
use std::collections::HashSet;
use regex::Regex;
use sha2::{Digest, Sha256};

// --------------------------
// 单元1：中文数字转阿拉伯数字（辅助工具）
// --------------------------
/// 把中文数字（零~九百九十九）转成 u32
fn chinese_num_to_u32(s: &str) -> Option<u32> {
    let map = |c: char| -> Option<u32> {
        match c {
            '零' => Some(0),
            '一' | '壹' => Some(1),
            '二' | '贰' | '两' => Some(2),
            '三' | '叁' => Some(3),
            '四' | '肆' => Some(4),
            '五' | '伍' => Some(5),
            '六' | '陆' => Some(6),
            '七' | '柒' => Some(7),
            '八' | '捌' => Some(8),
            '九' | '玖' => Some(9),
            _ => None,
        }
    };

    let mut result: u32 = 0;
    let mut temp: u32 = 0;
    let chars: Vec<char> = s.chars().collect();

    for &c in &chars {
        match c {
            '百' | '佰' => {
                result += temp * 100;
                temp = 0;
            }
            '十' | '拾' => {
                if temp == 0 {
                    temp = 1; // "十"开头 = 10
                }
                result += temp * 10;
                temp = 0;
            }
            _ => {
                let n = map(c)?;
                temp = temp * 10 + n;
            }
        }
    }
    result += temp;
    Some(result)
}

// --------------------------
// 单元2：条款号归一化
// --------------------------
/// 条款号归一化："第二十条" → "第20条"，"第22条"保持不变
pub fn normalize_article_number(raw: &str) -> Option<String> {
    let re = Regex::new(r"第([零一二三四五六七八九十百千0-9]+)条").ok()?;
    let caps = re.captures(raw)?;
    let num_str = caps.get(1)?.as_str();

    // 如果已经是纯数字，直接返回
    if num_str.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("第{}条", num_str));
    }

    // 中文数字转阿拉伯数字
    let num = chinese_num_to_u32(num_str)?;
    Some(format!("第{}条", num))
}

// --------------------------
// 单元3：法律依据字符串解析
// --------------------------
/// 从 "《政府采购法实施条例》第二十条" 里拆出法律名和条款号
pub fn parse_law_basis(text: &str) -> (String, Option<String>) {
    // 匹配书名号里的法律名
    let law_re = Regex::new(r"《([^》]+)》").unwrap();
    let law_name = law_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| text.to_string());

    // 匹配后面的条款号
    let article_re = Regex::new(r"第[零一二三四五六七八九十百千0-9]+条").unwrap();
    let article_no = article_re
        .find(text)
        .map(|m| normalize_article_number(m.as_str()))
        .flatten();

    (law_name, article_no)
}

// --------------------------
// 单元4：确定性 ID 生成
// --------------------------
/// 生成 law_id：SHA256(法律名) 前8位 + law_ 前缀
pub fn gen_law_id(law_name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(law_name.as_bytes());
    let hash = hex::encode(hasher.finalize());
    format!("law_{}", &hash[..8])
}

/// 生成 article_id：SHA256(law_id + 归一化条款号) 前8位 + art_ 前缀
pub fn gen_article_id(law_id: &str, article_no: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(law_id.as_bytes());
    hasher.update(article_no.as_bytes());
    let hash = hex::encode(hasher.finalize());
    format!("art_{}", &hash[..8])
}

// --------------------------
// 单元5：主函数 - 实体拆分 + 查重
// --------------------------
pub fn extract_and_dedup(
    candidates: Vec<Candidate>,
    existing_law_ids: &HashSet<String>,
) -> Vec<EntityDecision> {
    candidates
        .into_iter()
        .map(|cand| {
            // 构造风险实体
            let risk = RiskEntity {
                id: cand.risk_id.clone(),
                name: cand.risk_type.clone(),
                severity: cand.severity.clone(),
            };

            // 拆分所有法律依据
            let laws: Vec<LawArticleEntity> = cand
                .legal_basis
                .iter()
                .map(|basis| {
                    let (law_name, article_no) = parse_law_basis(basis);
                    let law_id = gen_law_id(&law_name);
                    let article_id = article_no
                        .as_ref()
                        .map(|no| gen_article_id(&law_id, no));

                    LawArticleEntity {
                        law_id,
                        law_name,
                        article_id,
                        article_no,
                    }
                })
                .collect();

            // 查重判断：只要有一个 law_id 不在库里，就标记为 New
            let has_new_law = laws.iter().any(|law| !existing_law_ids.contains(&law.law_id));
            let decision = if has_new_law {
                Decision::New
            } else {
                Decision::Exists
            };

            EntityDecision {
                candidate_id: cand.candidate_id,
                decision,
                risk,
                laws,
                snippet: cand.legal_basis.join("；"),
            }
        })
        .collect()
}

// --------------------------
// 单元测试
// --------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chinese_num() {
        assert_eq!(chinese_num_to_u32("二十"), Some(20));
        assert_eq!(chinese_num_to_u32("二十二"), Some(22));
        assert_eq!(chinese_num_to_u32("十"), Some(10));
        assert_eq!(chinese_num_to_u32("五"), Some(5));
        assert_eq!(chinese_num_to_u32("一百二十三"), Some(123));
    }

    #[test]
    fn test_normalize_article() {
        assert_eq!(
            normalize_article_number("第二十条"),
            Some("第20条".to_string())
        );
        assert_eq!(
            normalize_article_number("第二十二条"),
            Some("第22条".to_string())
        );
        assert_eq!(
            normalize_article_number("第5条"),
            Some("第5条".to_string())
        );
    }

    #[test]
    fn test_parse_law_basis() {
        let text = "《政府采购法实施条例》第二十条";
        let (name, article) = parse_law_basis(text);
        assert_eq!(name, "政府采购法实施条例");
        assert_eq!(article, Some("第20条".to_string()));

        // 无条款号的情况
        let text2 = "《政府采购法》";
        let (name2, article2) = parse_law_basis(text2);
        assert_eq!(name2, "政府采购法");
        assert!(article2.is_none());
    }

    #[test]
    fn test_law_id_consistent() {
        // 相同输入永远生成相同 ID
        let id1 = gen_law_id("政府采购法实施条例");
        let id2 = gen_law_id("政府采购法实施条例");
        assert_eq!(id1, id2);
        assert!(id1.starts_with("law_"));
        assert_eq!(id1.len(), 12); // law_ + 8位
    }

    #[test]
    fn test_dedup_logic() {
        let cand = Candidate {
            candidate_id: "c1".to_string(),
            risk_id: "risk_001".to_string(),
            severity: "high".to_string(),
            risk_type: "品牌指定".to_string(),
            legal_basis: vec!["《政府采购法实施条例》第二十条".to_string()],
            case_refs: vec![],
            source_quote: "".to_string(),
            reason: "".to_string(),
            suggestion: "".to_string(),
            confidence: 0.9,
        };

        // 空库 → New
        let empty = HashSet::new();
        let res = extract_and_dedup(vec![cand.clone()], &empty);
        assert_eq!(res[0].decision, Decision::New);

        // 库中已有 → Exists
        let law_id = gen_law_id("政府采购法实施条例");
        let mut existing = HashSet::new();
        existing.insert(law_id);
        let res = extract_and_dedup(vec![cand], &existing);
        assert_eq!(res[0].decision, Decision::Exists);
    }
}