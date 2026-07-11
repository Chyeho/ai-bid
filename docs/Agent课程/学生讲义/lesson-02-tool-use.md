# 第2课：Tool Use — 让 Agent 使用工具

> LLM 是大脑，工具是手脚。这节课让 Agent 学会调用外部函数。

---

## 学习目标

1. 理解 Function Calling 的全流程
2. 自己设计并实现 `Tool` trait
3. 让 Agent 在需要时自动选择并调用工具

---

## 核心概念

### Function Calling 不是一个 API，是一个流程

很多人以为"Function Calling"是某个 API 端点。不是的。它是一个**多轮对话协议**：

```
第1轮：你发消息 + tool definitions → LLM 回复"我要调用 get_time(timezone=...)"
第2轮：你执行 get_time，拿到结果 → 把结果追加到对话 → 再发给 LLM
第3轮：LLM 看到结果 → 生成最终文本回复
```

用你的 `ai-client` crate，代码大致长这样：

```rust
use ai_client::{LlmClient, ChatMessage, ToolDef};

let client = LlmClient::from_env()?;
let messages = vec![
    ChatMessage::system("你是一个助手，可以使用工具。"),
    ChatMessage::user("现在几点了？"),
];
let tools = vec![
    ToolDef {
        name: "get_time".into(),
        description: "获取当前时间".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": { "timezone": { "type": "string" } }
        }),
    },
];

let response = client.chat(&messages, &tools).await?;

if !response.tool_calls.is_empty() {
    // LLM 要调用工具 → 执行 → 追加 tool 消息 → 再调 chat
} else {
    // LLM 直接给了文本回复
    println!("{}", response.content.unwrap());
}
```

### 四种消息角色——多了谁？

上一课你用了 system / user / assistant。这节课新增第 4 种：

| 角色 | 含义 | 示例 |
|---|---|---|
| `system` | Agent 身份定义 | "你是一个助手，可以使用工具" |
| `user` | 用户输入 | "现在几点了？" |
| `assistant` | LLM 回复 | **两种形态**：纯文本 或 含 `tool_calls` 列表 |
| **`tool`** | **工具执行结果** | `"2026-07-11 14:30:00 CST"` |

> 注意：`tool` 消息必须带 `tool_call_id`，它要和之前 `assistant` 消息中 tool_calls 的 `id` 一一对应。LLM 靠这个 id 知道每个结果是哪个工具调用产生的。

### tool_calls 长什么样

LLM 返回的 JSON 中：

```json
{
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "get_current_time",
        "arguments": "{\"timezone\": \"Asia/Shanghai\"}"
      }
    }
  ]
}
```

### 你需要定义 tool definition

在请求的 `tools` 参数里，你要告诉 LLM 有哪些工具可用。格式是一段 JSON Schema：

```json
{
  "type": "function",
  "function": {
    "name": "get_current_time",
    "description": "获取指定时区的当前时间",
    "parameters": {
      "type": "object",
      "properties": {
        "timezone": {
          "type": "string",
          "description": "时区"
        }
      },
      "required": []
    }
  }
}
```

---

## 作业

### 基本要求

在第 1 课的代码基础上：

1. **设计 `Tool` trait**：至少包含 `name()`、`definition()`（返回 JSON Schema）、`execute()` 三个方法
2. **实现 2 个工具**：
   - `GetCurrentTime` — 获取当前时间，参数 `timezone`（用 `chrono` crate）
   - `Calculator` — 简单计算器，参数 `expression`（如 `"3 + 5 * 2"`）
3. **修改 Agent Loop**：
   - LLM 返回 `tool_calls` 时 → 追加 `assistant` 消息 → 逐个执行工具 → 每个结果追加 `tool` 消息 → 继续循环
   - LLM 返回纯文本时 → 打印回复
4. **更新 System Prompt**：告知 Agent 它拥有哪些工具、什么时候应该用
5. **限制轮数**：工具调用最多 5 轮，防止死循环

### 进阶（选做）

- 添加第 3 个工具 `get_weather(city)`：调 [wttr.in](https://wttr.in) 免费天气 API
- 工具执行出错时不 panic，把错误信息作为 `tool` 消息返回给 LLM

### 思考题

> LLM 为什么不直接执行工具，而是返回 tool_call 让你去执行？这种设计有什么好处？

---

## 参考资料

- [DashScope Function Calling 文档](https://help.aliyun.com/zh/model-studio/function-calling)
- [chrono crate](https://docs.rs/chrono/latest/chrono/)
- [JSON Schema 入门](https://json-schema.org/learn/getting-started-step-by-step)
