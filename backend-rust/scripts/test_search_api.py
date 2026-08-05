#!/usr/bin/env python3
"""
测试阿里云百炼 (DashScope) 联网搜索 API 是否返回标题和 URL。

用法：
    python scripts/test_search_api.py

会依次测试两条消息：
    1. 一个需要实时信息的问答（预期触发联网搜索）
    2. 打印完整响应结构，重点检查 search_info / doc_references 字段
"""

import os
import json
import requests

# ── 配置（与 .env 对齐） ────────────────────────────────────
def load_env():
    """手动解析 .env，避免引入 python-dotenv"""
    env = {}
    env_path = os.path.join(os.path.dirname(__file__), "..", ".env")
    if os.path.exists(env_path):
        with open(env_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                if "=" in line:
                    k, _, v = line.partition("=")
                    env[k.strip()] = v.strip().strip('"').strip("'")
    return env

env = load_env()
API_KEY = env.get("OPENAI_API_KEY", "")
BASE_URL = env.get("OPENAI_BASE_URL", "https://dashscope.aliyuncs.com/compatible-mode/v1")
MODEL = env.get("LLM_MODEL", "qwen3.6-flash")

# 构建 API URL
API_URL = BASE_URL.rstrip("/") + "/chat/completions"

print("=" * 70)
print("阿里云百炼 联网搜索 API 测试")
print("=" * 70)
print(f"端点: {API_URL}")
print(f"模型: {MODEL}")
print(f"Key:  {API_KEY[:12]}...{API_KEY[-4:]}" if len(API_KEY) > 16 else f"Key:  {API_KEY}")
print()

# ── 测试 1: 向 Qwen 提问需要搜索的问题 ────────────────────
print("【测试】提问一个需要联网搜索的问题...")
print("-" * 70)

payload = {
    "model": MODEL,
    "messages": [
        {"role": "user", "content": "2026年NBA总决赛冠军是哪支球队？请给出简要回答并标注来源。"}
    ],
    "max_tokens": 500,
    "temperature": 0.0,
    "enable_search": True,          # <-- 关键：开启联网搜索
}

headers = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
}

resp = requests.post(API_URL, json=payload, headers=headers, timeout=60)

print(f"HTTP 状态码: {resp.status_code}")
print()

# 先打印完整原始 JSON（美化）
try:
    data = resp.json()
    print("▼ 完整响应 JSON ▼")
    print(json.dumps(data, ensure_ascii=False, indent=2))
    print()
except Exception:
    print(f"非 JSON 响应: {resp.text[:1000]}")
    exit(1)

# ── 分析响应结构 ────────────────────────────────────────────
print("=" * 70)
print("【分析】搜索相关字段检查")
print("-" * 70)

# 1. 顶层 search_info
found_any = False
for field in ["search_info", "doc_references", "search_results", "references"]:
    if field in data:
        print(f"✅ 顶层. 找到字段 `{field}`:")
        print(json.dumps(data[field], ensure_ascii=False, indent=2))
        found_any = True

# 2. choices[0].message 内
message = data.get("choices", [{}])[0].get("message", {})
for field in ["search_info", "doc_references", "search_results", "references", "tool_calls"]:
    if field in message:
        print(f"✅ message. 找到字段 `{field}`:")
        print(json.dumps(message[field], ensure_ascii=False, indent=2))
        found_any = True

# 3. choices[0] 内
choice0 = data.get("choices", [{}])[0]
for field in ["search_info", "doc_references", "search_results"]:
    if field in choice0:
        print(f"✅ choices[0]. 找到字段 `{field}`:")
        print(json.dumps(choice0[field], ensure_ascii=False, indent=2))
        found_any = True

# 4. 检查文本中是否有角标 [1] [2]
text = message.get("content", "")
import re
citations = re.findall(r'\[\d+\]', text)
if citations:
    print(f"✅ 正文中发现角标引用: {citations}")
    found_any = True

if not found_any:
    print("❌ 未发现任何搜索引用信息（search_info / doc_references / 角标）")
    print()
    print("可能原因：")
    print("  1. MaaS 端点不支持 enable_search 参数")
    print("  2. 模型 qwen3.6-flash 不支持联网搜索")
    print("  3. 该问题未触发搜索引擎（模型认为不需要搜索）")
    print()
    print("接下来尝试再问一个明确需要搜索的问题...")

    # ── 测试 2: 明确要求搜索 ────────────────────────────────
    print()
    print("=" * 70)
    print("【测试2】明确要求搜索最新信息")
    print("-" * 70)

    payload2 = {
        "model": MODEL,
        "messages": [
            {"role": "user", "content": "请使用联网搜索功能，告诉我今天（2026年6月）的日期和最近一周的科技新闻头条。请在回答中使用[数字]角标标注来源。"}
        ],
        "max_tokens": 800,
        "temperature": 0.0,
        "enable_search": True,
    }

    resp2 = requests.post(API_URL, json=payload2, headers=headers, timeout=60)
    print(f"HTTP 状态码: {resp2.status_code}")
    print()
    try:
        data2 = resp2.json()
        print("▼ 完整响应 JSON ▼")
        print(json.dumps(data2, ensure_ascii=False, indent=2))
    except Exception:
        print(f"非 JSON 响应: {resp2.text[:1000]}")

print()
print("=" * 70)
print("测试完成")
print("=" * 70)
