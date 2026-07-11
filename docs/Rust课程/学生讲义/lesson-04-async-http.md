# 第4课：异步与 HTTP 实战

> Rust 前置最后一课。学完你就能调 DashScope API——Agent 第 1 课直接从这里开始。

---

## 学习目标

1. 理解 async/await 并写异步函数
2. 用 reqwest + serde + dotenv 调 HTTP API
3. 用 clap 解析命令行参数
4. 把前 4 课学到的全部技能组装成一个 CLI 工具

---

## 核心概念

### part A：async/await

```rust
// async 函数：定义时加 async，调用时加 .await
async fn fetch_title(url: &str) -> anyhow::Result<String> {
    let body = reqwest::get(url).await?.text().await?;
    Ok(body)
}

#[tokio::main]  // ← 启动异步运行时
async fn main() -> anyhow::Result<()> {
    let title = fetch_title("https://example.com").await?;
    println!("页面内容长度: {}", title.len());
    Ok(())
}
```

`.await` = "这里可能比较慢，好了叫我"。和 Python 的 `await`、JS 的 `await` 一样。

### part B：reqwest 调 HTTP API

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let api_key = std::env::var("DASHSCOPE_API_KEY")?;
    let client = Client::new();

    let body = json!({
        "model": "qwen-plus",
        "input": {
            "messages": [
                {"role": "system", "content": "你是一个有帮助的助手。"},
                {"role": "user", "content": "用一句话介绍 Rust。"}
            ]
        },
        "parameters": { "result_format": "message" }
    });

    let resp = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    // 检查 HTTP 状态码
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await?;
        anyhow::bail!("API 错误 {}: {}", status, text);
    }

    let result: serde_json::Value = resp.json().await?;
    let reply = &result["output"]["choices"][0]["message"]["content"];
    println!("LLM: {}", reply);
    Ok(())
}
```

### part C：clap 命令行参数

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "my-tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 调用 LLM 回答问题
    Ask {
        question: String,
    },
    /// 翻译文本
    Translate {
        text: String,
        #[arg(short, long, default_value = "en")]
        to: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Ask { question } => { /* 调 LLM */ }
        Command::Translate { text, to } => { /* 调 LLM 翻译 */ }
    }
    Ok(())
}
```

---

## 作业

### 基本要求

实现一个**命令行 AI 助手**：

```powershell
$ cargo run -- ask "Rust 的所有权是什么"
（调 DashScope API，打印 LLM 回复）

$ cargo run -- translate "Hello, world" --to zh
你好，世界

$ cargo run -- translate "今天天气真好" --to ja
今日はとてもいい天気です
```

1. 用 `clap` 实现两个子命令：`ask <问题>` 和 `translate <文本> --to <语言>`
2. 从 `.env` 读 `DASHSCOPE_API_KEY`
3. 构造合适的 System Prompt：
   - ask："你是一个有帮助的AI助手。"
   - translate："你是一个翻译助手，将用户输入翻译为{目标语言}，只输出译文。"
4. API 调用失败时打印错误信息，**不 panic**
5. 错误处理全部用 `anyhow::Result` + `?`

### 进阶（选做）

- 支持 `--model` 参数切换模型（默认 qwen-plus）
- 翻译结果追加保存到 `translations.jsonl`
- 如果 API 返回非 200，打印 HTTP 状态码和响应体

---

## 参考资料

- [reqwest](https://docs.rs/reqwest/latest/reqwest/) / [serde_json](https://docs.rs/serde_json/latest/serde_json/)
- [clap derive 文档](https://docs.rs/clap/latest/clap/_derive/)
- [dotenv](https://docs.rs/dotenv/latest/dotenv/)
- [Tokio 入门](https://tokio.rs/tokio/tutorial/hello-tokio)
