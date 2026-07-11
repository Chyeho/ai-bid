# 第5课：Multi-Agent — 让多个 Agent 协作

> 一个 Agent 不够用？上两个。一个找 bug，一个修 bug。

---

## 前置：Rust 并发基础（本课新知识）

本课需要的 Rust 知识之前没讲过，这里集中补上。

### tokio::spawn — 同时运行多个任务

```rust
// spawn 启动一个后台任务，返回 JoinHandle
let handle = tokio::spawn(async {
    // 这段代码在后台运行，不阻塞当前函数
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("后台任务完成");
});
// 主线程继续做其他事...
handle.await.unwrap();  // 等待后台任务结束
```

### broadcast::channel — 一人发送，多人接收

```rust
use tokio::sync::broadcast;

let (tx, _) = broadcast::channel::<String>(16);  // 容量 16

let mut rx1 = tx.subscribe();  // 订阅者 1
let mut rx2 = tx.subscribe();  // 订阅者 2

tx.send("hello".into()).unwrap();
// rx1 和 rx2 都能收到 "hello"
// 注意：subscribe() 之后才能收到消息，之前的收不到
```

### Arc — 多个地方共享同一个数据

```rust
use std::sync::Arc;

let data = Arc::new(vec![1, 2, 3]);    // 引用计数指针
let data2 = data.clone();               // clone 不复制数据，只增加引用计数
// data 和 data2 指向同一块内存
```

> 这三个概念是本节课作业的基础。作业里你会反复用到它们。

### 常见错误

```rust
// ❌ spawn 里的闭包直接用了局部变量
let config = LlmConfig::from_env()?;
tokio::spawn(async { config.chat(...) });  // 编译错误：config 活得不够久

// ✅ 用 Arc 包一层 + async move
let config = Arc::new(LlmConfig::from_env()?);
let c = config.clone();
tokio::spawn(async move { c.chat(...) });

// ❌ broadcast recv 收到 Lagged 错误后 panic
// ✅ 处理 Lagged：重新订阅即可
match rx.recv().await {
    Err(broadcast::error::RecvError::Lagged(n)) => {
        eprintln!("跳过了 {} 条消息，重新订阅", n);
        rx = tx.subscribe();  // 重新订阅
    }
    _ => { /* 正常处理 */ }
}

// ❌ 用 std::sync::Mutex 在 async 中 .lock().unwrap() → 死锁风险
// ✅ 用 tokio::sync::Mutex + .lock().await
use tokio::sync::Mutex;
let data = Arc::new(Mutex::new(Vec::new()));
// ...
let mut guard = data.lock().await;
guard.push(result);
```

---

## 学习目标

1. 理解多 Agent 协作的三个问题：怎么分工、怎么通信、怎么解决冲突
2. 用 `tokio::sync::broadcast` 实现 Agent 消息总线
3. 实现 Handoff 模式：A Agent 完成任务后交给 B Agent

---

## 核心概念

### 什么时候需要多个 Agent

```
单 Agent 够了：
  "今天天气怎么样？" → 一个 Agent 搞定

需要多 Agent：
  "审查这份代码的安全漏洞，然后修复它们"
  → Reviewer（擅长找问题） + Fixer（擅长改代码）
  → 各自有不同的 System Prompt 和工具集
```

### 三种通信模式

| 模式 | 做法 | 类比 |
|---|---|---|
| **Broadcast** | 一个 Agent 发消息，所有人能收到 | 群聊 @all |
| **Direct** | 指定发给某个 Agent | 私聊 |
| **Handoff** | 把整个任务转交出去，自己不继续了 | 转接电话 |

本课实现 Broadcast + Handoff。

### 代码审查双 Agent 的协作流程

```
Reviewer: 收到一段代码 → 分析 → 找到 3 个 bug
    ↓ (广播 "finding" 消息，含 bug 列表)
Fixer:   收到 bug 列表 → 修复代码 → 发 "request_review"
    ↓ (广播，附带修复后代码)
Reviewer: 收到修复代码 → 复查 → 2 个修好了，1 个没修好
    ↓ (再次广播)
Fixer:   修正第 3 个 bug → 再次请求复查
    ↓
Reviewer: 全部通过 → 广播 "approve" → 结束
```

### AgentBus 数据结构

```rust
struct AgentMessage {
    from: String,        // 谁发的
    msg_type: String,    // "finding" | "fix" | "request_review" | "approve"
    payload: String,     // 消息内容（JSON 字符串）
}
```

用 `tokio::sync::broadcast::channel` 实现。每个 Agent 通过 `subscribe()` 拿到自己的 Receiver，通过 `send()` 发消息。

---

## 作业

### 基本要求

实现一个"代码审查双 Agent"系统：

1. **Reviewer Agent**：
   - 接收一段 Rust 代码（讲师提供，含 3-5 个故意埋入的 bug）
   - 调 LLM 分析代码，找出所有 bug
   - 把 bug 列表广播出去（`msg_type: "finding"`）
   - 等待 Fixer 修复完成后复查

2. **Fixer Agent**：
   - 等待 Reviewer 的 `"finding"` 消息
   - 收到后，调 LLM 根据 bug 列表修复代码
   - 广播修复后的代码（`msg_type: "request_review"`）

3. **协作循环**：
   - Reviewer 收到修复代码 → 复查 → 还有问题就再广播
   - 最多 2 轮复查
   - 结束条件：全部通过 或 达到最大轮数

4. **最终输出**：原始代码、所有发现的问题、最终修复后的代码

### 进阶（选做）

- 加第 3 个 Agent：**Tester**，对修复后的代码自动写测试用例
- 给 Agent 加"关注列表"：每个 Agent 只处理自己关心的消息类型
- 所有 Agent 消息写入 `messages.jsonl`，方便调试

### 思考题

> 你们项目的 FactCheck、Procedure、SemanticRisk 三个 Agent 是并行审查同一条款的。这种模式下可能产生什么冲突？有什么解决思路？

---

## 参考资料

- [Tokio broadcast channel](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [Tokio spawn 并发](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html)
