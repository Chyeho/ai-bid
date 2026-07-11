# 第1课：Hello Agent

> 你的第一个 AI Agent。用你自己写的 `ai-client` crate，10 行代码变聊天机器人。

---

## 学习目标

1. 理解 Agent 的本质：while 循环 + 对话历史
2. 理解 System / User / Assistant 三种消息角色
3. 用自己写的 `ai-client` 实现命令行对话 Agent

> 本课不需要写任何 HTTP 请求代码。那是 Rust 大作业做的事，你已经做完了。

---

## 核心概念

### Agent 不是什么魔法，就是一段 while 循环

```rust
use ai_client::{LlmClient, ChatMessage};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = LlmClient::from_env()?;  // ← 你大作业写的

    let mut messages = vec![
        ChatMessage::system("你是一个有帮助的助手"),
    ];

    loop {
        // 1. 读用户输入
        let user_input = read_line();
        if user_input == "exit" { break; }

        // 2. 追加到历史
        messages.push(ChatMessage::user(&user_input));

        // 3. 调 LLM
        let response = client.chat(&messages, &[]).await?;

        // 4. 打印回复
        println!("Agent: {}", response.content);

        // 5. 追加到历史
        messages.push(ChatMessage::assistant(&response.content));
    }
    Ok(())
}
```

**这就是 Agent。** 你做的全部事情就是维护一个 `Vec<ChatMessage>`，每次把整个数组发给 LLM，把回复追加进去，然后循环。

### 三种消息角色

| 角色 | 谁说的 | 什么时候用 |
|---|---|---|
| `system` | 你 | 第一条消息，定义 Agent 的身份和行为 |
| `user` | 用户 | 每次用户输入 |
| `assistant` | LLM | 每次 LLM 回复 |

### LLM 没有记忆——你在替它记

每次调 `client.chat(&messages, &[])`，你把**完整的对话历史**传过去。LLM 看到历史里有 "我叫小明" 这条 user 消息，才会在回答 "你叫什么" 时说 "你叫小明"。

如果你只传最后一条消息，LLM 就不知道之前聊了什么。Agent 的"记忆"就是你手里的 `Vec<ChatMessage>`。

---

## 作业

### 基本要求

用你自己的 `ai-client` crate 实现命令行对话 Agent：

1. `Cargo.toml` 里加 `ai-client = { path = "../ai-client" }`（或 crates.io 版本）
2. `LlmClient::from_env()` 加载配置
3. 初始化 `Vec<ChatMessage>`，第一条是 system 消息
4. while 循环：读输入 → 加 User → `client.chat()` → 打印 → 加 Assistant
5. 输入 `exit` 退出，`history` 打印轮数

### 进阶（选做）

- 支持 `--model` 参数覆盖默认模型
- 彩色输出（system 蓝、user 绿、assistant 白）
- 输入 `save <文件名>` 把对话历史存成 JSON

### 思考题

> 你写的 Agent 和 ai-client crate 的边界在哪里？哪些逻辑属于"基础设施"（crate），哪些属于"Agent"（本课代码）？

---

## 参考资料

- 你的 `ai-client` crate（自己写的，不需要外部文档）
- [DashScope API 文档](https://help.aliyun.com/zh/model-studio/use-cases/text-generation)（如果 crate 出问题需要排查）
