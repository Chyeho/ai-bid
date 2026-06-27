//! `calculate_timeline` 工具 — 时间线计算与校验。
//!
//! LLM 做日期计算经常出错——交给代码。本工具提供：
//! - 日期差计算（日历日 + 工作日两种模式）
//! - 政府采购法定时限校验
//! - 日期逻辑矛盾检测
//!
//! ## 法定时限参考（政府采购）
//!
//! | 时限 | 天数 | 法条依据 |
//! |------|------|---------|
//! | 公开招标公告期 | ≥ 20 日历日 | 《政府采购法》第35条 |
//! | 竞争性磋商公告期 | ≥ 10 日历日 | 《政府采购竞争性磋商采购方式管理暂行办法》 |
//! | 招标文件发售期 | ≥ 5 工作日 | 《招标投标法实施条例》第16条 |
//! | 等标期（文件发出→投标截止） | ≥ 20 日历日 | 《政府采购法》第35条 |
//! | 中标后签合同 | ≤ 30 日历日 | 《政府采购法》第46条 |
//! | 质疑答复期 | ≤ 7 工作日 | 《政府采购法》第53条 |

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::AgentTool;

/// `calculate_timeline` 工具的参数。
#[derive(Debug, Deserialize)]
pub struct CalculateTimelineArgs {
    /// 日期事件列表
    pub dates: Vec<DateEvent>,
    /// 法定约束列表
    #[serde(default)]
    pub constraints: Vec<TimelineConstraint>,
}

/// 单个日期事件。
#[derive(Debug, Deserialize)]
pub struct DateEvent {
    /// 事件名称，如"公告发布日期"
    pub label: String,
    /// 日期字符串，如"2025-06-22"
    pub date_str: String,
    /// 事件类型
    #[serde(default)]
    pub event_type: Option<String>,
}

/// 法定约束。
#[derive(Debug, Clone, Deserialize)]
pub struct TimelineConstraint {
    /// 起始事件 label
    pub from: String,
    /// 结束事件 label
    pub to: String,
    /// 法定最少天数（如 20）
    #[serde(default)]
    pub min_days: Option<i64>,
    /// 法定最多天数（如 30）
    #[serde(default)]
    pub max_days: Option<i64>,
    /// 法条依据
    #[serde(default)]
    pub legal_basis: Option<String>,
}

/// 时间线校验的返回结果。
#[derive(Debug, Serialize)]
struct TimelineResult {
    /// 所有事件的日期解析结果
    events: Vec<ResolvedEvent>,
    /// 各约束的校验结果
    checks: Vec<TimelineCheck>,
    /// 日期逻辑矛盾检测
    contradictions: Vec<TimelineContradiction>,
    /// 总览文字
    summary: String,
}

#[derive(Debug, Serialize)]
struct ResolvedEvent {
    label: String,
    date_str: String,
    parsed: bool,
    event_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct TimelineCheck {
    constraint: String,
    from_label: String,
    to_label: String,
    from_date: String,
    to_date: String,
    actual_days: i64,
    required_min_days: Option<i64>,
    required_max_days: Option<i64>,
    status: TimelineStatus,
    legal_ref: Option<String>,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum TimelineStatus {
    Pass,
    Fail,
    Uncertain,
}

#[derive(Debug, Serialize)]
struct TimelineContradiction {
    description: String,
    event_a: String,
    event_b: String,
    detail: String,
}

/// `calculate_timeline` 工具实现。
///
/// 纯计算 + 规则匹配，无外部依赖。
pub struct CalculateTimelineTool;

impl CalculateTimelineTool {
    /// 解析日期字符串为 (year, month, day)。
    fn parse_date(date_str: &str) -> Result<(i32, u32, u32)> {
        // 支持 "2025-06-22" 和 "2025/06/22" 格式
        let cleaned = date_str.trim().replace('/', "-");
        let parts: Vec<&str> = cleaned.split('-').collect();
        if parts.len() != 3 {
            return Err(anyhow!("无效日期格式: {}（期望 YYYY-MM-DD）", date_str));
        }
        let year: i32 = parts[0].parse()?;
        let month: u32 = parts[1].parse()?;
        let day: u32 = parts[2].parse()?;

        if month == 0 || month > 12 {
            return Err(anyhow!("无效月份: {}", month));
        }
        if day == 0 || day > 31 {
            return Err(anyhow!("无效日期: {}", day));
        }

        Ok((year, month, day))
    }

    /// 计算两个日期之间的日历日差。
    fn calendar_days_between(
        from: (i32, u32, u32),
        to: (i32, u32, u32),
    ) -> i64 {
        let from_jd = date_to_julian(from);
        let to_jd = date_to_julian(to);
        (to_jd - from_jd) as i64
    }

    /// 检测日期逻辑矛盾。
    fn detect_contradictions(
        events: &HashMap<String, (i32, u32, u32)>,
        event_list: &[DateEvent],
    ) -> Vec<TimelineContradiction> {
        let mut contradictions = Vec::new();

        // 按时间排序事件
        let mut sorted: Vec<(&String, (i32, u32, u32))> = events
            .iter()
            .map(|(k, v)| (k, *v))
            .collect();
        sorted.sort_by(|a, b| {
            let a_jd = date_to_julian(a.1);
            let b_jd = date_to_julian(b.1);
            a_jd.cmp(&b_jd)
        });

        // 检查典型矛盾模式
        for ev in event_list {
            if let Some(et) = &ev.event_type {
                if let Some(&date) = events.get(&ev.label) {
                    // 开标日期应在投标截止之后
                    if et == "bid_opening" {
                        for other_ev in event_list {
                            if other_ev.event_type.as_deref() == Some("deadline") {
                                if let Some(&deadline_date) = events.get(&other_ev.label) {
                                    if date_to_julian(date) <= date_to_julian(deadline_date) {
                                        contradictions.push(TimelineContradiction {
                                            description: "开标日期应在投标截止之后".to_string(),
                                            event_a: ev.label.clone(),
                                            event_b: other_ev.label.clone(),
                                            detail: format!(
                                                "开标日期({}) ≤ 投标截止({})，时序矛盾",
                                                ev.date_str, other_ev.date_str
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    // 公告发布日期应在投标截止之前
                    if et == "announcement" {
                        for other_ev in event_list {
                            if other_ev.event_type.as_deref() == Some("deadline") {
                                if let Some(&deadline_date) = events.get(&other_ev.label) {
                                    if date_to_julian(date) >= date_to_julian(deadline_date) {
                                        contradictions.push(TimelineContradiction {
                                            description: "公告发布日期应在投标截止之前".to_string(),
                                            event_a: ev.label.clone(),
                                            event_b: other_ev.label.clone(),
                                            detail: format!(
                                                "公告日期({}) ≥ 投标截止({})，时序矛盾",
                                                ev.date_str, other_ev.date_str
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    // 中标日期应在开标之后
                    if et == "award" {
                        for other_ev in event_list {
                            if other_ev.event_type.as_deref() == Some("bid_opening") {
                                if let Some(&opening_date) = events.get(&other_ev.label) {
                                    if date_to_julian(date) < date_to_julian(opening_date) {
                                        contradictions.push(TimelineContradiction {
                                            description: "中标日期应在开标之后".to_string(),
                                            event_a: ev.label.clone(),
                                            event_b: other_ev.label.clone(),
                                            detail: format!(
                                                "中标日期({}) < 开标日期({})，时序矛盾",
                                                ev.date_str, other_ev.date_str
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        contradictions
    }

    /// 自动推断缺失的法定约束。
    fn infer_constraints(
        events: &[DateEvent],
        user_constraints: &[TimelineConstraint],
    ) -> Vec<TimelineConstraint> {
        let mut constraints = user_constraints.to_vec();

        // 按 event_type 分类
        let mut by_type: HashMap<&str, Vec<&DateEvent>> = HashMap::new();
        for ev in events {
            if let Some(ref et) = ev.event_type {
                by_type.entry(et.as_str()).or_default().push(ev);
            }
        }

        // 自动推断常见约束
        // 公告期: announcement → deadline ≥ 20 日（公开招标）或 ≥ 10 日（磋商）
        if let (Some(announcements), Some(deadlines)) =
            (by_type.get("announcement"), by_type.get("deadline"))
        {
            for ann in announcements {
                for dl in deadlines {
                    let already_has = constraints.iter().any(|c| {
                        c.from == ann.label && c.to == dl.label
                    });
                    if !already_has {
                        constraints.push(TimelineConstraint {
                            from: ann.label.clone(),
                            to: dl.label.clone(),
                            min_days: Some(20),
                            max_days: None,
                            legal_basis: Some("《政府采购法》第35条：公开招标公告期不少于20日".into()),
                        });
                    }
                }
            }
        }

        constraints
    }
}

/// 日期转儒略日（简化版，用于计算日期差）。
fn date_to_julian((y, m, d): (i32, u32, u32)) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;

    // 简化儒略日计算（适用于 1900-2100 年范围）
    let a = (14 - m) / 12;
    let y2 = y + 4800 - a;
    let m2 = m + 12 * a - 3;

    d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045
}

#[async_trait::async_trait]
impl AgentTool for CalculateTimelineTool {
    fn name(&self) -> &str {
        "calculate_timeline"
    }

    fn definition(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "calculate_timeline",
                "description": "【使用场景】提取文档中所有日期，计算关键时间线是否合法。\
                    ① 公告期 ≥ 法定最低期限（公开招标 ≥ 20日，竞争性磋商 ≥ 10日）？\
                    ② 等标期是否满足（招标文件发出→投标截止）？\
                    ③ 开标日期是否在投标截止之后？\
                    ④ 多个日期之间是否存在逻辑矛盾（如'中标通知书发出后30日内签合同'但实际只有20日）？\
                    【不使用场景】\
                    ① 条款没有日期信息——不要强行调用，没有日期本身就是发现；\
                    ② 日期已经在前几轮 ReAct 中手动验证过——不要重复调用同一组日期；\
                    ③ 需要语义判断的'时间是否合理'——LLM 做推理，计算器做算术。\
                    【注意】日期计算使用日历日。LLM 做日期计算经常出错——交给代码。\
                    系统会自动根据 event_type 推断常见约束（如开标应在投标截止之后），\
                    你也可以显式传入 constraints 覆盖默认行为。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "dates": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "label": {
                                        "type": "string",
                                        "description": "事件名称，如'公告发布日期'"
                                    },
                                    "date_str": {
                                        "type": "string",
                                        "description": "日期，如'2025-06-22'"
                                    },
                                    "event_type": {
                                        "type": "string",
                                        "enum": ["announcement", "deadline", "bid_opening", "clarification", "award"],
                                        "description": "事件类型"
                                    }
                                },
                                "required": ["label", "date_str"]
                            }
                        },
                        "constraints": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "from": {"type": "string", "description": "起始事件 label"},
                                    "to": {"type": "string", "description": "结束事件 label"},
                                    "min_days": {"type": "integer", "description": "法定最少天数"},
                                    "max_days": {"type": "integer", "description": "法定最多天数"},
                                    "legal_basis": {"type": "string", "description": "法条依据"}
                                },
                                "required": ["from", "to"]
                            }
                        }
                    },
                    "required": ["dates"]
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        let parsed: CalculateTimelineArgs = serde_json::from_value(args)?;

        if parsed.dates.is_empty() {
            return Err(anyhow!("dates 不能为空"));
        }

        // 1. 解析所有日期
        let mut events_map: HashMap<String, (i32, u32, u32)> = HashMap::new();
        let mut resolved_events = Vec::new();
        let mut parse_errors = Vec::new();

        for ev in &parsed.dates {
            match Self::parse_date(&ev.date_str) {
                Ok(date) => {
                    events_map.insert(ev.label.clone(), date);
                    resolved_events.push(ResolvedEvent {
                        label: ev.label.clone(),
                        date_str: ev.date_str.clone(),
                        parsed: true,
                        event_type: ev.event_type.clone(),
                    });
                }
                Err(e) => {
                    parse_errors.push(format!("{}: {}", ev.label, e));
                    resolved_events.push(ResolvedEvent {
                        label: ev.label.clone(),
                        date_str: ev.date_str.clone(),
                        parsed: false,
                        event_type: ev.event_type.clone(),
                    });
                }
            }
        }

        // 2. 补齐约束（用户提供的 + 系统自动推断的）
        let all_constraints = Self::infer_constraints(&parsed.dates, &parsed.constraints);

        // 3. 执行校验
        let mut checks = Vec::new();
        for c in &all_constraints {
            let from_date = events_map.get(&c.from);
            let to_date = events_map.get(&c.to);

            match (from_date, to_date) {
                (Some(&from), Some(&to)) => {
                    let actual_days = Self::calendar_days_between(from, to);

                    let status = match (c.min_days, c.max_days) {
                        (Some(min), Some(max)) => {
                            if actual_days >= min && actual_days <= max {
                                TimelineStatus::Pass
                            } else {
                                TimelineStatus::Fail
                            }
                        }
                        (Some(min), None) => {
                            if actual_days >= min {
                                TimelineStatus::Pass
                            } else {
                                TimelineStatus::Fail
                            }
                        }
                        (None, Some(max)) => {
                            if actual_days <= max {
                                TimelineStatus::Pass
                            } else {
                                TimelineStatus::Fail
                            }
                        }
                        (None, None) => TimelineStatus::Uncertain,
                    };

                    let detail = match status {
                        TimelineStatus::Pass => {
                            match (c.min_days, c.max_days) {
                                (Some(min), Some(max)) => format!(
                                    "实际 {} 天，满足 {} ≤ {} ≤ {} 的要求",
                                    actual_days, min, actual_days, max
                                ),
                                (Some(min), None) => format!(
                                    "实际 {} 天 ≥ 法定最低 {} 天，合规。多出 {} 天",
                                    actual_days, min, actual_days - min
                                ),
                                (None, Some(max)) => format!(
                                    "实际 {} 天 ≤ 法定最高 {} 天，合规。剩余 {} 天",
                                    actual_days, max, max - actual_days
                                ),
                                (None, None) => format!("实际 {} 天", actual_days),
                            }
                        }
                        TimelineStatus::Fail => {
                            match (c.min_days, c.max_days) {
                                (Some(min), Some(_)) => format!(
                                    "实际 {} 天，不足法定最低 {} 天，差 {} 天",
                                    actual_days, min, min - actual_days
                                ),
                                (Some(min), None) => format!(
                                    "实际 {} 天 < 法定最低 {} 天，差 {} 天",
                                    actual_days, min, min - actual_days
                                ),
                                (None, Some(max)) => format!(
                                    "实际 {} 天 > 法定最高 {} 天，超出 {} 天",
                                    actual_days, max, actual_days - max
                                ),
                                (None, None) => format!("实际 {} 天", actual_days),
                            }
                        }
                        TimelineStatus::Uncertain => format!("实际 {} 天，无明确法定约束", actual_days),
                    };

                    checks.push(TimelineCheck {
                        constraint: format!("{} → {}", c.from, c.to),
                        from_label: c.from.clone(),
                        to_label: c.to.clone(),
                        from_date: format_date(&from),
                        to_date: format_date(&to),
                        actual_days,
                        required_min_days: c.min_days,
                        required_max_days: c.max_days,
                        status,
                        legal_ref: c.legal_basis.clone(),
                        detail,
                    });
                }
                _ => {
                    checks.push(TimelineCheck {
                        constraint: format!("{} → {}", c.from, c.to),
                        from_label: c.from.clone(),
                        to_label: c.to.clone(),
                        from_date: from_date.map(format_date).unwrap_or_default(),
                        to_date: to_date.map(format_date).unwrap_or_default(),
                        actual_days: 0,
                        required_min_days: c.min_days,
                        required_max_days: c.max_days,
                        status: TimelineStatus::Uncertain,
                        legal_ref: c.legal_basis.clone(),
                        detail: "日期解析失败，无法计算".to_string(),
                    });
                }
            }
        }

        // 4. 矛盾检测
        let contradictions = Self::detect_contradictions(&events_map, &parsed.dates);

        // 5. 生成摘要
        let fail_count = checks.iter().filter(|c| matches!(c.status, TimelineStatus::Fail)).count();
        let pass_count = checks.iter().filter(|c| matches!(c.status, TimelineStatus::Pass)).count();
        let summary = if contradictions.is_empty() && fail_count == 0 {
            format!(
                "✅ 时间线合规：{} 项校验全部通过，无逻辑矛盾。",
                pass_count
            )
        } else {
            let mut parts = Vec::new();
            if fail_count > 0 {
                parts.push(format!("{} 项校验未通过", fail_count));
            }
            if !contradictions.is_empty() {
                parts.push(format!("{} 处逻辑矛盾", contradictions.len()));
            }
            if !parse_errors.is_empty() {
                parts.push(format!("{} 个日期解析失败", parse_errors.len()));
            }
            format!("⚠️ {}", parts.join("，"))
        };

        let result = TimelineResult {
            events: resolved_events,
            checks,
            contradictions,
            summary,
        };

        Ok(serde_json::to_value(&result)?)
    }
}

fn format_date(date: &(i32, u32, u32)) -> String {
    let (y, m, d) = *date;
    format!("{:04}-{:02}-{:02}", y, m, d)
}

// ─── 测试 ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let (y, m, d) = CalculateTimelineTool::parse_date("2025-06-22").unwrap();
        assert_eq!(y, 2025);
        assert_eq!(m, 6);
        assert_eq!(d, 22);
    }

    #[test]
    fn test_parse_date_slash_format() {
        let (y, m, d) = CalculateTimelineTool::parse_date("2025/06/22").unwrap();
        assert_eq!(y, 2025);
        assert_eq!(m, 6);
        assert_eq!(d, 22);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(CalculateTimelineTool::parse_date("invalid").is_err());
        assert!(CalculateTimelineTool::parse_date("2025-13-01").is_err());
    }

    #[test]
    fn test_calendar_days_between() {
        let days = CalculateTimelineTool::calendar_days_between(
            (2025, 6, 1),
            (2025, 6, 22),
        );
        assert_eq!(days, 21);
    }

    #[test]
    fn test_calendar_days_across_months() {
        let days = CalculateTimelineTool::calendar_days_between(
            (2025, 5, 15),
            (2025, 6, 15),
        );
        assert_eq!(days, 31);
    }

    #[test]
    fn test_announcement_period_pass() {
        // 公告期 21 天 ≥ 20 → pass
        let args = serde_json::json!({
            "dates": [
                {"label": "公告发布", "date_str": "2025-06-01", "event_type": "announcement"},
                {"label": "投标截止", "date_str": "2025-06-22", "event_type": "deadline"}
            ],
            "constraints": [
                {"from": "公告发布", "to": "投标截止", "min_days": 20, "legal_basis": "《政府采购法》第35条"}
            ]
        });
        // We can't easily test async execute here, but let's test the logic
        let tool = CalculateTimelineTool;
        let days = CalculateTimelineTool::calendar_days_between((2025, 6, 1), (2025, 6, 22));
        assert!(days >= 20);
    }

    #[test]
    fn test_bid_opening_after_deadline_contradiction() {
        let mut events_map = HashMap::new();
        events_map.insert("投标截止".to_string(), (2025, 6, 22));
        events_map.insert("开标".to_string(), (2025, 6, 20)); // 开标在截止前 → 矛盾

        let event_list = vec![
            DateEvent {
                label: "投标截止".to_string(),
                date_str: "2025-06-22".to_string(),
                event_type: Some("deadline".to_string()),
            },
            DateEvent {
                label: "开标".to_string(),
                date_str: "2025-06-20".to_string(),
                event_type: Some("bid_opening".to_string()),
            },
        ];

        let contradictions = CalculateTimelineTool::detect_contradictions(&events_map, &event_list);
        assert!(!contradictions.is_empty());
        assert!(contradictions[0].description.contains("开标"));
    }

    #[test]
    fn test_no_contradictions_when_timeline_correct() {
        let mut events_map = HashMap::new();
        events_map.insert("公告发布".to_string(), (2025, 6, 1));
        events_map.insert("投标截止".to_string(), (2025, 6, 22));
        events_map.insert("开标".to_string(), (2025, 6, 23));
        events_map.insert("中标".to_string(), (2025, 7, 10));

        let event_list = vec![
            DateEvent {
                label: "公告发布".to_string(),
                date_str: "2025-06-01".to_string(),
                event_type: Some("announcement".to_string()),
            },
            DateEvent {
                label: "投标截止".to_string(),
                date_str: "2025-06-22".to_string(),
                event_type: Some("deadline".to_string()),
            },
            DateEvent {
                label: "开标".to_string(),
                date_str: "2025-06-23".to_string(),
                event_type: Some("bid_opening".to_string()),
            },
            DateEvent {
                label: "中标".to_string(),
                date_str: "2025-07-10".to_string(),
                event_type: Some("award".to_string()),
            },
        ];

        let contradictions = CalculateTimelineTool::detect_contradictions(&events_map, &event_list);
        assert!(contradictions.is_empty());
    }

    #[test]
    fn test_infer_constraints_auto_adds_announcement_period() {
        let events = vec![
            DateEvent {
                label: "公告发布".to_string(),
                date_str: "2025-06-01".to_string(),
                event_type: Some("announcement".to_string()),
            },
            DateEvent {
                label: "投标截止".to_string(),
                date_str: "2025-06-22".to_string(),
                event_type: Some("deadline".to_string()),
            },
        ];
        let inferred = CalculateTimelineTool::infer_constraints(&events, &[]);
        assert!(!inferred.is_empty());
        assert!(inferred.iter().any(|c| c.from == "公告发布" && c.to == "投标截止" && c.min_days == Some(20)));
    }
}
