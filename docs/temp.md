RawDocument {
    document_id: "efec145e-...",   // 文档唯一ID
    source_path: "tests/...pdf",    // 源文件路径
    pages: [                        // ← 92页，每页一个对象
        {
            page_index: 0,          // 页码 (0-based)
            width: 595.32,          // 页面尺寸 (pt)
            height: 841.92,
            text: "广东省政府采购\n竞争性磋商文件\n...",  // ← 清洗后的纯文本
            words: [               // ← 每个词的坐标 (用于定位/高亮)
                { text: "广东省政府采购", bbox: {x0, top, x1, bottom} },
                { text: "竞", bbox: {...} },
                ...
            ],
            tables: [],            // ← 表格 (二维数组)
            lines: [],             // ← 线段
            rects: [],             // ← 矩形区域
        }
    ]
}
