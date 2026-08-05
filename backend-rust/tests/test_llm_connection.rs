//! 千问 LLM 连接测试 — 验证 .env 配置和 API 连通性。
//!
//! 运行:
//!   cargo test --test test_llm_connection -- --nocapture
//!
//! 或独立运行:
//!   cargo run --bin test_llm

use ai_bid::services::llm_client::create_llm_client;

/// 测试工厂函数 — 根据当前 .env 中 AIBID_LLM_PROTOCOL 创建客户端。
#[tokio::test]
async fn test_llm_factory() {
    dotenv::dotenv().ok();

    let protocol = std::env::var("AIBID_LLM_PROTOCOL").unwrap_or_else(|_| "dashscope".to_string());
    println!("协议: {}", protocol);

    let result = create_llm_client();
    assert!(
        result.is_ok(),
        "LLM 客户端创建失败 (protocol={}): {:?}",
        protocol,
        result.err()
    );
    println!("✅ LLM 客户端创建成功");
}

/// 测试 DashScope 原生 API 联网搜索 — 验证 search_info 返回标题+URL。
///
/// 调用 DashScope 原生 Text Generation API（流式 + enable_search），
/// 解析 SSE 流第一个 chunk 中的 search_info.search_results，
/// 打印每条结果的 index / title / url。
#[tokio::test]
async fn test_dashscope_search_info() {
    dotenv::dotenv().ok();

    let api_key = std::env::var("DASHSCOPE_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
        .expect("DASHSCOPE_API_KEY 或 OPENAI_API_KEY 未设置");

    let model = std::env::var("DASHSCOPE_MODEL").unwrap_or_else(|_| "qwen-plus".to_string());

    let key_preview = if api_key.len() > 12 {
        format!(
            "{}...{}",
            &api_key[..8],
            &api_key[api_key.len().saturating_sub(4)..]
        )
    } else {
        "***".to_string()
    };
    println!("=== DashScope 原生联网搜索测试 ===");
    println!(
        "端点: https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"
    );
    println!("模型: {}", model);
    println!("Key:  {}", key_preview);
    println!();

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let body = serde_json::json!({
        "model": model,
        "input": {
            "messages": [
                {"role": "user", "content": "今天（2026年6月）中国有哪些科技新闻？列出3条并用[ref_<数字>]标注来源。"}
            ]
        },
        "parameters": {
            "result_format": "message",
            "max_tokens": 800,
            "enable_search": true,
            "search_options": {
                "enable_source": true,
                "enable_citation": true,
                "citation_format": "[ref_<number>]",
                "search_strategy": "pro",
                "forced_search": true,
            },
            "stream": true,
            "incremental_output": true,
        }
    });

    let response = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&body)
        .send()
        .await
        .expect("HTTP request failed");

    assert!(
        response.status().is_success(),
        "API returned error: {}",
        response.status()
    );

    // 读取完整 SSE 文本
    let sse_text = response.text().await.expect("读取 SSE 流失败");

    // 解析 SSE：找到第一个 data: 行
    let mut search_info_found: Option<serde_json::Value> = None;
    let mut full_content = String::new();

    for line in sse_text.lines() {
        let line = line.trim();
        if line.is_empty() || line == "data:[DONE]" {
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data.is_empty() {
                continue;
            }
            if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                // 第一个 chunk 检查 search_info
                if search_info_found.is_none()
                    && let Some(si) = chunk["output"]["search_info"].as_object()
                {
                    search_info_found = Some(serde_json::json!(si));
                }
                // 收集正文
                if let Some(content) = chunk["output"]["choices"][0]["message"]["content"].as_str()
                {
                    full_content.push_str(content);
                }
            }
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if let Some(ref si) = search_info_found {
        println!("✅ 找到 search_info！");
        println!();
        let results = si["search_results"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        println!("搜索到 {} 条结果：", results);
        println!();

        if let Some(items) = si["search_results"].as_array() {
            for item in items {
                let idx = item["index"].as_i64().unwrap_or(0);
                let title = item["title"].as_str().unwrap_or("无标题");
                let url = item["url"].as_str().unwrap_or("无URL");
                let site = item["site_name"].as_str().unwrap_or("");
                println!("  [{idx}] {title}");
                println!("      {url}");
                if !site.is_empty() {
                    println!("      来源: {site}");
                }
                println!();
            }
        }
    } else {
        println!("❌ 未找到 search_info");
        println!();
        println!("可能原因：");
        println!("  1. 模型 {} 不支持 search_options.enable_source", model);
        println!("  2. 当前 API Key 权限不足");
        println!("  3. search_info 在非第一个 chunk 中");
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("【模型回答正文】(前 500 字符)");
    let preview: String = full_content.chars().take(500).collect();
    println!("{}", preview);
    println!();

    assert!(
        search_info_found.is_some(),
        "DashScope 原生 API 应返回 search_info"
    );
}

#[tokio::test]
async fn test_llm_connection() {
    dotenv::dotenv().ok();

    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let api_base = std::env::var("OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen-max".to_string());

    let key_preview = if api_key.len() > 12 {
        format!(
            "{}...{}",
            &api_key[..8],
            &api_key[api_key.len().saturating_sub(4)..]
        )
    } else {
        "***".to_string()
    };
    println!("端点: {}", api_base);
    println!("模型: {}", model);
    println!("Key:  {}", key_preview);

    let url = format!("{}/chat/completions", api_base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "user", "content": "回复 OK"}
        ],
        "max_tokens": 10,
        "temperature": 0.0,
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .expect("HTTP request failed");

    assert!(response.status().is_success(), "API returned error status");
    let body: serde_json::Value = response.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    println!("回复: {}", content);
    assert!(!content.is_empty());
}
