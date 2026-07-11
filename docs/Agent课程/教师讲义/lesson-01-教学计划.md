# 第1课教学计划：Hello Agent

---

## 教学目标

学员用自己写的 `ai-client` crate 实现第一个 Agent。关键是理解：Agent = while 循环 + 消息历史。HTTP 调用已经在 Rust 大作业里封装好了，本课不碰。

---

## 教学内容安排

### 0-10min：开场

- 展示最终效果：命令行 Agent 在对话
- "这段代码的核心只有 10 行。你们 Rust 大作业写的 `ai-client` 就是它的发动机。"
- 放出架构图：`ai-client::chat()` ← while 循环 ← Vec<ChatMessage>

### 10-30min：概念讲解

**三个概念，不讲 API 格式：**

1. **消息角色**（8min）：system = 规则书、user = 用户说、assistant = LLM 说
2. **LLM 没有记忆**（7min）：演示——只发最后一条消息 → LLM 不知道之前聊了什么 → 发完整历史 → LLM "记住"了。这个 demo 比任何解释都有力
3. **Agent 循环**（5min）：while true { 读输入 → push User → chat → 打印 → push Assistant }

**不讲的内容：** DashScope API 请求格式、HTTP 状态码——ai-client 已经封装了。如果学员的 crate 有问题，1v1 排查。

### 30-45min：作业说明

- `Cargo.toml` 里加 `ai-client = { path = "..." }`
- 代码骨架：init client → init messages → while loop
- 核心：只关注 Agent 逻辑，不关注 HTTP

### 45-60min：Q&A + 读项目代码

- 让学员打开 `backend-rust/src/services/llm_client.rs`，对比自己的 `ai-client`
- "你们的 crate 和项目的 llm_client.rs 做的事一样——封装 HTTP 调用。项目代码多了 tool_choice 和流式，但核心结构是一样的。"

---

## 学员常见坑

| 坑 | 怎么帮 |
|---|---|
| ai-client 编译报错 | Cargo.toml 里 path 写错了，检查相对路径 |
| client.chat() 返回 Err | .env 文件没在当前目录，或 DASHSCOPE_API_KEY 没设 |
| LLM 不记得之前说了什么 | 检查 messages 数组——可能只发了最后一条 |
| ai-client crate 的 ChatMessage 和本课用的不一致 | 让学员确认字段名和 enum 变体 |

---

## 评分标准

| 项 | 权重 | 怎么检查 |
|---|---|---|
| 编译（含 ai-client 依赖） | 30% | `cargo check` |
| 能对话 | 20% | "你好"→ 有回复 |
| 有记忆 | 20% | "我叫小明"→"我叫什么？"→ 回复"小明" |
| exit/history | 10% | exit 退出，history 显示轮数 |
| clippy | 10% | 0 warning |
| 思考题 | 10% | ai-client vs Agent 的边界划分 |

---

## 与项目的关联

让学员看 `backend-rust/src/services/llm_client.rs`。他们的 `ai-client` 和项目的 `DashScopeNativeClient` 做的事一样——都实现了 LLM 调用的封装。项目的多了 tool_choice 参数和流式支持，但核心的 `chat(messages, tools)` 接口一模一样。
