"""测试 qwq-plus 是否能正常调用。

用法: python scripts/test_qwq.py
超时: 180 秒（QwQ 深度推理模型较慢）
"""
import os, sys, json, time, urllib.request, urllib.error

API_KEY = "sk-ws-H.RYDPDDM.9kSk.MEUCIAelZhZ-bugo08GfPZmx61Sxw76RfY_0wh25cHafUK2IAiEAxonjTxcHqmz2251vMSRHbh0cteMKFXhfJ2VaYawrsLM"
MODEL = "qwq-plus"
URL = "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"

payload = json.dumps({
    "model": MODEL,
    "input": {
        "messages": [
            {"role": "user", "content": "你好，请用一句话介绍你自己"}
        ]
    },
    "parameters": {
        "max_tokens": 200,
        "temperature": 0.7
    }
}).encode("utf-8")

req = urllib.request.Request(URL, data=payload, headers={
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
    "X-DashScope-SSE": "disable",  # 非流式
})

print(f"[{time.strftime('%H:%M:%S')}] 正在调用 {MODEL} (超时 180s)...")
start = time.time()

try:
    with urllib.request.urlopen(req, timeout=180) as resp:
        body = resp.read().decode("utf-8")
        elapsed = time.time() - start
        data = json.loads(body)

        # 检查错误
        if "code" in data and data["code"]:
            print(f"[{time.strftime('%H:%M:%S')}] ❌ API 返回错误 (耗时 {elapsed:.1f}s):")
            print(json.dumps(data, indent=2, ensure_ascii=False))
            sys.exit(1)

        # 提取回答
        output = data.get("output", {})
        choices = output.get("choices", [])
        if choices:
            msg = choices[0].get("message", {})
            content = msg.get("content", "")
            reasoning = msg.get("reasoning_content", "")
            print(f"[{time.strftime('%H:%M:%S')}] ✅ 调用成功 (耗时 {elapsed:.1f}s)")
            if reasoning:
                print(f"--- reasoning ({len(reasoning)} chars) ---")
                print(reasoning[:500])
            print(f"--- answer ({len(content)} chars) ---")
            print(content[:500])
        else:
            print(f"[{time.strftime('%H:%M:%S')}] ⚠️ 无 choices 返回 (耗时 {elapsed:.1f}s):")
            print(json.dumps(data, indent=2, ensure_ascii=False)[:1000])

except urllib.error.HTTPError as e:
    elapsed = time.time() - start
    body = e.read().decode("utf-8", errors="replace")
    print(f"[{time.strftime('%H:%M:%S')}] ❌ HTTP {e.code} (耗时 {elapsed:.1f}s)")
    print(body[:1000])
except Exception as e:
    elapsed = time.time() - start
    print(f"[{time.strftime('%H:%M:%S')}] ❌ 异常 (耗时 {elapsed:.1f}s): {e}")
