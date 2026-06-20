"""
PDF 内容提取桥接脚本。

当 Rust 侧 pdfplumber (lopdf) 无法解析畸形 PDF 时，
调用本脚本用 Python pdfplumber (pdfminer.six) 兜底提取。

用法:
    python scripts/pdf_extract.py <input.pdf> <output.json>

输出格式与 Rust 的 RawDocument 对齐，含:
    - document_id, source_path
    - pages[].page_index, width, height, text
    - pages[].words[].text, bbox.{x0, top, x1, bottom}
    - pages[].tables[].rows[][]
    - pages[].lines[].bbox, pages[].rects[].bbox
"""

import json
import re
import sys
import uuid
from pathlib import Path

import pdfplumber


def clean_layout_text(text: str) -> str:
    """清洗 layout 文本：去除排版空格噪音，保留逻辑结构。

    政府标书等 PDF 用绝对定位渲染每个字符，导致 layout=True 输出的 text
    包含大量空格用于对齐。此函数：
    - 删除完全空白行
    - 将连续空格（>2 个）压缩为空行分隔符
    - 合并被空格拆散的汉字（如 竞 争 性 磋 商 → 竞争性磋商）
    """
    lines = text.split("\n")

    # 标记每行的有效内容占比
    cleaned = []
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        # 合并汉字之间的单空格（中文竖排/分散对齐导致的）
        # "竞 争 性 磋 商 文 件" → "竞争性磋商文件"
        stripped = re.sub(r"([一-鿿])\s+(?=[一-鿿])", r"\1", stripped)
        # 压缩连续空格
        stripped = re.sub(r" {2,}", "  ", stripped)
        cleaned.append(stripped)

    return "\n".join(cleaned)


def reconstruct_text_from_words(words) -> str:
    """从单词坐标重建干净文本（当 layout text 不可用时兜底）。

    按 y 坐标分行，行内按 x 坐标排序，自动检测列/块边界。
    """
    if not words:
        return ""

    # 估算行高
    heights = [w["bbox"]["bottom"] - w["bbox"]["top"] for w in words]
    line_height = sorted(heights)[len(heights) // 2]

    # 按 y 分组为行
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

    # 行内排序 + 合并
    lines = []
    for row in rows:
        row.sort(key=lambda w: w["bbox"]["x0"])
        # 检测大间距（列分隔），用 2 个空格表示
        parts = []
        current = [row[0]]
        # 估算字符宽
        avg_w = (row[0]["bbox"]["x1"] - row[0]["bbox"]["x0"]) / max(len(row[0]["text"]), 1)
        col_gap = avg_w * 8  # 列间距阈值

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


def extract_page(page):
    """提取单页全部内容，返回与 RawPage 对齐的 dict."""
    # 文本（layout=True 保留版面结构）
    raw_text = page.extract_text(layout=True) or ""

    # 降级：layout=True 对缺少 ToUnicode CMap 的 CID 字体 PDF（如政府标书）
    # 可能返回空字符串，此时尝试 layout=False 做基础提取
    if not raw_text.strip():
        print(f"  [警告] layout=True 返回空文本，尝试 layout=False 降级提取...")
        raw_text = page.extract_text(layout=False) or ""

    # 单词 + 包围盒
    words = []
    for w in page.extract_words():
        words.append({
            "text": w["text"],
            "bbox": {
                "x0": w["x0"],
                "top": w["top"],
                "x1": w["x1"],
                "bottom": w["bottom"],
            },
        })

    # 清洗文本：处理绝对定位 PDF 的排版空格噪音
    text = clean_layout_text(raw_text)

    # 如果清洗后仍然太空洞（空白占比 > 80%），用单词坐标重建
    if len(text) < len(raw_text) * 0.2:
        print(f"  [优化] 检测到高空白占比 ({len(raw_text)}→{len(text)} 字符)，用单词坐标重建文本...")
        text = reconstruct_text_from_words(words)

    # 表格
    tables = []
    for table in page.extract_tables():
        tables.append({"rows": table})

    # 线段
    lines = []
    for line in page.lines:
        lines.append({
            "bbox": {
                "x0": line["x0"],
                "top": line["top"],
                "x1": line["x1"],
                "bottom": line["bottom"],
            },
        })

    # 矩形
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
        "page_index": page.page_number - 1,  # 转为 0-based，与 Rust 一致
        "width": page.width,
        "height": page.height,
        "text": text,
        "words": words,
        "tables": tables,
        "lines": lines,
        "rects": rects,
    }


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
