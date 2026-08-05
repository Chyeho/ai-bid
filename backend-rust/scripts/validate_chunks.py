#!/usr/bin/env python3
"""Chunk 输出验证脚本 — 检查 chunking 结果是否存在验证问题。"""

import json
import sys
import os
from collections import Counter


def validate_chunks(name, path):
    with open(path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    chunks = data['chunks']
    stats = data['stats']

    issues = []

    print(f"\n{'='*60}")
    print(f"  验证: {name}")
    print(f"{'='*60}")

    # 0. 基本统计
    print(f"\n--- 基本统计 ---")
    print(f"总 Chunk 数: {stats['total_chunks']}")
    print(f"类型分布: {stats['type_counts']}")
    print(f"大小 — 总计: {stats['total_chars']}, 平均: {stats['avg_chunk_size']}, 最小: {stats['min_chunk_size']}, 最大: {stats['max_chunk_size']}")

    # 1. ID 连续性检查
    print(f"\n--- ID 连续性 ---")
    id_issues = []
    for i, chunk in enumerate(chunks):
        expected_id = f"ch_{i:03d}"
        if chunk['chunk_id'] != expected_id:
            id_issues.append(f"  ch[{i}] expected {expected_id}, got {chunk['chunk_id']}")
    if id_issues:
        for issue in id_issues:
            print(issue)
        issues.extend(id_issues)
    else:
        print("  PASS — 所有 ID 连续")

    # 2. page 范围合法性
    print(f"\n--- page 范围 ---")
    page_issues = []
    page_spans = []
    for chunk in chunks:
        if chunk['page_start'] > chunk['page_end']:
            page_issues.append(
                f"  {chunk['chunk_id']}: page_start({chunk['page_start']}) > page_end({chunk['page_end']})"
            )
        span = chunk['page_end'] - chunk['page_start']
        page_spans.append(span)
    if page_issues:
        for issue in page_issues:
            print(issue)
        issues.extend(page_issues)
    else:
        print("  PASS — 所有 page_start <= page_end")

    # 大篇幅跨度 chunk
    large_spans = [
        (c['chunk_id'], c['page_end'] - c['page_start'],
         c['page_start'], c['page_end'], len(c['text']))
        for c in chunks if c['page_end'] - c['page_start'] > 5
    ]
    if large_spans:
        print(f"  WARNING — {len(large_spans)} 个 chunk 页面跨度 > 5:")
        for cid, span, start, end, tlen in sorted(large_spans, key=lambda x: -x[1])[:10]:
            print(f"    {cid}: pp{start}-{end} (跨度 {span} 页, 文本 {tlen} 字符)")
    else:
        print("  PASS — 无大跨度 chunk")

    # 3. 必要字段非空
    print(f"\n--- 字段完整性 ---")
    field_issues = []
    for chunk in chunks:
        cid = chunk['chunk_id']
        if not chunk['text'].strip():
            field_issues.append(f"  {cid}: text 为空")
        if not chunk['section_path']:
            field_issues.append(f"  {cid}: section_path 为空")
        if not chunk['source_block_ids']:
            field_issues.append(f"  {cid}: source_block_ids 为空")
    if field_issues:
        for issue in field_issues:
            print(issue)
        issues.extend(field_issues)
    else:
        print("  PASS — 所有字段非空")

    # 4. 大小分布
    print(f"\n--- 大小分布 ---")
    sizes = [len(c['text']) for c in chunks]
    size_buckets = {
        "<50": 0, "50-100": 0, "100-500": 0,
        "500-1000": 0, "1000-1500": 0, ">1500": 0
    }
    for s in sizes:
        if s < 50:
            size_buckets["<50"] += 1
        elif s < 100:
            size_buckets["50-100"] += 1
        elif s < 500:
            size_buckets["100-500"] += 1
        elif s < 1000:
            size_buckets["500-1000"] += 1
        elif s <= 1500:
            size_buckets["1000-1500"] += 1
        else:
            size_buckets[">1500"] += 1

    for bucket, count in size_buckets.items():
        bar = "#" * min(count, 40) if count > 0 else ""
        print(f"  {bucket:>10}: {count:>4} {bar}")

    if size_buckets[">1500"] > 0:
        oversized = [(c['chunk_id'], len(c['text']))
                      for c in chunks if len(c['text']) > 1500]
        print(f"  WARNING — {size_buckets['>1500']} 个 chunk 超过 split_max_len(1500):")
        for cid, slen in oversized[:5]:
            print(f"    {cid}: {slen} 字符")
        issues.append(f"发现 {size_buckets['>1500']} 个超长 chunk")

    # 5. Split chunk 校验
    print(f"\n--- Split chunk 校验 ---")
    split_groups = {}
    for chunk in chunks:
        ct = chunk['chunk_type']
        if isinstance(ct, dict) and ct.get('type') == 'Split':
            key = tuple(chunk['section_path'])
            if key not in split_groups:
                split_groups[key] = []
            split_groups[key].append(chunk)

    split_issues = []
    for key, group in split_groups.items():
        total = group[0]['chunk_type']['total']
        if len(group) != total:
            split_issues.append(
                f"  path={key[-1]}: expected {total} parts, got {len(group)}"
            )
        parts = sorted([c['chunk_type']['part'] for c in group])
        if parts != list(range(1, total + 1)):
            split_issues.append(
                f"  path={key[-1]}: parts {parts} not consecutive 1..{total}"
            )
        # Check that all parts have text
        for c in group:
            if len(c['text']) == 0:
                split_issues.append(
                    f"  {c['chunk_id']}: Split part with empty text"
                )

    if split_issues:
        for issue in split_issues:
            print(issue)
        issues.extend(split_issues)
    else:
        print(f"  PASS — {len(split_groups)} 组 Split chunk 全部校验通过")

    # Show split group details
    for key, group in sorted(split_groups.items(), key=lambda x: len(x[1]), reverse=True):
        total = group[0]['chunk_type']['total']
        total_len = sum(len(c['text']) for c in group)
        print(f"  {key[-1][:50]}: {len(group)}/{total} parts, {total_len} total chars")

    # 6. 去重检查
    print(f"\n--- 去重检查 ---")
    text_counter = Counter()
    for chunk in chunks:
        t = chunk['text']
        if len(t) >= 20:
            text_counter[t] += 1
    dups = [(t, c) for t, c in text_counter.items() if c > 1]
    if dups:
        print(f"  WARNING — {len(dups)} 条重复文本:")
        for t, c in dups[:5]:
            preview = t[:60].replace('\n', '\\n')
            print(f"    出现 {c} 次: \"{preview}...\"")
    else:
        print("  PASS — 无重复内容")

    # 7. Merged 类型检查
    merged_chunks = [
        c for c in chunks
        if isinstance(c['chunk_type'], dict) and c['chunk_type'].get('type') == 'Merged'
    ]
    print(f"\n--- Merged chunk 检查 ---")
    print(f"  共 {len(merged_chunks)} 个 Merged chunk")
    merge_rules = Counter()
    merge_child_counts = []
    for mc in merged_chunks:
        merge_rules[mc['chunk_type'].get('rule', 'unknown')] += 1
        merge_child_counts.append(mc['chunk_type'].get('child_count', 0))
    print(f"  规则分布: {dict(merge_rules)}")
    if merge_child_counts:
        print(f"  子节点数: min={min(merge_child_counts)}, max={max(merge_child_counts)}, avg={sum(merge_child_counts)/len(merge_child_counts):.1f}")

    # 8. embed_text 检查
    print(f"\n--- embed_text 检查 ---")
    embed_issues = []
    for chunk in chunks:
        et = chunk['embed_text']
        if not et.strip():
            embed_issues.append(f"  {chunk['chunk_id']}: embed_text 为空")
        if chunk['section_path'] and not et.startswith("【"):
            embed_issues.append(f"  {chunk['chunk_id']}: embed_text 缺少层级前缀")
    if embed_issues:
        for issue in embed_issues[:5]:
            print(issue)
    else:
        print("  PASS — 所有 embed_text 正常")

    # 9. min_chunk_size 检查
    print(f"\n--- min_chunk_size 检查 (阈值: 50) ---")
    tiny = [(c['chunk_id'], len(c['text']), c['text'][:50])
            for c in chunks if len(c['text']) < 50]
    if tiny:
        print(f"  WARNING — {len(tiny)} 个 chunk < 50 字符:")
        for cid, slen, preview in tiny[:10]:
            preview_clean = preview.replace('\n', '\\n')
            print(f"    {cid}: {slen} chars — \"{preview_clean}\"")
    else:
        print("  PASS — 所有 chunk >= min_chunk_size(50)")

    # 10. 文本内容质量抽查
    print(f"\n--- 文本内容质量 ---")
    # 检查是否有仅数字/符号的chunk
    garbage = []
    for chunk in chunks:
        t = chunk['text']
        alpha_ratio = sum(1 for c in t if c.isalpha() or '一' <= c <= '鿿') / max(len(t), 1)
        if alpha_ratio < 0.3 and len(t) > 20:
            garbage.append((chunk['chunk_id'], alpha_ratio, t[:60]))
    if garbage:
        print(f"  WARNING — {len(garbage)} 个 chunk 文本质量可疑 (字母/汉字占比<30%):")
        for cid, ratio, preview in garbage[:5]:
            preview_clean = preview.replace('\n', '\\n')
            print(f"    {cid}: ratio={ratio:.2f} — \"{preview_clean}\"")
    else:
        print("  PASS — 所有 chunk 文本质量正常")

    # Summary
    total_issues = len(issues)
    print(f"\n{'='*60}")
    if total_issues == 0:
        print(f"   {name}: 验证通过，无严重问题")
    else:
        print(f"   {name}: 发现 {total_issues} 个问题")
    print(f"{'='*60}")

    return issues


if __name__ == '__main__':
    import glob as glob_mod

    # 数据目录优先级: AIBID_DATA_DIR > ../ > ./
    data_root = os.environ.get("AIBID_DATA_DIR", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    chunks_dir = os.path.join(data_root, "output", "chunks")
    if not os.path.isdir(chunks_dir):
        # Try parent directory
        chunks_dir = os.path.join(os.path.dirname(data_root), "output", "chunks")
    if not os.path.isdir(chunks_dir):
        print(f"[ERROR] chunks 目录不存在: {chunks_dir}")
        print(f"  设置 AIBID_DATA_DIR 环境变量指定项目根目录")
        sys.exit(1)

    os.chdir(data_root)

    # 自动发现所有 chunks JSON 文件
    chunk_files = sorted(glob_mod.glob(os.path.join(chunks_dir, "*_chunks.json")))
    if not chunk_files:
        # 如果传入命令行参数，使用它们
        chunk_files = sys.argv[1:] if len(sys.argv) > 1 else []
    if not chunk_files:
        print(f"[ERROR] 未找到 chunks JSON 文件: {chunks_dir}")
        print(f"  用法: python validate_chunks.py [file1.json file2.json ...]")
        sys.exit(1)

    all_issues = {}
    for path in chunk_files:
        name = os.path.splitext(os.path.basename(path))[0].replace("_chunks", "")
        try:
            issues = validate_chunks(name, path)
            all_issues[name] = issues
        except Exception as e:
            print(f"\n[ERROR] 验证失败: {name} — {e}")
            all_issues[name] = [f"脚本异常: {e}"]

    print(f"\n\n{'='*60}")
    print(f"  总结")
    print(f"{'='*60}")
    for name, issues in all_issues.items():
        status = "PASS" if len(issues) == 0 else f"{len(issues)} 个问题"
        print(f"  {name}: {status}")
