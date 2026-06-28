#!/usr/bin/env python3
"""
最终验证：测试所有协议/参数组合，确认联网搜索是否能返回 title + URL。

结论来自阿里云官方文档对比（WebSearch 确认）：
  - DashScope 原生协议：✅ 支持 search_info（含标题 + URL）
  - OpenAI 兼容协议：  ❌ 不支持返回搜索来源

本脚本验证三种路径：
  1. MaaS OpenAI兼容端点（当前项目使用）
  2. DashScope 原生协议 + qwen-plus
  3. DashScope 原生协议 + qwen-turbo（search_strategy 强搜）
"""

import os, json, requests

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
HEADERS = {"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"}

print("=" * 72)
print("联网搜索 search_info 返回验证 — 三种路径对比")
print("=" * 72)
print()

# ── 路径1: MaaS OpenAI 兼容端点（当前项目） ────────────────
print("【路径1】MaaS OpenAI 兼容端点（当前项目使用）")
print(f"  端点: {env['OPENAI_BASE_URL']}")
print(f"  模型: {env['LLM_MODEL']}")
print()

resp1 = requests.post(
    env["OPENAI_BASE_URL"].rstrip("/") + "/chat/completions",
    json={"model": env["LLM_MODEL"], "messages": [
        {"role": "user", "content": "2026年6月的中国科技新闻有哪些？请用角标标注来源。"}
    ], "max_tokens": 600, "enable_search": True},
    headers=HEADERS, timeout=60
)
d1 = resp1.json()
content1 = d1.get("choices", [{}])[0].get("message", {}).get("content", "")
has_search_info = "search_info" in d1 or "doc_references" in d1
print(f"  search_info 字段: {'✅ 存在' if has_search_info else '❌ 不存在'}")
# 检查 usage 里是否有 search plugin
usage1 = d1.get("usage", {})
has_plugin = "plugins" in usage1 and "search" in usage1.get("plugins", {})
print(f"  usage.plugins.search: {'✅ ' + str(usage1['plugins']['search']) if has_plugin else '❌ 未找到（搜索可能未实际触发）'}")
print()

# ── 路径2: DashScope 原生 API + qwen-plus ─────────────────
print("【路径2】DashScope 原生 API → qwen-plus")
print(f"  端点: https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
print(f"  模型: qwen-plus")
print()

resp2 = requests.post(
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation",
    json={"model": "qwen-plus", "input": {"messages": [
        {"role": "user", "content": "2026年6月的中国科技新闻有哪些？请用角标标注来源。"}
    ]}, "parameters": {
        "result_format": "message",
        "max_tokens": 600,
        "enable_search": True,
    }},
    headers=HEADERS, timeout=60
)
d2 = resp2.json()
print(f"  HTTP: {resp2.status_code}")

# 深度查找 search_info / url / title
def deep_find(obj, keys, results):
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k in keys: results.append((k, v))
            deep_find(v, keys, results)
    elif isinstance(obj, list):
        for item in obj: deep_find(item, keys, results)

found2 = []
deep_find(d2, {"search_info", "doc_references", "search_results", "url", "citation"}, found2)
search_plugin2 = d2.get("usage", {}).get("plugins", {}).get("search", {})
print(f"  搜索是否触发: {'✅ ' + str(search_plugin2) if search_plugin2 else '❌ 未触发'}")
print(f"  搜索引用字段: ", end="")
if found2:
    for k, v in found2: print(f"\n    ✅ {k}: {json.dumps(v, ensure_ascii=False)[:300]}")
else:
    print("❌ 未找到 search_info / url / citation")
print()

# ── 路径3: DashScope 原生 + qwen-turbo + search_strategy 强搜 ──
print("【路径3】DashScope 原生 API → qwen-turbo + search_strategy (强制搜索)")
print()

resp3 = requests.post(
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation",
    json={"model": "qwen-turbo", "input": {"messages": [
        {"role": "user", "content": "2026年6月中国科技新闻，请用[1][2]角标标注来源"}
    ]}, "parameters": {
        "result_format": "message",
        "max_tokens": 600,
        "enable_search": True,
        "search_strategy": "pro",  # pro: 强制搜索
    }},
    headers=HEADERS, timeout=60
)
d3 = resp3.json()
print(f"  HTTP: {resp3.status_code}")

search_plugin3 = d3.get("usage", {}).get("plugins", {}).get("search", {})
found3 = []
deep_find(d3, {"search_info", "doc_references", "search_results", "url", "citation"}, found3)
print(f"  搜索是否触发: {'✅ ' + str(search_plugin3) if search_plugin3 else '❌ 未触发'}")
print(f"  搜索引用字段: ", end="")
if found3:
    for k, v in found3: print(f"\n    ✅ {k}: {json.dumps(v, ensure_ascii=False)[:500]}")
else:
    print("❌ 未找到 search_info / url / citation")

# ── 打印路径2完整响应（供检查） ─────────────────────────────
print()
print("=" * 72)
print("【附录】路径2 完整响应 JSON（前 2000 字符）")
print("-" * 72)
print(json.dumps(d2, ensure_ascii=False, indent=2)[:2000])

print()
print("=" * 72)
print("最终结论")
print("-" * 72)
