# Rust 毕业设计：ai-client — 封装 LLM 调用库

> 1 天。把 4 课学的全部技能组装成你自己的 crate。Agent 课程 7 周全靠它。

---

## 你要做什么

实现一个 `ai-client` crate，封装 DashScope API 调用。Agent 第 1 课直接 `use ai_client::LlmClient`。

---

## 功能要求

### 基本要求

```rust
use ai_client::{LlmClient, ChatMessage, ToolDef, ToolCall};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 从 .env 加载配置
    let client = LlmClient::from_env()?;

    // 2. 构建消息
    let messages = vec![
        ChatMessage::system("你是一个有帮助的AI助手。"),
        ChatMessage::user("用一句话介绍 Rust。"),
    ];

    // 3. 调用 LLM（无工具）
    let response = client.chat(&messages, &[]).await?;
    println!("{}", response.content);

    // 4. 调用 LLM（带工具）
    let tools = vec![
        ToolDef {
            name: "get_time".into(),
            description: "获取当前时间".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "timezone": {"type": "string", "description": "时区"}
                }
            }),
        },
    ];
    let response = client.chat(&messages, &tools).await?;

    // 5. 检查是否有工具调用
    if !response.tool_calls.is_empty() {
        for tc in &response.tool_calls {
            println!("LLM 想调用: {} ({})", tc.name, tc.arguments);
        }
    }

    Ok(())
}
```

### 必须实现的类型

| 类型 | 字段 | 说明 |
|---|---|---|
| `LlmClient` | `api_key`, `model`, `endpoint` | HTTP 客户端封装 |
| `ChatMessage` | enum: System/User/Assistant/Tool | 四种消息角色 |
| `ToolDef` | `name`, `description`, `parameters` | 工具定义（JSON Schema） |
| `ToolCall` | `id`, `name`, `arguments` | LLM 返回的工具调用 |
| `LlmResponse` | `content: Option<String>`, `tool_calls: Vec<ToolCall>` | LLM 回复 |

### 必须实现的方法

| 方法 | 签名 | 说明 |
|---|---|---|
| `from_env` | `fn from_env() -> anyhow::Result<Self>` | 从 `.env` 读 `DASHSCOPE_API_KEY` 和 `DASHSCOPE_MODEL` |
| `chat` | `async fn chat(&self, messages: &[ChatMessage], tools: &[ToolDef]) -> anyhow::Result<LlmResponse>` | 调 DashScope API |

---

## 验收标准

你自己写一个 `examples/demo.rs`，跑通以下场景：

```powershell
# 场景1：普通对话
$ cargo run --example demo
> LLM: Rust 是一门注重安全、并发和性能的系统编程语言...

# 场景2：工具调用
（在 demo 里注册一个 get_time 工具）
> LLM 想调用工具: get_time({"timezone": "Asia/Shanghai"})

# 场景3：错误处理
（故意写错 API Key）
> 错误: API 返回 401 Unauthorized
```

---

## 项目搭建

```powershell
cargo new --lib ai-client    # 创建 library crate
cd ai-client
```

`Cargo.toml` 添加依赖：

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
dotenv = "0.15"
```

`src/lib.rs` 是 library 的入口——所有 `pub` 的类型和方法写在这里。`examples/demo.rs` 测试你的库：

```rust
use ai_client::LlmClient;
```

然后 `cargo run --example demo` 即可运行。

**关键**：library 里的类型和方法要加 `pub` 才能被外部使用。如果忘了，`use ai_client::LlmClient` 会报 "not found"。

## ChatMessage 序列化要点

DashScope 期望 `{"role": "user", "content": "..."}` 格式。用 serde 的 tag 属性：

```rust
#[derive(serde::Serialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum ChatMessage {
    System { content: String },
    User { content: String },
    #[serde(rename_all = "camelCase")]
    Assistant {
        content: Option<String>,
        tool_calls: Option<Vec<ToolCall>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}
```

## 提示

- `LlmClient::chat()` 就是第 4 课 `main.rs` 里的 HTTP 代码，搬进 struct 方法即可
- `ToolDef` → 请求里的 `tools` 数组，`ToolCall` → 响应里的 `tool_calls` 数组
- 注意 DashScope 带 `tool_calls` 时 Assistant 消息的 JSON 结构不同

---

## 评分

| 项 | 权重 | 通过条件 |
|---|---|---|
| 编译 | 30% | `cargo check` 无错 |
| 基本对话 | 25% | `chat(&messages, &[])` 返回 LLM 文本回复 |
| 工具调用 | 25% | `chat(&messages, &tools)` 能解析 tool_calls |
| 错误处理 | 10% | 网络错误/API 错误不 panic |
| 代码可读性 | 10% | 结构清晰，关键位置有注释 |
