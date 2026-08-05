#!/usr/bin/env python3
"""
正确姿势：DashScope 原生 API + 流式 + search_options.enable_source=True

关键发现（来自阿里云官方文档）：
  - search_info 只在 DashScope 原生协议下返回
  - search_info 只在**流式输出**的第一个 chunk 中包含
  - 需要 search_options.enable_source=True 才返回搜索来源
"""

import os, json, requests, sseclient

def load_env():
    env = {}
    env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
    if os.path.exists(env_path):
        with open(env_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"): continue
                if "=" in line:
                    k, _, v = line.partition("=")
                    env[k.strip()] = v.strip().strip('"').strip("'")
    return env

env = load_env()
API_KEY = env["OPENAI_API_KEY"]

# DashScope 原生 API 端点
NATIVE_URL = "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"

print("=" * 72)
print("DashScope 原生 API — 流式 + search_options.enable_source")
print("=" * 72)

# ── 测试: 流式模式 ──────────────────────────────────────
print()
print("【测试】qwen-turbo + stream + search_options.enable_source=True")
print("-" * 72)

payload = {
    "model": "qwen-turbo",
    "input": {
        "messages": [
            {"role": "user", "content": "2026年6月中国科技新闻有哪些？列出3条并标注来源。"}
        ]
    },
    "parameters": {
        "result_format": "message",
        "max_tokens": 800,
        "enable_search": True,
        "search_options": {
            "enable_source": True,           # 🔑 关键！返回搜索来源信息
            "enable_citation": True,         # 开启角标标注
            "citation_format": "[ref_<number>]",
            "search_strategy": "pro",        # 搜索 10 条
            "forced_search": True,           # 强制搜索
        },
        "stream": True,                      # 🔑 必须流式！
        "incremental_output": True,
    }
}

headers = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
    "Accept": "text/event-stream",
}

resp = requests.post(NATIVE_URL, json=payload, headers=headers, stream=True, timeout=120)
print(f"HTTP 状态码: {resp.status_code}")
print()

if resp.status_code != 200:
    print(f"错误: {resp.text[:1000]}")
    exit(1)

# ── 解析 SSE 流 ──────────────────────────────────────────
client = sseclient.SSEClient(resp)
first_chunk = True
full_content = ""
search_info = None

for event in client.events():
    if event.data == "" or event.data == "[DONE]":
        continue
    try:
        chunk = json.loads(event.data)
    except json.JSONDecodeError:
        continue

    # 第一个 chunk 检查 search_info
    if first_chunk:
        first_chunk = False
        output = chunk.get("output", {})
        si = output.get("search_info")
        if si:
            search_info = si
            print("✅✅✅ 找到 search_info！✅✅✅")
            print()
            print(json.dumps(si, ensure_ascii=False, indent=2))
        else:
            print("第一个 chunk 的顶层 keys:", list(chunk.keys()))
            if "output" in chunk:
                print("output 的 keys:", list(chunk["output"].keys()))

    # 收集正文
    choices = chunk.get("output", {}).get("choices", [])
    if choices:
        content = choices[0].get("message", {}).get("content", "")
        if content:
            full_content += content

print()
print("-" * 72)
print("【模型回答正文】")
print(full_content[:1500])

print()
print("=" * 72)
print("最终结论")
print("-" * 72)

if search_info:
    print("✅ DashScope 原生 API 确实返回了 search_info！")
    print()
    sr = search_info.get("search_results", [])
    print(f"搜索到 {len(sr)} 条结果：")
    for item in sr:
        idx = item.get("index", "?")
        title = item.get("title", "无标题")
        url = item.get("url", "无URL")
        print(f"  [{idx}] {title}")
        print(f"      {url}")
else:
    print("❌ 仍未找到 search_info")
    print()
    print("可能原因：")
    print("  1. 当前 API Key 权限不足（MaaS 密钥 vs DashScope 密钥）")
    print("  2. 模型 qwen-turbo 版本不支持此参数")
    print("  3. search_options 参数名可能有差异")
