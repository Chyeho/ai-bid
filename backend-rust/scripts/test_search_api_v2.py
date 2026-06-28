#!/usr/bin/env python3
"""
测试2：直接调用 DashScope 原生 API（非 OpenAI 兼容模式）
对比 MaaS 端点 vs DashScope 官方端点，看 search_info 是否被返回。

DashScope 原生 API 文档：
  https://help.aliyun.com/zh/model-studio/use-qwen-by-calling-api

原生端点：https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation
参数：enable_search: true 放在 input 层级
"""

import os
import json
import requests

# ── 配置 ────────────────────────────────────────────────────
def load_env():
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
API_KEY = env.get("OPENAI_API_KEY", "")  # DashScope API Key（兼容）

print("=" * 70)
print("测试2: DashScope 原生 API 联网搜索")
print("=" * 70)

# ── 原生 DashScope API ───────────────────────────────────────
# 官方端点
NATIVE_URL = "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"
MODEL = "qwen-plus"  # 原生 API 用 qwen-plus / qwen-max / qwen-turbo

print(f"端点: {NATIVE_URL}")
print(f"模型: {MODEL}")
print()

# --- 测试 A: enable_search=true ---
print("【测试 A】原生 API + enable_search=true")
print("-" * 70)

payload = {
    "model": MODEL,
    "input": {
        "messages": [
            {"role": "user", "content": "2026年NBA总决赛冠军是哪支球队？请简要回答并标注来源。"}
        ]
    },
    "parameters": {
        "result_format": "message",
        "max_tokens": 500,
        "temperature": 0.0,
        "enable_search": True,        # 原生 API：放在 parameters 下
    }
}

headers = {
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
}

resp = requests.post(NATIVE_URL, json=payload, headers=headers, timeout=60)
print(f"HTTP 状态码: {resp.status_code}")
try:
    data = resp.json()
    print(json.dumps(data, ensure_ascii=False, indent=2))
except Exception:
    print(f"响应: {resp.text[:2000]}")

# ── 检查 search_info ─────────────────────────────────────────
print()
print("=" * 70)
print("【分析】在所有返回数据中查找搜索相关字段")
print("-" * 70)

def deep_find(obj, target_keys, path=""):
    """递归查找目标 key"""
    results = []
    if isinstance(obj, dict):
        for k, v in obj.items():
            current_path = f"{path}.{k}" if path else k
            if k in target_keys:
                results.append((current_path, v))
            results.extend(deep_find(v, target_keys, current_path))
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            results.extend(deep_find(item, target_keys, f"{path}[{i}]"))
    return results

target = ["search_info", "doc_references", "search_results", "references", "url", "link", "source"]
found = deep_find(data, target)

if found:
    for path, value in found:
        print(f"✅ {path}:")
        print(f"   {json.dumps(value, ensure_ascii=False, indent=4)[:500]}")
else:
    print("❌ 原生 API 也未返回 search_info / url / link 等字段")
    print()
    print("这意味着：")
    print("  - 当前 API Key 可能没有开通联网搜索配额")
    print("  - 或者 DashScope 联网搜索是独立计费功能，需在控制台单独开启")

print()
print("=" * 70)
print("测试完成 — 来对比一下 MaaS 端点 vs 原生端点的结果")
print("=" * 70)
