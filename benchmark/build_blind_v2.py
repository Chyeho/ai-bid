from __future__ import annotations

import csv
import hashlib
import io
import json
import sys
import time
import urllib.request
from datetime import datetime
from pathlib import Path
from urllib.parse import urlparse

from pypdf import PdfReader, PdfWriter
from reportlab.lib.pagesizes import A4
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

from build_dataset import LEGAL, TAXONOMY


ROOT = Path(__file__).resolve().parent
DATASET_ROOT = ROOT / "blind-v2"
SOURCE_DIR = DATASET_ROOT / "sources"
MUTATED_DIR = DATASET_ROOT / "mutated"
DATA_DIR = DATASET_ROOT / "data"
RENDER_DIR = DATASET_ROOT / "rendered"
FONT_PATH = Path("C:/Windows/Fonts/simhei.ttf")
VERSION = "blind-v2.0"

SOURCES = [
    (
        "BLIND-001",
        "王寺街道农村市场化保洁项目",
        "陕西省政府采购项目",
        2026,
        "环卫服务",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c21f9a63addd019b6d24355c7e72.pdf?accessCode=ebd5b2d34426ce688c58112365d6b7af",
    ),
    (
        "BLIND-002",
        "2025年纸质图书采购项目",
        "宝鸡文理学院",
        2025,
        "图书",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c50f97c135510198120e7f4471df.pdf?accessCode=bdc8322b37e13948d7f1817fca04c179",
    ),
    (
        "BLIND-003",
        "书博区域事务委托管理服务",
        "陕西省政府采购项目",
        2025,
        "服务",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/610134/2024/12/5/8a69c6f491b2400b01938bb93fc142a4/gpx-template/8a69c61493d24f8301940231729a6fdb.pdf?accessCode=00f88ef98228422c7f2b9cee72de6924",
    ),
    (
        "BLIND-004",
        "县医院血液透析机采购项目",
        "陕西省政府采购项目",
        2025,
        "医疗设备",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c9729455527101946e606cd6100f.pdf?accessCode=709032e0f6dfe65764cf3abf9f5f1058",
    ),
    (
        "BLIND-005",
        "中央财政林业草原改革发展资金国土绿化项目",
        "陕西省政府采购项目",
        2026,
        "林业服务",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c5549a644c2c019d98d2a35c0ce8.pdf?accessCode=d67884a77e4694b9f1b2ae69e862bfae",
    ),
    (
        "BLIND-006",
        "2026年上海文庙展陈服务项目",
        "上海市黄浦区政府采购项目",
        2026,
        "展陈服务",
        "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/20728f8a-e6bd-4730-8c3e-d455cf4f9857.pdf",
    ),
    (
        "BLIND-007",
        "学校大宗食材肉蛋采购项目",
        "陕西省政府采购项目",
        2025,
        "食材配送",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c6ae98b38d940198e48d098d64b3.pdf?accessCode=161e1f20fe81801c3d888e648f0d7fc5",
    ),
    (
        "BLIND-008",
        "上海市黄浦区公开招标采购项目",
        "上海市黄浦区政府采购项目",
        2026,
        "综合",
        "https://jpg.zfcg.sh.gov.cn/sh-gov-open-doc/1108QZ/79c74e5f-0f4d-443a-9757-56237b5a4dc5.pdf",
    ),
    (
        "BLIND-009",
        "西安市红光公园2026年维护管护项目",
        "陕西省政府采购项目",
        2026,
        "园林服务",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c21f9a63addd019d0448159d4581.pdf?accessCode=db403457214a8c45b8087d41f3af5d59",
    ),
    (
        "BLIND-010",
        "宁陕县其他国土绿化项目",
        "陕西省政府采购项目",
        2026,
        "林业服务",
        "https://www.ccgp-shaanxi.gov.cn/gpx-bid-file/ZF_JGBM_000003/zone/2025/1/12/project/gpx-template/8a69c7659a636e49019c2669608c5824.pdf?accessCode=c2bcc63c0d693cdd81a753ae40af722e",
    ),
]

# 与 v1 的 45 条措辞完全分离。每类两条，首次运行前一次性冻结。
BLIND_VARIANTS = {
    "C01_LOCAL_REGISTRATION": [
        "资格审查仅认可工商登记住所在本区的法人单位，注册地址在区外的供应商不进入后续评审。",
        "投标主体应在采购人所在市领取营业执照并持续纳税满二十四个月，异地企业的响应文件直接退回。",
    ],
    "C02_BRAND_LOCK": [
        "门禁摄像机须使用大华DH系列原装产品，技术指标相同或更高的其他厂牌也不予响应。",
        "服务器只准配置浪潮指定机型，评审时不得以兼容设备或等效方案替代。",
    ],
    "C03_UNRELATED_CERT": [
        "供应商必须持有全国物业服务百强企业荣誉方可通过资格审查，本采购标的为普通办公设备供货。",
        "资格条件要求投标人取得五星级品牌认证证书，未说明该证书与本项目服务能力的关联。",
    ],
    "C04_REGIONAL_PERFORMANCE": [
        "业绩评分只统计采购人所在省份内签订并验收的合同，省外同规模同类型项目一律不计分。",
        "资格审查须提交本县三个行政事业单位的同类案例，其他地区完成的案例不被认可。",
    ],
    "C05_SCALE_THRESHOLD": [
        "投标企业实缴资本须达到六千万元且上一年度净资产不低于一亿元，否则按无效投标处理。",
        "供应商最近一年主营业务收入必须达到采购预算十五倍以上，未达到该规模不得参与。",
    ],
    "H01_SHORT_DEADLINE": [
        "采购文件于2026年8月3日开始提供，投标文件须在2026年8月15日9时前递交并随即开标。",
        "自供应商下载招标文件之日起十二个自然日截止收件，不因法定节假日顺延。",
    ],
    "H02_EXCESSIVE_DEPOSIT": [
        "本项目预算为500万元，投标保证金固定收取25万元，未在开标前到账的响应无效。",
        "投标人应缴纳采购预算总额6%的保证金，并且只允许从基本账户以现金转账方式支付。",
    ],
    "H03_OEM_AUTHORIZATION": [
        "经销商须把制造商针对本项目签发的专项授权原件装入资格证明文件，缺少该文件即终止审查。",
        "非生产企业参加投标时，原厂项目授权书和售后背书函均属于资格审查必备材料。",
    ],
    "H04_SUBJECTIVE_SCORING": [
        "评委认为实施方案非常好的得10分、较好的得6分、普通的得2分，各档没有列明可核验条件。",
        "根据方案的美观性、感染力和评审专家总体感觉在0至15分之间自由给分，不设置评分刻度。",
    ],
    "H05_LOCAL_AWARD": [
        "投标人获得本市政府部门授予的荣誉每个加2分，其他城市同层级同性质荣誉不加分。",
        "评分只认可本省主管部门颁发的优秀服务企业称号，持有省外奖项的供应商不得分。",
    ],
    "M01_VAGUE_ACCEPTANCE": [
        "成果是否验收合格以采购人使用人员感觉满意为准，不另设测试方法、指标或书面复核程序。",
        "项目完成标准由采购人现场口头确认，采购人可不说明拒绝验收的具体理由。",
    ],
    "M02_UNBOUNDED_IP": [
        "供应商在承接项目前形成的软件组件、算法和工具也须永久无偿转让，并承担全部无限额索赔。",
        "即使侵权源于采购人指定的素材，供应商仍须承担一切责任且赔偿金额不设最高限额。",
    ],
    "M03_UNILATERAL_CHANGE": [
        "履约期间采购人可随时增加任意数量的服务事项，供应商不得增加费用或申请延长工期。",
        "所有后续新增功能自动包含在原合同价内，是否属于新增需求由采购人单方决定。",
    ],
    "M04_CONFLICTING_DATES": [
        "投标须知载明2026年9月8日10时停止收件，同一条款又称2026年9月6日17时以后不再接收。",
        "答疑文件写明开标时间为2026年10月20日9时，日程表同时要求当日8时前完成开标。",
    ],
    "M05_UNCLEAR_PENALTY": [
        "供应商出现采购人认为不适当的情形时，每次违约金可在合同价1%至20%之间任选并重复累计。",
        "任何履约偏差均可由采购人自行确定处罚金额，条款未规定计算基数、触发条件和累计上限。",
    ],
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_jsonl(path: Path, rows: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as stream:
        for row in rows:
            stream.write(json.dumps(row, ensure_ascii=False) + "\n")


def write_csv(path: Path, rows: list[dict]) -> None:
    if not rows:
        return
    flattened = []
    for row in rows:
        item = dict(row)
        for key in ("section_path", "legal_basis", "legal_urls", "match_aliases"):
            if isinstance(item.get(key), list):
                item[key] = "|".join(str(value) for value in item[key])
        flattened.append(item)
    with path.open("w", encoding="utf-8-sig", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(flattened[0]))
        writer.writeheader()
        writer.writerows(flattened)


def download(url: str, destination: Path) -> None:
    request = urllib.request.Request(
        url,
        headers={
            "User-Agent": "Mozilla/5.0 (compatible; TenderBlindBenchmark/2.0)",
            "Accept": "application/pdf,*/*",
        },
    )
    last_error = None
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                body = response.read()
            if not body.startswith(b"%PDF"):
                raise ValueError(f"响应不是PDF，前20字节={body[:20]!r}")
            destination.write_bytes(body)
            return
        except Exception as exc:
            last_error = exc
            time.sleep(2**attempt)
    raise RuntimeError(str(last_error))


def wrap_text(text: str, width: int = 37) -> list[str]:
    return [text[index : index + width] for index in range(0, len(text), width)]


def make_blind_page(document_id: str, clauses: list[dict]) -> bytes:
    buffer = io.BytesIO()
    pdfmetrics.registerFont(TTFont("SimHeiBlind", str(FONT_PATH)))
    pdf = canvas.Canvas(buffer, pagesize=A4)
    pdf.setFont("SimHeiBlind", 15)
    pdf.drawString(56, 800, "采购文件补充条款")
    pdf.setFont("SimHeiBlind", 9)
    pdf.drawString(56, 780, f"文件编号：{document_id}")
    y = 744
    labels = ["一", "二", "三"]
    for index, clause in enumerate(clauses):
        pdf.setFont("SimHeiBlind", 11)
        pdf.drawString(56, y, f"补充条款{labels[index]}")
        y -= 23
        pdf.setFont("SimHeiBlind", 10)
        for line in wrap_text(clause["source_quote"]):
            pdf.drawString(72, y, line)
            y -= 18
        y -= 22
    pdf.setFont("SimHeiBlind", 8)
    pdf.drawString(56, 36, "内部盲测样本：不得用于训练、提示词调整或人工预览后重跑。")
    pdf.save()
    return buffer.getvalue()


def verify_freeze() -> int:
    freeze_path = DATA_DIR / "freeze_manifest.json"
    if not freeze_path.exists():
        print("未冻结")
        return 2
    freeze = json.loads(freeze_path.read_text(encoding="utf-8"))
    mismatches = []
    for relative, expected in freeze["files"].items():
        path = DATASET_ROOT / relative
        actual = sha256(path) if path.exists() else "missing"
        if actual != expected:
            mismatches.append({"file": relative, "expected": expected, "actual": actual})
    print(
        json.dumps(
            {"verified": not mismatches, "mismatches": mismatches},
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0 if not mismatches else 3


def main() -> int:
    if "--verify" in sys.argv:
        return verify_freeze()
    freeze_path = DATA_DIR / "freeze_manifest.json"
    if freeze_path.exists():
        print("blind-v2 已冻结；如需校验请使用 --verify，禁止覆盖。", file=sys.stderr)
        return 2
    for directory in (SOURCE_DIR, MUTATED_DIR, DATA_DIR, RENDER_DIR):
        directory.mkdir(parents=True, exist_ok=True)
    if not FONT_PATH.exists():
        raise FileNotFoundError(f"缺少中文字体：{FONT_PATH}")

    taxonomy = {item["code"]: item for item in TAXONOMY}
    critical = [item["code"] for item in TAXONOMY[:5]]
    high = [item["code"] for item in TAXONOMY[5:10]]
    medium = [item["code"] for item in TAXONOMY[10:15]]
    variant_counters = {code: 0 for code in BLIND_VARIANTS}
    annotations = []
    source_manifest = []

    for index, (doc_id, title, purchaser, year, domain, url) in enumerate(SOURCES):
        source_path = SOURCE_DIR / f"{doc_id}.pdf"
        mutated_path = MUTATED_DIR / f"{doc_id}_mutated.pdf"
        if not source_path.exists():
            print(f"[{index + 1:02d}/10] 下载 {doc_id} {urlparse(url).netloc}", flush=True)
            download(url, source_path)
        reader = PdfReader(str(source_path), strict=False)
        original_pages = len(reader.pages)
        codes = [critical[index % 5], high[index % 5], medium[index % 5]]
        clauses = []
        for code in codes:
            item = taxonomy[code]
            variant_index = variant_counters[code]
            variant_counters[code] += 1
            clauses.append(
                {**item, "source_quote": BLIND_VARIANTS[code][variant_index]}
            )

        page_reader = PdfReader(io.BytesIO(make_blind_page(doc_id, clauses)))
        writer = PdfWriter()
        for page in reader.pages:
            writer.add_page(page)
        writer.add_page(page_reader.pages[0])
        with mutated_path.open("wb") as stream:
            writer.write(stream)

        for ordinal, clause in enumerate(clauses, 1):
            legal_items = [LEGAL[key] for key in clause["legal"]]
            annotations.append(
                {
                    "benchmark_version": VERSION,
                    "split": "test",
                    "document_id": doc_id,
                    "finding_id": f"{doc_id}-F{ordinal:02d}",
                    "annotation_origin": "frozen_synthetic_blind_injection",
                    "category_code": clause["code"],
                    "risk_type": clause["name"],
                    "severity": clause["severity"],
                    "is_critical": clause["critical"],
                    "page_number": original_pages + 1,
                    "section_path": ["采购文件补充条款"],
                    "source_quote": clause["source_quote"],
                    "reason": clause["reason"],
                    "suggestion": clause["suggestion"],
                    "legal_basis": [entry["title"] for entry in legal_items],
                    "legal_urls": [entry["url"] for entry in legal_items],
                    "match_aliases": clause["aliases"],
                    "quote_similarity_threshold": 0.4,
                    "source_file": f"sources/{doc_id}.pdf",
                    "mutated_file": f"mutated/{doc_id}_mutated.pdf",
                    "review_status": "frozen_before_first_run",
                    "annotator": "blind-benchmark-builder-v2",
                }
            )
        source_manifest.append(
            {
                "document_id": doc_id,
                "split": "test",
                "title": title,
                "purchaser_or_source": purchaser,
                "publication_year": year,
                "domain": domain,
                "source_url": url,
                "source_host": urlparse(url).netloc,
                "original_pages": original_pages,
                "injection_page": original_pages + 1,
                "source_sha256": sha256(source_path),
                "mutated_sha256": sha256(mutated_path),
                "source_file": f"sources/{doc_id}.pdf",
                "mutated_file": f"mutated/{doc_id}_mutated.pdf",
            }
        )

    write_jsonl(DATA_DIR / "annotations.jsonl", annotations)
    write_csv(DATA_DIR / "annotations.csv", annotations)
    write_jsonl(DATA_DIR / "source_manifest.jsonl", source_manifest)
    write_csv(DATA_DIR / "source_manifest.csv", source_manifest)

    frozen_files = {}
    for path in sorted(
        list(SOURCE_DIR.glob("*.pdf"))
        + list(MUTATED_DIR.glob("*.pdf"))
        + [
            DATA_DIR / "annotations.jsonl",
            DATA_DIR / "annotations.csv",
            DATA_DIR / "source_manifest.jsonl",
            DATA_DIR / "source_manifest.csv",
        ]
    ):
        frozen_files[path.relative_to(DATASET_ROOT).as_posix()] = sha256(path)
    freeze = {
        "benchmark_version": VERSION,
        "status": "frozen_before_first_run",
        "frozen_at": datetime.now().isoformat(timespec="seconds"),
        "documents": len(SOURCES),
        "annotations": len(annotations),
        "critical_annotations": sum(row["is_critical"] for row in annotations),
        "neutral_clause_headings": True,
        "v1_source_overlap": 0,
        "files": frozen_files,
    }
    freeze_path.write_text(
        json.dumps(freeze, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps(freeze, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
