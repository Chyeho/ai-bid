#!/usr/bin/env python3
"""
验证 DashScope 联网搜索返回内容是否符合 WebSearchResult 格式要求。

模拟 Rust 代码 DashScopeSearchBackend::search() 的完整调用链路：
  1. 构造 system prompt（含 search_context 领域提示）
  2. 调用 DashScope 原生 API（流式 + enable_search + forced_search）
  3. 从 SSE 第一个 chunk 提取 search_info.search_results[]
  4. 拼接所有 chunk 中的 content 作为 answer
  5. 返回 { answer, sources }

验证要点：
  - answer 非空（模型基于搜索结果生成的综合回答）
  - sources 包含 [{title, url, site_name}] 结构
  - 搜索策略 pro、forced_search 生效
"""

import os, json, requests, sseclient, sys

def load_env():
    """从 .env 文件加载环境变量"""
    env = {}
    env_paths = [
        os.path.join(os.path.dirname(__file__), "..", ".env"),
        os.path.join(os.path.dirname(__file__), ".env"),
    ]
    for env_path in env_paths:
        if os.path.exists(env_path):
            with open(env_path, encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line or line.startswith("#"):
                        continue
                    if "=" in line:
                        k, _, v = line.partition("=")
                        env[k.strip()] = v.strip().strip('"').strip("'")
            break
    return env


def test_search(question, search_context, api_key, model="qwen-turbo"):
    """
    完全模拟 Rust DashScopeSearchBackend::search() 的调用方式。
    """

    # ── 1. 构造 context_hint（与 Rust 代码完全一致）─────────────────
    context_map = {
        "法规": "你需要在政府采购法律法规领域搜索。优先查找法律、行政法规、部门规章中的禁止性条款。",
        "案例": "你需要在政府采购投诉处理案例领域搜索。优先查找财政部投诉处理决定、行政复议案例。",
        "负面清单": "你需要在政府采购负面清单领域搜索。优先查找各级政府发布的政府采购负面行为清单。",
        "标准范本": "你需要在政府采购标准招标文件领域搜索。对比财政部标准招标文件模板。",
        "历史审查记录": "你需要查找类似条款的历史审查记录。",
    }
    context_hint = context_map.get(search_context, "")

    # ── 2. 构造 system prompt ───────────────────────────────────────
    system_msg = (
        f"你是政府采购合规研究助手。{context_hint}\n"
        "请在互联网搜索后给出准确、有引用的回答。用[ref_<数字>]标注来源。"
    )

    # ── 3. 构造请求体（与 Rust 代码完全一致）─────────────────────────
    body = {
        "model": model,
        "input": {
            "messages": [
                {"role": "system", "content": system_msg},
                {"role": "user", "content": question},
            ]
        },
        "parameters": {
            "result_format": "message",
            "max_tokens": 1000,
            "enable_search": True,
            "search_options": {
                "enable_source": True,
                "enable_citation": True,
                "citation_format": "[ref_<number>]",
                "search_strategy": "pro",
                "forced_search": True,
            },
            "stream": True,
            "incremental_output": True,
        },
    }

    endpoint = "https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation"
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "Accept": "text/event-stream",
    }

    print(f"  模型: {model}")
    print(f"  端点: {endpoint}")
    print(f"  搜索上下文: {search_context}")
    print()

    resp = requests.post(endpoint, json=body, headers=headers, stream=True, timeout=120)
    print(f"  HTTP 状态码: {resp.status_code}")

    if resp.status_code != 200:
        print(f"  ❌ 错误响应: {resp.text[:1000]}")
        return None

    # ── 4. 解析 SSE 流（与 Rust 代码完全一致）────────────────────────
    client = sseclient.SSEClient(resp)
    search_info = None
    answer = ""

    for event in client.events():
        if event.data == "" or event.data == "[DONE]":
            continue
        try:
            chunk = json.loads(event.data)
        except json.JSONDecodeError:
            continue

        # 第一个有效 chunk 提取 search_info
        if search_info is None:
            si = chunk.get("output", {}).get("search_info")
            if si:
                search_info = si

        # 拼接 content
        choices = chunk.get("output", {}).get("choices", [])
        if choices:
            content = choices[0].get("message", {}).get("content", "")
            if content:
                answer += content

    # ── 5. 提取 sources（与 Rust 代码完全一致）───────────────────────
    sources = []
    if search_info:
        search_results = search_info.get("search_results", [])
        for item in search_results:
            sources.append({
                "title": item.get("title", ""),
                "url": item.get("url", ""),
                "site_name": item.get("site_name", ""),
            })

    # ── 6. 兜底：无 answer 但有 sources 时构造摘要 ──────────────────
    if not answer.strip() and sources:
        answer = f"搜索到 {len(sources)} 条相关结果，详见来源链接。"

    # ── 7. 构造 WebSearchResult ─────────────────────────────────────
    result = {
        "answer": answer,
        "sources": sources,
    }

    return result


def validate_result(result, test_label):
    """验证返回结果是否符合 WebSearchResult 格式要求。"""
    print(f"【{test_label}】")
    print("=" * 72)

    if result is None:
        print("  ❌ 测试失败: API 调用返回 None")
        return False

    checks = []

    # 检查 1: answer 非空
    answer_ok = bool(result.get("answer", "").strip())
    checks.append(("answer 非空", answer_ok))
    if answer_ok:
        print(f"  ✅ answer: {len(result['answer'])} 字符")
    else:
        print("  ❌ answer 为空")

    # 检查 2: sources 是列表
    sources = result.get("sources", [])
    sources_is_list = isinstance(sources, list)
    checks.append(("sources 是列表", sources_is_list))
    if sources_is_list:
        print(f"  ✅ sources: {len(sources)} 条来源")
    else:
        print("  ❌ sources 不是列表")

    # 检查 3: 每条 source 包含必要字段
    all_sources_valid = True
    for i, src in enumerate(sources):
        has_title = bool(src.get("title", "").strip())
        has_url = bool(src.get("url", "").strip())
        # site_name 可选
        if not has_title or not has_url:
            all_sources_valid = False
            print(f"  ❌ sources[{i}] 缺少字段: title={has_title}, url={has_url}")
    checks.append(("所有 source 含 title+url", all_sources_valid))
    if all_sources_valid and sources:
        print(f"  ✅ 所有来源均含 title + url")
        for i, src in enumerate(sources[:5]):
            print(f"     [{i+1}] {src['title'][:60]}")
            print(f"         {src['url'][:80]}")
            if src.get("site_name"):
                print(f"         site: {src['site_name']}")

    # 检查 4: answer 包含引用标记 [ref_
    has_citation = "[ref_" in result.get("answer", "")
    checks.append(("answer 含引用标记", has_citation))
    if has_citation:
        print(f"  ✅ answer 包含 [ref_N] 引用标记")
    else:
        print(f"  ⚠️  answer 不包含引用标记（非必须，但期望有）")

    print()
    print("─" * 72)
    print("【answer 全文】")
    print(result["answer"][:2000])
    print("─" * 72)

    passed = all(ok for _, ok in checks)
    print(f"\n  结论: {'✅ 全部通过' if passed else '❌ 存在失败项'}")
    return passed


def main():
    env = load_env()
    api_key = env.get("DASHSCOPE_API_KEY") or env.get("OPENAI_API_KEY")
    if not api_key:
        print("❌ 未找到 API 密钥。请设置 DASHSCOPE_API_KEY 或 OPENAI_API_KEY 环境变量。")
        sys.exit(1)

    model = env.get("DASHSCOPE_SEARCH_MODEL", "qwen-turbo")
    print(f"DashScope 搜索后端验证测试")
    print(f"  API Key: {api_key[:20]}...")
    print()

    # ── 测试用例：模拟真实审查场景 ──────────────────────────────────
    test_cases = [
        {
            "label": "法规搜索 — 地域限制条款",
            "question": "政府采购招标文件中设置'投标人须在项目所在地设有常驻服务机构'条款，"
                       "是否有法律法规禁止这种地域限制性条款？请列举具体法条。",
            "search_context": "法规",
        },
        {
            "label": "案例搜索 — 联合体投标",
            "question": "政府采购中，招标文件规定'不接受联合体投标'是否合法？"
                       "财政部是否有相关投诉处理案例？",
            "search_context": "案例",
        },
        {
            "label": "负面清单 — 供应商资格条件",
            "question": "政府采购中设置'ISO9001质量管理体系认证'作为投标资格条件，"
                       "是否属于负面清单禁止的行为？",
            "search_context": "负面清单",
        },
    ]

    all_passed = True
    for tc in test_cases:
        print()
        print("=" * 72)
        print(f"测试: {tc['label']}")
        print(f"问题: {tc['question']}")
        print("=" * 72)
        print()

        result = test_search(
            question=tc["question"],
            search_context=tc["search_context"],
            api_key=api_key,
            model=model,
        )
        passed = validate_result(result, tc["label"])
        if not passed:
            all_passed = False

    print()
    print("=" * 72)
    print(f"总结: {'✅ 全部测试通过' if all_passed else '❌ 部分测试失败'}")
    print("=" * 72)


if __name__ == "__main__":
    main()
