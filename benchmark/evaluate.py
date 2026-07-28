from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from difflib import SequenceMatcher
from pathlib import Path


def load_jsonl(path: Path) -> list[dict]:
    rows = []
    with path.open("r", encoding="utf-8-sig") as f:
        for line_number, line in enumerate(f, 1):
            if line.strip():
                try:
                    rows.append(json.loads(line))
                except json.JSONDecodeError as exc:
                    raise ValueError(f"{path}:{line_number}: {exc}") from exc
    return rows


def normalize(value: object) -> str:
    text = str(value or "").lower()
    return re.sub(r"[\s，。；：、“”‘’（）()【】\[\]《》<>—\-_/]+", "", text)


def quote_similarity(a: object, b: object) -> float:
    left, right = normalize(a), normalize(b)
    if not left or not right:
        return 0.0
    if left in right or right in left:
        # 预测结果可能把同一条款块的多项原文合并引用；只要完整包含
        # 真值引文，就视为引文定位成功，不能按总长度比例惩罚。
        return 1.0
    return SequenceMatcher(None, left, right).ratio()


def normalize_category_code(value: object) -> str:
    code = normalize(value)
    return re.sub(r"^[a-z]\d+", "", code)


def type_matches(gold: dict, pred: dict) -> bool:
    gold_code = normalize_category_code(gold.get("category_code"))
    pred_code = normalize_category_code(pred.get("category_code"))
    if gold_code and pred_code and gold_code == pred_code:
        return True
    target = normalize(pred.get("risk_type") or pred.get("category"))
    candidates = [gold.get("risk_type"), gold.get("category_code"), *gold.get("match_aliases", [])]
    return any(normalize(value) in target or target in normalize(value) for value in candidates if value and target)


def severity_group(value: object) -> str:
    value = normalize(value)
    mapping = {
        "critical": "critical", "重大": "critical", "严重": "critical",
        "high": "high", "高": "high", "高风险": "high",
        "medium": "medium", "中": "medium", "中风险": "medium",
        "low": "low", "低": "low", "低风险": "low",
        "info": "info", "提示": "info",
    }
    return mapping.get(value, value or "unknown")


def safe_div(a: int, b: int) -> float:
    return a / b if b else 0.0


def metric(tp: int, fp: int, fn: int) -> dict:
    precision = safe_div(tp, tp + fp)
    recall = safe_div(tp, tp + fn)
    f1 = safe_div(2 * precision * recall, precision + recall)
    return {
        "tp": tp, "fp": fp, "fn": fn,
        "precision": round(precision, 4),
        "recall": round(recall, 4),
        "f1": round(f1, 4),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="标书审核 Silver Benchmark 自动评分")
    parser.add_argument("--gold", type=Path, required=True)
    parser.add_argument("--pred", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--include-splits", default="dev,test")
    parser.add_argument(
        "--include-documents",
        default="",
        help="逗号分隔的document_id；用于探针或部分断点运行",
    )
    args = parser.parse_args()

    splits = {part.strip() for part in args.include_splits.split(",") if part.strip()}
    gold = [row for row in load_jsonl(args.gold) if row.get("split") in splits]
    include_documents = {
        part.strip() for part in args.include_documents.split(",") if part.strip()
    }
    if include_documents:
        gold = [row for row in gold if row.get("document_id") in include_documents]
    predictions = load_jsonl(args.pred)
    gold_by_doc = defaultdict(list)
    pred_by_doc = defaultdict(list)
    for row in gold:
        gold_by_doc[row["document_id"]].append(row)
    for row in predictions:
        pred_by_doc[row.get("document_id")].append(row)

    matches = []
    missed = []
    false_positives = []
    out_of_scope = []
    used_pred_ids = set()

    for doc_id, gold_rows in gold_by_doc.items():
        injection_pages = {int(row["page_number"]) for row in gold_rows}
        in_scope_predictions = []
        for pred_index, pred in enumerate(pred_by_doc.get(doc_id, [])):
            try:
                page = int(pred.get("page_number"))
            except (TypeError, ValueError):
                page = None
            quote_in_scope = any(
                quote_similarity(
                    gold_row.get("source_quote"),
                    pred.get("source_quote") or pred.get("context"),
                )
                >= float(gold_row.get("quote_similarity_threshold", 0.45))
                for gold_row in gold_rows
            )
            if page not in injection_pages and not quote_in_scope:
                out_of_scope.append(pred)
            else:
                in_scope_predictions.append((pred_index, pred))

        for gold_row in gold_rows:
            candidates = []
            for pred_index, pred in in_scope_predictions:
                key = (doc_id, pred_index)
                if key in used_pred_ids or not type_matches(gold_row, pred):
                    continue
                similarity = quote_similarity(gold_row["source_quote"], pred.get("source_quote") or pred.get("context"))
                threshold = float(gold_row.get("quote_similarity_threshold", 0.45))
                if similarity >= threshold:
                    severity_bonus = 0.05 if severity_group(gold_row["severity"]) == severity_group(pred.get("severity")) else 0
                    candidates.append((similarity + severity_bonus, pred_index, pred, similarity))
            if candidates:
                _, pred_index, pred, similarity = max(candidates, key=lambda item: item[0])
                used_pred_ids.add((doc_id, pred_index))
                matches.append({
                    "document_id": doc_id,
                    "finding_id": gold_row["finding_id"],
                    "risk_id": pred.get("risk_id") or pred.get("issueNo"),
                    "category_code": gold_row["category_code"],
                    "gold_severity": gold_row["severity"],
                    "pred_severity": pred.get("severity"),
                    "gold_is_critical": bool(gold_row.get("is_critical")),
                    "pred_is_critical": bool(pred.get("is_critical")),
                    "quote_similarity": round(similarity, 4),
                })
            else:
                missed.append(gold_row)

        for pred_index, pred in in_scope_predictions:
            if (doc_id, pred_index) not in used_pred_ids:
                false_positives.append(pred)

    overall = metric(len(matches), len(false_positives), len(missed))
    critical_gold = [row for row in gold if row.get("is_critical") is True or severity_group(row.get("severity")) == "critical"]
    matched_ids = {row["finding_id"] for row in matches}
    critical_detection_hit = sum(row["finding_id"] in matched_ids for row in critical_gold)
    critical_detection_recall = safe_div(critical_detection_hit, len(critical_gold))
    matched_critical_ids = {
        row["finding_id"]
        for row in matches
        if row.get("gold_is_critical")
    }
    critical_flag_hit = sum(
        row["finding_id"] in matched_critical_ids
        and (
            row.get("pred_is_critical") is True
            or severity_group(row.get("pred_severity")) == "critical"
        )
        for row in matches
    )
    critical_recall = safe_div(critical_flag_hit, len(critical_gold))
    critical_prediction_count = sum(
        row.get("pred_is_critical") is True
        or severity_group(row.get("pred_severity")) == "critical"
        for row in matches
    ) + sum(
        pred.get("is_critical") is True
        or severity_group(pred.get("severity")) == "critical"
        for pred in false_positives
    )
    critical_precision = safe_div(critical_flag_hit, critical_prediction_count)
    critical_severity_hit = sum(
        row.get("gold_is_critical") is True
        and severity_group(row.get("gold_severity"))
        == severity_group(row.get("pred_severity"))
        for row in matches
    )
    critical_severity_recall = safe_div(
        critical_severity_hit, len(critical_gold)
    )
    severity_agreement = safe_div(
        sum(
            severity_group(row.get("gold_severity"))
            == severity_group(row.get("pred_severity"))
            for row in matches
        ),
        len(matches),
    )

    by_category = {}
    for category in sorted({row["category_code"] for row in gold}):
        category_gold = [row for row in gold if row["category_code"] == category]
        category_hits = sum(row["finding_id"] in matched_ids for row in category_gold)
        by_category[category] = {
            "gold": len(category_gold),
            "hit": category_hits,
            "recall": round(safe_div(category_hits, len(category_gold)), 4),
        }

    by_document = {}
    for doc_id, rows in sorted(gold_by_doc.items()):
        hits = sum(row["finding_id"] in matched_ids for row in rows)
        by_document[doc_id] = {
            "gold": len(rows),
            "hit": hits,
            "recall": round(safe_div(hits, len(rows)), 4),
        }

    result = {
        "benchmark": "silver-v1.0",
        "scope": {
            "splits": sorted(splits),
            "gold_findings": len(gold),
            "documents": len(gold_by_doc),
            "note": "仅评价人工注入页；原文其他页预测不计入误报。",
        },
        "overall": overall,
        "critical": {
            "gold": len(critical_gold),
            "hit": critical_flag_hit,
            "recall": round(critical_recall, 4),
            "detection_hit": critical_detection_hit,
            "detection_recall": round(critical_detection_recall, 4),
            "severity_correct": critical_severity_hit,
            "severity_recall": round(critical_severity_recall, 4),
            "target": 0.95,
            "passed": critical_recall >= 0.95,
            "predicted": critical_prediction_count,
            "precision": round(critical_precision, 4),
            "precision_target": 0.80,
            "precision_passed": critical_precision >= 0.80,
        },
        "severity_agreement_on_matches": round(severity_agreement, 4),
        "release_gate": {
            "f1_target": 0.80,
            "precision_target": 0.75,
            "critical_recall_target": 0.95,
            "critical_precision_target": 0.80,
            "passed": (
                overall["f1"] >= 0.80
                and overall["precision"] >= 0.75
                and critical_recall >= 0.95
                and critical_precision >= 0.80
            ),
        },
        "by_category": by_category,
        "by_document": by_document,
        "matches": matches,
        "missed": missed,
        "false_positives": false_positives,
        "out_of_scope_predictions": out_of_scope,
    }
    output = json.dumps(result, ensure_ascii=False, indent=2)
    print(output)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(output, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
