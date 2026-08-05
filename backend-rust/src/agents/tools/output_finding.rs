//! `output_finding` 终端工具。
//!
//! 工具名为兼容旧 Agent 配置而保留；新协议一次输出零到多条风险发现。

use anyhow::Result;

use super::AgentTool;

pub struct OutputFindingTool;

#[async_trait::async_trait]
impl AgentTool for OutputFindingTool {
    fn name(&self) -> &str {
        "output_finding"
    }

    fn definition(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "output_finding",
                "description": "批量输出当前条款的最终审查结论。必须逐段检查并一次列出所有相互独立的问题；\
                    例如同一条款同时存在地域限制、保证金超限、单方变更时，应输出3条finding。\
                    没有风险时返回findings=[]，不要创建“无风险”占位项。\
                    findings最多5条；若仍有独立问题未展开，has_more=true。\
                    每条source_quote只引用支撑该风险的最小充分原文；每条reason独立说明事实、规则和结论。\
                    confidence应诚实校准：≥0.9=法规与案例双支撑；0.75-0.89=有直接法规；\
                    0.6-0.74=主要依赖语义判断；低于0.6不得输出high。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "findings": {
                            "type": "array",
                            "maxItems": 5,
                            "description": "相互独立的风险发现；无风险时为空数组。",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "no_risk": {
                                        "type": "boolean",
                                        "description": "批量格式中必须为false；无风险请返回空findings。"
                                    },
                                    "severity": {
                                        "type": "string",
                                        "enum": ["high", "medium", "low", "info"],
                                        "description": "high=必须修改，medium=建议修改，low=优化建议，info=信息提示。"
                                    },
                                    "is_critical": {
                                        "type": "boolean",
                                        "description": "是否属于会造成不合理排除、投标无效或严重破坏公平竞争的重大/红线问题。重大问题仍使用severity=high。"
                                    },
                                    "critical_reason": {
                                        "type": "string",
                                        "description": "重大问题判定依据；非重大问题必须为空字符串。"
                                    },
                                    "risk_type": {
                                        "type": "string",
                                        "description": "面向用户的风险类型标签。"
                                    },
                                    "category_code": {
                                        "type": "string",
                                        "enum": [
                                            "LOCAL_REGISTRATION", "BRAND_LOCK", "UNRELATED_CERT",
                                            "REGIONAL_PERFORMANCE", "SCALE_THRESHOLD",
                                            "SHORT_DEADLINE", "EXCESSIVE_DEPOSIT", "OEM_AUTHORIZATION",
                                            "SUBJECTIVE_SCORING", "LOCAL_AWARD", "VAGUE_ACCEPTANCE",
                                            "UNBOUNDED_IP", "UNILATERAL_CHANGE", "CONFLICTING_DATES",
                                            "UNCLEAR_PENALTY", "OTHER"
                                        ],
                                        "description": "稳定风险分类编码。必须从枚举选择；确实不属于已知15类时使用OTHER。"
                                    },
                                    "source_quote": {
                                        "type": "string",
                                        "description": "从条款原文逐字摘录、只支撑本条风险的最小充分证据。"
                                    },
                                    "legal_basis": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                        "description": "直接支撑本条风险的法条引用列表。"
                                    },
                                    "reason": {
                                        "type": "string",
                                        "description": "针对本条风险的完整推理链：原文事实→适用规则→风险结论。"
                                    },
                                    "suggestion": {
                                        "type": "string",
                                        "description": "针对本条风险的可执行修改建议。"
                                    },
                                    "confidence": {
                                        "type": "number",
                                        "minimum": 0.0,
                                        "maximum": 1.0,
                                        "description": "本条风险的置信度。"
                                    }
                                },
                                "required": [
                                    "no_risk", "severity", "is_critical", "critical_reason",
                                    "risk_type", "category_code", "source_quote", "legal_basis",
                                    "reason", "suggestion", "confidence"
                                ]
                            }
                        },
                        "has_more": {
                            "type": "boolean",
                            "description": "是否仍有因5条上限或上下文不足而未展开的独立问题。"
                        },
                        "coverage": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "已检查的风险域，如qualification、procedure、scoring、demand、contract。"
                        }
                    },
                    "required": ["findings", "has_more", "coverage"]
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        Ok(args)
    }
}
