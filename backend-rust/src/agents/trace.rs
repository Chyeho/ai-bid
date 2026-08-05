//! 审查追溯日志 — 记录每次 ReAct 循环的完整执行过程。
//!
//! 设计文档 §10.1 定义的 TraceEvent / TraceSession 体系。
//! MVP 阶段：简单的事件列表 + stdout 输出。Phase 3 扩展为 JSONL 文件写入。

use chrono::Utc;
use serde::Serialize;
use std::fs;

/// 追溯事件类型。
#[derive(Debug, Clone, Serialize)]
pub enum TraceEventType {
    #[serde(rename = "session_start")]
    SessionStart,
    #[serde(rename = "session_end")]
    SessionEnd,
    #[serde(rename = "turn_start")]
    TurnStart,
    #[serde(rename = "agent_thought")]
    AgentThought,
    #[serde(rename = "tool_call")]
    ToolCall,
    #[serde(rename = "tool_result")]
    ToolResult,
    #[serde(rename = "output_finding")]
    OutputFinding,
    #[serde(rename = "tier_change")]
    TierChange,
    #[serde(rename = "empty_search")]
    EmptySearch,
    #[serde(rename = "max_turns")]
    MaxTurns,
    #[serde(rename = "agent_bus_recv")]
    AgentBusRecv,
    #[serde(rename = "agent_bus_send")]
    AgentBusSend,
}

/// 单条追溯事件。
#[derive(Debug, Clone, Serialize)]
pub struct TraceEvent {
    /// 事件唯一 ID（UUID v4）
    pub event_id: String,
    /// 审查会话 ID
    pub session_id: String,
    /// Agent 名称
    pub agent_name: String,
    /// 事件类型
    pub event_type: TraceEventType,
    /// 当前 turn 编号
    pub turn: u32,
    /// 时间戳
    pub timestamp: String,
    /// 条款 chunk_id
    pub clause_id: Option<String>,
    /// 风险发现 ID（如已输出）
    pub risk_id: Option<String>,
    /// 事件摘要
    pub summary: String,
    /// 结构化 payload
    pub payload: serde_json::Value,
}

impl TraceEvent {
    pub fn new(
        session_id: &str,
        agent_name: &str,
        event_type: TraceEventType,
        turn: u32,
        clause_id: Option<&str>,
        summary: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            agent_name: agent_name.to_string(),
            event_type,
            turn,
            timestamp: Utc::now().to_rfc3339(),
            clause_id: clause_id.map(|s| s.to_string()),
            risk_id: None,
            summary: summary.to_string(),
            payload,
        }
    }
}

/// 审查追溯日志。
///
/// MVP 阶段：事件收集到内存，审查结束后 flush 到 JSONL 文件。
pub struct TraceLog {
    pub events: Vec<TraceEvent>,
    pub session_id: String,
    pub output_dir: String,
}

impl TraceLog {
    /// 创建新的 TraceLog。
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            session_id: String::new(),
            output_dir: String::new(),
        }
    }

    /// 初始化会话。
    pub fn init_session(&mut self, session_id: &str, output_dir: &str) {
        self.session_id = session_id.to_string();
        self.output_dir = output_dir.to_string();
        self.events.clear();
    }

    /// 记录一个事件。
    pub fn log(
        &mut self,
        event_type: TraceEventType,
        turn: u32,
        clause_id: Option<&str>,
        summary: &str,
        payload: serde_json::Value,
    ) {
        let event = TraceEvent::new(
            &self.session_id,
            "agent",
            event_type,
            turn,
            clause_id,
            summary,
            payload,
        );
        self.events.push(event);
    }

    /// Flush 所有事件到 JSONL 文件。
    pub fn flush(&self) -> Result<(), std::io::Error> {
        if self.events.is_empty() || self.output_dir.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(&self.output_dir)?;
        let path = format!("{}/{}_trace.jsonl", self.output_dir, self.session_id);
        let mut content = String::new();
        for event in &self.events {
            content.push_str(&serde_json::to_string(event).unwrap());
            content.push('\n');
        }
        fs::write(&path, content)?;
        println!("Trace 日志已写入: {} ({} 条事件)", path, self.events.len());
        Ok(())
    }
}

impl Default for TraceLog {
    fn default() -> Self {
        Self::new()
    }
}
