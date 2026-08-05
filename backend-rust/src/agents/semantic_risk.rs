//! SemanticRiskAgent — 隐性风险识别 Agent。
//!
//! 设计文档 §6.2 ④ 定义：通过语义分析发现隐性品牌指向、地域偏好、排他性条件。
//!
//! ## 审查内容
//! - 隐性品牌指向（参数组合后恰好只匹配某一品牌）
//! - 隐性地域偏好（"本地服务机构"等变相表述）
//! - 隐性排他性（资格+评分组合后形成壁垒）
//! - 跨条款关联风险（不同章节条款组合后形成排他性）
//!
//! ## 审查方式
//! 深度 ReAct（6-10 轮）。语义分析 + 跨条款搜索 + SessionGraph 关联拓扑利用。

use crate::agents::react_loop::{LlmClient, ReActLoop};
use crate::agents::registry::AgentRegistry;
use crate::agents::tools::ToolRegistry;
use crate::agents::types::AgentId;

pub use crate::agents::prompts::SEMANTIC_RISK_SYSTEM_PROMPT;

/// 创建 SemanticRiskAgent 的 ReActLoop 实例。
pub fn create_semantic_risk_agent(llm: Box<dyn LlmClient>, tools: ToolRegistry) -> ReActLoop {
    let registry = AgentRegistry::builtin();
    let def = registry
        .get(AgentId::SemanticRisk)
        .expect("SemanticRiskAgent 定义必须存在于 AgentRegistry");
    let config = def.to_agent_config();
    ReActLoop::new(config, llm, tools)
}
