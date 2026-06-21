"""
PDF 内容提取桥接脚本。

当 Rust 侧 pdfplumber (lopdf) 无法解析畸形 PDF 时，
调用本脚本用 Python pdfplumber (pdfminer.six) 兜底提取。

用法:
    python scripts/pdf_extract.py <input.pdf> <output.json>

输出格式与 Rust 的 RawDocument 对齐，含:
    - document_id, source_path
    - pages[].page_index, width, height, text
    - pages[].words[].id, text, bbox.{x0, top, x1, bottom}
    - pages[].blocks[].id, type, text, bbox
    - pages[].tables[].id, bbox, rows[][]
    - pages[].lines[].bbox, pages[].rects[].bbox

ID 命名规则:
    - 单词: "w_{页码}_{序号}"    如 "w_3_15"
    - 表格: "t_{页码}_{序号}"    如 "t_2_0"
    - 段落: "b_{页码}_{序号}"    如 "b_5_3"
"""

import json
import re
import sys
import uuid
from pathlib import Path

import pdfplumber


# ─── 文本清洗 ───────────────────────────────────────────────

def clean_layout_text(text: str) -> str:
    """清洗 layout 文本：去除排版空格噪音，保留逻辑结构。"""
    lines = text.split("\n")
    cleaned = []
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        stripped = re.sub(r"([一-鿿])\s+(?=[一-鿿])", r"\1", stripped)
        stripped = re.sub(r" {2,}", "  ", stripped)
        cleaned.append(stripped)
    return "\n".join(cleaned)


def reconstruct_text_from_words(words: list) -> str:
    """从单词坐标重建干净文本（当 layout text 不可用时兜底）。"""
    if not words:
        return ""

    heights = [w["bbox"]["bottom"] - w["bbox"]["top"] for w in words]
    line_height = sorted(heights)[len(heights) // 2]

    sorted_words = sorted(words, key=lambda w: (w["bbox"]["top"], w["bbox"]["x0"]))
    rows = []
    current_row = [sorted_words[0]]
    current_top = sorted_words[0]["bbox"]["top"]

    for w in sorted_words[1:]:
        if w["bbox"]["top"] - current_top < line_height * 1.2:
            current_row.append(w)
        else:
            rows.append(current_row)
            current_row = [w]
            current_top = w["bbox"]["top"]
    rows.append(current_row)

    lines = []
    for row in rows:
        row.sort(key=lambda w: w["bbox"]["x0"])
        parts = []
        current = [row[0]]
        avg_w = (row[0]["bbox"]["x1"] - row[0]["bbox"]["x0"]) / max(len(row[0]["text"]), 1)
        col_gap = avg_w * 8

        for w in row[1:]:
            gap = w["bbox"]["x0"] - current[-1]["bbox"]["x1"]
            if gap < col_gap:
                current.append(w)
            else:
                parts.append("".join(w["text"] for w in current))
                current = [w]
        parts.append("".join(w["text"] for w in current))

        line = "  ".join(p for p in parts if p)
        if line.strip():
            lines.append(line)

    return "\n".join(lines)


# ─── 段落块计算 ─────────────────────────────────────────────

HEADING_GAP_RATIO = 1.8


def compute_blocks(words: list, page_index: int) -> list:
    """从单词列表计算出语义段落块，每块含 id / type / text / bbox。"""
    if not words:
        return []

    heights = [w["bbox"]["bottom"] - w["bbox"]["top"] for w in words]
    line_height = sorted(heights)[len(heights) // 2]

    sorted_words = sorted(words, key=lambda w: (w["bbox"]["top"], w["bbox"]["x0"]))

    # Step 1: 按 y 分组为行
    text_rows = []
    current_row = [sorted_words[0]]
    current_top = sorted_words[0]["bbox"]["top"]

    for w in sorted_words[1:]:
        if w["bbox"]["top"] - current_top < line_height * 1.2:
            current_row.append(w)
        else:
            text_rows.append(current_row)
            current_row = [w]
            current_top = w["bbox"]["top"]
    text_rows.append(current_row)

    # Step 2: 按行间距合并为段落
    blocks = []
    block_rows = []
    prev_bottom = None

    for i, row in enumerate(text_rows):
        row_top = min(w["bbox"]["top"] for w in row)
        row_bottom = max(w["bbox"]["bottom"] for w in row)

        start_new = prev_bottom is not None and (row_top - prev_bottom) > line_height * HEADING_GAP_RATIO

        if start_new and block_rows:
            blocks.append(_build_block(block_rows, page_index, len(blocks)))
            block_rows = []

        block_rows.append(row)
        prev_bottom = row_bottom

        if i == len(text_rows) - 1:
            blocks.append(_build_block(block_rows, page_index, len(blocks)))

    return blocks


def _build_block(rows: list, page_index: int, block_index: int) -> dict:
    """将一组行构建为一个块 dict。"""
    all_words = [w for row in rows for w in row]

    x0 = min(w["bbox"]["x0"] for w in all_words)
    top = min(w["bbox"]["top"] for w in all_words)
    x1 = max(w["bbox"]["x1"] for w in all_words)
    bottom = max(w["bbox"]["bottom"] for w in all_words)

    row_texts = []
    for row in rows:
        sorted_row = sorted(row, key=lambda w: w["bbox"]["x0"])
        text = "".join(w["text"] for w in sorted_row)
        if text.strip():
            row_texts.append(text)

    block_type = "heading" if (len(rows) == 1 and len(all_words) <= 10) else "paragraph"

    return {
        "id": f"b_{page_index}_{block_index}",
        "type": block_type,
        "text": "\n".join(row_texts),
        "bbox": {"x0": x0, "top": top, "x1": x1, "bottom": bottom},
    }


# ─── 单页提取 ───────────────────────────────────────────────

def extract_page(page):
    """提取单页全部内容，返回与 Rust RawPage 对齐的 dict。"""
    page_index = page.page_number - 1  # 0-based

    # 1. 文本
    raw_text = page.extract_text(layout=True) or ""
    if not raw_text.strip():
        print(f"  [警告] layout=True 返回空文本，尝试 layout=False 降级提取...")
        raw_text = page.extract_text(layout=False) or ""

    # 2. 单词（带 ID）
    words = []
    for i, w in enumerate(page.extract_words()):
        words.append({
            "id": f"w_{page_index}_{i}",
            "text": w["text"],
            "bbox": {
                "x0": w["x0"],
                "top": w["top"],
                "x1": w["x1"],
                "bottom": w["bottom"],
            },
        })

    # 文本清洗
    text = clean_layout_text(raw_text)
    if len(text) < len(raw_text) * 0.2:
        print(f"  [优化] 检测到高空白占比 ({len(raw_text)}→{len(text)} 字符)，用单词坐标重建文本...")
        text = reconstruct_text_from_words(words)

    # 3. 段落块
    blocks = compute_blocks(words, page_index)

    # 4. 表格（带 ID + bbox，使用 find_tables 获取坐标）
    tables = []
    try:
        found_tables = page.find_tables()
    except Exception:
        found_tables = []
    extracted_data = page.extract_tables()

    for i, table_obj in enumerate(found_tables):
        try:
            bbox = {
                "x0": table_obj.bbox[0],
                "top": table_obj.bbox[1],
                "x1": table_obj.bbox[2],
                "bottom": table_obj.bbox[3],
            }
        except (AttributeError, IndexError, TypeError):
            bbox = None

        rows = extracted_data[i] if i < len(extracted_data) else []
        tables.append({
            "id": f"t_{page_index}_{i}",
            "bbox": bbox,
            "rows": rows,
        })

    # 如果 find_tables 为空但 extract_tables 有数据，兜底
    if not found_tables and extracted_data:
        for i, rows in enumerate(extracted_data):
            tables.append({
                "id": f"t_{page_index}_{i}",
                "bbox": None,
                "rows": rows,
            })

    # 5. 线段
    lines_data = []
    for line in page.lines:
        lines_data.append({
            "bbox": {
                "x0": line["x0"],
                "top": line["top"],
                "x1": line["x1"],
                "bottom": line["bottom"],
            },
        })

    # 6. 矩形
    rects = []
    for rect in page.rects:
        rects.append({
            "bbox": {
                "x0": rect["x0"],
                "top": rect["top"],
                "x1": rect["x1"],
                "bottom": rect["bottom"],
            },
        })

    return {
        "page_index": page_index,
        "width": page.width,
        "height": page.height,
        "text": text,
        "words": words,
        "blocks": blocks,
        "tables": tables,
        "lines": lines_data,
        "rects": rects,
    }


# ─── 入口 ───────────────────────────────────────────────────

def main():
    if len(sys.argv) != 3:
        print(f"用法: {sys.argv[0]} <input.pdf> <output.json>", file=sys.stderr)
        sys.exit(1)

    input_path = sys.argv[1]
    output_path = sys.argv[2]

    pdf = pdfplumber.open(input_path)
    try:
        pages = []
        for page in pdf.pages:
            pages.append(extract_page(page))

        doc = {
            "document_id": str(uuid.uuid4()),
            "source_path": input_path,
            "pages": pages,
        }

        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(doc, f, ensure_ascii=False, indent=2)

        print(f"PDF raw JSON 已生成 (Python 兜底): {output_path}")
    finally:
        pdf.close()


if __name__ == "__main__":
    main()
