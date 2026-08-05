"""测试 OpenAI 兼容端点 (qwen3.6-flash) — 快速对比。

用法: python scripts/test_openai_compat.py
"""
import json, time, urllib.request, urllib.error

API_KEY = "sk-ws-H.RYDPDDM.9kSk.MEUCIAelZhZ-bugo08GfPZmx61Sxw76RfY_0wh25cHafUK2IAiEAxonjTxcHqmz2251vMSRHbh0cteMKFXhfJ2VaYawrsLM"
URL = "https://llm-ekm11owggxl14t8r.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions"
MODEL = "qwen3.6-flash"

payload = json.dumps({
    "model": MODEL,
    "messages": [
        {"role": "user", "content": "你好，请用一句话介绍你自己"}
    ],
    "max_tokens": 200,
}).encode("utf-8")

req = urllib.request.Request(URL, data=payload, headers={
    "Authorization": f"Bearer {API_KEY}",
    "Content-Type": "application/json",
})

print(f"[{time.strftime('%H:%M:%S')}] 正在调用 {MODEL} (OpenAI兼容, 超时 30s)...")
start = time.time()

try:
    with urllib.request.urlopen(req, timeout=30) as resp:
        body = resp.read().decode("utf-8")
        elapsed = time.time() - start
        data = json.loads(body)

        choices = data.get("choices", [])
        if choices:
            content = choices[0].get("message", {}).get("content", "")
            print(f"[{time.strftime('%H:%M:%S')}] ✅ 调用成功 (耗时 {elapsed:.1f}s)")
            print(f"--- answer ---")
            print(content[:500])
        else:
            print(f"[{time.strftime('%H:%M:%S')}] ⚠️ 无 choices")
            print(json.dumps(data, indent=2, ensure_ascii=False)[:500])

except urllib.error.HTTPError as e:
    elapsed = time.time() - start
    body = e.read().decode("utf-8", errors="replace")
    print(f"[{time.strftime('%H:%M:%S')}] ❌ HTTP {e.code} (耗时 {elapsed:.1f}s)")
    print(body[:1000])
except Exception as e:
    print(f"[{time.strftime('%H:%M:%S')}] ❌ {e}")
