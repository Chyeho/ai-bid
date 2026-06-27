//! 标书文本脱敏服务
//!
//! 在将 embed_text 发送到远程 Embedding API 之前，使用正则替换掉
//! 结构化敏感信息。替换后的文本保留语义骨架，不影响嵌入向量的搜索质量。
//!
//! ## 覆盖范围
//!
//! | 类型 | 检测方式 | 替换占位符 | 对搜索的影响 |
//! |------|---------|-----------|-------------|
//! | 手机号 | `1[3-9]\d{9}` | `[联系电话]` | 极低 |
//! | 座机号 | `0\d{2,3}-\d{7,8}` | `[联系电话]` | 极低 |
//! | 金额 | `\d+\.?\d*[万元亿]` | `[金额]` | 低 |
//! | 日期 | `\d{4}年\d{1,2}月\d{1,2}日` | `[日期]` | 低 |
//! | 邮箱 | `[\w.-]+@[\w.-]+` | `[邮箱]` | 极低 |
//! | 身份证 | `\d{17}[\dXx]` | `[证件号]` | 极低 |
//!
//! ## 未覆盖（需要 NER / 实体字典）
//!
//! 单位名称、项目名称、人名——无固定正则模式，暂不覆盖。
//! 这些实体在嵌入搜索中携带语义信号（如"XX公司"→"供应商"），
//! 替换反而可能降低搜索精度。通信层应在 HTTPS 加密基础上
//! 评估合规风险。

use regex::Regex;
use std::sync::LazyLock;

// ─── 预编译正则（进程生命周期内编译一次）─────────────────────

static PHONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"1[3-9]\d-?\d{4}-?\d{4}").unwrap());

static LANDLINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"0\d{2,3}[-\s]?\d{7,8}").unwrap());

static AMOUNT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d+(?:\.\d{1,2})?\s*[万元亿]").unwrap());

static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d{4}\s*年\s*\d{1,2}\s*月\s*\d{1,2}\s*日").unwrap());

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\w.\-+]+@[\w.\-]+\.[a-zA-Z]{2,}").unwrap());

static ID_CARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{17}[\dXx]\b").unwrap());

// ─── 公开接口 ────────────────────────────────────────────────

/// 对标书文本执行正则脱敏。
///
/// 替换策略：保守匹配，只替换明确模式的结构化敏感信息。
/// 不尝试检测单位名称 / 人名等无固定模式的内容。
///
/// # 示例
///
/// ```
/// let input = "联系人张三，电话13812345678，投标截止2024年6月15日。";
/// let output = desensitize(input);
/// assert!(output.contains("[联系电话]"));
/// assert!(output.contains("[日期]"));
/// assert!(output.contains("张三")); // 人名不替换
/// ```
pub fn desensitize(text: &str) -> String {
    let mut result = text.to_string();

    // 替换顺序：邮箱和身份证优先（模式更特化，避免被后续宽模式误匹配）
    result = EMAIL_RE.replace_all(&result, "[邮箱]").to_string();
    result = ID_CARD_RE.replace_all(&result, "[证件号]").to_string();
    result = PHONE_RE.replace_all(&result, "[联系电话]").to_string();
    result = LANDLINE_RE.replace_all(&result, "[联系电话]").to_string();
    // 日期必须在金额之前：避免"2024年"被金额正则的\d+匹配
    result = DATE_RE.replace_all(&result, "[日期]").to_string();
    result = AMOUNT_RE.replace_all(&result, "[金额]").to_string();

    result
}

// ─── 测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phone_replacement() {
        let input = "联系人张三，电话13812345678。";
        let output = desensitize(input);
        assert!(!output.contains("13812345678"));
        assert!(output.contains("[联系电话]"));
    }

    #[test]
    fn test_landline_replacement() {
        let input = "咨询电话：0769-22812345。";
        let output = desensitize(input);
        assert!(!output.contains("0769-22812345"));
        assert!(output.contains("[联系电话]"));
    }

    #[test]
    fn test_date_replacement() {
        let input = "投标截止日期：2024年06月15日。";
        let output = desensitize(input);
        assert!(!output.contains("2024年06月15日"));
        assert!(output.contains("[日期]"));
    }

    #[test]
    fn test_amount_replacement() {
        let input = "投标保证金：贰万元整（20000元）。";
        let output = desensitize(input);
        assert!(output.contains("[金额]"));
    }

    #[test]
    fn test_email_replacement() {
        let input = "电子邮箱：contact@example.com。";
        let output = desensitize(input);
        assert!(!output.contains("contact@example.com"));
        assert!(output.contains("[邮箱]"));
    }

    #[test]
    fn test_id_card_replacement() {
        let input = "法定代表人身份证号：440106199001011234。";
        let output = desensitize(input);
        assert!(!output.contains("440106199001011234"));
        assert!(output.contains("[证件号]"));
    }

    #[test]
    fn test_semantic_preservation() {
        let input = "供应商应具备本地服务机构，提供7×24小时技术支持。";
        let output = desensitize(input);
        assert!(output.contains("本地服务机构"));
        assert!(output.contains("技术支持"));
        assert!(output.contains("7×24小时"));
        // 不应误匹配为手机号
        assert!(!output.contains("[联系电话]"));
    }
}
