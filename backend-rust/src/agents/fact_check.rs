//! FactCheckAgent — 事实核查 Agent。
//!
//! 设计文档 §6.2 ① 定义：从原文提取结构化事实，与法规阈值对照。客观题。
//!
//! ## 审查内容
//! - 时限是否满足法定要求？
//! - 预算金额是否超限、采购方式是否与金额匹配？
//! - 必须章节（资格要求、合同主要条款等）是否齐全？
//! - 项目编号、联系方式等关键格式信息是否完整？
//!
//! ## 审查方式
//! 轻量 ReAct（2-4 轮）。提取事实 → 搜阈值表 → 交叉比对 → 输出。
//! 结构化输出，可程序校验。
//!
//! ## 路由覆盖
//! L1 条款（格式/信息类）→ 仅 FactCheckAgent；L2+ 条款 → 作为辅助为 ProcedureAgent 提供金额/时限提取结果。

use crate::agents::react_loop::{LlmClient, ReActLoop};
use crate::agents::registry::AgentRegistry;
use crate::agents::tools::ToolRegistry;
use crate::agents::types::AgentId;

// 向后兼容：重新导出 system prompt
pub use crate::agents::prompts::FACT_CHECK_SYSTEM_PROMPT;

/// 创建 FactCheckAgent 的 ReActLoop 实例（便捷工厂）。
///
/// Phase 2: 委托给 AgentRegistry::instantiate()（Builder 模式）。
/// 向后兼容原有的直接调用方式。
pub fn create_fact_check_agent(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> ReActLoop {
    let registry = AgentRegistry::builtin();
    let def = registry
        .get(AgentId::FactCheck)
        .expect("FactCheckAgent 定义必须存在于 AgentRegistry");
    let config = def.to_agent_config();
    ReActLoop::new(config, llm, tools)
}
