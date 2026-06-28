//! `output_finding` 工具 — 输出审查结论。
//!
//! 这是 ReAct 循环的"终端"工具。Agent 调用此工具表示证据已经充分，
//! 可以下结论。ReAct 循环检测到此工具调用后立即退出，返回 RiskFinding。
//!
//! 工具实现本身是一个透传：将 LLM 传入的参数原样返回，
//! ReActLoop 从 tool_call arguments 中解析 RiskFinding。

use anyhow::Result;

use super::AgentTool;

/// `output_finding` 工具的参数即 RiskFinding 的所有字段。
/// 这里不重复定义结构体——直接在 definition 中描述 JSON Schema，
/// execute 中透传 LLM 传入的参数。
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
                "description": "输出审查结论——你对该条款的最终判断。\
                    调用此工具表示你认为证据已经充分，可以下结论。\
                    调用后本条款的审查循环立即结束。\
                    【使用场景】① 你已完成完整的推理链——理解条款 → 搜索法规 → 搜索案例 → \
                    （如需要）search_document 发现关联章节 → read_section 精读确认—— \
                    且每一步都有实际证据支撑，此时输出结论。\
                    ② 经过充分搜索确实找不到相关法规/案例支撑风险判定—— \
                    此时输出 no_risk=true，这是负责任的审查结论，不是'没审出来'。\
                    【不使用场景 — 出现以下情况不要输出，继续审查】\
                    ① 搜到了相关法规但还没看到法规全文——先 read_section 或其他方式确认法规内容。\
                    ② search_document 返回了关联条款但你还没精读确认——先 read_section。\
                    ③ 推理链有跳跃（'这看起来像地域歧视'但没说明为什么构成地域歧视、依据哪条法规）\
                    ——补全推理逻辑再输出。\
                    ④ 你只搜了一个 category 且结果为空——换 category 或换搜索词再试一次。\
                    【confidence 校准指南 — 必须诚实】\
                    · ≥0.9: 有法规原文直接适用 + 有案例佐证。\
                    · 0.75–0.89: 有法规原文适用但案例缺失或不完全匹配。\
                    · 0.6–0.74: 仅凭语义判断（'这很像地域歧视'）但没有直接法规支撑—— \
                    此时 severity 不应设为 high，且应标注推荐人工复核。\
                    · <0.6: 不应输出 high severity 的判定；可输出 medium/low 或走人工复核。\
                    【注意】\
                    · no_risk=true 和 severity=high 一样是严肃结论——两者都需要充分证据。\
                    · reason 必须写完整的推理链条（读了什么→搜了什么→为什么这样判定），\
                    不只是重复 risk_type。推理链是后续 Legal Verify 和人工复核的基础。\
                    · source_quote 必须来自 read_section 返回的原文，精确到字。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "risk_id": {
                            "type": "string",
                            "description": "风险唯一标识，格式 R_XXX（如 R_001）。框架会自动填充此字段，LLM 无需输出。"
                        },
                        "no_risk": {
                            "type": "boolean",
                            "description": "是否判定为无风险。true = 该条款合规，不需要后续处理。"
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["high", "medium", "low", "info"],
                            "description": "风险严重程度。high=红线问题必须改，medium=建议修改，low=优化建议，info=信息性（no_risk时必须用info）"
                        },
                        "risk_type": {
                            "type": "string",
                            "description": "风险类型标签。如：地域歧视/品牌指定/程序违规/资质排他/评分倾斜/需求不清/合同违规/格式缺失/组合风险/无风险"
                        },
                        "source_quote": {
                            "type": "string",
                            "description": "从 read_section 返回的原文中逐字摘录的违规文本。no_risk=true 时可为空字符串。"
                        },
                        "legal_basis": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "法条引用列表。每条格式：'法条名称（web_search 返回的 source url）'。如 ['《政府采购法》第5条 https://...', '《实施条例》第20条(二) https://...']。如果搜索结果中无对应 URL，可省略。"
                        },
                        "reason": {
                            "type": "string",
                            "description": "完整推理链：读了什么 → 搜了什么 → 为什么这样判定。引用搜索结果中的法条/案例时，使用 Markdown 链接格式：[法条名称](URL)。例如：'根据[《政府采购法》第5条](https://...)，禁止以地域作为供应商资格条件…'。框架会自动在末尾追加完整的 📎 搜索来源清单。"
                        },
                        "suggestion": {
                            "type": "string",
                            "description": "修改建议。no_risk=true 时可为空字符串。"
                        },
                        "confidence": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "description": "整体置信度。≥0.9=法规+案例双支撑；0.75-0.89=有法规但案例缺失；0.6-0.74=仅语义判断；<0.6=不应输出high"
                        },
                        "clause_ids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "关联的条款chunk_id列表。对于单个条款审查，直接传 [\"ch_XXX\"]。框架会自动填充，此字段可选。"
                        }
                    },
                    "required": ["no_risk", "severity", "risk_type", "source_quote", "legal_basis", "reason", "suggestion", "confidence"]
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value> {
        // output_finding 是终端工具：LLM 传入的参数就是 RiskFinding 的 JSON。
        // ReAct 循环通过检查 tool_call.name == "output_finding" 来检测退出条件，
        // 然后从 tool_call.arguments 中解析 RiskFinding。
        // 这里直接透传参数。
        Ok(args)
    }
}
