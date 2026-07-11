//! `output_verification_batch` 工具 — 批量法条验证的终端工具。
//!
//! LegalVerifyAgent 在批量模式下使用此工具一次性输出多条验证结论，
//! 触发 ReAct 循环退出（与 `output_finding` 类似）。
//!
//! ## 与 `output_finding` 的区别
//!
//! - `output_finding` — 单条审查结论（主审查流程使用）
//! - `output_verification_batch` — 批量验证结论（LegalVerify 批量模式专用）

use anyhow::Result;
use serde_json::Value;

use super::AgentTool;

pub struct OutputVerificationBatchTool;

#[async_trait::async_trait]
impl AgentTool for OutputVerificationBatchTool {
    fn name(&self) -> &str {
        "output_verification_batch"
    }

    fn definition(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "output_verification_batch",
                "description": "输出所有法条验证的批量结论。每次调用必须包含所有待验证 finding 的完整结果。调用此工具后审查结束。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "verifications": {
                            "type": "array",
                            "description": "所有待验证 finding 的验证结果列表",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "risk_id": {
                                        "type": "string",
                                        "description": "待验证的 risk_id（如 R_001）"
                                    },
                                    "is_valid": {
                                        "type": "boolean",
                                        "description": "法条引用是否真实、准确、适用。true=验证通过，false=需修正"
                                    },
                                    "corrected_legal_basis": {
                                        "type": "array",
                                        "items": { "type": "string" },
                                        "description": "修正后的法条引用列表。is_valid=true 时可复用原始引用；is_valid=false 时必须提供正确的引用（含 URL 链接，Markdown 格式）"
                                    },
                                    "confidence": {
                                        "type": "number",
                                        "minimum": 0.0,
                                        "maximum": 1.0,
                                        "description": "验证置信度。基于搜索结果的法条匹配度、时效性、适用范围"
                                    },
                                    "reason": {
                                        "type": "string",
                                        "description": "验证推理：搜了什么 → 找到了什么 → 为什么通过/修正/降级"
                                    }
                                },
                                "required": ["risk_id", "is_valid", "corrected_legal_basis", "confidence", "reason"]
                            }
                        }
                    },
                    "required": ["verifications"]
                }
            }
        })
    }

    async fn execute(&self, _args: Value) -> Result<Value> {
        // 批量验证的 arguments 由 ReAct 循环直接解析，
        // 不需要在此处做额外处理。
        Ok(serde_json::json!({"status": "batch_verification_received"}))
    }
}
