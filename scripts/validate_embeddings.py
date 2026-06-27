#!/usr/bin/env python3
"""
BGE-M3 嵌入质量验证脚本

覆盖 V5.3 (L2 归一化)、V5.4 (语义区分度)、V5.5 (维度坍塌检测)、V5.7 (序列化往返)。

用法:
  python scripts/validate_embeddings.py output/embeddings/{stem}_embedding_index/
  python scripts/validate_embeddings.py output/embeddings/{stem}_embedding_index/ --verbose

依赖: 仅标准库 (json, struct, math, pathlib, sys)
"""

import json
import math
import struct
import sys
from pathlib import Path

# ─── 配置 ────────────────────────────────────────────────────

BASELINE = {
    "l2_norm_tolerance": 0.0001,       # 范数容差
    "value_mean_abs_max": 0.01,        # 值中心化阈值
    "same_section_cos_min": 0.70,      # 同章节最低 mean_cos
    "cross_section_cos_max": 0.80,     # 跨章节最高 mean_cos (理想情况)
    "semantic_gap_min": 0.05,          # 同章节-跨章节最小差值
    "zero_vectors_max": 0,             # 零向量上限
    "dead_dimensions_max": 0,          # 死维度上限
}

# ─── 加载 ────────────────────────────────────────────────────


def load_index(index_dir: str) -> tuple[list[dict], list[list[float]]]:
    """加载 chunk_meta.json + vectors_1024d_f32le.bin"""
    meta_path = Path(index_dir) / "chunk_meta.json"
    bin_path = Path(index_dir) / "vectors_1024d_f32le.bin"

    if not meta_path.exists():
        raise FileNotFoundError(f"元数据文件不存在: {meta_path}")
    if not bin_path.exists():
        raise FileNotFoundError(f"向量文件不存在: {bin_path}")

    with open(meta_path, "r", encoding="utf-8") as f:
        meta = json.load(f)

    chunk_count = meta["chunk_count"]
    dim = meta["dimension"]
    expected_size = chunk_count * dim * 4
    actual_size = bin_path.stat().st_size

    if actual_size != expected_size:
        raise ValueError(
            f"向量文件大小不匹配: 期望 {expected_size} bytes "
            f"({chunk_count} × {dim} × 4), 实际 {actual_size} bytes"
        )

    with open(bin_path, "rb") as f:
        data = f.read()

    vecs = []
    for i in range(chunk_count):
        offset = i * dim * 4
        vec = list(struct.unpack_from(f"<{dim}f", data, offset))
        vecs.append(vec)

    return meta, vecs


# ─── V5.3: L2 归一化 ─────────────────────────────────────────


def check_l2_normalization(vecs: list[list[float]], tolerance: float = 0.0001) -> dict:
    """检查所有向量 L2 范数是否为 1.0"""
    norms = []
    zero_count = 0
    for i, v in enumerate(vecs):
        norm = math.sqrt(sum(x * x for x in v))
        norms.append(norm)
        if norm == 0.0:
            zero_count += 1

    min_norm = min(norms)
    max_norm = max(norms)
    mean_norm = sum(norms) / len(norms)
    std_norm = math.sqrt(sum((n - mean_norm) ** 2 for n in norms) / len(norms))

    all_ok = (abs(min_norm - 1.0) <= tolerance and
              abs(max_norm - 1.0) <= tolerance and
              zero_count == 0)

    return {
        "min": round(min_norm, 6),
        "max": round(max_norm, 6),
        "mean": round(mean_norm, 6),
        "std": round(std_norm, 8),
        "zero_vectors": zero_count,
        "pass": all_ok,
    }


# ─── V5.4: 语义区分度 ─────────────────────────────────────────


def _dot(v1: list[float], v2: list[float]) -> float:
    return sum(a * b for a, b in zip(v1, v2))


def _section_parent(path: list[str], depth: int = 1) -> str:
    """提取 section_path 的前 depth 级作为分组键"""
    if not path or depth <= 0:
        return "__orphan__"
    return "||".join(path[:depth])


def check_semantic_discrimination(
    meta: dict, vecs: list[list[float]], max_pairs: int = 5000, verbose: bool = False
) -> dict:
    """比较同章节 vs 跨章节的余弦相似度

    分层策略:
      - depth=1: 一级章节（如"第二章"），粗粒度，同章节内主题可能差异大
      - depth=2: 前两级章节，较细粒度
      - depth=N: 取前 N-1 级（即 chunck 的直接父路径），最细粒度

    判据（两层）:
      1. 语义间隙 (semantic_gap): 同一父章节的 pair 均值 - 不同父章节的 pair 均值
         → 必须 > 0.05（统计显著）
      2. 相对比值 (separation_ratio): 同一父章节均值 / 不同父章节均值
         → 必须 > 1.10（同章节至少比跨章节高 10%）

    为什么不用绝对阈值:
      标书文档天然共享大量法律/政府采购语言（"供应商""投标""政府采购法"等），
      即使跨章节的 chunk 也有中等余弦相似度（~0.55）。
      绝对阈值会误杀正常的高基线场景。
    """
    chunks = meta["chunks"]
    n = len(vecs)
    if n < 10:
        return {"error": f"chunk 数太少 (n={n})，至少需要 10 个", "pass": None}

    import random
    rng = random.Random(42)

    def stats(vals: list[float]) -> dict:
        if not vals:
            return {"count": 0, "mean": None, "max": None, "min": None}
        return {
            "count": len(vals),
            "mean": round(sum(vals) / len(vals), 4),
            "max": round(max(vals), 4),
            "min": round(min(vals), 4),
        }

    # ── 多深度分析 ──
    depth_results = {}
    for depth in [1, 2]:
        same_candidates: list[tuple[int, int]] = []
        cross_candidates: list[tuple[int, int]] = []

        for i in range(n):
            pi = _section_parent(chunks[i].get("section_path", []), depth=depth)
            for j in range(i + 1, n):
                pj = _section_parent(chunks[j].get("section_path", []), depth=depth)
                if pi == pj:
                    same_candidates.append((i, j))
                else:
                    cross_candidates.append((i, j))

        same_sample = rng.sample(same_candidates, min(max_pairs, len(same_candidates))) if same_candidates else []
        cross_sample = rng.sample(cross_candidates, min(max_pairs, len(cross_candidates))) if cross_candidates else []

        same_cos = [_dot(vecs[i], vecs[j]) for i, j in same_sample]
        cross_cos = [_dot(vecs[i], vecs[j]) for i, j in cross_sample]

        ss = stats(same_cos)
        cs = stats(cross_cos)
        gap = round(ss["mean"] - cs["mean"], 4) if ss["mean"] and cs["mean"] else None
        depth_results[depth] = {
            "same_section": ss,
            "cross_section": cs,
            "semantic_gap": gap,
        }

    # ── 直接父路径分析（最有区分力的维度） ──
    # 使用 section_path 前 N-1 级（即 chunk 的直接父节点路径）
    parent_same: list[tuple[int, int]] = []
    parent_cross: list[tuple[int, int]] = []

    for i in range(n):
        pi_full = chunks[i].get("section_path", [])
        # 父路径 = 除最后一级外的所有级别
        pi_parent = "||".join(pi_full[:-1]) if len(pi_full) > 1 else (pi_full[0] if pi_full else "__orphan__")
        for j in range(i + 1, n):
            pj_full = chunks[j].get("section_path", [])
            pj_parent = "||".join(pj_full[:-1]) if len(pj_full) > 1 else (pj_full[0] if pj_full else "__orphan__")
            if pi_parent == pj_parent:
                parent_same.append((i, j))
            else:
                parent_cross.append((i, j))

    parent_same_sample = rng.sample(parent_same, min(max_pairs, len(parent_same))) if parent_same else []
    parent_cross_sample = rng.sample(parent_cross, min(max_pairs, len(parent_cross))) if parent_cross else []

    parent_same_cos = [_dot(vecs[i], vecs[j]) for i, j in parent_same_sample]
    parent_cross_cos = [_dot(vecs[i], vecs[j]) for i, j in parent_cross_sample]

    parent_ss = stats(parent_same_cos)
    parent_cs = stats(parent_cross_cos)
    parent_gap = round(parent_ss["mean"] - parent_cs["mean"], 4) if parent_ss["mean"] and parent_cs["mean"] else None

    # ── 全量采样 ──
    all_candidates = [(i, j) for i in range(n) for j in range(i + 1, n)]
    all_sample = rng.sample(all_candidates, min(1000, len(all_candidates))) if all_candidates else []
    all_cos = [_dot(vecs[i], vecs[j]) for i, j in all_sample]
    all_stats = stats(all_cos)

    # ── 判据（使用 parent 级别，区分力最强） ──
    gap = parent_gap
    ratio = None
    if parent_ss["mean"] and parent_cs["mean"] and parent_cs["mean"] > 0:
        ratio = round(parent_ss["mean"] / parent_cs["mean"], 4)

    pass_gap = gap is not None and gap >= BASELINE["semantic_gap_min"]
    pass_ratio = ratio is not None and ratio >= 1.10

    return {
        "depth_analysis": depth_results,
        "parent_section": {
            "same": parent_ss,
            "cross": parent_cs,
        },
        "parent_semantic_gap": gap,
        "parent_separation_ratio": ratio,
        "all_sample": all_stats,
        "pass_gap": pass_gap,
        "pass_ratio": pass_ratio,
        "pass": pass_gap and pass_ratio,
    }


# ─── V5.5: 维度坍塌检测 ───────────────────────────────────────


def check_dimensional_collapse(vecs: list[list[float]], verbose: bool = False) -> dict:
    """检测零向量、全等对、死维度、值分布"""
    n = len(vecs)
    d = len(vecs[0]) if vecs else 0

    # 值分布
    all_vals = [v for vec in vecs for v in vec]
    val_min = min(all_vals)
    val_max = max(all_vals)
    val_mean = sum(all_vals) / len(all_vals)
    val_std = math.sqrt(sum((x - val_mean) ** 2 for x in all_vals) / len(all_vals))

    # 检查 NaN/Inf
    nan_count = sum(1 for x in all_vals if math.isnan(x) or math.isinf(x))

    # 零向量
    zero_vec_count = 0
    for v in vecs:
        norm = math.sqrt(sum(x * x for x in v))
        if norm == 0.0:
            zero_vec_count += 1

    # 全等对检测 (采样)
    identical_pairs = 0
    if n <= 200:
        for i in range(n):
            for j in range(i + 1, n):
                cos = _dot(vecs[i], vecs[j])
                if cos > 0.99999:  # 接近 1.0 = 全等
                    identical_pairs += 1

    # 死维度: 每个维度的方差
    dead_dims = 0
    dim_variances = []
    for dim_idx in range(d):
        col = [vecs[i][dim_idx] for i in range(n)]
        col_mean = sum(col) / n
        col_var = sum((x - col_mean) ** 2 for x in col) / n
        dim_variances.append(col_var)
        if col_var < 1e-12:  # 方差接近 0 → 死维度
            dead_dims += 1

    dim_var_min = min(dim_variances)
    dim_var_max = max(dim_variances)
    dim_var_mean = sum(dim_variances) / d

    # 饱和度: [-0.5, 0.5] 之外的比例
    saturation_count = sum(1 for x in all_vals if abs(x) > 0.5)
    saturation_ratio = saturation_count / len(all_vals)

    all_pass = (
        zero_vec_count <= BASELINE["zero_vectors_max"]
        and dead_dims <= BASELINE["dead_dimensions_max"]
        and nan_count == 0
        and abs(val_mean) <= BASELINE["value_mean_abs_max"]
    )

    return {
        "value_min": round(val_min, 6),
        "value_max": round(val_max, 6),
        "value_mean": round(val_mean, 6),
        "value_std": round(val_std, 6),
        "nan_inf_count": nan_count,
        "zero_vectors": zero_vec_count,
        "identical_pairs": identical_pairs,
        "dead_dimensions": dead_dims,
        "dim_var_min": round(dim_var_min, 8),
        "dim_var_max": round(dim_var_max, 8),
        "dim_var_mean": round(dim_var_mean, 8),
        "saturation_ratio": round(saturation_ratio, 6),
        "pass": all_pass,
    }


# ─── V5.7: 序列化往返 ─────────────────────────────────────────


def check_serialization_roundtrip(meta: dict, vecs: list[list[float]]) -> dict:
    """验证 .bin 文件大小与元数据一致，向量值合法"""
    chunk_count = meta["chunk_count"]
    dim = meta["dimension"]

    # 元数据内部一致性
    meta_chunk_count = len(meta["chunks"])
    meta_consistent = (meta_chunk_count == chunk_count)

    # 向量文件大小已由 load_index 校验

    # 抽样逐元素精度检查 (加载时 struct.unpack 已做)
    # 这里验证 f32 范围内无溢出
    all_pass = True
    bad_count = 0
    for vec in vecs:
        for val in vec:
            if math.isnan(val) or math.isinf(val) or abs(val) > 1e6:
                bad_count += 1
                all_pass = False

    return {
        "meta_chunks_match": meta_consistent,
        "chunk_count": chunk_count,
        "dimension": dim,
        "binary_size_bytes": chunk_count * dim * 4,
        "bad_values": bad_count,
        "pass": all_pass and meta_consistent,
    }


# ─── 主流程 ───────────────────────────────────────────────────


def main():
    if len(sys.argv) < 2:
        print("用法: python validate_embeddings.py <index_dir> [--verbose]")
        print("示例: python validate_embeddings.py output/embeddings/清华_embedding_index/")
        sys.exit(1)

    index_dir = sys.argv[1]
    verbose = "--verbose" in sys.argv

    print("=" * 60)
    print(f"Embedding Quality Report")
    print(f"Index: {index_dir}")
    print("=" * 60)

    # 加载
    try:
        meta, vecs = load_index(index_dir)
    except Exception as e:
        print(f"\n[FATAL] 加载失败: {e}")
        sys.exit(1)

    n = meta["chunk_count"]
    d = meta["dimension"]
    doc_id = meta["document_id"]
    print(f"\n  Document ID:  {doc_id}")
    print(f"  Chunk count:  {n}")
    print(f"  Dimension:    {d}")
    print(f"  Model:        BGE-M3 (gpahal/bge-m3-onnx-int8)")

    results = {}
    all_pass = True

    # ── V5.3 ──
    print(f"\n{'─' * 60}")
    print("[V5.3] L2 Normalization")
    print(f"{'─' * 60}")
    r53 = check_l2_normalization(vecs, BASELINE["l2_norm_tolerance"])
    results["V5.3"] = r53
    status = "PASS" if r53["pass"] else "FAIL"
    if not r53["pass"]:
        all_pass = False
    print(f"  L2 norm:  min={r53['min']}, max={r53['max']}, mean={r53['mean']}, std={r53['std']}")
    print(f"  Zero vectors: {r53['zero_vectors']}")
    print(f"  Status:  [{status}]")

    # ── V5.4 ──
    print(f"\n{'─' * 60}")
    print("[V5.4] Semantic Discrimination")
    print(f"{'─' * 60}")
    r54 = check_semantic_discrimination(meta, vecs, verbose=verbose)
    results["V5.4"] = r54
    if "error" in r54:
        print(f"  SKIP: {r54['error']}")
    else:
        # 多深度分析
        for depth in [1, 2]:
            dr = r54["depth_analysis"][depth]
            ss = dr["same_section"]
            cs = dr["cross_section"]
            print(f"  depth={depth} (前{depth}级):  "
                  f"same {ss['count']:>5} pairs, mean_cos={ss['mean'] or 'N/A':>7}  |  "
                  f"cross {cs['count']:>5} pairs, mean_cos={cs['mean'] or 'N/A':>7}  |  "
                  f"gap={dr['semantic_gap'] or 'N/A'}")
        # Parent 级别 (最细)
        ps = r54["parent_section"]
        print(f"  parent (直接父):  same {ps['same']['count']:>5} pairs, mean_cos={ps['same']['mean'] or 'N/A':>7}  |  "
              f"cross {ps['cross']['count']:>5} pairs, mean_cos={ps['cross']['mean'] or 'N/A':>7}  |  "
              f"gap={r54['parent_semantic_gap'] or 'N/A'}")
        if verbose and ps["same"]["mean"]:
            print(f"                    same range: [{ps['same']['min']}, {ps['same']['max']}]")
            print(f"                    cross range: [{ps['cross']['min']}, {ps['cross']['max']}]")
        # 全量采样
        al = r54["all_sample"]
        print(f"  all-sample:        {al['count']:>5} pairs, mean_cos={al['mean'] or 'N/A':>7}")

        # 判据
        print(f"  Separation ratio (same/cross): {r54['parent_separation_ratio']}  "
              f"(threshold: ≥1.10)")
        print(f"  Semantic gap:                 {r54['parent_semantic_gap']}  "
              f"(threshold: ≥{BASELINE['semantic_gap_min']})")
        sub_pass = []
        sub_pass.append(f"ratio {'✓' if r54['pass_ratio'] else '✗'}")
        sub_pass.append(f"gap {'✓' if r54['pass_gap'] else '✗'}")
        print(f"  Status:  [{'PASS' if r54['pass'] else 'FAIL'}] ({', '.join(sub_pass)})")
        if not r54["pass"]:
            all_pass = False

    # ── V5.5 ──
    print(f"\n{'─' * 60}")
    print("[V5.5] Dimensional Collapse Detection")
    print(f"{'─' * 60}")
    r55 = check_dimensional_collapse(vecs, verbose=verbose)
    results["V5.5"] = r55
    status = "PASS" if r55["pass"] else "FAIL"
    if not r55["pass"]:
        all_pass = False
    print(f"  Value range:     [{r55['value_min']}, {r55['value_max']}]")
    print(f"  Value mean:      {r55['value_mean']}  (abs < {BASELINE['value_mean_abs_max']})")
    print(f"  Value std:       {r55['value_std']}")
    print(f"  NaN/Inf count:   {r55['nan_inf_count']}")
    print(f"  Zero vectors:    {r55['zero_vectors']}")
    print(f"  Identical pairs: {r55['identical_pairs']}")
    print(f"  Dead dimensions: {r55['dead_dimensions']}/{d}")
    print(f"  Dim variance:    min={r55['dim_var_min']}, max={r55['dim_var_max']}, mean={r55['dim_var_mean']}")
    print(f"  Saturation (>|0.5|): {r55['saturation_ratio']:.4%}")
    print(f"  Status:  [{status}]")

    # ── V5.7 ──
    print(f"\n{'─' * 60}")
    print("[V5.7] Serialization Round-trip")
    print(f"{'─' * 60}")
    r57 = check_serialization_roundtrip(meta, vecs)
    results["V5.7"] = r57
    status = "PASS" if r57["pass"] else "FAIL"
    if not r57["pass"]:
        all_pass = False
    print(f"  Meta chunks match: {r57['meta_chunks_match']}")
    print(f"  Binary file:       {r57['binary_size_bytes']} bytes ({r57['chunk_count']} × {r57['dimension']} × 4)")
    print(f"  Bad values (NaN/Inf): {r57['bad_values']}")
    print(f"  Status:  [{status}]")

    # ── OVERALL ──
    print(f"\n{'=' * 60}")
    passed = sum(1 for r in results.values() if r["pass"])
    total = len(results)
    print(f"OVERALL: {passed}/{total} PASSED")
    print(f"{'=' * 60}")

    if not all_pass:
        print("\n⚠  WARNING: Some checks failed. See above for details.")
        sys.exit(1)
    else:
        print("\n✓  All automated checks passed.")

    return results


if __name__ == "__main__":
    main()
