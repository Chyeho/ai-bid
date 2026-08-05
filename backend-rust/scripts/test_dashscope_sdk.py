#!/usr/bin/env python3
"""
测试 DashScope 原生 Python SDK 能否获取联网搜索源（标题+URL）。

与 test_search_api_v4.py 的区别：
  - v4: 直接用 HTTP + SSE 调 DashScope 原生 REST API
  - 本脚本: 使用官方 `dashscope` Python SDK（更接近文档示例）

关键点（来自阿里云文档）：
  - search_info 只在 DashScope 原生协议下返回
  - search_info 只在流式输出的第一个 chunk 中包含
  - 需 enable_search=True + search_options.enable_source=True
  - MaaS 工作空间密钥需设置 base_http_api_url

用法：
    python scripts/test_dashscope_sdk.py
"""

import os
import sys
import json
import dashscope


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

# ── 配置 ───────────────────────────────────────────────────
# 优先用 DASHSCOPE_API_KEY，回退到 OPENAI_API_KEY
API_KEY = env.get("DASHSCOPE_API_KEY") or env.get("OPENAI_API_KEY", "")
MODEL = env.get("DASHSCOPE_MODEL", "qwen-plus")
BASE_URL = env.get("OPENAI_BASE_URL", "")

print("=" * 72)
print("DashScope Python SDK — 联网搜索源测试")
print("=" * 72)
print(f"API Key:  {API_KEY[:12]}...{API_KEY[-4:]}" if len(API_KEY) > 16 else f"API Key:  {API_KEY}")
print(f"Model:    {MODEL}")
try:
    print(f"SDK ver:  {dashscope.__version__}")
except AttributeError:
    print("SDK ver:  (unknown)")
print()

# ── 判断是否为 MaaS 工作空间密钥 ──────────────────────────
# MaaS 密钥格式: sk-ws-... 需要设置 base_http_api_url
IS_MAAS_KEY = API_KEY.startswith("sk-ws-")
if IS_MAAS_KEY:
    print("[检测] MaaS 工作空间密钥，将设置 base_http_api_url")
    if BASE_URL:
        from urllib.parse import urlparse
        parsed = urlparse(BASE_URL)
        workspace_host = parsed.hostname
        maas_api_url = f"https://{workspace_host}/api/v1"
        dashscope.base_http_api_url = maas_api_url
        print(f"  -> base_http_api_url = {maas_api_url}")
    else:
        print("  WARNING: 未找到 OPENAI_BASE_URL，使用默认端点（搜索可能不可用）")
else:
    print("[检测] 标准 DashScope API 密钥，使用默认端点")


# ── 测试 1: 流式 + enable_source=True ──────────────────────
def test_stream_with_source():
    print()
    print("-" * 72)
    print("[Test 1] stream + enable_source=True + forced_search")
    print("-" * 72)

    responses = dashscope.Generation.call(
        api_key=API_KEY,
        model=MODEL,
        messages=[
            {"role": "user", "content": "2026年6月中国科技新闻有哪些？列出3条并标注来源。"}
        ],
        result_format="message",
        enable_search=True,
        search_options={
            "enable_source": True,
            "enable_citation": True,
            "citation_format": "[ref_<number>]",
            "forced_search": True,
        },
        stream=True,
        incremental_output=True,
    )

    first_chunk = True
    full_content = ""
    search_info = None

    for resp in responses:
        if resp.status_code != 200:
            print(f"FAILED: HTTP {resp.status_code}: {resp.message}")
            return None

        if first_chunk:
            first_chunk = False
            output = resp.output
            if output:
                si = output.get("search_info")
                if si:
                    search_info = si
                    print("OK: search_info found in first chunk!")
                    print()
                    sr = si.get("search_results", [])
                    print(f"   {len(sr)} search results:")
                    for item in sr:
                        idx = item.get("index", "?")
                        title = item.get("title", "(no title)")
                        url = item.get("url", "(no URL)")
                        print(f"     [{idx}] {title}")
                        print(f"         {url}")
                else:
                    print("first chunk output keys:", list(output.keys()) if output else "None")

        choices = resp.output.choices if resp.output else None
        if choices:
            content = choices[0].message.content if choices[0].message else ""
            if content:
                full_content += content

    print()
    print("[Response text] (first 1000 chars)")
    print(full_content[:1000])
    return search_info


# ── 测试 2: 非流式（对照）──────────────────────────────────
def test_nonstream():
    print()
    print("-" * 72)
    print("[Test 2] non-stream + enable_source=True (control)")
    print("-" * 72)

    resp = dashscope.Generation.call(
        api_key=API_KEY,
        model=MODEL,
        messages=[
            {"role": "user", "content": "今天杭州天气怎么样？"}
        ],
        result_format="message",
        enable_search=True,
        search_options={
            "enable_source": True,
            "enable_citation": True,
            "forced_search": True,
        },
        stream=False,
    )

    print(f"HTTP status: {resp.status_code}")
    if resp.status_code != 200:
        print(f"FAILED: {resp.message}  (code={resp.code})")
        return None

    output = resp.output
    print(f"output keys: {list(output.keys()) if output else 'None'}")

    si = output.get("search_info") if output else None
    if si:
        print("OK: search_info also returned in non-stream mode!")
        sr = si.get("search_results", [])
        print(f"   {len(sr)} search results:")
        for item in sr:
            idx = item.get("index", "?")
            title = item.get("title", "(no title)")
            url = item.get("url", "(no URL)")
            print(f"     [{idx}] {title}")
            print(f"         {url}")
    else:
        print("NO: search_info not in non-stream response")

    text = output.choices[0].message.content if output and output.choices else ""
    print()
    print("[Response text] (first 500 chars)")
    print(text[:500] if text else "(empty)")

    return si


# ── 测试 3: 不开启 enable_source（对照）────────────────────
def test_no_source():
    print()
    print("-" * 72)
    print("[Test 3] stream + enable_source=False (control)")
    print("-" * 72)

    responses = dashscope.Generation.call(
        api_key=API_KEY,
        model=MODEL,
        messages=[
            {"role": "user", "content": "2026年NBA总冠军是谁？"}
        ],
        result_format="message",
        enable_search=True,
        search_options={
            "enable_source": False,
            "forced_search": True,
        },
        stream=True,
        incremental_output=True,
    )

    first_chunk = True
    full_content = ""
    search_info = None

    for resp in responses:
        if resp.status_code != 200:
            print(f"FAILED: HTTP {resp.status_code}: {resp.message}")
            break

        if first_chunk:
            first_chunk = False
            output = resp.output
            if output:
                si = output.get("search_info")
                if si:
                    search_info = si
                    print("UNEXPECTED: search_info returned even with enable_source=False!")
                    print(json.dumps(si, ensure_ascii=False, indent=2))
                else:
                    print("OK: no search_info when enable_source=False (expected)")

        choices = resp.output.choices if resp.output else None
        if choices:
            content = choices[0].message.content if choices[0].message else ""
            if content:
                full_content += content

    print()
    print("[Response text] (first 500 chars)")
    print(full_content[:500])
    return search_info


# ── Main ────────────────────────────────────────────────────
if __name__ == "__main__":
    if not API_KEY:
        print("ERROR: no API Key found in .env")
        sys.exit(1)

    result1 = test_stream_with_source()
    result2 = test_nonstream()
    result3 = test_no_source()

    print()
    print("=" * 72)
    print("Summary")
    print("=" * 72)
    print(f"  Test1 (stream + source):    {'OK: has search_info' if result1 else 'NO search_info'}")
    print(f"  Test2 (non-stream + source):{'OK: has search_info' if result2 else 'NO search_info'}")
    print(f"  Test3 (stream - source):    {'UNEXPECTED: has' if result3 else 'OK: no search_info (expected)'}")

    if result1:
        print()
        print("SUCCESS: DashScope Python SDK can retrieve search sources (title+URL)!")
    elif IS_MAAS_KEY:
        print()
        print("NOTE: using MaaS workspace key. If Test1 failed, search_info may not be")
        print("supported via MaaS + DashScope SDK. You may need to:")
        print("  1. Use OpenAI-compatible endpoint with enable_search (test_search_api_v4.py)")
        print("  2. Or use DashScope native REST API directly (also v4)")
