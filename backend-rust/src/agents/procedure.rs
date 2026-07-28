//! ProcedureAgent — 采购程序合规审查 Agent。
//!
//! 设计文档 §6.2 ② 定义：审查采购方式、公告期限、保证金、评审程序等流程合规性。
//!
//! ## 审查内容
//! - 采购方式选择是否满足法定适用条件
//! - 公告期限是否符合法定要求
//! - 保证金比例是否超标
//! - 评审委员会组成是否合规
//! - 废标/流标处理程序是否明确
//!
//! ## 审查方式
//! 标准 ReAct（4-6 轮）。识别程序条款 → 搜索对应法规 → 逐条对照 → 输出。

use crate::agents::react_loop::{LlmClient, ReActLoop};
use crate::agents::registry::AgentRegistry;
use crate::agents::tools::ToolRegistry;
use crate::agents::types::AgentId;

pub use crate::agents::prompts::PROCEDURE_SYSTEM_PROMPT;

/// 创建 ProcedureAgent 的 ReActLoop 实例。
pub fn create_procedure_agent(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> ReActLoop {
    let registry = AgentRegistry::builtin();
    let def = registry
        .get(AgentId::Procedure)
        .expect("ProcedureAgent 定义必须存在于 AgentRegistry");
    let config = def.to_agent_config();
    ReActLoop::new(config, llm, tools)
}
