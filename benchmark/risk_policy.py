"""Benchmark/production-compatible risk taxonomy post-processing.

This script deliberately uses only a prediction's structured fields and exact
source quote. It does not inspect gold labels, so it is safe for offline replay.
"""

from __future__ import annotations

import argparse
import json
import re
from difflib import SequenceMatcher
from pathlib import Path


CATEGORY_NAMES = {
    "LOCAL_REGISTRATION": "地域注册限制",
    "BRAND_LOCK": "指定品牌且不接受同等产品",
    "UNRELATED_CERT": "设置与履约无关的资格条件",
    "REGIONAL_PERFORMANCE": "特定区域业绩限制",
    "SCALE_THRESHOLD": "以经营规模设置资格门槛",
    "SHORT_DEADLINE": "投标准备期不足",
    "EXCESSIVE_DEPOSIT": "投标保证金比例过高",
    "OEM_AUTHORIZATION": "将厂家授权作为资格条件",
    "SUBJECTIVE_SCORING": "主观评分未细化量化",
    "LOCAL_AWARD": "本地奖项加分",
    "VAGUE_ACCEPTANCE": "验收标准模糊",
    "UNBOUNDED_IP": "知识产权责任无限扩大",
    "UNILATERAL_CHANGE": "采购人可单方无限变更需求",
    "CONFLICTING_DATES": "关键日期相互矛盾",
    "UNCLEAR_PENALTY": "违约责任口径不清",
}

ALIASES = {
    "UNRELATED_CERTIFICATE": "UNRELATED_CERT",
    "UNRELATED_CERTIFICATION": "UNRELATED_CERT",
    "UNRELATED_QUALIFICATION": "UNRELATED_CERT",
    "IRRELEVANT_CERTIFICATE": "UNRELATED_CERT",
    "SHORT_PREPARATION_PERIOD": "SHORT_DEADLINE",
    "UNREASONABLE_TIME_LIMIT": "SHORT_DEADLINE",
    "UNREASONABLE_PREPARATION_TIME": "SHORT_DEADLINE",
    "SHORT_BIDDING_PERIOD": "SHORT_DEADLINE",
    "TIME_LIMIT": "SHORT_DEADLINE",
    "ASSET_THRESHOLD": "SCALE_THRESHOLD",
    "ASSET_REQUIREMENT": "SCALE_THRESHOLD",
    "CAPITAL_THRESHOLD": "SCALE_THRESHOLD",
    "MANUFACTURER_AUTHORIZATION": "OEM_AUTHORIZATION",
    "FACTORY_AUTHORIZATION": "OEM_AUTHORIZATION",
    "SCORING_DISCRETION": "SUBJECTIVE_SCORING",
    "UNSPECIFIED_SCORING": "SUBJECTIVE_SCORING",
    "UNQUANTIFIED_ASSESSMENT": "SUBJECTIVE_SCORING",
    "LOCAL_CERTIFICATE_BONUS": "LOCAL_AWARD",
    "LOCAL_HONOR_BONUS": "LOCAL_AWARD",
    "UNCLEAR_ACCEPTANCE_CRITERIA": "VAGUE_ACCEPTANCE",
    "ACCEPTANCE_CRITERIA": "VAGUE_ACCEPTANCE",
    "AMBIGUOUS_ACCEPTANCE": "VAGUE_ACCEPTANCE",
    "UNCLEAR_ACCEPTANCE": "VAGUE_ACCEPTANCE",
    "UNLIMITED_IP_LIABILITY": "UNBOUNDED_IP",
    "IP_LIABILITY": "UNBOUNDED_IP",
    "UNDEFINED_PENALTY": "UNCLEAR_PENALTY",
    "UNLIMITED_PENALTY": "UNCLEAR_PENALTY",
    "UNCLEAR_CONTRACTUAL_RESPONSIBILITY": "UNCLEAR_PENALTY",
    "DATE_CONFLICT": "CONFLICTING_DATES",
    "关键日期矛盾": "CONFLICTING_DATES",
}


def any_word(text: str, words: tuple[str, ...]) -> bool:
    return any(word in text for word in words)


def clean_code(value: object) -> str:
    code = re.sub(r"[^A-Z0-9_]", "", str(value or "").upper())
    return re.sub(r"^[A-Z]\d+_", "", code)


def category_from_evidence(text: str) -> str | None:
    regional = any_word(text, ("本市", "本区", "本县", "本省", "当地", "所在地", "所在区县", "采购人所在地"))
    if regional and any_word(text, ("注册", "分公司", "分支机构", "经营满")) and any_word(text, ("须", "必须", "仅限", "不接受", "资格")):
        return "LOCAL_REGISTRATION"
    if regional and any_word(text, ("业绩", "案例", "合同")) and any_word(text, ("须", "必须", "仅限", "不认可", "不予认可", "不得")):
        return "REGIONAL_PERFORMANCE"
    if regional and any_word(text, ("奖项", "荣誉", "获奖", "证书", "诚信企业")) and any_word(text, ("加分", "得分", "评分", "分")):
        return "LOCAL_AWARD"
    if any_word(text, ("注册资本", "营业收入", "资产总额", "净资产")) and any_word(text, ("不得低于", "不少于", "以上", "门槛", "资格")):
        return "SCALE_THRESHOLD"
    if any_word(text, ("品牌", "商标", "型号")) and any_word(text, ("仅", "只能", "唯一", "指定", "不接受", "不得偏离")):
        return "BRAND_LOCK"
    if any_word(text, ("原厂", "厂家", "制造商")) and any_word(text, ("授权", "承诺函", "证明")) and any_word(text, ("资格", "无效", "废标", "必须", "须")):
        return "OEM_AUTHORIZATION"
    if any_word(text, ("认证", "证书", "荣誉", "示范企业")) and any_word(text, ("资格", "无效", "废标", "不通过", "必须", "须提供", "无关")):
        return "UNRELATED_CERT"
    if "投标保证金" in text and any_word(text, ("3%", "4%", "5%", "百分之三", "百分之四", "百分之五", "比例过高")):
        return "EXCESSIVE_DEPOSIT"
    if any_word(text, ("投标截止", "开标", "投标准备", "获取招标文件")) and any_word(text, ("3日", "5日", "不足", "少于", "仅有", "仅")):
        return "SHORT_DEADLINE"
    if any_word(text, ("评分", "得分", "评委")) and any_word(text, ("酌情", "自行掌握", "主观", "优良", "满意程度", "综合判断")):
        return "SUBJECTIVE_SCORING"
    if "验收" in text and any_word(text, ("满意", "自行判断", "无异议", "未明确", "不明确", "无需说明")):
        return "VAGUE_ACCEPTANCE"
    if any_word(text, ("知识产权", "侵权", "专利", "既有软件", "权利")) and any_word(text, ("全部责任", "一切责任", "无限", "无上限", "既有", "永久归")):
        return "UNBOUNDED_IP"
    if any_word(text, ("单方", "新增需求", "任意变更", "采购人有权变更")) and any_word(text, ("不得调整", "不调整", "无条件", "原合同范围", "费用", "工期")):
        return "UNILATERAL_CHANGE"
    if any_word(text, ("日期", "截止", "开标时间")) and any_word(text, ("矛盾", "不一致", "另一处", "分别为", "同时规定")):
        return "CONFLICTING_DATES"
    if any_word(text, ("违约金", "违约责任", "处罚")) and any_word(text, ("重复", "累计", "无上限", "自行决定", "不明确", "不清")):
        return "UNCLEAR_PENALTY"
    return None


def canonical_category(row: dict) -> str:
    evidence = category_from_evidence(str(row.get("source_quote") or ""))
    if evidence:
        return evidence
    for value in (row.get("category_code"), row.get("risk_type")):
        code = clean_code(value)
        if code in CATEGORY_NAMES:
            return code
        if code in ALIASES:
            return ALIASES[code]
    return clean_code(row.get("category_code") or row.get("risk_type"))


def is_critical(code: str, quote: str) -> bool:
    checks = {
        "LOCAL_REGISTRATION": any_word(quote, ("注册", "分公司", "分支机构")) and any_word(quote, ("本市", "本区", "本县", "所在地", "外地")),
        "BRAND_LOCK": any_word(quote, ("品牌", "商标", "型号")) and any_word(quote, ("仅", "只能", "唯一", "不接受", "指定")),
        "UNRELATED_CERT": any_word(quote, ("认证", "证书", "荣誉", "示范企业")) and any_word(quote, ("资格", "无效", "废标", "不通过", "必须", "须", "无关")),
        "REGIONAL_PERFORMANCE": any_word(quote, ("业绩", "案例", "合同")) and any_word(quote, ("本市", "本区", "本县", "本省", "当地", "所在区县")),
        "SCALE_THRESHOLD": any_word(quote, ("注册资本", "营业收入", "资产总额", "净资产")) and any_word(quote, ("不得低于", "不少于", "以上", "资格")),
    }
    return bool(checks.get(code, False))


def normalize_quote(value: object) -> str:
    return re.sub(r"\s+", "", str(value or ""))


def postprocess(rows: list[dict]) -> tuple[list[dict], dict]:
    retained: list[dict] = []
    rejected = 0
    for original in rows:
        row = dict(original)
        quote = str(row.get("source_quote") or "").strip()
        looks_like_heading = len(quote) < 20 and not any_word(
            quote,
            ("须", "必须", "不得", "不接受", "不予", "否则", "仅限", "无上限", "永久归", "承担", "得分", "加分"),
        )
        if not row.get("no_risk") and (
            not quote
            or looks_like_heading
            or (
                str(row.get("severity") or "").lower() == "info"
                and any_word(quote, ("未提及", "未发现", "未说明", "需要进一步确认", "建议进一步审查"))
            )
        ):
            rejected += 1
            continue
        code = canonical_category(row)
        row["category_code"] = code
        if code in CATEGORY_NAMES:
            row["risk_type"] = CATEGORY_NAMES[code]
        critical = not row.get("no_risk") and is_critical(code, quote)
        row["is_critical"] = critical
        row["critical_reason"] = (
            f"命中重大问题分类 {code}，且原文证据满足红线判定条件。" if critical else ""
        )
        if critical:
            row["severity"] = "high"
        normalized = normalize_quote(quote)
        duplicate_index = next(
            (
                index
                for index, existing in enumerate(retained)
                if str(existing.get("document_id") or "") == str(row.get("document_id") or "")
                and existing.get("category_code") == code
                and (
                    normalized in normalize_quote(existing.get("source_quote"))
                    or normalize_quote(existing.get("source_quote")) in normalized
                    or SequenceMatcher(
                        None,
                        normalized,
                        normalize_quote(existing.get("source_quote")),
                    ).ratio() >= 0.75
                )
            ),
            None,
        )
        if duplicate_index is None:
            retained.append(row)
        elif float(row.get("confidence") or 0) > float(retained[duplicate_index].get("confidence") or 0):
            retained[duplicate_index] = row
    output = retained
    return output, {
        "input": len(rows),
        "retained": len(output),
        "rejected_evidence": rejected,
        "deduplicated": len(rows) - rejected - len(output),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="统一分类、证据过滤、Critical策略和跨Agent去重")
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stats", type=Path)
    args = parser.parse_args()
    with args.input.open("r", encoding="utf-8-sig") as stream:
        rows = [json.loads(line) for line in stream if line.strip()]
    output, stats = postprocess(rows)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as stream:
        for row in output:
            stream.write(json.dumps(row, ensure_ascii=False) + "\n")
    if args.stats:
        args.stats.write_text(json.dumps(stats, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(stats, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
