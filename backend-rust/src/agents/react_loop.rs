//! ReAct 循环引擎 — Agent 审查的核心运行时。
//!
//! 设计文档 §7.2-7.3 定义的 while 循环模式：
//! ```text
//! for turn in range(MAX_TURNS):      // ← Rust 代码在循环
//!     response = llm.chat(conversation, tools=[...])
//!     if response.has_tool_call("output_finding"):
//!         return risk_finding          // Agent 认为证据够了，输出
//!     // 否则执行工具调用，结果追加到对话历史
//! ```
//!
//! ## 条款级风险分级 (L1/L2/L3)
//!
//! 每条条款携带 Coordinator 预判的 tier，控制 max_turns：
//! - L1: 6 turns（纯信息/格式条款）
//! - L2: 10 turns（标准审查）
//! - L3: 12 turns（深度审查）
//!
//! 审查过程中支持动态升降级（turn 2 检测）。

use crate::agents::bus::{AgentBus, BusMessage};
use crate::agents::review_event::{ReviewEvent, ReviewEventBus};
use crate::agents::session_graph::SessionGraph;
use crate::agents::trace::{TraceEventType, TraceLog};
use crate::agents::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::task::JoinSet;

// ─── LLM 客户端抽象 ───────────────────────────────────────────

/// LLM 返回的工具调用。
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// LLM 的一次响应。
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// 文本回复（可能为 None，当 LLM 只返回 tool_calls 时）
    pub content: Option<String>,
    /// LLM 在调用工具前的推理/思考文本（ReAct Thought）。
    ///
    /// 来源优先级:
    /// 1. API 响应中的 `reasoning_content` 字段（DeepSeek-R1、qwq 等推理模型）
    /// 2. 当 `content` 与 `tool_calls` 同时存在时，`content` 识别为 thought
    /// 3. 仅 content（无工具调用）→ content 直接作为回答，thought 为 None
    pub thought: Option<String>,
    /// 工具调用列表
    pub tool_calls: Vec<ToolCall>,
}

impl LlmResponse {
    /// 检查是否包含 output_finding 工具调用（触发批量审查循环退出）。
    pub fn has_output_finding(&self) -> bool {
        self.tool_calls.iter().any(|tc| tc.name == "output_finding")
    }

    /// 获取第一个 output_finding 工具调用的 arguments（RiskFinding JSON）。
    pub fn get_finding(&self) -> Option<&serde_json::Value> {
        self.tool_calls
            .iter()
            .find(|tc| tc.name == "output_finding")
            .map(|tc| &tc.arguments)
    }

    /// 检查是否包含 answer_user 工具调用（触发 ChatAgent 循环退出）。
    pub fn has_answer_user(&self) -> bool {
        self.tool_calls.iter().any(|tc| tc.name == "answer_user")
    }

    /// 获取第一个 answer_user 工具调用的 arguments（构建 ChatResponse 用）。
    pub fn get_answer(&self) -> Option<&serde_json::Value> {
        self.tool_calls
            .iter()
            .find(|tc| tc.name == "answer_user")
            .map(|tc| &tc.arguments)
    }
}

/// ★ 工具选择策略 —— 控制 LLM 是否必须调用工具。
///
/// 用于解决 LLM 在 react_loop 中以文本输出结论、拒绝调用 `output_finding`
/// 工具的问题。引擎可以通过此参数主动收回终止控制权。
#[derive(Debug, Clone)]
pub enum ToolChoice {
    /// 不限制 —— LLM 自由选择文本回复或工具调用（默认行为）
    Auto,
    /// 必须调用某个工具（不限具体哪个）
    Required,
    /// 只能调用指定的工具（强制终止 —— 用于最后一轮）
    Specific { name: String },
}

impl ToolChoice {
    /// 序列化为 DashScope API 的 `tool_choice` 字段值。
    pub fn to_dashscope_value(&self) -> serde_json::Value {
        match self {
            ToolChoice::Auto => serde_json::Value::Null,
            ToolChoice::Required => serde_json::json!("required"),
            ToolChoice::Specific { name } => serde_json::json!({
                "type": "function",
                "function": { "name": name }
            }),
        }
    }

    /// 序列化为 OpenAI 兼容 API 的 `tool_choice` 字段值。
    pub fn to_openai_value(&self) -> serde_json::Value {
        match self {
            ToolChoice::Auto => serde_json::json!("auto"),
            ToolChoice::Required => serde_json::json!("required"),
            ToolChoice::Specific { name } => serde_json::json!({
                "type": "function",
                "function": { "name": name }
            }),
        }
    }
}

/// LLM 客户端抽象 trait。
///
/// 解耦 ReAct 循环与具体 LLM 提供商。
/// MVP 使用 OpenAI 兼容 API，后续可添加 Anthropic 原生实现。
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    /// 发送消息到 LLM，返回响应。
    ///
    /// * `messages` — 对话历史（system/user/assistant/tool 消息）
    /// * `tools` — 可用工具的 JSON Schema 定义列表
    /// * `tool_choice` — ★ 工具选择策略（Auto/Required/Specific）
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
        tool_choice: &ToolChoice,
    ) -> Result<LlmResponse>;
}

// ─── 对话消息类型 ─────────────────────────────────────────────

/// ReAct 循环中使用的对话消息（与提供商无关）。
#[derive(Debug, Clone)]
pub enum ChatMessage {
    System { content: String },
    User { content: String },
    Assistant {
        content: Option<String>,
        tool_calls: Option<Vec<ToolCall>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}

// ─── 共享 Helper ───────────────────────────────────────────────

/// 执行 LLM 返回的工具调用并将结果追加到对话历史。
///
/// ★ 这是 ReActLoop 和 ChatAgent 共享的公共逻辑。
/// 批量审查特有的逻辑（搜索缓存、空结果升级、打印日志）
/// 保留在 ReActLoop::react_loop() 内部，不纳入此 helper。
pub async fn execute_tool_calls(
    response: &LlmResponse,
    tools: &crate::agents::tools::ToolRegistry,
    conversation: &mut Vec<ChatMessage>,
) -> Result<()> {
    let assistant_tool_calls = response.tool_calls.clone();
    conversation.push(ChatMessage::Assistant {
        content: response.content.clone(),
        tool_calls: if assistant_tool_calls.is_empty() {
            None
        } else {
            Some(assistant_tool_calls.clone())
        },
    });

    // 如果没有工具调用，提示继续
    if assistant_tool_calls.is_empty() {
        conversation.push(ChatMessage::User {
            content: "请继续——调用工具搜索证据或输出结论。".to_string(),
        });
        return Ok(());
    }

    for tc in &assistant_tool_calls {
        let result = if let Some(tool) = tools.get(&tc.name) {
            match tool.execute(tc.arguments.clone()).await {
                Ok(val) => val,
                Err(e) => serde_json::json!({ "error": format!("{}", e) }),
            }
        } else {
            serde_json::json!({
                "error": format!("工具 '{}' 未注册", tc.name)
            })
        };
        conversation.push(ChatMessage::Tool {
            tool_call_id: tc.id.clone(),
            content: serde_json::to_string(&result).unwrap_or_default(),
        });
    }
    Ok(())
}

// ─── ReActLoop ─────────────────────────────────────────────────

/// ReAct 循环引擎 — Agent 审查的运行时。
///
/// 持有 LLM 客户端、工具注册表、AgentBus 引用、SessionGraph 引用、TraceLog 引用。
/// 每个 ReActLoop 实例对应一个 Agent 类型（FactCheck / Procedure / SemanticRisk / ...）。
pub struct ReActLoop {
    /// Agent 配置
    pub config: AgentConfig,
    /// LLM 客户端
    pub llm: Box<dyn LlmClient>,
    /// 工具注册表
    pub tools: crate::agents::tools::ToolRegistry,
    /// AgentBus 发送端（可选，MVP 阶段可省略）
    pub bus: Option<Arc<AgentBus>>,
    /// ★ Agent 持有的专属 Receiver（通过 bus.subscribe() 获取）
    /// 每轮 try_recv() 循环排空，避免多 Agent 并发下消息丢失
    pub bus_rx: Option<Mutex<broadcast::Receiver<BusMessage>>>,
    /// ★ SessionGraph 引用（Blackboard 拉取侧）
    pub graph: Option<Arc<SessionGraph>>,
    /// 搜索缓存：(query, category) → 搜索结果 JSON
    pub search_cache: Mutex<HashMap<(String, String), serde_json::Value>>,
    /// 审查追溯日志
    pub trace: Arc<Mutex<TraceLog>>,
    /// ★ stderr 打印锁：多个 Agent 并行时，确保每个 Agent 的多行日志块不交叠。
    /// 仅用于 eprintln 序列化，不在 await 期间持有。
    pub print_lock: Option<Arc<std::sync::Mutex<()>>>,
    /// SSE 实时推送通道（可选，仅 HTTP server 模式启用）
    pub review_events: Option<Arc<ReviewEventBus>>,
}

impl ReActLoop {
    /// 创建新的 ReActLoop 实例。
    pub fn new(
        config: AgentConfig,
        llm: Box<dyn LlmClient>,
        tools: crate::agents::tools::ToolRegistry,
    ) -> Self {
        Self {
            config,
            llm,
            tools,
            bus: None,
            bus_rx: None,
            graph: None,
            search_cache: Mutex::new(HashMap::new()),
            trace: Arc::new(Mutex::new(TraceLog::new())),
            print_lock: None,
            review_events: None,
        }
    }

    /// 设置 AgentBus（同时持有 Sender + 专属 Receiver）。
    ///
    /// ★ Phase 2 增强：调用 `bus.subscribe()` 获取 Agent 专属 Receiver，
    /// 存入 `bus_rx`。此后每轮 `try_recv()` 循环排空，避免多 Agent 并发丢消息。
    pub fn with_bus(mut self, bus: Arc<AgentBus>) -> Self {
        let rx = bus.subscribe();
        self.bus_rx = Some(Mutex::new(rx));
        self.bus = Some(bus);
        self
    }

    /// ★ 新增: 设置 SessionGraph（Blackboard 拉取侧）。
    pub fn with_graph(mut self, graph: Arc<SessionGraph>) -> Self {
        self.graph = Some(graph);
        self
    }

    /// ★ 设置 stderr 打印锁，确保并行 Agent 的多行日志不交叠。
    pub fn with_print_lock(mut self, lock: Arc<std::sync::Mutex<()>>) -> Self {
        self.print_lock = Some(lock);
        self
    }

    /// 设置 SSE 实时推送通道（仅在 HTTP server 模式下启用）。
    pub fn with_review_events(mut self, events: Arc<ReviewEventBus>) -> Self {
        self.review_events = Some(events);
        self
    }

    // ── 主入口 ──────────────────────────────────────────────

    /// 审查一组条款。每个条款运行独立的 ReAct 循环。
    pub async fn review(&self, clauses: &[ReviewClause]) -> Vec<RiskFinding> {
        let mut findings = Vec::with_capacity(clauses.len());
        let total = clauses.len();

        for (idx, clause) in clauses.iter().enumerate() {
            // 优先从 SessionGraph 获取全局唯一 risk_id，避免多 Agent 并发下 ID 碰撞。
            // 无 graph 时（LegalVerify/Debate 等独立 ReActLoop）回退到 per-agent 编号。
            let risk_id = self
                .graph
                .as_ref()
                .map(|g| g.next_risk_id())
                .unwrap_or_else(|| format!("R_{:03}", idx + 1));
            let finding = self.react_loop(clause, &risk_id).await;
            findings.push(finding);

            // 每审完一条条款后，发送 AgentProgress（SSE 实时推送）
            if let Some(ref events) = self.review_events {
                let raw_findings = findings.iter().filter(|f| !f.no_risk).count();
                events.emit(&ReviewEvent::AgentProgress {
                    agent_id: self.config.name.clone(),
                    agent_label: self.config.name.clone(),
                    clauses_done: idx + 1,
                    clauses_total: total,
                    raw_findings,
                    status: "running".to_string(),
                });
            }
        }

        findings
    }

    /// 审查单条条款（公开入口，供并行调度使用）。
    ///
    /// 与 `react_loop` 功能相同，但作为公开 API 暴露，
    /// 使外部并行调度器可以为每条条款创建独立 task。
    pub async fn review_single(&self, clause: &ReviewClause, risk_id: &str) -> RiskFinding {
        self.react_loop(clause, risk_id).await
    }

    // ── 核心 ReAct 循环 ─────────────────────────────────────

    /// 单条款 ReAct 循环。
    ///
    /// ```text
    /// conversation = [system_prompt, user(clause_text)]
    /// while turn < max_turns:
    ///     poll AgentBus → inject bus messages
    ///     response = llm.chat(conversation, tools)
    ///     if output_finding → parse RiskFinding, exit
    ///     execute tool_calls → append results
    /// max_turns exhausted → force_output
    /// ```
    async fn react_loop(&self, clause: &ReviewClause, risk_id: &str) -> RiskFinding {
        let agent_name = &self.config.name;
        let initial_tier = clause.tier;
        let max_turns = clause.effective_max_turns(self.config.default_max_turns);
        let mut tier = initial_tier;
        let mut tier_escalated = false;
        let mut consecutive_empty_searches = 0u32;
        let mut web_search_count = 0u32; // 硬性限制 web_search 调用次数
        let mut consecutive_duplicate_searches = 0u32; // 连续重复搜索结果计数
        let mut last_search_urls: Vec<String> = Vec::new(); // 上次搜索的 top-3 URL

        // ── 条款头日志 ──
        let _print_lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
        let sep = "═".repeat(60);
        eprintln!("\n{sep}\n{rid} | {cid} | tier={tier} | max_turns={max} | pages {ps}-{pe}\n{sep}",
            sep=sep, rid=risk_id, cid=clause.chunk_id, tier=initial_tier, max=max_turns,
            ps=clause.page_start+1, pe=clause.page_end+1);
        eprintln!("章节: {}", clause.section_path.join(" > "));
        let text_preview = if clause.text.chars().count() > 500 {
            format!("{}…[截断]", clause.text.chars().take(500).collect::<String>())
        } else {
            clause.text.clone()
        };
        eprintln!("条款文本 ({} 字符):\n{}\n", clause.text.chars().count(), text_preview);
        drop(_print_lock);

        // 构建初始对话
        let mut conversation: Vec<ChatMessage> = vec![
            ChatMessage::System {
                content: self.config.system_prompt.clone(),
            },
            ChatMessage::User {
                content: self.format_clause_prompt(clause),
            },
        ];

        let mut turn = 0u32;
        while turn < max_turns as u32 {
            turn += 1;

            // SSE: turn_start
            if let Some(ref events) = self.review_events {
                events.emit(&ReviewEvent::Trace {
                    event_type: "turn_start".to_string(),
                    agent_name: agent_name.clone(),
                    turn,
                    clause_id: Some(clause.chunk_id.clone()),
                    summary: format!("{} 第 {} 轮审查", agent_name, turn),
                    payload: None,
                });
            }

            // ── Step 0a: Query SessionGraph — 拉取已知上下文 ──
            if let Some(graph) = &self.graph {
                let ctx = graph.query_clause_context(&clause.chunk_id);
                if ctx.has_prior_risks() || !ctx.reviewed_by.is_empty() {
                    let mut graph_msg = String::from("[Session 记忆] 以下条款已被审查或存在已知发现:\n");

                    if !ctx.reviewed_by.is_empty() {
                        graph_msg.push_str(&format!(
                            "已审查 Agent: {}\n",
                            ctx.reviewed_by
                                .iter()
                                .map(|a| a.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }

                    if ctx.has_prior_risks() {
                        graph_msg.push_str("已知风险:\n");
                        graph_msg.push_str(&ctx.risk_summary());
                    }

                    if !ctx.linked_chunks.is_empty() {
                        graph_msg.push_str("\n关联条款:\n");
                        for lc in &ctx.linked_chunks {
                            graph_msg.push_str(&format!(
                                "- {} ({})\n",
                                lc.chunk_id, lc.reason
                            ));
                        }
                    }

                    if !ctx.same_law_chunks.is_empty() {
                        graph_msg.push_str("\n引用相同法条的其他条款:\n");
                        for cid in &ctx.same_law_chunks {
                            graph_msg.push_str(&format!("- {}\n", cid));
                        }
                    }

                    if !ctx.contradictions.is_empty() {
                        graph_msg.push_str("\n⚠️ 已知条款矛盾:\n");
                        for lc in &ctx.contradictions {
                            graph_msg.push_str(&format!(
                                "- 与 {} 矛盾: {}\n",
                                lc.chunk_id, lc.reason
                            ));
                        }
                    }

                    conversation.push(ChatMessage::System {
                        content: graph_msg,
                    });
                }

                // 记录当前 Agent 已审查此条款
                if let Some(agent_id) = AgentId::from_str(agent_name) {
                    graph.add_reviewed_by(&clause.chunk_id, agent_id);
                }
            }

            // ── Step 0b: AgentBus poll — 使用 Agent 持有的 Receiver 增量拉取 ──
            if let Some(rx) = &self.bus_rx {
                let mut rx_guard = rx.lock().await;
                while let Ok(msg) = rx_guard.try_recv() {
                    // 不接收自己发送的消息
                    let own_id = AgentId::from_str(agent_name);
                    if own_id.map_or(true, |oid| msg.from != oid) {
                        // Trace: 记录接收事件
                        {
                            let mut trace = self.trace.lock().await;
                            trace.log(
                                TraceEventType::AgentBusRecv,
                                turn,
                                Some(&clause.chunk_id),
                                &format!(
                                    "Received bus msg from {}: {}",
                                    msg.from, msg.summary
                                ),
                                serde_json::json!({
                                    "from": msg.from.to_string(),
                                    "risk_type": msg.risk_type,
                                    "clause_ids": msg.clause_ids,
                                    "topic": format!("{:?}", msg.topic),
                                }),
                            );
                        }
                        conversation.push(ChatMessage::System {
                            content: format!(
                                "[AgentBus] {} 发现 {} 风险: {}\n涉及条款: {}\n如果你审查的条款与此相关，用 search_document 和 read_section 做交叉验证。",
                                msg.from, msg.severity, msg.summary,
                                msg.clause_ids.join(", ")
                            ),
                        });
                    }
                }
            }

            // ── Step 2: LLM 推理 ──

            // ── Turn 剩余轮次预警 + tool_choice 控制 ──
            let remaining = max_turns as u32 - turn;
            let tool_choice = if remaining <= 1 {
                // 最后一轮：锁定 output_finding，引擎收回终止控制权
                ToolChoice::Specific { name: "output_finding".to_string() }
            } else if remaining == 2 {
                // 倒数第二轮：要求必须调用工具，阻止纯文本输出
                ToolChoice::Required
            } else {
                ToolChoice::Auto
            };

            if remaining == 3 {
                conversation.push(ChatMessage::System {
                    content: format!(
                        "⏳ 剩余 {} 轮审查机会（条款 {}）。请开始汇总已收集的信息，减少探索性搜索。\n如果已有足够证据判定风险（或无风险），即可准备调用 output_finding。",
                        remaining, clause.chunk_id
                    ),
                });
            } else if remaining == 2 {
                conversation.push(ChatMessage::System {
                    content: format!(
                        "⚠️ 剩余 {} 轮审查机会（条款 {}）。请汇总已收集的信息，准备调用 output_finding。\n不要再开启新的搜索方向——基于已有信息做出判定即可。",
                        remaining, clause.chunk_id
                    ),
                });
            } else if remaining <= 1 {
                conversation.push(ChatMessage::System {
                    content: format!(
                        "🛑 这是对条款 {} 的最后一轮审查！本轮**只能**调用 output_finding 输出结论。\n\
                         no_risk=true 也比被截断好——截断会丢失所有已完成的审查工作。\n\
                         立即基于已收集的信息 + 条款原文 + 已知法规常识，调用 output_finding。",
                        clause.chunk_id
                    ),
                });
            }

            let tool_defs = self.tools.definitions_filtered(&self.config.tool_names);
            let response = match self
                .llm
                .chat(&conversation, &tool_defs, &tool_choice)
                .await
            {
                Ok(r) => {
                    // SSE: agent_thought (推理摘要)
                    if let Some(ref events) = self.review_events {
                        let thought_summary = r.content.as_ref()
                            .map(|c| c.chars().take(200).collect::<String>())
                            .unwrap_or_default();
                        if !thought_summary.is_empty() {
                            events.emit(&ReviewEvent::Trace {
                                event_type: "agent_thought".to_string(),
                                agent_name: agent_name.clone(),
                                turn,
                                clause_id: Some(clause.chunk_id.clone()),
                                summary: thought_summary,
                                payload: None,
                            });
                        }
                    }

                    // ── 详细日志：LLM 响应（加锁避免并行 Agent 交叠）──
                    {
                        let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
                        eprintln!(
                            "\n── [{agent} turn {turn}/{max}] ─────────────────────────────────────────────",
                            agent = agent_name, turn = turn, max = max_turns
                        );
                        // 完整输出 LLM 的推理内容（不截断）
                        if let Some(ref content) = r.content {
                            if !content.is_empty() {
                                eprintln!("💭 推理内容 ({} 字符):", content.chars().count());
                                for line in content.lines() {
                                    eprintln!("   {}", line);
                                }
                            }
                        }
                        // 工具调用及参数
                        if !r.tool_calls.is_empty() {
                            eprintln!("🔧 工具调用 ({} 个):", r.tool_calls.len());
                            for tc in &r.tool_calls {
                                let args_str = serde_json::to_string(&tc.arguments)
                                    .unwrap_or_else(|_| "(序列化失败)".to_string());
                                eprintln!("   → {} (id={})", tc.name, tc.id);
                                eprintln!("      args: {}", args_str);
                            }
                        } else {
                            eprintln!("🔧 工具调用: (无)");
                        }
                    } // 释放打印锁
                    r
                }
                Err(e) => {
                    // LLM 调用失败 → 输出错误 finding
                    return RiskFinding {
                        risk_id: risk_id.to_string(),
                        clause_ids: vec![clause.chunk_id.clone()],
                        block_ids: Vec::new(),
                        agent: agent_name.clone(),
                        no_risk: true,
                        severity: RiskSeverity::Info,
                        risk_type: "LLM调用失败".to_string(),
                        source_quote: String::new(),
                        legal_basis: Vec::new(),
                        case_refs: Vec::new(),
                        reason: format!("LLM API 调用失败: {}", e),
                        suggestion: "请检查 API 配置后重试。".to_string(),
                        confidence: 0.0,
                        initial_tier,
                        final_tier: tier,
                        tier_escalated,
                        truncated: false,
                        suggested_agent: None,
                        citations: Vec::new(),
                        page_number: None,
                        section_path: None,
                        context: None,
                    };
                }
            };

            // ── Step 2.5: 二次 AgentBus poll ──
            // Step 0b 的 poll 发生在 LLM 调用之前。如果其他 Agent 的广播
            // 恰好在 LLM 调用期间到达，Step 0b 会错过。此处补 poll 一次，
            // 确保在 output_finding 之前能感知到最新的跨 Agent 消息。
            if let Some(rx) = &self.bus_rx {
                let mut rx_guard = rx.lock().await;
                while let Ok(msg) = rx_guard.try_recv() {
                    let own_id = AgentId::from_str(agent_name);
                    if own_id.map_or(true, |oid| msg.from != oid) {
                        {
                            let mut trace = self.trace.lock().await;
                            trace.log(
                                TraceEventType::AgentBusRecv,
                                turn,
                                Some(&clause.chunk_id),
                                &format!(
                                    "Late bus msg from {}: {}",
                                    msg.from, msg.summary
                                ),
                                serde_json::json!({
                                    "from": msg.from.to_string(),
                                    "risk_type": msg.risk_type,
                                    "clause_ids": msg.clause_ids,
                                    "topic": format!("{:?}", msg.topic),
                                    "stage": "pre_output",
                                }),
                            );
                        }
                        conversation.push(ChatMessage::System {
                            content: format!(
                                "[AgentBus] {} 发现 {} 风险: {}\n涉及条款: {}\n如果你审查的条款与此相关，用 search_document 和 read_section 做交叉验证。",
                                msg.from, msg.severity, msg.summary,
                                msg.clause_ids.join(", ")
                            ),
                        });
                    }
                }
            }

            // ── Step 3: 检查 output_finding ──
            if response.has_output_finding() {
                if let Some(args) = response.get_finding() {
                    // ── 始终打印 output_finding 原始参数（加锁）──
                    let raw_pretty = serde_json::to_string_pretty(args);
                    {
                        let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
                        eprintln!("📤 output_finding 原始参数:");
                        eprintln!("{}", raw_pretty.as_deref().unwrap_or(&format!("{:?}", args)));
                    }

                    // ── 预处理 args：修复 LLM 常见的 JSON 格式错误 ──
                    let mut fixed_args = args.clone();
                    // 修复 clause_ids: 如果是字符串 "[...]" → 尝试解析为数组
                    if let Some(cids) = fixed_args.get("clause_ids") {
                        if cids.is_string() {
                            if let Ok(parsed) = serde_json::from_str::<Vec<String>>(cids.as_str().unwrap()) {
                                fixed_args["clause_ids"] = serde_json::json!(parsed);
                            } else {
                                // 无法解析，删除让 #[serde(default)] 兜底
                                fixed_args.as_object_mut().unwrap().remove("clause_ids");
                            }
                        }
                    }
                    // 确保 clause_ids 存在（#[serde(default)] 已处理，但显式添加更安全）
                    if !fixed_args.as_object().map(|o| o.contains_key("clause_ids")).unwrap_or(false) {
                        fixed_args["clause_ids"] = serde_json::json!([]);
                    }

                    match serde_json::from_value::<RiskFinding>(fixed_args) {
                        Ok(mut finding) => {
                            finding.clause_ids = vec![clause.chunk_id.clone()];
                            finding.agent = agent_name.clone();
                            finding.initial_tier = initial_tier;
                            finding.final_tier = tier;
                            finding.tier_escalated = tier_escalated;
                            finding.truncated = false;
                            finding.risk_id = risk_id.to_string();

                            // ── 自动填充定位字段（§6.4 框架填充约定） ──
                            // types.rs 注释声明"框架从关联 ReviewClause 自动填充"，
                            // 此前缺失实现 → 前端收到 page_number=null → 显示"页码待定位"。
                            // +1: clause.page_start 是 0-based，前端 PDF 页码是 1-based
                            // 且 JS 中 0 是 falsy 会导致 if(!page)return 短路
                            finding.page_number = Some(clause.page_start + 1);
                            finding.section_path = Some(clause.section_path.clone());
                            finding.context = Some(clause.text.chars().take(500).collect());
                            // block_ids 暂不自动填充（clause 不含 source_block_ids），
                            // 前端会走 text-match / bbox-api fallback 路径。

                            // ── 自动填充 citations：从 search_cache 提取所有搜索来源 URL ──
                            finding.citations = self.extract_citations().await;
                            if !finding.citations.is_empty() {
                                let refs: Vec<String> = finding
                                    .citations
                                    .iter()
                                    .enumerate()
                                    .map(|(i, c)| {
                                        if c.site_name.is_empty() {
                                            format!("[{}] {} — {}", i + 1, c.title, c.url)
                                        } else {
                                            format!(
                                                "[{}] {} — {} ({})",
                                                i + 1,
                                                c.title,
                                                c.url,
                                                c.site_name
                                            )
                                        }
                                    })
                                    .collect();
                                finding.reason = format!(
                                    "{}\n\n📎 搜索来源:\n{}",
                                    finding.reason,
                                    refs.join("\n")
                                );
                            }

                            {
                                let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
                                eprintln!("✅ output_finding 解析成功，审查完成");
                            }

                            // SSE: output_finding
                            if let Some(ref events) = self.review_events {
                                let sev_str = match finding.severity {
                                    RiskSeverity::High => "high",
                                    RiskSeverity::Medium => "medium",
                                    RiskSeverity::Low => "low",
                                    RiskSeverity::Info => "info",
                                };
                                events.emit(&ReviewEvent::Trace {
                                    event_type: "output_finding".to_string(),
                                    agent_name: agent_name.clone(),
                                    turn,
                                    clause_id: Some(clause.chunk_id.clone()),
                                    summary: format!("发现: {} ({})", finding.risk_type, sev_str),
                                    payload: Some(serde_json::json!({
                                        "risk_id": risk_id,
                                        "severity": sev_str,
                                        "risk_type": finding.risk_type,
                                        "confidence": finding.confidence,
                                    })),
                                });
                            }

                            // ── 实时广播 High 风险到 AgentBus ──
                            // 不等所有条款完成，让其他 Agent 在本轮即可感知
                            if finding.severity == RiskSeverity::High {
                                if let Some(bus) = &self.bus {
                                    if let Some(agent_id) = AgentId::from_str(agent_name) {
                                        bus.broadcast(
                                            agent_id.clone(),
                                            finding.severity,
                                            &finding.reason,
                                            &finding.clause_ids,
                                            &finding.risk_type,
                                        );
                                        // Trace: 记录发送事件
                                        {
                                            let mut trace = self.trace.lock().await;
                                            trace.log(
                                                TraceEventType::AgentBusSend,
                                                turn,
                                                Some(&clause.chunk_id),
                                                &format!(
                                                    "High risk broadcast: {} ({})",
                                                    finding.risk_type, finding.severity
                                                ),
                                                serde_json::json!({
                                                    "from": agent_id.to_string(),
                                                    "risk_type": finding.risk_type,
                                                    "clause_ids": finding.clause_ids,
                                                    "severity": "high",
                                                }),
                                            );
                                        }
                                        // SSE: agent_bus_send
                                        if let Some(ref events) = self.review_events {
                                            events.emit(&ReviewEvent::Trace {
                                                event_type: "agent_bus_send".to_string(),
                                                agent_name: agent_name.clone(),
                                                turn,
                                                clause_id: Some(clause.chunk_id.clone()),
                                                summary: format!("High risk broadcast: {} ({})", finding.risk_type, finding.severity),
                                                payload: Some(serde_json::json!({
                                                    "from": agent_id.to_string(),
                                                    "risk_type": finding.risk_type,
                                                    "clause_ids": finding.clause_ids,
                                                    "severity": "high",
                                                })),
                                            });
                                        }
                                    }
                                }
                            }

                            return finding;
                        }
                        Err(e) => {
                            // ── 关键调试：打印原始 JSON + 解析错误（加锁）──
                            let raw_json = serde_json::to_string_pretty(args)
                                .unwrap_or_else(|_| format!("{:?}", args));
                            {
                                let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
                                eprintln!(
                                    "⚠️  output_finding JSON 解析失败!\n\
                                     ─── LLM 原始 output_finding arguments ───\n\
                                     {raw}\n\
                                     ─── END ───\n\
                                     解析错误: {err}\n\
                                     期望 8 个必填字段: no_risk, severity, risk_type, source_quote, legal_basis, reason, suggestion, confidence",
                                    raw = raw_json, err = e,
                                );
                            }
                            // 追加详细的重试提示
                            conversation.push(ChatMessage::Tool {
                                tool_call_id: "output_finding".to_string(),
                                content: format!(
                                    "output_finding 参数解析错误: {}\n\
                                     请检查是否包含全部 8 个必填字段:\n\
                                     no_risk, severity, risk_type, source_quote,\n\
                                     legal_basis, reason, suggestion, confidence\n\
                                     legal_basis 必须是数组（即使为空写 []）。\n\
                                     clause_ids 必须是字符串数组，如 [\"ch_000\"]。\n\
                                     请修正后重新调用 output_finding。",
                                    e
                                ),
                            });
                            continue;
                        }
                    }
                }
            }

            // ── Step 4: 执行工具调用 ──
            // 先追加 assistant 消息
            let assistant_tool_calls: Vec<ToolCall> = response.tool_calls.clone();
            conversation.push(ChatMessage::Assistant {
                content: response.content,
                tool_calls: if assistant_tool_calls.is_empty() {
                    None
                } else {
                    Some(assistant_tool_calls.clone())
                },
            });

            // 如果没有工具调用且没有 output_finding，LLM 只是回复了文本
            // 追加一个提示让它继续
            if assistant_tool_calls.is_empty() {
                conversation.push(ChatMessage::User {
                    content: "请继续审查——调用工具搜索证据或输出结论。如果证据已充分，请调用 output_finding。"
                        .to_string(),
                });
                continue;
            }

            // 执行每个工具调用
            for tc in &assistant_tool_calls {
                let tool_name = &tc.name;

                // SSE: tool_call
                if let Some(ref events) = self.review_events {
                    let query = tc.arguments.get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let summary = if query.is_empty() {
                        format!("调用工具: {}", tool_name)
                    } else {
                        format!("{}: {}", tool_name,
                            query.chars().take(80).collect::<String>())
                    };
                    events.emit(&ReviewEvent::Trace {
                        event_type: "tool_call".to_string(),
                        agent_name: agent_name.clone(),
                        turn,
                        clause_id: Some(clause.chunk_id.clone()),
                        summary,
                        payload: None,
                    });
                }

                // 搜索缓存逻辑（search_knowledge / web_search 共用）
                let result = if self.is_search_tool(tool_name) {
                    self.cached_search_knowledge(&tc.arguments).await
                } else if let Some(tool) = self.tools.get(tool_name) {
                    match tool.execute(tc.arguments.clone()).await {
                        Ok(val) => val,
                        Err(e) => serde_json::json!({ "error": format!("{}", e) }),
                    }
                } else {
                    let available: Vec<String> = self.tools.definitions_filtered(&self.config.tool_names)
                        .iter()
                        .filter_map(|d| d.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()).map(String::from))
                        .collect();
                    serde_json::json!({
                        "error": format!("工具 '{}' 未注册。当前可用工具: {}。请只使用以上工具。", tool_name, available.join(", "))
                    })
                };

                // ── 工具结果摘要（加锁避免并行 Agent 交叠）──
                {
                    let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
                    match tool_name.as_str() {
                        "read_section" => {
                            let title = result.get("section_path")
                                .and_then(|p| p.as_array())
                                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" > "))
                                .unwrap_or_else(|| "(未知)".to_string());
                            let chars = result.get("char_count")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                            eprintln!("📖 read_section → {} ({} 字符)", title, chars);
                        }
                        "search_knowledge" | "web_search" | "search_document" => {
                            let hit_count = Self::count_search_hits(&result);
                            let query = tc.arguments.get("query").and_then(|q| q.as_str()).unwrap_or("?");
                            let cat = tc.arguments.get("category").and_then(|c| c.as_str()).unwrap_or("");
                            let cat_str = if cat.is_empty() { String::new() } else { format!(" [{}]", cat) };
                            eprintln!("🔍 {} → \"{}\"{} = {} 条结果", tool_name, query, cat_str, hit_count);
                            // 打印前几条标题（兼容 sources 和 hits 两种格式）
                            let items: Option<&Vec<serde_json::Value>> = result
                                .get("sources")
                                .and_then(|s| s.as_array())
                                .or_else(|| result.get("hits").and_then(|h| h.as_array()))
                                .or_else(|| result.as_array());
                            if let Some(arr) = items {
                                for (i, h) in arr.iter().take(3).enumerate() {
                                    let t = h.get("title").and_then(|t| t.as_str()).unwrap_or("?");
                                    // WebSource 格式 (DashScope/SearXNG): title + url
                                    let url = h.get("url").and_then(|u| u.as_str()).unwrap_or("");
                                    if !url.is_empty() {
                                        eprintln!("   #{}. {} — {}", i + 1, t, url);
                                    } else {
                                        // SearchHit 格式 (search_document): title + score + snippet
                                        let s = h.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
                                        let snip = h.get("snippet").and_then(|s| s.as_str()).unwrap_or("");
                                        let snip_short: String = snip.chars().take(200).collect();
                                        eprintln!("   #{}. [score={:.2}] {} — {}", i + 1, s, t, snip_short);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                } // 释放打印锁

                // SSE: tool_result
                if let Some(ref events) = self.review_events {
                    let summary = if self.is_search_tool(tool_name) {
                        let hits = Self::count_search_hits(&result);
                        format!("{} 返回 {} 条结果", tool_name, hits)
                    } else if tool_name == "read_section" {
                        let chars = result.get("char_count")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        format!("read_section 返回 {} 字符", chars)
                    } else {
                        format!("{} 执行完成", tool_name)
                    };
                    let payload = if self.is_search_tool(tool_name) || tool_name == "search_document" {
                        let sources = Self::extract_search_sources_for_sse(&result, 5);
                        if sources.is_empty() {
                            None
                        } else {
                            Some(serde_json::json!({ "sources": sources }))
                        }
                    } else {
                        None
                    };
                    events.emit(&ReviewEvent::Trace {
                        event_type: "tool_result".to_string(),
                        agent_name: agent_name.clone(),
                        turn,
                        clause_id: Some(clause.chunk_id.clone()),
                        summary,
                        payload,
                    });
                }

                // 检测空搜索结果
                if self.is_search_tool(tool_name) || tool_name == "search_document" {
                    let hit_count = Self::count_search_hits(&result);

                    if hit_count == 0 {
                        consecutive_empty_searches += 1;
                        // ── 三级空结果升级策略 ──
                        if consecutive_empty_searches == 1 {
                            // L1: 第 1 次空 → 温和提示换策略
                            conversation.push(ChatMessage::Tool {
                                tool_call_id: tc.id.clone(),
                                content: serde_json::to_string(&result).unwrap_or_default(),
                            });
                            conversation.push(ChatMessage::System {
                                content: "搜索返回 0 条结果。换一组关键词或换 category 再试一次。若下次仍为空，必须基于已读原文+已知法规常识直接 output_finding。"
                                    .to_string(),
                            });
                            continue;
                        } else if consecutive_empty_searches == 2 {
                            // L2: 连续 2 次空 → 强硬指令：禁止再搜
                            conversation.push(ChatMessage::Tool {
                                tool_call_id: tc.id.clone(),
                                content: serde_json::to_string(&result).unwrap_or_default(),
                            });
                            conversation.push(ChatMessage::System {
                                content: "🛑 连续 2 次搜索返回空结果。你已用完搜索机会。\n\
                                    禁止调用 web_search 或 search_document。\n\
                                    基于已读的条款原文 + 已知的法规常识，立即调用 output_finding 输出结论。\n\
                                    在 reason 开头标注：『搜索未返回结果，以下判定基于已知法规常识。』\n\
                                    不要再搜索了。现在调用 output_finding。"
                                    .to_string(),
                            });
                            continue;
                        } else {
                            // L3: 连续 3+ 次空（Agent 无视了 L2 指令）→ 最后通牒
                            conversation.push(ChatMessage::Tool {
                                tool_call_id: tc.id.clone(),
                                content: serde_json::to_string(&result).unwrap_or_default(),
                            });
                            conversation.push(ChatMessage::System {
                                content: "⛔ 这是第 3 次空搜索。你的下一个动作必须是 output_finding。\n\
                                    不调用 output_finding 将导致 max_turns 耗尽、审查截断。\n\
                                    立即输出 output_finding，no_risk 设为 true 亦可。"
                                    .to_string(),
                            });
                            // 不 continue——让正常流程追加 tool result（保持对话一致性）
                        }
                    } else {
                        consecutive_empty_searches = 0;

                        // ── 搜索重复检测：若连续 2 次搜索返回相同 top-3 URL，触发强制 output ──
                        if self.is_search_tool(tool_name) {
                            let current_urls = Self::extract_top_urls(&result, 3);
                            if !current_urls.is_empty() && current_urls == last_search_urls {
                                consecutive_duplicate_searches += 1;
                                if consecutive_duplicate_searches >= 2 {
                                    conversation.push(ChatMessage::Tool {
                                        tool_call_id: tc.id.clone(),
                                        content: serde_json::to_string(&result).unwrap_or_default(),
                                    });
                                    conversation.push(ChatMessage::System {
                                        content: "🛑 连续 2 次搜索返回相同的结果列表，搜索引擎对不同 query 返回了相同内容。\n\
                                            你已用完有效的搜索机会。禁止再调用 web_search。\n\
                                            基于已搜索到的信息 + 条款原文 + 已知法规常识，立即调用 output_finding 输出结论。\n\
                                            在 reason 开头标注：『联网搜索未返回差异化结果，以下判定基于已知法规常识。』\n\
                                            不要再搜索了。现在调用 output_finding。"
                                            .to_string(),
                                    });
                                    continue;
                                }
                            } else {
                                consecutive_duplicate_searches = 0;
                                last_search_urls = current_urls;
                            }
                        }

                        // 硬性限制: web_search 最多 5 次（与 prompt 中的约束一致）
                        if self.is_search_tool(tool_name) {
                            web_search_count += 1;
                            if web_search_count >= 5 {
                                conversation.push(ChatMessage::Tool {
                                    tool_call_id: tc.id.clone(),
                                    content: serde_json::to_string(&result).unwrap_or_default(),
                                });
                                conversation.push(ChatMessage::System {
                                    content: "🛑 你已调用 web_search 5 次，达到硬性上限。\n\
                                        禁止再调用 web_search。\n\
                                        基于已搜索到的信息 + 条款原文 + 已知法规常识，立即调用 output_finding 输出结论。\n\
                                        不要再搜索了。现在调用 output_finding。"
                                            .to_string(),
                                });
                                continue;
                            }
                        }
                    }
                }

                conversation.push(ChatMessage::Tool {
                    tool_call_id: tc.id.clone(),
                    content: serde_json::to_string(&result).unwrap_or_default(),
                });
            }

            // ── Step 5: Turn 2 动态升降级检测 ──
            if turn == 2 {
                let (new_tier, escalated) =
                    self.check_tier_escalation(&conversation, initial_tier, tier_escalated);
                if new_tier != tier {
                    let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
                    eprintln!("🔄 分级变化: {} → {} (escalated={})", tier, new_tier, escalated);
                }
                tier = new_tier;
                tier_escalated = escalated;
            }
        }

        // ── max_turns 耗尽 → 强制输出 ──
        let summary = format!(
            "执行了 {} 轮审查（上限 {} 轮），Agent: {}，条款: {}",
            turn, max_turns, agent_name, clause.chunk_id
        );
        {
            let _lock = self.print_lock.as_ref().map(|l| l.lock().unwrap());
            eprintln!("⛔ max_turns 耗尽! {}", summary);
        }
        RiskFinding::truncated_finding(
            risk_id.to_string(),
            clause.chunk_id.clone(),
            agent_name,
            initial_tier,
            tier,
            &summary,
        )
    }

    // ── 辅助方法 ────────────────────────────────────────────

    /// 格式化为发送给 Agent 的条款审查提示。
    fn format_clause_prompt(&self, clause: &ReviewClause) -> String {
        let tier_hint = match clause.tier {
            RiskTier::Low => "【L1 - 快速扫描】此条款为格式/信息类，风险极低。请在 2-6 轮内提取关键事实、与阈值对照后输出结论（no_risk=true 或若有格式缺失则标记）。",
            RiskTier::Medium => "【L2 - 标准审查】请按标准流程审查：精读条款 → 搜索法规 → 搜索案例（如需要）→ 输出结论。",
            RiskTier::High => "【L3 - 深度审查】此条款含高风险关键词（品牌/地域/排他性），请深度审查：web_search(法规→案例) → search_document(跨条款联动) → read_section(精读确认) → 输出结论。",
        };

        format!(
            "{}\n\n【条款信息】\nchunk_id: {}\n章节路径: {}\n页码: {}-{}\n\n【条款文本】\n{}",
            tier_hint,
            clause.chunk_id,
            clause.section_path.join(" > "),
            clause.page_start + 1,
            clause.page_end + 1,
            clause.text
        )
    }

    /// Turn 2 动态升降级检测。
    ///
    /// 检查前 2 轮对话内容，判断是否需要升级或降级。
    /// - 升级触发：Agent 表达了高风险怀疑（"可能存在"/"值得深挖"/"疑似"）
    /// - 降级触发：无高风险信号
    fn check_tier_escalation(
        &self,
        conversation: &[ChatMessage],
        current_tier: RiskTier,
        already_escalated: bool,
    ) -> (RiskTier, bool) {
        if already_escalated {
            return (current_tier, true);
        }

        // 拼接前 2 轮对话检查 Agent 是否表达了高风险怀疑
        let suspicious_phrases = [
            "可能存在", "值得深挖", "需要进一步", "不排除", "疑似",
            "涉嫌", "潜在风险", "值得关注", "需进一步核实",
        ];

        let combined: String = conversation
            .iter()
            .filter_map(|msg| match msg {
                ChatMessage::Assistant { content, .. } => content.clone(),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        let has_suspicious = suspicious_phrases
            .iter()
            .any(|phrase| combined.contains(phrase));

        match (current_tier, has_suspicious) {
            // L1/L2 + 可疑信号 → 升级到 L3
            (RiskTier::Low, true) | (RiskTier::Medium, true) => {
                (RiskTier::High, true)
            }
            // L3 + 无信号 → 降级到 L2
            (RiskTier::High, false) => {
                (RiskTier::Medium, false)
            }
            _ => (current_tier, false),
        }
    }

    /// 判断是否为搜索类工具（兼容新旧工具名）。
    fn is_search_tool(&self, name: &str) -> bool {
        name == "web_search" || name == "search_knowledge"
    }

    /// 统计搜索结果条数，兼容多种后端格式。
    ///
    /// - DashScope/SearXNG (web_search): JSON 中包含 `sources` 数组
    /// - search_document: JSON 中包含 `hits` 数组
    /// - 旧格式: 结果本身是顶层数组
    fn count_search_hits(result: &serde_json::Value) -> usize {
        // 1) DashScope / SearXNG 统一格式: { "answer": "...", "sources": [...] }
        if let Some(arr) = result.get("sources").and_then(|s| s.as_array()) {
            return arr.len();
        }
        // 2) search_document 格式: { "hits": [...] }
        if let Some(arr) = result.get("hits").and_then(|h| h.as_array()) {
            return arr.len();
        }
        // 3) 旧/未知格式：顶层数组
        result.as_array().map(|a| a.len()).unwrap_or(0)
    }

    /// 从搜索结果中提取 top-N 的 source URL，用于重复检测。
    ///
    /// 兼容 DashScope / SearXNG 格式的 `sources` 数组。
    fn extract_top_urls(result: &serde_json::Value, n: usize) -> Vec<String> {
        if let Some(arr) = result.get("sources").and_then(|s| s.as_array()) {
            arr.iter()
                .filter_map(|item| {
                    item.get("url")
                        .or_else(|| item.get("link"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                })
                .take(n)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 从搜索结果中提取 top-N 条 {title, url}，用于 SSE 推送前端展示。
    fn extract_search_sources_for_sse(result: &serde_json::Value, n: usize) -> Vec<serde_json::Value> {
        let mut items: Vec<serde_json::Value> = Vec::new();
        // 1) DashScope / SearXNG: sources 数组
        if let Some(arr) = result.get("sources").and_then(|s| s.as_array()) {
            for item in arr.iter().take(n) {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let url = item.get("url")
                    .or_else(|| item.get("link"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !url.is_empty() {
                    items.push(serde_json::json!({ "title": title, "url": url }));
                }
            }
        }
        // 2) search_document: hits 数组（没有 URL，用 score + snippet 代替）
        if items.is_empty() {
            if let Some(arr) = result.get("hits").and_then(|h| h.as_array()) {
                for item in arr.iter().take(n) {
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    if !title.is_empty() {
                        items.push(serde_json::json!({
                            "title": title,
                            "score": format!("{:.2}", score)
                        }));
                    }
                }
            }
        }
        items
    }

    /// 带缓存的 search_knowledge / web_search 调用。
    async fn cached_search_knowledge(
        &self,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        let query = args
            .get("query")
            .and_then(|q| q.as_str())
            .unwrap_or("");
        let category = args
            .get("category")
            .and_then(|c| c.as_str())
            .unwrap_or("法规");

        let cache_key = (query.to_string(), category.to_string());

        // 检查缓存
        {
            let cache = self.search_cache.lock().await;
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        // 执行实际搜索
        // 同时兼容旧名 search_knowledge 和新名 web_search
        let result = if let Some(tool) = self.tools.get("web_search")
            .or_else(|| self.tools.get("search_knowledge"))
        {
            match tool.execute(args.clone()).await {
                Ok(val) => val,
                Err(e) => serde_json::json!({ "error": format!("{}", e) }),
            }
        } else {
            serde_json::json!({ "error": "web_search / search_knowledge 工具未注册" })
        };

        // 写入缓存
        {
            let mut cache = self.search_cache.lock().await;
            cache.insert(cache_key, result.clone());
        }

        result
    }

    /// 从 search_cache 中提取所有搜索来源 URL，去重后返回 Citation 列表。
    ///
    /// 遍历 search_cache 中每条搜索结果，提取 `sources` 数组中的
    /// `(title, url, site_name)` 三元组，按 URL 去重（同一 URL 只保留首次出现）。
    /// 用于自动填充 RiskFinding.citations 字段。
    async fn extract_citations(&self) -> Vec<Citation> {
        let cache = self.search_cache.lock().await;
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut citations: Vec<Citation> = Vec::new();

        for value in cache.values() {
            if let Some(sources) = value.get("sources").and_then(|s| s.as_array()) {
                for source in sources {
                    let url = source
                        .get("url")
                        .and_then(|u| u.as_str())
                        .unwrap_or("")
                        .to_string();
                    // 跳过空 URL 或已见过的 URL
                    if url.is_empty() || seen_urls.contains(&url) {
                        continue;
                    }
                    seen_urls.insert(url.clone());
                    citations.push(Citation {
                        title: source
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string(),
                        url,
                        site_name: source
                            .get("site_name")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }

        citations
    }
}

// ─── 并行条款审查调度器 ────────────────────────────────────────

/// 并行审查多条条款。
///
/// 为每条条款创建独立的 LLM 客户端和工具集，通过 `tokio::task::JoinSet`
/// 并行执行 ReAct 循环，受 `Semaphore` 控制最大并发数。
///
/// # 参数
///
/// * `clauses` — 待审查条款列表
/// * `make_agent` — Agent 工厂闭包，接收 (LLM客户端, 工具集) → 返回完全配置的 ReActLoop
/// * `llm_factory` — LLM 客户端工厂（每条 task 调用一次，创建独立实例）
/// * `tools_factory` — 工具集工厂（每条 task 调用一次，创建独立实例）
/// * `max_parallel` — 最大并行审查条款数（Semaphore permits）
/// * `graph` — SessionGraph（用于生成全局唯一 risk_id，None 时回退到索引编号）
/// * `review_events` — SSE 推送通道（None 时不推送）
/// * `agent_name` — Agent 名称（用于日志和进度事件）
#[allow(clippy::too_many_arguments)]
pub async fn review_clauses_parallel<F>(
    clauses: &[ReviewClause],
    make_agent: F,
    llm_factory: &(dyn Fn() -> Box<dyn LlmClient> + Send + Sync),
    tools_factory: &(dyn Fn() -> crate::agents::tools::ToolRegistry + Send + Sync),
    max_parallel: usize,
    graph: Option<Arc<SessionGraph>>,
    review_events: Option<Arc<ReviewEventBus>>,
    agent_name: &str,
) -> Vec<RiskFinding>
where
    F: Fn(Box<dyn LlmClient>, crate::agents::tools::ToolRegistry) -> ReActLoop + Send + Sync + 'static,
{
    if clauses.is_empty() {
        return vec![];
    }

    let sem = Arc::new(tokio::sync::Semaphore::new(max_parallel.max(1)));
    let total = clauses.len();
    let done = Arc::new(AtomicUsize::new(0));
    let raw_findings_total = Arc::new(AtomicUsize::new(0));
    let mut join_set = JoinSet::new();

    for (idx, clause) in clauses.iter().enumerate() {
        let llm = llm_factory();
        let tools = tools_factory();
        let agent = make_agent(llm, tools);
        let clause = clause.clone();
        let sem = sem.clone();
        let graph = graph.clone();
        let events = review_events.clone();
        let name = agent_name.to_string();
        let done = done.clone();
        let raw_findings_total = raw_findings_total.clone();

        join_set.spawn(async move {
            let _permit = sem.acquire_owned().await;
            let risk_id = graph
                .as_ref()
                .map(|g| g.next_risk_id())
                .unwrap_or_else(|| format!("R_{:03}", idx + 1));
            let finding = agent.review_single(&clause, &risk_id).await;

            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if !finding.no_risk {
                raw_findings_total.fetch_add(1, Ordering::Relaxed);
            }

            // SSE 实时进度推送
            if let Some(ref events) = events {
                events.emit(&ReviewEvent::AgentProgress {
                    agent_id: name.clone(),
                    agent_label: name.clone(),
                    clauses_done: n,
                    clauses_total: total,
                    raw_findings: raw_findings_total.load(Ordering::Relaxed),
                    status: if n >= total {
                        "completed".to_string()
                    } else {
                        "running".to_string()
                    },
                });
            }

            (idx, finding)
        });
    }

    // 收集结果，按原始顺序排列
    let mut findings: Vec<Option<RiskFinding>> = (0..total).map(|_| None).collect();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((idx, finding)) => {
                findings[idx] = Some(finding);
            }
            Err(e) => {
                // task panic — 为该 clause 生成占位 finding
                eprintln!("[PARALLEL] 条款审查 task 异常: {}", e);
            }
        }
    }

    // 补齐缺失的 finding（task panic 等情况）
    findings
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            f.unwrap_or_else(|| {
                RiskFinding::truncated_finding(
                    format!("R_{:03}", i + 1),
                    clauses[i].chunk_id.clone(),
                    agent_name,
                    clauses[i].tier,
                    clauses[i].tier,
                    "并行审查 task 异常终止",
                )
            })
        })
        .collect()
}
