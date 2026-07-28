//! `validate_calculation` 工具 — 验证标书中的数值计算。
//!
//! LLM 做数学不可靠，代码执行精确。本工具提供：
//! - 数学表达式求值（加减乘除、括号）
//! - 法定阈值比对（如履约保证金 ≤ 10%）
//! - 合规判定（compliant / violation / uncertain）
//!
//! ## 典型使用场景
//!
//! ① 价格分计算公式是否公平（权重 × 报价/基准价）
//! ② 履约保证金比例是否超法定上限（≤ 10%）
//! ③ 付款比例是否合规（预付款 + 进度款 + 尾款 = 合同金额）
//! ④ 评分加权是否满 100%

use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;

use super::AgentTool;

/// `validate_calculation` 工具的参数。
#[derive(Debug, Deserialize)]
pub struct ValidateCalculationArgs {
    /// 数学表达式，如 "(履约保证金 / 合同金额) * 100"
    pub formula: String,
    /// 变量→数值映射，如 {"履约保证金": 500000, "合同金额": 5000000}
    pub values: HashMap<String, f64>,
    /// 可选法定阈值表达式，如 "≤ 10"
    #[serde(default)]
    pub legal_threshold: Option<String>,
}

/// 计算验证的返回结果。
#[derive(Debug, serde::Serialize)]
struct CalculationResult {
    /// 计算结果数值
    computed: f64,
    /// 计算表达式（变量已代入）
    resolved_formula: String,
    /// 法定阈值描述（如有）
    threshold: Option<String>,
    /// 合规判定
    verdict: Verdict,
    /// 法条依据（如设置阈值时提供）
    legal_ref: Option<String>,
    /// 计算步骤说明
    steps: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Compliant,
    Violation,
    Uncertain,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Compliant => write!(f, "compliant"),
            Verdict::Violation => write!(f, "violation"),
            Verdict::Uncertain => write!(f, "uncertain"),
        }
    }
}

/// `validate_calculation` 工具实现。
///
/// 纯计算工具，无外部依赖。
pub struct ValidateCalculationTool;

impl ValidateCalculationTool {
    /// 解析并计算数学表达式。
    ///
    /// 支持的运算符：+ - * / ( )
    /// 变量名由 `values` 中的 key 匹配，支持中文字段名。
    fn evaluate(
        formula: &str,
        values: &HashMap<String, f64>,
    ) -> Result<(f64, String, Vec<String>)> {
        let mut steps: Vec<String> = Vec::new();
        let mut resolved = formula.to_string();

        // 1. 替换变量为数值
        for (var, val) in values {
            resolved = resolved.replace(var, &val.to_string());
        }

        steps.push(format!("变量代入: {}", resolved));

        // 2. 分词 + 求值（递归下降 / 调度场算法）
        let tokens = tokenize(&resolved)?;
        let (result, _) = parse_expression(&tokens, 0)?;

        steps.push(format!("计算结果: {}", result));

        Ok((result, resolved, steps))
    }

    /// 解析阈值表达式。
    ///
    /// 支持格式：
    /// - "≤ 10" → 结果不应超过 10
    /// - "≥ 20" → 结果不应低于 20
    /// - "== 100" → 结果应等于 100
    /// - "< 30" → 结果应小于 30
    /// - "> 0" → 结果应大于 0
    /// - "10 ≤ x ≤ 100" → 结果应在 [10, 100] 区间内
    fn parse_threshold(threshold: &str) -> Result<(f64, f64, String)> {
        let t = threshold.trim();

        // 区间: "10 ≤ x ≤ 100" 或 "10 <= x <= 100"
        let range_patterns = [
            (
                r"(\d+\.?\d*)\s*[≤<=]+\s*[xX]\s*[≤<=]+\s*(\d+\.?\d*)",
                "区间",
            ),
            (
                r"(\d+\.?\d*)\s*[≥>=]+\s*[xX]\s*[≥>=]+\s*(\d+\.?\d*)",
                "区间",
            ),
        ];

        for (pattern, _) in &range_patterns {
            if let Some(caps) = regex_lite_capture(pattern, t)
                && caps.len() >= 3
            {
                let lo: f64 = caps[1].parse()?;
                let hi: f64 = caps[2].parse()?;
                return Ok((lo, hi, format!("应在 [{}, {}] 区间内", lo, hi)));
            }
        }

        // 简单比较: "≤ 10", "≥ 20", "== 100", "< 30", "> 0"
        if let Some(rest) = t.strip_prefix("≤").or_else(|| t.strip_prefix("<=")) {
            let val: f64 = rest.trim().parse()?;
            return Ok((f64::NEG_INFINITY, val, format!("≤ {}", val)));
        }
        if let Some(rest) = t.strip_prefix("≥").or_else(|| t.strip_prefix(">=")) {
            let val: f64 = rest.trim().parse()?;
            return Ok((val, f64::INFINITY, format!("≥ {}", val)));
        }
        if let Some(rest) = t.strip_prefix("==").or_else(|| t.strip_prefix("=")) {
            let val: f64 = rest.trim().parse()?;
            return Ok((val, val, format!("= {}", val)));
        }
        if let Some(rest) = t.strip_prefix("<") {
            let val: f64 = rest.trim().parse()?;
            return Ok((f64::NEG_INFINITY, val - f64::EPSILON, format!("< {}", val)));
        }
        if let Some(rest) = t.strip_prefix(">") {
            let val: f64 = rest.trim().parse()?;
            return Ok((val + f64::EPSILON, f64::INFINITY, format!("> {}", val)));
        }

        Err(anyhow!("无法解析阈值表达式: {}", threshold))
    }

    /// 判定合规性。
    fn judge(computed: f64, lo: f64, hi: f64) -> Verdict {
        if computed >= lo && computed <= hi {
            Verdict::Compliant
        } else {
            Verdict::Violation
        }
    }

    /// 获取阈值对应的法条依据。
    fn threshold_legal_basis(threshold: &str) -> Option<String> {
        let t = threshold.to_lowercase();
        if t.contains("10") && (t.contains("保证金") || t.contains("履约")) {
            Some("《政府采购法实施条例》第48条：履约保证金不得超过合同金额的10%".into())
        } else if t.contains("20") && (t.contains("日") || t.contains("公告") || t.contains("等标"))
        {
            Some("《政府采购法》第35条：公开招标公告期不少于20日".into())
        } else if t.contains("100") && (t.contains("评分") || t.contains("权重")) {
            Some("评分权重总和须等于100%".into())
        } else if t.contains("30") && (t.contains("预付款") || t.contains("预付")) {
            Some("《政府采购法实施条例》第48条：预付款不得超过合同金额的30%".into())
        } else {
            None
        }
    }
}

/// 简单正则捕获（避免引入 regex crate）。
fn regex_lite_capture(pattern: &str, text: &str) -> Option<Vec<String>> {
    // 仅支持本模块内的几个固定模式
    if pattern.contains("≤") || pattern.contains("<=") {
        // 区间模式: 数字 ≤ x ≤ 数字
        let cleaned: String = text
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        let parts: Vec<&str> = cleaned.split_whitespace().collect();
        if parts.len() >= 5
            && let (Ok(lo), Ok(hi)) = (parts[0].parse::<f64>(), parts[4].parse::<f64>())
        {
            return Some(vec![String::new(), lo.to_string(), hi.to_string()]);
        }
        // 也尝试 "10≤x≤100" 无空格格式
        for sep in &["≤x≤", "≤X≤", "<=x<=", "<=X<="] {
            if let Some(pos) = text.find(sep) {
                let left = &text[..pos];
                let right = &text[pos + sep.len()..];
                if let (Ok(lo), Ok(hi)) = (left.trim().parse::<f64>(), right.trim().parse::<f64>())
                {
                    return Some(vec![String::new(), lo.to_string(), hi.to_string()]);
                }
            }
        }
    }
    None
}

// ─── 简单表达式求值器 ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Mul,
    Div,
    LParen,
    RParen,
}

fn tokenize(expr: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num_str: String = chars[start..i].iter().collect();
            let num: f64 = num_str
                .parse()
                .map_err(|_| anyhow!("无效数字: {}", num_str))?;
            tokens.push(Token::Number(num));
        } else {
            match c {
                '+' => tokens.push(Token::Plus),
                '-' => tokens.push(Token::Minus),
                '*' => tokens.push(Token::Mul),
                '/' => tokens.push(Token::Div),
                '(' => tokens.push(Token::LParen),
                ')' => tokens.push(Token::RParen),
                other => return Err(anyhow!("不支持的字符: '{}'", other)),
            }
            i += 1;
        }
    }
    Ok(tokens)
}

/// 递归下降解析 — 入口：解析加减法（最低优先级）。
fn parse_expression(tokens: &[Token], pos: usize) -> Result<(f64, usize)> {
    parse_add_sub(tokens, pos)
}

/// 解析加减法。
fn parse_add_sub(tokens: &[Token], pos: usize) -> Result<(f64, usize)> {
    let (mut left, mut pos) = parse_mul_div(tokens, pos)?;

    while pos < tokens.len() {
        match tokens[pos] {
            Token::Plus => {
                let (right, new_pos) = parse_mul_div(tokens, pos + 1)?;
                left += right;
                pos = new_pos;
            }
            Token::Minus => {
                let (right, new_pos) = parse_mul_div(tokens, pos + 1)?;
                left -= right;
                pos = new_pos;
            }
            _ => break,
        }
    }

    Ok((left, pos))
}

/// 解析乘除法。
fn parse_mul_div(tokens: &[Token], pos: usize) -> Result<(f64, usize)> {
    let (mut left, mut pos) = parse_primary(tokens, pos)?;

    while pos < tokens.len() {
        match tokens[pos] {
            Token::Mul => {
                let (right, new_pos) = parse_primary(tokens, pos + 1)?;
                left *= right;
                pos = new_pos;
            }
            Token::Div => {
                let (right, new_pos) = parse_primary(tokens, pos + 1)?;
                if right == 0.0 {
                    return Err(anyhow!("除零错误"));
                }
                left /= right;
                pos = new_pos;
            }
            _ => break,
        }
    }

    Ok((left, pos))
}

/// 解析基本单元：数字 或 括号表达式。
fn parse_primary(tokens: &[Token], pos: usize) -> Result<(f64, usize)> {
    if pos >= tokens.len() {
        return Err(anyhow!("表达式不完整：期望数字或括号"));
    }
    match tokens[pos] {
        Token::Number(n) => Ok((n, pos + 1)),
        Token::LParen => {
            let (val, pos) = parse_add_sub(tokens, pos + 1)?;
            if pos < tokens.len() && tokens[pos] == Token::RParen {
                Ok((val, pos + 1))
            } else {
                Err(anyhow!("缺少右括号"))
            }
        }
        Token::Minus => {
            // 一元负号
            let (val, pos) = parse_primary(tokens, pos + 1)?;
            Ok((-val, pos))
        }
        _ => Err(anyhow!("意外的 token: {:?}", tokens[pos])),
    }
}

// ─── AgentTool 实现 ────────────────────────────────────────────

#[async_trait::async_trait]
impl AgentTool for ValidateCalculationTool {
    fn name(&self) -> &str {
        "validate_calculation"
    }

    fn definition(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "validate_calculation",
                "description": "【使用场景】验证标书中的数值计算——LLM 做数学不可靠，代码执行精确。\
                    ① 价格分计算公式是否公平（权重 × 报价/基准价）；\
                    ② 履约保证金比例是否超法定上限（履约保证金 / 合同金额 ≤ 10%）；\
                    ③ 付款比例是否合规（预付款 + 进度款 + 尾款 = 合同金额）；\
                    ④ 评分加权是否满 100%。\
                    【不使用场景】需要语义理解的'合理性'判断——LLM 做推理，计算器做算术。\
                    【注意】formula 支持加减乘除和括号，不支持复杂函数。\
                    变量名支持中文（如'履约保证金'），会自动替换为 values 中对应的数值。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "formula": {
                            "type": "string",
                            "description": "数学表达式，如 '(履约保证金 / 合同金额) * 100'。支持 + - * / ( )，变量名可包含中文。"
                        },
                        "values": {
                            "type": "object",
                            "description": "变量→数值映射，如 {\"履约保证金\": 500000, \"合同金额\": 5000000}。键为 formula 中的变量名，值为 f64 数值。"
                        },
                        "legal_threshold": {
                            "type": "string",
                            "description": "法定阈值表达式，如 '≤ 10'（结果不应超过10）、'≥ 20'（结果不应低于20）、'10 ≤ x ≤ 100'（区间）。可选参数。"
                        }
                    },
                    "required": ["formula", "values"]
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let parsed: ValidateCalculationArgs = serde_json::from_value(args)?;

        if parsed.values.is_empty() {
            return Err(anyhow!("values 不能为空"));
        }

        // 1. 计算
        let (computed, resolved_formula, steps) = Self::evaluate(&parsed.formula, &parsed.values)?;

        // 2. 阈值判定（如有）
        let (threshold_desc, verdict, legal_ref) =
            if let Some(ref threshold) = parsed.legal_threshold {
                let threshold = threshold.trim();
                if threshold.is_empty() {
                    (None, Verdict::Uncertain, None)
                } else {
                    match Self::parse_threshold(threshold) {
                        Ok((lo, hi, desc)) => {
                            let v = Self::judge(computed, lo, hi);
                            let legal = Self::threshold_legal_basis(threshold);
                            (Some(desc), v, legal)
                        }
                        Err(_) => (
                            Some(format!("无法解析: {}", threshold)),
                            Verdict::Uncertain,
                            None,
                        ),
                    }
                }
            } else {
                (None, Verdict::Uncertain, None)
            };

        let result = CalculationResult {
            computed: (computed * 10000.0).round() / 10000.0, // 保留 4 位小数
            resolved_formula,
            threshold: threshold_desc,
            verdict,
            legal_ref,
            steps,
        };

        Ok(serde_json::to_value(&result)?)
    }
}

// ─── 测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_addition() {
        let mut vals = HashMap::new();
        vals.insert("a".to_string(), 3.0);
        vals.insert("b".to_string(), 5.0);
        let (result, _, _) = ValidateCalculationTool::evaluate("a + b", &vals).unwrap();
        assert!((result - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_complex_formula() {
        let mut vals = HashMap::new();
        vals.insert("履约保证金".to_string(), 500000.0);
        vals.insert("合同金额".to_string(), 5000000.0);
        let (result, _, _) =
            ValidateCalculationTool::evaluate("(履约保证金 / 合同金额) * 100", &vals).unwrap();
        assert!((result - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_multiplication_division() {
        let mut vals = HashMap::new();
        vals.insert("x".to_string(), 10.0);
        vals.insert("y".to_string(), 2.0);
        let (result, _, _) = ValidateCalculationTool::evaluate("x * y / 5", &vals).unwrap();
        assert!((result - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_threshold_le() {
        let (lo, hi, _desc) = ValidateCalculationTool::parse_threshold("≤ 10").unwrap();
        assert!(lo.is_infinite() && lo < 0.0); // NEG_INFINITY
        assert!((hi - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_threshold_ge() {
        let (lo, hi, _desc) = ValidateCalculationTool::parse_threshold("≥ 20").unwrap();
        assert!((lo - 20.0).abs() < 0.001);
        assert!(hi.is_infinite() && hi > 0.0);
    }

    #[test]
    fn test_parse_threshold_eq() {
        let (lo, hi, _desc) = ValidateCalculationTool::parse_threshold("== 100").unwrap();
        assert!((lo - 100.0).abs() < 0.001);
        assert!((hi - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_threshold_range() {
        let (lo, hi, _desc) = ValidateCalculationTool::parse_threshold("10 ≤ x ≤ 100").unwrap();
        assert!((lo - 10.0).abs() < 0.001);
        assert!((hi - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_judge_compliant() {
        assert!(matches!(
            ValidateCalculationTool::judge(8.0, f64::NEG_INFINITY, 10.0),
            Verdict::Compliant
        ));
    }

    #[test]
    fn test_judge_violation() {
        assert!(matches!(
            ValidateCalculationTool::judge(12.0, f64::NEG_INFINITY, 10.0),
            Verdict::Violation
        ));
    }

    #[test]
    fn test_guarantee_deposit_check() {
        let mut vals = HashMap::new();
        vals.insert("履约保证金".to_string(), 800000.0);
        vals.insert("合同金额".to_string(), 5000000.0);
        let (computed, _, _) =
            ValidateCalculationTool::evaluate("(履约保证金 / 合同金额) * 100", &vals).unwrap();
        // 16% > 10% → violation
        let (lo, hi, _) = ValidateCalculationTool::parse_threshold("≤ 10").unwrap();
        assert!(matches!(
            ValidateCalculationTool::judge(computed, lo, hi),
            Verdict::Violation
        ));
    }

    #[test]
    fn test_tokenize_chinese_vars() {
        // 中文变量名在 evaluate 阶段已被替换，tokenize 只处理替换后的表达式
        // (500000 / 5000000) * 100 → 7 tokens: LParen Number Div Number RParen Mul Number
        let tokens = tokenize("(500000 / 5000000) * 100").unwrap();
        assert_eq!(tokens.len(), 7);
    }

    #[test]
    fn test_division_by_zero() {
        let mut vals = HashMap::new();
        vals.insert("a".to_string(), 10.0);
        vals.insert("b".to_string(), 0.0);
        let result = ValidateCalculationTool::evaluate("a / b", &vals);
        assert!(result.is_err());
    }
}
