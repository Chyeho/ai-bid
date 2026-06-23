Chunk 切分质量修复方案
Context
对清华大学深圳国际研究生院智慧校园项目标书切分结果的审查发现 6 类问题，根因分两层：

上游 sectionize：缺少 "第X部分" 模式 → 无 Level 1；cjk_numbered 过度匹配长法规引用 → 标题含正文噪音；7 个 orphan blocks 内容丢失
下游 chunking：无碎片合并后处理 → ch_066 (23字) 等极小块；embed_text 未截断长路径 → 向量嵌入噪音
改动概览
修改 4 个文件，按优先级排列：

改动 1：sectionize — 新增 "第X部分" Level 1 模式
文件：src/services/sectionize_service.rs

在 HEADING_PATTERNS 数组最前面新增：

HeadingPattern {
    pattern_type: "part",
    level: 1,
    regex: Regex::new(r"^第[一二三四五六七八九十百千]+部分").expect("part regex"),
},
级别分配调整：插入 Level 1 后，现有 chapter 改 Level 2，section 和 cjk_numbered 改 Level 3，后续依次 +1。但这会破坏现有 golden data。更好的方案：将 part 设为 Level 1，chapter 保持 Level 1（并列），section 保持 Level 2 —— 因为中文标书实际使用"第X部分"和"第X章"不会同时出现。

改动 2：sectionize — 标题长度上限过滤
文件：src/services/sectionize_service.rs

在标题扫描循环中，对 Level 1~3 的匹配也增加长度上限（目前仅 Level 4+ 有 >80 字符过滤）。添加：

// 当前仅对 level >= 4 执行
if pattern.level >= 4 && title.chars().count() > 80 { continue; }
// 改为全部 level 执行，但阈值随 level 不同：
// Level 1-2: > 40 字符 → 跳过（章/节标题本身短）
// Level 3:   > 60 字符 → 跳过
// Level 4-5: > 80 字符 → 跳过（保持不变）
这解决 "一、《深圳经济特区政府采购条例》第五十七条..." (70+ chars) 被错误识别为 Level 2 标题的问题。

改动 3：chunking — 碎片合并后处理
文件：src/services/chunking_service.rs、src/domain/chunk.rs

3a. ChunkingConfig 新增字段（chunk.rs）：

pub min_chunk_size: usize,  // 默认 30，低于此值的 chunk 合并到相邻 chunk
3b. 新增 merge_tiny_chunks() 函数（chunking_service.rs）：

在 chunk_sections() 末尾调用。逻辑：

扫描所有 chunk，找到 text.chars().count() < min_chunk_size 的 chunk
将其合并到前一个相邻 chunk（若不存在则合并到后一个）
合并后更新 chunk 类型为 Merged { rule: "tiny_merge", child_count }
若合并后总长超过 split_max_len，仍然保持合并（tiny chunk 通常只有几十字，不会超限）
改动 4：chunking — embed_text 路径元素截断
文件：src/services/chunking_service.rs

在 Chunk::embed_text() 方法中，对每个 section_path 元素增加长度截断：

fn truncate_title(title: &str, max_len: usize) -> String {
    if title.chars().count() <= max_len {
        title.to_string()
    } else {
        format!("{}…", title.chars().take(max_len).collect::<String>())
    }
}
默认 max_len = 40。路径元素 > 40 字符时截断为前 40 字符 + "…"。

改动 5：main.rs — 累加 orphan block 内容为兜底 chunk
文件：src/main.rs

在 chunking 步骤后，检查 sections_output.stats.orphan_blocks > 0：

从 RawDocument 收集所有已分配到 section 的 block_ids
找出未分配的 block_ids
将其内容拼接为一个或多个兜底 chunk（ChunkType::Leaf，section_path = ["未归类内容"]）
追加到 chunks 列表末尾，重新分配 chunk_id
影响评估
改动	影响范围	风险
1. part 模式	新增 Level 1，已有的 section tree 结构会变化	低 — 仅增加新层级，不破坏现有匹配
2. 标题长度过滤	过滤掉 >40/60/80 字符的假标题	中 — 需确认真实长标题（如某些法规名称）不被误杀
3. 碎片合并	极小 chunk 被吸收	低 — 纯后处理，不影响核心逻辑
4. embed 截断	仅影响 embed_text 输出，不改 chunk.text	低 — 纯展示层
5. orphan 兜底	恢复丢失内容	低 — 追加而非修改
验证方法
cargo test — 全部 34 个已有测试 + 新增测试通过
cargo run -- "tests/file/清华大学深圳国际研究生院智慧校园项目公开招标文件.pdf" — 重新生成 chunks JSON
检查新输出：
stats 中 orphan 相关指标
无 text < 30 字符的 chunk（碎片已合并）
embed_text 无超长路径元素
存在 Level 1 section（来自 "第X部分" 匹配）
orphan blocks 内容以兜底 chunk 形式出现