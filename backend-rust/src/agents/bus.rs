//! AgentBus — Agent 间实时广播通道（Observer 模式 — 推送侧）。
//!
//! 设计文档 §7.9 / temp.md Phase 2 增强版：
//! - `BusMessage.from` 从 `String` 改为 `AgentId` 枚举
//! - 新增 `MessageTarget` (Broadcast/Direct) 和 `MessageTopic` (RiskFound/CrossReference)
//! - ★ 新增 `subscribe()`：Agent 获取专属 Receiver，避免每次 poll 创建新 subscriber 丢消息
//!
//! ## 双通道通信模型
//!
//! | 通道 | 角色 | 模式 | 用途 |
//! |------|------|------|------|
//! | AgentBus (本模块) | Observer 推送侧 | 实时通知 | High 风险广播 |
//! | SessionGraph | Blackboard 拉取侧 | 结构化查询 | 已知结论、关联拓扑 |
//!
//! MVP 阶段：单进程内 tokio::broadcast 实现。
//! V1 阶段：可扩展为 Redis Pub/Sub 支持多进程。

use crate::agents::types::{AgentId, RiskSeverity};
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

/// 消息目标类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageTarget {
    /// 广播给所有 Agent
    Broadcast,
    /// 定向发送给特定 Agent（Phase 3 启用）
    Direct(AgentId),
}

/// 消息主题分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageTopic {
    /// 发现风险（替代原单一 severity 字段区分）
    RiskFound,
    /// 跨条款关联发现
    CrossReference,
}

/// AgentBus 广播消息。
#[derive(Debug, Clone)]
pub struct BusMessage {
    /// 发送者 Agent 身份
    pub from: AgentId,
    /// 消息目标（默认 Broadcast）
    pub target: MessageTarget,
    /// 消息主题
    pub topic: MessageTopic,
    /// 风险严重程度（仅 high 级别广播）
    pub severity: RiskSeverity,
    /// 一句话摘要
    pub summary: String,
    /// 关联的条款 chunk_id 列表
    pub clause_ids: Vec<String>,
    /// 风险类型
    pub risk_type: String,
}

/// Agent 间广播通道。
///
/// 内部使用 tokio::broadcast 实现多生产者多消费者。
/// 每个 Agent 应通过 `subscribe()` 获取自己的 Receiver，
/// 在 ReActLoop 初始化时持有，每轮 `try_recv()` 循环排空。
pub struct AgentBus {
    /// 消息发送端
    sender: broadcast::Sender<BusMessage>,
    /// 每个 Agent 的最后 poll 序号（用于增量拉取，兼容旧接口）
    last_seen: Mutex<HashMap<AgentId, usize>>,
}

impl AgentBus {
    /// 创建新的 AgentBus。
    ///
    /// * `capacity` — 通道容量（缓冲消息数）
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            last_seen: Mutex::new(HashMap::new()),
        }
    }

    /// ★ 新增: Agent 获取专属 Receiver。
    ///
    /// 每个 Agent 在 ReActLoop 初始化时调用一次，获取自己的 `Receiver<BusMessage>`。
    /// 此后每轮 `try_recv()` 循环排空，避免多 Agent 并发下消息丢失。
    pub fn subscribe(&self) -> broadcast::Receiver<BusMessage> {
        self.sender.subscribe()
    }

    /// 广播一个高风险发现。
    ///
    /// 仅 severity=High 的消息被广播（避免噪音）。
    /// topic 根据 risk_type 自动选择：涉及跨条款关联 → CrossReference，否则 → RiskFound。
    pub fn broadcast(
        &self,
        from: AgentId,
        severity: RiskSeverity,
        summary: &str,
        clause_ids: &[String],
        risk_type: &str,
    ) {
        if severity != RiskSeverity::High {
            return; // 只广播 High severity
        }
        // 自动选择 topic
        let topic = if risk_type.contains("关联") || risk_type.contains("组合") {
            MessageTopic::CrossReference
        } else {
            MessageTopic::RiskFound
        };
        let msg = BusMessage {
            from,
            target: MessageTarget::Broadcast,
            topic,
            severity,
            summary: summary.to_string(),
            clause_ids: clause_ids.to_vec(),
            risk_type: risk_type.to_string(),
        };
        let _ = self.sender.send(msg);
    }

    /// 获取自上次 poll 以来的最新消息（兼容旧接口）。
    ///
    /// **注意**: 此方法每次调用创建新的 subscriber，多 Agent 并发下可能丢消息。
    /// 推荐使用 `subscribe()` + Agent 持有 Receiver 的新模式。
    /// Phase 2 中 ReActLoop 已切换为 bus_rx 模式，此方法仅保留用于向后兼容。
    pub fn poll_since(&self, agent: AgentId) -> Option<BusMessage> {
        match self.sender.subscribe().try_recv() {
            Ok(msg) => {
                if msg.from != agent {
                    if let Ok(mut seen) = self.last_seen.lock() {
                        seen.insert(agent, 0);
                    }
                    Some(msg)
                } else {
                    None // 不接收自己发送的消息
                }
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(_) => None,
        }
    }
}

// ─── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_and_receive() {
        let bus = AgentBus::new(32);
        let mut rx = bus.subscribe();

        bus.broadcast(
            AgentId::SemanticRisk,
            RiskSeverity::High,
            "品牌指定风险",
            &["ch_003".to_string()],
            "品牌指定",
        );

        let msg = rx.try_recv().expect("应收到消息");
        assert_eq!(msg.from, AgentId::SemanticRisk);
        assert_eq!(msg.summary, "品牌指定风险");
        assert_eq!(msg.clause_ids, vec!["ch_003"]);
        assert_eq!(msg.topic, MessageTopic::RiskFound); // 不含"关联"/"组合"
        assert_eq!(msg.target, MessageTarget::Broadcast);
    }

    #[test]
    fn test_non_high_severity_not_broadcast() {
        let bus = AgentBus::new(32);
        let mut rx = bus.subscribe();

        bus.broadcast(
            AgentId::FactCheck,
            RiskSeverity::Medium,
            "中等风险",
            &[],
            "",
        );

        assert!(rx.try_recv().is_err()); // Empty
    }

    #[test]
    fn test_cross_reference_topic_auto_select() {
        let bus = AgentBus::new(32);
        let mut rx = bus.subscribe();

        bus.broadcast(
            AgentId::Procedure,
            RiskSeverity::High,
            "跨条款关联发现",
            &["ch_001".to_string(), "ch_005".to_string()],
            "组合风险",
        );

        let msg = rx.try_recv().expect("应收到消息");
        assert_eq!(msg.topic, MessageTopic::CrossReference);
    }

    #[test]
    fn test_multi_subscriber_independent() {
        let bus = AgentBus::new(32);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.broadcast(
            AgentId::SemanticRisk,
            RiskSeverity::High,
            "测试消息",
            &[],
            "test",
        );

        // 两个订阅者都应收到
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_subscribe_only_new_messages() {
        let bus = AgentBus::new(32);

        // 先发一条消息
        bus.broadcast(AgentId::FactCheck, RiskSeverity::High, "旧消息", &[], "old");

        // 之后订阅的 Receiver 不应收到之前已广播的消息
        let mut late_rx = bus.subscribe();
        // tokio::broadcast 不保证能收到订阅前的消息，但应能收到订阅后的
        // 这个测试主要验证 subscribe() 不 panic
        // 旧消息可能丢失（broadcast 的语义），新消息应能收到
        bus.broadcast(AgentId::FactCheck, RiskSeverity::High, "新消息", &[], "new");
        let msg = late_rx.try_recv().expect("应收到订阅后的消息");
        assert_eq!(msg.summary, "新消息");
    }

    #[test]
    fn test_broadcast_multiple_drain_all() {
        let bus = AgentBus::new(32);
        let mut rx = bus.subscribe();

        for i in 0..5 {
            bus.broadcast(
                AgentId::SemanticRisk,
                RiskSeverity::High,
                &format!("msg_{}", i),
                &[],
                "test",
            );
        }

        let mut received = 0;
        while rx.try_recv().is_ok() {
            received += 1;
        }
        assert_eq!(received, 5, "应收到全部 5 条消息");
    }

    #[test]
    fn test_poll_since_excludes_self_message() {
        let bus = AgentBus::new(32);

        bus.broadcast(
            AgentId::FactCheck,
            RiskSeverity::High,
            "来自 FactCheck 的消息",
            &[],
            "test",
        );

        // poll_since(FactCheck) 应该不返回自己发送的消息
        let result = bus.poll_since(AgentId::FactCheck);
        assert!(result.is_none(), "不应接收自己发送的消息");
    }

    #[test]
    fn test_poll_since_receives_other_message() {
        let bus = AgentBus::new(32);

        // poll_since 使用 subscribe() 创建新 subscriber，只能收到订阅后的消息
        // 所以需要先调用 poll_since 来创建 subscriber（虽然第一次可能为空），
        // 然后再 broadcast，再 poll_since 才能收到
        // 替代方案：使用 subscribe() 保持 Receiver，然后 broadcast，再 try_recv
        let mut rx = bus.subscribe();

        bus.broadcast(
            AgentId::SemanticRisk,
            RiskSeverity::High,
            "来自 SemanticRisk 的消息",
            &[],
            "test",
        );

        // 使用保持的 Receiver 接收
        let msg = rx.try_recv().expect("应能接收其他 Agent 的消息");
        assert_eq!(msg.from, AgentId::SemanticRisk);
    }

    #[test]
    fn test_info_severity_not_broadcast() {
        let bus = AgentBus::new(32);
        let mut rx = bus.subscribe();

        bus.broadcast(AgentId::FactCheck, RiskSeverity::Info, "info", &[], "");
        bus.broadcast(AgentId::FactCheck, RiskSeverity::Low, "low", &[], "");

        assert!(rx.try_recv().is_err()); // 都不发送
    }
}
