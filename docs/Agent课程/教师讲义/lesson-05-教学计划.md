# 第5课教学计划：Multi-Agent

---

## 教学目标

学员理解多 Agent 协作的核心：消息传递。用 `tokio::sync::broadcast` 实现 Reviewer ↔ Fixer 的审查-修复-复查循环。

---

## 教学内容安排

### 0-10min：上周回顾

### 10-25min：🦀 Rust 并发基础（本课穿插）

> 这三个概念在 Rust 前置课程里没教，放这里因为不用它们就写不了多 Agent。

1. **tokio::spawn**（5min）：演示 spawn → JoinHandle → await
2. **broadcast::channel**（5min）：演示 subscribe → send → recv，强调 subscribe 必须在 send 之前
3. **Arc**（5min）：演示 clone 不复制数据，用于多个 spawn 共享同一份配置

### 25-45min：Agent 概念讲解

**重点讲通信**：不要讲一堆 Agent 框架概念，聚焦一个核心问题——"两个 Agent 怎么对话？"

1. 用**群聊类比**解释 Broadcast：一个人说话，大家都能听到，但每个人只关心自己感兴趣的内容
2. 画时序图展示 Reviewer → Fixer → Reviewer 的三轮交互
3. 演示 `tokio::sync::broadcast::channel` 的基本用法

### 30-45min：并发模型

- `tokio::spawn` 启动两个 Agent 并行运行
- `rx.recv().await` 阻塞等待消息
- 退出条件：Reviewer 广播 "approve" → 两个 Agent 都退出
- 死锁问题：A 等 B、B 等 A → 超时处理

### 45-60min：作业说明

- 提供一段含 3-5 个故意 bug 的 Rust 代码（约 30 行）
- 学员写两个 Agent 的 System Prompt 和协作逻辑
- **关键**：Fixer 只能根据 Reviewer 的意见修复，不能自己发现新 bug

---

## 学员常见坑

| 坑 | 怎么帮 |
|---|---|
| broadcast Receiver 收不到消息 | `subscribe()` 必须在 `send()` 之前调用 |
| 两个 Agent 互相死等 | 加 `tokio::time::timeout` |
| Reviewer 第二次审查用的还是原始代码 | 修复后代码要传入复查上下文 |
| 不清楚两个 Agent 怎么"并行" | 用 `tokio::spawn` 各自启动 |

---

## 评分标准

| 项 | 权重 | 怎么检查 |
|---|---|---|
| Reviewer 找到 bug | 20% | 输出的 bug 列表覆盖埋入的 80% |
| Fixer 正确修复 | 20% | 修复后代码不再有对应 bug |
| 复查循环跑通 | 25% | Reviewer 复查→确认修复或要求再改 |
| 消息格式规范 | 15% | JSON 格式的 payload，带 from/msg_type |
| 思考题 | 10% | 项目多 Agent 并行审查的冲突分析 |
| clippy | 10% | |

---

## 与项目的关联

让学员看 `backend-rust/src/agents/bus.rs`。他们的 `broadcast::channel` 是基础版，项目的 AgentBus 增加了消息过滤（不收自己发的）、结构化消息类型（risk_type/clause_ids）、try_recv 非阻塞轮询。
