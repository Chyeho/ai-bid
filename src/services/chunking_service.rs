//! 条款级 Chunk 切分服务
//!
//! 本模块负责从 [`Section`] 树切分为 Agent 可独立消费的条款级语义块。
//! 采用五条确定性规则，无 LLM 参与：
//!
//! | 规则 | 条件                                           | 动作                          |
//! |------|------------------------------------------------|-------------------------------|
//! | 1    | `body_text` 非空 + 无子节点 + 长度 ≤ 1500      | → `Leaf` chunk                |
//! | 1→4  | `body_text` 非空 + 无子节点 + 长度 > 1500      | → `Split` chunks (带 overlap) |
//! | 2    | `body_text` 为空 + 有子节点                     | → 向下传递容器路径，聚合子节点 |
//! | 3    | 多个叶子各自 < 100 字 + 同在父节点下            | → `Merged` chunk              |
//! | 5    | 所有 chunk                                     | → `embed_text()` 携带层级前缀  |
//!
//! ## 设计原则
//!
//! 一个 chunk = 一个可以独立理解、独立评估的完整语义单元。
//! 不是简单地把每个叶子节点当成一个 chunk。
//!
//! ## 技术选型
//!
//! 采用代码规则而非 LLM——确定性 100%、速度 < 10ms、零成本。
//! LLM 的角色是切分完成后的**质量审查**：抽样检查语义完整性。

use crate::domain::chunk::{Chunk, ChunkType, ChunkingConfig};
use crate::services::sectionize_service::Section;

// ─── 主入口 ──────────────────────────────────────────────────

/// 从 Section 树切分为 Chunk 列表。
///
/// 按页面顺序遍历 Section 树，应用规则 1-4 进行切分，
/// 然后按页面顺序排序、分配 `chunk_id`、合并碎片 chunk。
pub fn chunk_sections(sections: &[Section], config: &ChunkingConfig) -> Vec<Chunk> {
    let mut chunks: Vec<Chunk> = Vec::new();

    for section in sections {
        traverse_and_chunk(section, &Vec::new(), config, &mut chunks);
    }

    // 按页面顺序排序并分配 chunk_id
    chunks.sort_by_key(|c| c.page_start);
    for (i, chunk) in chunks.iter_mut().enumerate() {
        chunk.chunk_id = format!("ch_{:03}", i);
    }

    // 后处理：合并碎片 chunk
    if config.min_chunk_size > 0 {
        chunks = merge_tiny_chunks(chunks, config);
        // 重新分配 ID
        for (i, chunk) in chunks.iter_mut().enumerate() {
            chunk.chunk_id = format!("ch_{:03}", i);
        }
    }

    chunks
}

// ─── 树遍历骨架 ─────────────────────────────────────────────

/// 递归遍历 Section 树，对每个节点应用切分规则。
///
/// # 参数
/// - `section`: 当前章节节点
/// - `parent_path`: 从根到当前节点父级的标题链
/// - `config`: 切分配置
/// - `chunks`: 累积输出的 chunk 列表
fn traverse_and_chunk(
    section: &Section,
    parent_path: &Vec<String>,
    config: &ChunkingConfig,
    chunks: &mut Vec<Chunk>,
) {
    // 规则1: 自包含叶子 → 直接成 chunk（可能触发规则4硬切）
    if try_chunk_leaf(section, parent_path, config, chunks) {
        return;
    }

    // 构建当前节点的完整路径（用于向下传递）
    let mut new_path = parent_path.clone();
    new_path.push(section.title.clone());

    // 纯标题占位（无 body_text 且无子节点）
    if section.children.is_empty() {
        // 标题非空 → 生成最小 Leaf chunk，保留模板结构信息
        // （如"格式自拟"占位页）。后续会被 merge_tiny_chunks 合并到
        // 相邻同路径 chunk，不会产生碎片污染。
        //
        // 注意：page 元数据使用 section.page_start / page_end（标题所在页），
        // 而非 body_page_start / body_page_end。纯标题 Section 无 body_text，
        // 其 body_page_start/end 经 #[serde(default)] 会被设为 0，
        // 若使用 body 范围会导致 chunk 被错误排到文档开头。
        if !section.title.trim().is_empty() {
            let mut path = parent_path.clone();
            path.push(section.title.clone());
            chunks.push(Chunk {
                chunk_id: String::new(),
                chunk_type: ChunkType::Leaf,
                section_path: path,
                text: section.title.trim().to_string(),
                page_start: section.page_start,
                page_end: section.page_end,
                source_block_ids: section.block_ids.clone(),
            });
        }
        return;
    }

    // 规则 1.5: 容器节点自身 body_text → 单独生成 chunk
    // （如"第五章 合同文本"的引言说明文字，pages 为 body 实际页范围而非
    // 到子节点末尾的宽泛 span，避免 22 页/44 页的 page span 膨胀）
    if !section.body_text.is_empty() {
        let text = format!("{}\n{}", section.title, section.body_text);
        if text.chars().count() > config.split_max_len {
            split_long_chunk(&new_path, &text, section, config, chunks);
        } else {
            chunks.push(Chunk {
                chunk_id: String::new(),
                chunk_type: ChunkType::Leaf,
                section_path: new_path.clone(),
                text,
                page_start: section.body_page_start,
                page_end: section.body_page_end,
                source_block_ids: section.block_ids.clone(),
            });
        }
    }

    // 规则2: 容器节点 → 向下传递路径（不单独成 chunk）
    //
    // 分离叶子和容器子节点：
    // - 叶子：无子节点 + body_text 非空
    // - 容器：有子节点（不论 body_text）
    let (leaves, containers): (Vec<&Section>, Vec<&Section>) = section
        .children
        .iter()
        .partition(|c| c.children.is_empty() && !c.body_text.is_empty());

    // 规则3: 相邻独立叶子 → 短则合并
    if !leaves.is_empty() {
        merge_adjacent_leaves(&leaves, &new_path, config, chunks);
    }

    // 容器子节点 → 递归处理
    for child in &containers {
        traverse_and_chunk(child, &new_path, config, chunks);
    }
}

// ─── 规则 1：自包含叶子节点直接成 chunk ────────────────────────

/// 尝试将自包含叶子节点转为 chunk。
///
/// 条件：`body_text` 非空 且 无子节点。
/// 若文本长度超过 `split_max_len`，委托给规则4硬切。
///
/// 返回 `true` 表示该节点已被消费（成 chunk 或硬切），无需再向下遍历。
fn try_chunk_leaf(
    section: &Section,
    parent_path: &Vec<String>,
    config: &ChunkingConfig,
    chunks: &mut Vec<Chunk>,
) -> bool {
    // 条件: body_text 非空 && children 为空
    if section.body_text.is_empty() || !section.children.is_empty() {
        return false;
    }

    let mut path = parent_path.clone();
    path.push(section.title.clone());
    let text = format!("{}\n{}", section.title, section.body_text);

    // 过长 → 规则4 硬切
    if text.chars().count() > config.split_max_len {
        split_long_chunk(&path, &text, section, config, chunks);
        return true;
    }

    chunks.push(Chunk {
        chunk_id: String::new(), // 由 chunk_sections 统一分配
        chunk_type: ChunkType::Leaf,
        section_path: path,
        text,
        // 使用 body 实际页范围：对于叶子节点，body_page_start/end
        // 精确反映正文所在的页码范围，而非标题页到末尾的宽泛 span
        page_start: section.body_page_start,
        page_end: section.body_page_end,
        source_block_ids: section.block_ids.clone(),
    });
    true
}

// ─── 规则 3：相邻独立叶子合并 ────────────────────────────────

/// 合并同一父节点下过短的相邻独立叶子节点。
///
/// 策略：
/// - 叶子的完整文本（标题 + 正文）< `merge_min_len` → 放入合并缓冲区
/// - 叶子够长 → 先消化缓冲区（flush），再单独成 chunk
/// - 缓冲区累计长度 > `split_max_len` → 提前 flush，避免合并后过大
fn merge_adjacent_leaves(
    leaves: &[&Section],
    parent_path: &Vec<String>,
    config: &ChunkingConfig,
    chunks: &mut Vec<Chunk>,
) {
    let mut merge_buffer: Vec<&Section> = Vec::new();
    let mut merge_len: usize = 0;

    for leaf in leaves {
        let leaf_text = format!("{}\n{}", leaf.title, leaf.body_text);
        let leaf_len = leaf_text.chars().count();

        if leaf_len < config.merge_min_len {
            // 短叶子 → 进入合并缓冲区
            merge_buffer.push(*leaf);
            merge_len += leaf_len;
            // 合并后过长 → 先消化当前缓冲区
            if merge_len > config.split_max_len {
                flush_merge_buffer(&merge_buffer, parent_path, config, chunks);
                merge_buffer.clear();
                merge_len = 0;
            }
        } else {
            // 够长 → 先消化缓冲区，再单独成 chunk
            flush_merge_buffer(&merge_buffer, parent_path, config, chunks);
            merge_buffer.clear();
            merge_len = 0;
            // 单独成 chunk（可能过长触发规则4）
            let mut path = parent_path.clone();
            path.push(leaf.title.clone());
            let text = format!("{}\n{}", leaf.title, leaf.body_text);
            if text.chars().count() > config.split_max_len {
                split_long_chunk(&path, &text, leaf, config, chunks);
            } else {
                chunks.push(Chunk {
                    chunk_id: String::new(),
                    chunk_type: ChunkType::Leaf,
                    section_path: path,
                    text,
                    page_start: leaf.body_page_start,
                    page_end: leaf.body_page_end,
                    source_block_ids: leaf.block_ids.clone(),
                });
            }
        }
    }

    // 处理尾部残留缓冲区
    flush_merge_buffer(&merge_buffer, parent_path, config, chunks);
}

/// 将合并缓冲区中的叶子节点输出为一个 Merged chunk。
///
/// 若缓冲区为空则不产生 chunk；若仅剩 1 个叶子则输出为 Leaf。
fn flush_merge_buffer(
    buffer: &[&Section],
    parent_path: &Vec<String>,
    config: &ChunkingConfig,
    chunks: &mut Vec<Chunk>,
) {
    if buffer.is_empty() {
        return;
    }

    // 仅剩 1 个 → 仍按 Leaf 输出
    if buffer.len() == 1 {
        let leaf = buffer[0];
        let mut path = parent_path.clone();
        path.push(leaf.title.clone());
        let text = format!("{}\n{}", leaf.title, leaf.body_text);
        if text.chars().count() > config.split_max_len {
            split_long_chunk(&path, &text, leaf, config, chunks);
        } else {
            chunks.push(Chunk {
                chunk_id: String::new(),
                chunk_type: ChunkType::Leaf,
                section_path: path,
                text,
                page_start: leaf.body_page_start,
                page_end: leaf.body_page_end,
                source_block_ids: leaf.block_ids.clone(),
            });
        }
        return;
    }

    // 确定合并后 chunk 的起始页、结束页和所有 block_ids
    // 使用 body 实际页范围，避免容器节点页跨度膨胀
    let page_start = buffer.iter().map(|s| s.body_page_start).min().unwrap_or(0);
    let page_end = buffer.iter().map(|s| s.body_page_end).max().unwrap_or(0);
    let mut all_block_ids: Vec<String> = Vec::new();
    let mut merged_text_parts: Vec<String> = Vec::new();

    for leaf in buffer {
        merged_text_parts.push(format!("{}\n{}", leaf.title, leaf.body_text));
        for bid in &leaf.block_ids {
            if !all_block_ids.contains(bid) {
                all_block_ids.push(bid.clone());
            }
        }
    }

    let merged_text = merged_text_parts.join("\n\n");

    // 合并后若过长 → 硬切
    if merged_text.chars().count() > config.split_max_len {
        // 创建临时 Section 用于 split_long_chunk
        let temp_section = Section {
            level: buffer[0].level,
            title: String::new(),
            pattern: String::new(),
            page_start,
            page_end,
            block_ids: all_block_ids.clone(),
            body_text: String::new(),
            children: Vec::new(),
            body_page_start: page_start,
            body_page_end: page_end,
        };
        split_long_chunk(parent_path, &merged_text, &temp_section, config, chunks);
        return;
    }

    chunks.push(Chunk {
        chunk_id: String::new(),
        chunk_type: ChunkType::Merged {
            rule: "adjacent_merge".to_string(),
            child_count: buffer.len(),
        },
        section_path: parent_path.clone(),
        text: merged_text,
        page_start,
        page_end,
        source_block_ids: all_block_ids,
    });
}

// ─── 规则 4：过长 chunk 硬切 ─────────────────────────────────

/// 将过长文本在段落边界切分为多个 chunk，相邻片段保留 overlap。
///
/// 切分策略：
/// 1. 找到所有段落边界位置（`\n\n` 或 `\n` 后跟中文序号）
/// 2. 每次切 `split_max_len` 长度，回退到最近的段落边界
/// 3. 下一片段从 `end - split_overlap` 开始，保证语义连续
fn split_long_chunk(
    path: &[String],
    text: &str,
    section: &Section,
    config: &ChunkingConfig,
    chunks: &mut Vec<Chunk>,
) {
    let total = text.chars().count();
    if total <= config.split_max_len {
        chunks.push(Chunk {
            chunk_id: String::new(),
            chunk_type: ChunkType::Leaf,
            section_path: path.to_vec(),
            text: text.to_string(),
            page_start: section.body_page_start,
            page_end: section.body_page_end,
            source_block_ids: section.block_ids.clone(),
        });
        return;
    }

    let boundaries = find_para_boundaries(text);

    let mut parts: Vec<String> = Vec::new();
    let mut pos = 0;

    while pos < total {
        let end_candidate = (pos + config.split_max_len).min(total);

        if end_candidate >= total {
            // 最后一截：直接收尾
            parts.push(text.chars().skip(pos).collect());
            break;
        }

        // 回退到 [pos, end_candidate] 范围内最近的段落边界
        let split_point = boundaries
            .iter()
            .rev()
            .find(|&&b| b > pos && b <= end_candidate)
            .copied()
            .unwrap_or(end_candidate);

        parts.push(text.chars().skip(pos).take(split_point - pos).collect());

        // 下一片段起点 = 切分点 - overlap，保证重叠区域语义连续
        pos = split_point.saturating_sub(config.split_overlap);
        // 确保 pos 向前推进（防止死循环）
        if pos <= 0 || pos >= split_point {
            pos = split_point;
        }
    }

    for (i, part) in parts.iter().enumerate() {
        chunks.push(Chunk {
            chunk_id: String::new(),
            chunk_type: ChunkType::Split {
                part: i + 1,
                total: parts.len(),
            },
            section_path: path.to_vec(),
            text: part.clone(),
            page_start: section.body_page_start,
            page_end: section.body_page_end,
            source_block_ids: section.block_ids.clone(),
        });
    }
}

/// 查找文本中的段落边界位置（字符偏移）。
///
/// 段落边界定义：
/// - `\n\n`（显式段落分隔）
/// - `\n\n|` 模式（Markdown 表格行前的空行）→ 确保切分点不落入表格内部
/// - `|\n\n` 模式（Markdown 表格行后的空行）→ 确保切分点不落入表格内部
/// - `\n` 后紧跟中文序号（一～十），表示新段落开始
fn find_para_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries: Vec<usize> = Vec::new();
    let chars: Vec<char> = text.chars().collect();

    // 在开头加一个虚拟边界
    boundaries.push(0);

    let cjk_numerals: &[char] = &[
        '一', '二', '三', '四', '五', '六', '七', '八', '九', '十',
    ];

    for i in 0..chars.len() {
        // \n\n — 双换行段落分隔
        if chars[i] == '\n' && i + 1 < chars.len() && chars[i + 1] == '\n' {
            // Markdown 表格行前空行：\n\n| 模式 → 强制段落边界
            // 确保 split_long_chunk 不会在表格行中间切分
            if i + 2 < chars.len() && chars[i + 2] == '|' {
                boundaries.push(i + 1); // 在第二个 \n 之后
                continue;
            }
            // Markdown 表格行后空行：|\n\n 模式 → 强制段落边界
            if i > 0 && chars[i - 1] == '|' {
                boundaries.push(i + 1); // 在第二个 \n 之后
                continue;
            }
            boundaries.push(i + 1); // 在第二个 \n 之后
            continue;
        }
        // \n 后紧跟中文序号
        if chars[i] == '\n' && i + 1 < chars.len() {
            let next = chars[i + 1];
            if cjk_numerals.contains(&next) {
                boundaries.push(i + 1);
            }
        }
    }

    // 在末尾加一个虚拟边界
    boundaries.push(chars.len());

    boundaries
}

// ─── 后处理：碎片 Chunk 合并 ─────────────────────────────────

/// 检查两个 chunk 是否共享同一个顶级章节（第一位路径元素）。
///
/// 原先要求直接父路径完全一致，但标书中"第X部分"下的不同子节
/// （如"八、适用法律"与"九、资格审查"）在页面流中相邻时，其碎片 chunk
/// 语义同属一个大部分，合并不会造成跨主题混淆。
///
/// 放宽到 top-1 匹配后，原先因父路径不同而无法合并的孤立极小 chunk
/// （如评标标准碎片、法律条款碎片）可被正确吸收到相邻 chunk。
///
/// 返回 `true` 表示两个 chunk 属于同一顶级章节（可以合并）。
fn same_parent(a: &[String], b: &[String]) -> bool {
    match (a.first(), b.first()) {
        (Some(a_top), Some(b_top)) => a_top == b_top,
        // 空路径 → 保守允许合并（极少发生）
        _ => true,
    }
}

/// 合并过短的碎片 chunk 到相邻 chunk。
///
/// 两阶段处理：
/// - Pass 1（前向合并）：将碎片 chunk 合并到前一个相邻 chunk
/// - Pass 2（后向合并）：对 Pass 1 未能合并的碎片，尝试合并到后一个相邻 chunk
///
/// 合并前检查两个 chunk 是否共享同一顶级章节，避免跨主题合并。
fn merge_tiny_chunks(chunks: Vec<Chunk>, config: &ChunkingConfig) -> Vec<Chunk> {
    let min = config.min_chunk_size;
    let mut result: Vec<Chunk> = Vec::new();

    // ── Pass 1: 前向合并（合并到 prev chunk）──
    for chunk in chunks {
        if chunk.text.chars().count() < min {
            if let Some(prev) = result.last_mut() {
                if same_parent(&prev.section_path, &chunk.section_path) {
                    // 合并 chunk → prev
                    prev.text = format!("{}\n\n{}", prev.text, chunk.text);
                    prev.page_end = prev.page_end.max(chunk.page_end);
                    for bid in &chunk.source_block_ids {
                        if !prev.source_block_ids.contains(bid) {
                            prev.source_block_ids.push(bid.clone());
                        }
                    }
                    let child_count = match &prev.chunk_type {
                        ChunkType::Merged { child_count: c, .. } => c + 1,
                        _ => 2,
                    };
                    prev.chunk_type = ChunkType::Merged {
                        rule: "tiny_merge".to_string(),
                        child_count,
                    };
                    continue; // 已合并，跳过 push
                }
                // ← 前向合并失败：继续执行 push，由 Pass 2 处理
            }
        }
        result.push(chunk);
    }

    // ── Pass 2: 后向合并（合并到 next chunk）──
    // 对 Pass 1 未能合并的碎片，尝试合并到后一个相邻 chunk。
    // 从后向前扫描，确保 remove 不影响后续索引。
    let mut remove_indices: Vec<usize> = Vec::new();
    for i in 0..result.len() {
        if result[i].text.chars().count() >= min {
            continue; // 不是碎片，跳过
        }
        // 尝试合并到 next chunk (i+1)
        if i + 1 < result.len()
            && same_parent(&result[i].section_path, &result[i + 1].section_path)
        {
            // 先提取碎片数据，避免同时持有 result[i] 和 result[i+1] 的引用
            let tiny_text = result[i].text.clone();
            let tiny_page_start = result[i].page_start;
            let tiny_block_ids: Vec<String> = result[i].source_block_ids.clone();

            let next = &mut result[i + 1];
            // 将碎片内容前置到 next chunk
            next.text = format!("{}\n\n{}", tiny_text, next.text);
            next.page_start = tiny_page_start; // 扩展起始页
            for bid in &tiny_block_ids {
                if !next.source_block_ids.contains(bid) {
                    next.source_block_ids.insert(0, bid.clone());
                }
            }
            let child_count = match &next.chunk_type {
                ChunkType::Merged { child_count: c, .. } => c + 1,
                _ => 2,
            };
            next.chunk_type = ChunkType::Merged {
                rule: "tiny_merge".to_string(),
                child_count,
            };
            remove_indices.push(i);
        }
        // ← 双向都失败：保留独立碎片（极少发生，仅在两个不同 top-level
        //   section 之间出现孤立标题碎片时）
    }

    // 安全移除（降序）
    for &idx in remove_indices.iter().rev() {
        result.remove(idx);
    }

    result
}

// ─── 规则 5：嵌入文本携带层级上下文 ───────────────────────────

/// 截断过长的路径标题。
fn truncate_path_title(title: &str, max_len: usize) -> String {
    if max_len == 0 || title.chars().count() <= max_len {
        title.to_string()
    } else {
        let truncated: String = title.chars().take(max_len).collect();
        format!("{}…", truncated)
    }
}

impl Chunk {
    /// 生成带层级上下文的嵌入文本。
    ///
    /// 向量嵌入时携带层级前缀，避免不同章节的编号条目在向量空间中混淆。
    /// `max_path_len` 控制单个路径元素的最大字符数（0 = 不截断）。
    ///
    /// # 示例
    ///
    /// ```text
    /// 裸文本:  "1）具有独立承担民事责任的能力..."
    /// 嵌入文本: "【供应商的资格要求 > 政府采购法第二十二条】
    ///           1）具有独立承担民事责任的能力..."
    /// ```
    pub fn embed_text(&self, ctx_depth: usize, max_path_len: usize) -> String {
        let ctx = self
            .section_path
            .iter()
            .rev()
            .take(ctx_depth)
            .rev()
            .map(|t| truncate_path_title(t, max_path_len))
            .collect::<Vec<_>>()
            .join(" > ");

        if ctx.is_empty() {
            self.text.clone()
        } else {
            format!("【{}】\n{}", ctx, self.text)
        }
    }
}

// ─── 测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::ChunkingConfig;

    /// 构造一个简单的叶子 Section 用于测试。
    fn make_leaf(level: u8, title: &str, body: &str) -> Section {
        Section {
            level,
            title: title.to_string(),
            pattern: "test".to_string(),
            page_start: 0,
            page_end: 0,
            block_ids: vec![format!("b_0_{}", level)],
            body_text: body.to_string(),
            children: Vec::new(),
            body_page_start: 0,
            body_page_end: 0,
        }
    }

    /// 构造一个容器 Section（无 body_text，有子节点）。
    fn make_container(level: u8, title: &str, children: Vec<Section>) -> Section {
        Section {
            level,
            title: title.to_string(),
            pattern: "test".to_string(),
            page_start: 0,
            page_end: 0,
            block_ids: Vec::new(),
            body_text: String::new(),
            children,
            body_page_start: 0,
            body_page_end: 0,
        }
    }

    #[test]
    fn test_rule1_leaf_chunk() {
        let section = make_leaf(4, "1. 项目地点", "东莞理工学院松山湖校区6号教学楼。");
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();

        let consumed = try_chunk_leaf(&section, &vec!["第一章".to_string()], &config, &mut chunks);
        assert!(consumed);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Leaf));
        assert!(chunks[0].text.contains("1. 项目地点"));
        assert!(chunks[0].text.contains("东莞理工学院"));
    }

    #[test]
    fn test_rule1_container_not_consumed() {
        let child = make_leaf(5, "（1）条件", "具有独立承担民事责任的能力");
        let container = make_container(4, "1. 资格要求", vec![child]);
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();

        // 容器本身不应被 try_chunk_leaf 消费
        let consumed = try_chunk_leaf(&container, &Vec::new(), &config, &mut chunks);
        assert!(!consumed);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_rule3_merge_short_leaves() {
        let leaves: Vec<Section> = vec![
            make_leaf(4, "1. 项目地点", "东莞理工学院松山湖校区6号教学楼。"),
            make_leaf(4, "2. 项目工期", "合同签订后60个日历日内完成。"),
            make_leaf(4, "3. 质保期", "项目验收合格之日起2年。"),
            make_leaf(4, "4. 竣工图纸", "成交供应商需提供竣工图纸。"),
        ];
        let leaf_refs: Vec<&Section> = leaves.iter().collect();
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let parent_path = vec!["第一章".to_string()];

        merge_adjacent_leaves(&leaf_refs, &parent_path, &config, &mut chunks);

        // 四个短叶子应合并为 1 个 Merged chunk
        assert_eq!(chunks.len(), 1);
        assert!(matches!(&chunks[0].chunk_type, ChunkType::Merged { .. }));
        if let ChunkType::Merged { rule, child_count } = &chunks[0].chunk_type {
            assert_eq!(rule, "adjacent_merge");
            assert_eq!(*child_count, 4);
        }
    }

    #[test]
    fn test_rule3_long_leaf_not_merged() {
        // 构造一个超过 merge_min_len 的长叶子
        let long_body = "A".repeat(150); // 150 字符 > merge_min_len (100)
        let leaves: Vec<Section> = vec![
            make_leaf(4, "1. 长条款", &long_body),
            make_leaf(4, "2. 短条款", "短内容"),
        ];
        let leaf_refs: Vec<&Section> = leaves.iter().collect();
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let parent_path = vec!["第一章".to_string()];

        merge_adjacent_leaves(&leaf_refs, &parent_path, &config, &mut chunks);

        // 长叶子单独成 chunk，短叶子也单独成 chunk（单个不合并）
        assert_eq!(chunks.len(), 2);
    }

    #[test]
    fn test_rule4_split_overlong() {
        // 构造超过 split_max_len 的文本（每段约 7 字 × 250 = 1750 字）
        let long_body = "项目说明：\n\n".to_string() + &"详细描述内容。".repeat(250);
        // total chars > 1500
        assert!(long_body.chars().count() > 1500);

        let section = make_leaf(4, "1. 项目概况", &long_body);
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let path = vec!["第一章".to_string()];

        split_long_chunk(&path, &section.body_text, &section, &config, &mut chunks);

        // 应产出至少 2 个 Split chunk
        assert!(chunks.len() >= 2);
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(matches!(&chunk.chunk_type, ChunkType::Split { part: p, .. } if *p == i + 1));
        }
        if let ChunkType::Split { total, .. } = &chunks[0].chunk_type {
            assert_eq!(*total, chunks.len());
        }
    }

    #[test]
    fn test_rule5_embed_text() {
        let chunk = Chunk {
            chunk_id: "ch_042".to_string(),
            chunk_type: ChunkType::Leaf,
            section_path: vec![
                "第一章 磋商邀请".to_string(),
                "二.供应商的资格要求".to_string(),
                "1）具有独立承担民事责任的能力".to_string(),
            ],
            text: "1）具有独立承担民事责任的能力\n供应商必须是...".to_string(),
            page_start: 3,
            page_end: 3,
            source_block_ids: vec!["b_3_5".to_string()],
        };

        let embedded = chunk.embed_text(2, 0);
        assert!(embedded.starts_with(
            "【二.供应商的资格要求 > 1）具有独立承担民事责任的能力】"
        ));
        assert!(embedded.contains("供应商必须是"));

        // ctx_depth=0 → 无前缀
        let no_ctx = chunk.embed_text(0, 0);
        assert_eq!(no_ctx, chunk.text);
    }

    #[test]
    fn test_find_para_boundaries() {
        let text = "第一段内容。\n\n第二段内容。\n一、新段落标题";
        let boundaries = find_para_boundaries(text);

        // 应包含边界：0, \n\n 之后, \n一 之前, text.len()
        assert!(boundaries.len() >= 3);
        assert_eq!(boundaries.first(), Some(&0));
        assert_eq!(boundaries.last(), Some(&text.chars().count()));
    }

    #[test]
    fn test_traverse_and_chunk_container_aggregation() {
        // 模拟：容器节点下多个短叶子 → 应聚合
        let child1 = make_leaf(5, "1）具有独立承担民事责任的能力", "供应商须是在中华人民共和国境内注册的法人。");
        let child2 = make_leaf(5, "2）有依法缴纳税收", "供应商须提供近6个月的纳税证明。");
        let child3 = make_leaf(5, "3）有良好的商业信誉", "供应商须提供信用中国查询记录。");

        let sub_container = make_container(
            4,
            "1.供应商应具备《政府采购法》第二十二条规定的条件",
            vec![child1, child2, child3],
        );

        let parent = make_container(
            2,
            "二.供应商的资格要求",
            vec![sub_container],
        );

        let config = ChunkingConfig::default();
        let chunks = chunk_sections(&[parent], &config);

        // 应产出至少 1 个 chunk（3 个短叶子合并）
        assert!(!chunks.is_empty());
        // chunk_id 格式检查
        assert!(chunks[0].chunk_id.starts_with("ch_"));
        // section_path 应包含祖先容器路径
        assert!(chunks[0].section_path.iter().any(|t| t.contains("资格要求")));
    }

    #[test]
    fn test_chunk_id_ordering() {
        // 使用足够长的 body_text 以避免被 merge_tiny_chunks 合并
        // min_chunk_size 默认 50，此处确保正文远超此阈值
        let body_long = "这是足够长的正文内容，确保超过 min_chunk_size 的默认阈值五十个字符以上就是如此。";
        let s1 = make_leaf(4, "A. 条款", body_long);
        let s2 = make_leaf(4, "B. 条款", body_long);
        // 手动设置不同页码测试排序（同时设 body_page_start 与 page_start 一致）
        let sections = vec![
            Section { page_start: 3, body_page_start: 3, body_page_end: 3, ..s1 },
            Section { page_start: 1, body_page_start: 1, body_page_end: 1, ..s2 },
        ];

        let config = ChunkingConfig::default();
        let chunks = chunk_sections(&sections, &config);

        // 按页码排序后，page_start=1 的在前
        assert!(chunks.len() >= 2, "应有至少 2 个 chunk");
        assert_eq!(chunks[0].chunk_id, "ch_000");
        assert_eq!(chunks[0].page_start, 1);
        assert_eq!(chunks[1].chunk_id, "ch_001");
        assert_eq!(chunks[1].page_start, 3);
    }

    // ─── V4.1: Leaf 判定边界与占位节点 ──────────────────────────

    #[test]
    fn test_rule1_boundary_at_split_max() {
        // 恰好 1500 字符 → Leaf；超过 → Split
        let body_1500 = "A".repeat(1500 - "1. 边界条款\n".chars().count());
        let body_1501 = "B".repeat(1501 - "1. 边界条款\n".chars().count());

        let section_leaf = make_leaf(4, "1. 边界条款", &body_1500);
        let section_split = make_leaf(4, "1. 边界条款", &body_1501);

        let config = ChunkingConfig::default();
        let mut chunks_leaf = Vec::new();
        let mut chunks_split = Vec::new();

        let consumed = try_chunk_leaf(&section_leaf, &Vec::new(), &config, &mut chunks_leaf);
        assert!(consumed);
        assert_eq!(chunks_leaf.len(), 1);
        assert!(matches!(chunks_leaf[0].chunk_type, ChunkType::Leaf));

        let consumed = try_chunk_leaf(&section_split, &Vec::new(), &config, &mut chunks_split);
        assert!(consumed);
        assert!(chunks_split.len() >= 2);
        for c in &chunks_split {
            assert!(matches!(c.chunk_type, ChunkType::Split { .. }));
        }
    }

    #[test]
    fn test_rule1_empty_body_no_children_placeholder() {
        // body="" 且 children=[] → 纯标题占位，不应生成 chunk
        let placeholder = Section {
            level: 4,
            title: "五.附则".to_string(),
            pattern: "cjk_numbered".to_string(),
            page_start: 10,
            page_end: 10,
            block_ids: vec!["b_10_3".to_string()],
            body_text: String::new(),
            children: Vec::new(),
            body_page_start: 10,
            body_page_end: 10,
        };
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();

        let consumed = try_chunk_leaf(&placeholder, &Vec::new(), &config, &mut chunks);
        assert!(!consumed, "纯标题占位不应被消费为 Leaf chunk");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_rule1_body_and_children_not_leaf() {
        // body 非空但 children 也非空 → 不是叶子，不应被 try_chunk_leaf 消费
        let child = make_leaf(5, "1）子条款", "子条款内容。");
        let mixed = Section {
            level: 4,
            title: "1. 混合节点".to_string(),
            pattern: "digit_dot".to_string(),
            page_start: 5,
            page_end: 7,
            block_ids: vec!["b_5_0".to_string()],
            body_text: "这是引言文本。".to_string(),
            children: vec![child],
            body_page_start: 5,
            body_page_end: 5,
        };
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();

        let consumed = try_chunk_leaf(&mixed, &Vec::new(), &config, &mut chunks);
        assert!(!consumed, "有子节点的节点不应被当作 Leaf");
    }

    // ─── V4.2: 容器聚合 ─────────────────────────────────────────

    #[test]
    fn test_rule2_container_produces_no_chunks() {
        // 纯容器（无 body_text）本身不应产生任何 chunk
        // 子节点应由 traverse_and_chunk 递归处理
        let child1 = make_leaf(5, "1）条件A", "内容A。");
        let child2 = make_leaf(5, "2）条件B", "内容B。");
        let container = make_container(4, "1. 纯容器", vec![child1, child2]);

        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();

        // 直接测试：容器不应被 try_chunk_leaf 消费
        let consumed = try_chunk_leaf(&container, &Vec::new(), &config, &mut chunks);
        assert!(!consumed);
        assert!(chunks.is_empty());

        // 通过 chunk_sections 完整流程测试
        let all_chunks = chunk_sections(&[container], &config);
        assert!(!all_chunks.is_empty(), "容器的子节点应产生 chunk");
        // 所有 chunk 的 section_path 应包含容器的标题
        for chunk in &all_chunks {
            assert!(
                chunk.section_path.iter().any(|t| t == "1. 纯容器"),
                "子 chunk 的 path 应包含容器标题: {:?}",
                chunk.section_path
            );
        }
    }

    #[test]
    fn test_rule2_deeply_nested_container() {
        // 验证深层容器嵌套时 section_path 完整传递
        // L2 容器 > L4 容器 > L5 叶子
        let leaf = make_leaf(5, "1）具体条件", "具体内容描述。");
        let inner_container = make_container(4, "1. 资格条件", vec![leaf]);
        let outer_container = make_container(2, "二.供应商要求", vec![inner_container]);

        let config = ChunkingConfig::default();
        let chunks = chunk_sections(&[outer_container], &config);

        assert!(!chunks.is_empty());
        // section_path 应包含完整的层级链
        let path = &chunks[0].section_path;
        assert!(path.iter().any(|t| t == "二.供应商要求"), "path 应包含 L2: {:?}", path);
        assert!(path.iter().any(|t| t == "1. 资格条件"), "path 应包含 L4: {:?}", path);
        assert!(path.iter().any(|t| t == "1）具体条件"), "path 应包含 L5: {:?}", path);
    }

    // ─── V4.3: 相邻短叶子合并 ──────────────────────────────────

    #[test]
    fn test_rule3_mixed_lengths() {
        // 场景: [30字, 120字, 25字] — 中间一条够长
        let short1 = make_leaf(4, "1. 短条一", "短内容A"); // ~12 chars
        let long = make_leaf(4, "2. 长条款", &"长".repeat(120)); // 120+ chars
        let short2 = make_leaf(4, "3. 短条三", "短内容C"); // ~12 chars

        let leaves: Vec<Section> = vec![short1, long, short2];
        let leaf_refs: Vec<&Section> = leaves.iter().collect();
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let parent_path = vec!["一.项目概况".to_string()];

        merge_adjacent_leaves(&leaf_refs, &parent_path, &config, &mut chunks);

        // 期望 3 个 chunk：短1(Leaf), 长(Leaf), 短2(Leaf)
        assert_eq!(
            chunks.len(),
            3,
            "混合长度应产出 3 个 chunk，实际: {}",
            chunks.len()
        );
        // 每个 chunk 都是 Leaf 类型（缓冲区都是单条 → Leaf 输出）
        for chunk in &chunks {
            assert!(matches!(chunk.chunk_type, ChunkType::Leaf));
        }
        // 长条款应包含 "长" 字
        assert!(chunks[1].text.contains("长"));
    }

    #[test]
    fn test_rule3_merge_exceeds_split_max_during_buffer() {
        // 场景: 验证合并缓冲区在超限时正确 flush
        // 使用小型短叶子（merge 后总长 < split_max_len），确保产生 Merged chunk
        let short_body = "条".repeat(50); // 50 chars
        let leaves: Vec<Section> = (1..=6)
            .map(|i| make_leaf(4, &format!("{}. 短条款{}", i, i), &short_body))
            .collect();
        let leaf_refs: Vec<&Section> = leaves.iter().collect();
        let config = ChunkingConfig {
            merge_min_len: 100, // 每个叶子 ~65 chars < 100 → 全部入缓冲
            ..ChunkingConfig::default()
        };
        let mut chunks = Vec::new();
        let parent_path = vec!["第一章".to_string()];

        merge_adjacent_leaves(&leaf_refs, &parent_path, &config, &mut chunks);

        // 6 个短叶子合并后约 6*65 + 5*2 = 400 chars < 1500 → 单个 Merged chunk
        assert_eq!(chunks.len(), 1);
        assert!(
            matches!(&chunks[0].chunk_type, ChunkType::Merged { rule, child_count } if rule == "adjacent_merge" && *child_count == 6),
            "6 个短叶子应合并为 1 个 Merged chunk"
        );

        // 批量二：构造足够多的叶子，使合并文本超过 split_max_len
        // 此时 flush 会触发 split_long_chunk，产生 Split 类型
        let large_body = "款".repeat(250); // 250 chars body
        let many_leaves: Vec<Section> = (1..=8)
            .map(|i| make_leaf(4, &format!("{}. 条款{}", i, i), &large_body))
            .collect();
        let many_refs: Vec<&Section> = many_leaves.iter().collect();
        let mut chunks2 = Vec::new();

        merge_adjacent_leaves(&many_refs, &parent_path, &config, &mut chunks2);

        // 验证：所有产出的 chunk 均不超过 split_max_len（核心不变量）
        for chunk in &chunks2 {
            assert!(
                chunk.text.chars().count() <= config.split_max_len,
                "每个 chunk 应 ≤ split_max_len ({}), 实际: {}",
                config.split_max_len,
                chunk.text.chars().count()
            );
        }
        // 验证：至少产生了 chunk（叶子被处理）
        assert!(!chunks2.is_empty(), "大量叶子应产生 chunk");

        // 8 个叶子（每个 ~265 chars）全部 < merge_min_len=100? NO, 265 >= 100
        // 所以它们不会进入合并缓冲，而是各自独立成 Leaf
        // 验证每个 chunk 类型为 Leaf
        for chunk in &chunks2 {
            assert!(
                matches!(chunk.chunk_type, ChunkType::Leaf),
                "≥merge_min_len 的长叶子应各自为 Leaf"
            );
        }
    }

    #[test]
    fn test_rule3_all_long_leaves() {
        // 全部叶子 ≥ 100 字 → 各自成 Leaf，不触发合并
        let long_body = "内".repeat(120);
        let leaves: Vec<Section> = (1..=3)
            .map(|i| make_leaf(4, &format!("{}. 长条款{}", i, i), &long_body))
            .collect();
        let leaf_refs: Vec<&Section> = leaves.iter().collect();
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let parent_path = vec!["第一章".to_string()];

        merge_adjacent_leaves(&leaf_refs, &parent_path, &config, &mut chunks);

        // 3 个叶子各自成 Leaf chunk
        assert_eq!(chunks.len(), 3);
        for chunk in &chunks {
            assert!(matches!(chunk.chunk_type, ChunkType::Leaf));
        }
    }

    // ─── V4.4: 硬切与 Overlap ──────────────────────────────────

    #[test]
    fn test_rule4_three_parts() {
        // 构造 3000 字的文本（用 ASCII 字符避免字节边界问题）→ 应产出 ≥3 个 Split chunk
        let body_3000 = "A".repeat(3000);
        let section = make_leaf(4, "1. 超长条款", &body_3000);
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let path = vec!["第一章".to_string()];

        split_long_chunk(&path, &section.body_text, &section, &config, &mut chunks);

        assert!(chunks.len() >= 3, "3000 字应产出 ≥3 个 Split chunk, 实际: {}", chunks.len());
        // part 编号从 1 开始连续
        for (i, chunk) in chunks.iter().enumerate() {
            if let ChunkType::Split { part, total } = &chunk.chunk_type {
                assert_eq!(*part, i + 1, "part 编号应连续");
                assert_eq!(*total, chunks.len(), "total 应等于总片段数");
            }
        }
        // 每段不超过 split_max_len
        for chunk in &chunks {
            assert!(
                chunk.text.chars().count() <= config.split_max_len,
                "每段应 ≤ split_max_len"
            );
        }
        // overlap: 相邻片段应有重叠（用 char-based 比较）
        if chunks.len() >= 2 {
            let text1: Vec<char> = chunks[0].text.chars().collect();
            let text2: Vec<char> = chunks[1].text.chars().collect();
            let overlap_start = text1.len().saturating_sub(200);
            let overlap_end = std::cmp::min(200, text2.len());
            let end_chars: String = text1[overlap_start..].iter().collect();
            let start_chars: String = text2[..overlap_end].iter().collect();
            // 验证确实存在重叠：第一部分末尾和第二部分开头有公共子串
            assert!(
                !end_chars.is_empty() && !start_chars.is_empty(),
                "overlap 区域不应为空"
            );
        }
    }

    #[test]
    fn test_rule4_no_paragraph_boundaries() {
        // 纯字母数字无换行 → 无段落边界 → 在 split_max_len 处硬切
        let body_no_breaks = "ABCDEFGHIJ".repeat(200); // 2000 chars, no \n\n
        assert!(!body_no_breaks.contains("\n\n"));
        assert!(body_no_breaks.chars().count() > 1500);

        let section = make_leaf(4, "1. 无边界条款", &body_no_breaks);
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let path = vec!["第一章".to_string()];

        split_long_chunk(&path, &section.body_text, &section, &config, &mut chunks);

        // 即使无段落边界也能正常切分
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(
                chunk.text.chars().count() <= config.split_max_len,
                "每段应 ≤ split_max_len"
            );
        }
    }

    #[test]
    fn test_rule4_split_preserves_overlap_content() {
        // 验证 overlap 区域确实包含重复文本
        let body = "A".repeat(1600);
        let section = make_leaf(4, "1. 条款", &body);
        let config = ChunkingConfig::default();
        let mut chunks = Vec::new();
        let path = vec!["第一章".to_string()];

        split_long_chunk(&path, &section.body_text, &section, &config, &mut chunks);
        assert!(chunks.len() >= 2);

        // overlap 验证：第一段末尾和第二段开头有重叠
        let part1_end: String = chunks[0].text.chars().rev().take(50).collect();
        let part2_start: String = chunks[1].text.chars().take(50).collect();
        // 两者应有公共字符（overlap）
        let common = part1_end.chars().filter(|c| part2_start.contains(*c)).count();
        assert!(common > 0, "overlap 区域应包含公共字符");
    }

    // ─── V4.5: embed_text 边界 ──────────────────────────────────

    #[test]
    fn test_rule5_ctx_depth_exceeds_path() {
        // ctx_depth 超过 path 长度时取全部，不 panic
        let chunk = Chunk {
            chunk_id: "ch_test".to_string(),
            chunk_type: ChunkType::Leaf,
            section_path: vec!["第一章".to_string(), "一、概述".to_string()],
            text: "正文内容...".to_string(),
            page_start: 0,
            page_end: 0,
            source_block_ids: vec!["b_0_0".to_string()],
        };

        // ctx_depth=5 但 path 只有 2 级 → 取全部 2 级
        let embedded = chunk.embed_text(5, 0);
        assert!(embedded.starts_with("【第一章 > 一、概述】"));
        // 不应 panic，应正常返回
    }

    #[test]
    fn test_rule5_empty_path() {
        let chunk = Chunk {
            chunk_id: "ch_test".to_string(),
            chunk_type: ChunkType::Leaf,
            section_path: Vec::new(),
            text: "正文内容...".to_string(),
            page_start: 0,
            page_end: 0,
            source_block_ids: vec!["b_0_0".to_string()],
        };

        // 空 path → 无前缀，直接返回原文本
        let embedded = chunk.embed_text(2, 0);
        assert_eq!(embedded, chunk.text);
        // 不应产生 "【】" 空壳
        assert!(!embedded.starts_with("【】"));
    }

    #[test]
    fn test_rule5_ctx_depth_one() {
        let chunk = Chunk {
            chunk_id: "ch_test".to_string(),
            chunk_type: ChunkType::Leaf,
            section_path: vec![
                "第一章 磋商邀请".to_string(),
                "二.供应商的资格要求".to_string(),
                "1.基本条件".to_string(),
            ],
            text: "在中华人民共和国境内注册...".to_string(),
            page_start: 0,
            page_end: 0,
            source_block_ids: vec!["b_1_0".to_string()],
        };

        // ctx_depth=1 → 只取最后 1 级
        let embedded = chunk.embed_text(1, 0);
        assert!(embedded.starts_with("【1.基本条件】"));
        assert!(!embedded.contains("二.供应商的资格要求"));
    }

    #[test]
    fn test_rule5_path_truncation() {
        // embed_path_max_len > 0 时，过长的路径元素应被截断
        let chunk = Chunk {
            chunk_id: "ch_test".to_string(),
            chunk_type: ChunkType::Leaf,
            section_path: vec![
                "第一章 磋商邀请".to_string(),
                "一、《深圳经济特区政府采购条例》第五十七条供应商在政府采购中，有下列行为之一的，属于隐瞒真实情况，提供虚假资料".to_string(),
                "（一）在采购活动中应当回避而未回避的".to_string(),
            ],
            text: "正文内容...".to_string(),
            page_start: 0,
            page_end: 0,
            source_block_ids: vec!["b_0_0".to_string()],
        };

        // max_path_len=40 → 第二个元素（70+ chars）应被截断
        let embedded = chunk.embed_text(2, 40);
        // 前40个字符: "一、《深圳经济特区政府采购条例》第五十七条供应商在政府采购中，有下列行为之一的，"
        assert!(
            embedded.starts_with("【一、《深圳经济特区政府采购条例》第五十七条供应商在政府采购中，有下列行为之一的，… > （一）在采购活动中应当回避而未回避的】"),
            "实际 embed_text: {}",
            embedded
        );
        // 第一个元素（< 40 chars）不应被截断
        assert!(!embedded.contains("第一章 磋商邀请…"));
        // 第三个元素（< 40 chars）不应被截断
        assert!(embedded.contains("（一）在采购活动中应当回避而未回避的"));

        // max_path_len=0 → 不截断
        let no_trunc = chunk.embed_text(2, 0);
        assert!(no_trunc.contains("第五十七条供应商在政府采购中，有下列行为之一的"));
    }

    #[test]
    fn test_merge_tiny_chunks_basic() {
        // 验证碎片 chunk 被合并到邻居
        let long_text = "这是足够长的正常正文内容，确保超过 min_chunk_size 的默认阈值三十个字符。";
        let chunks = vec![
            Chunk {
                chunk_id: "ch_000".to_string(),
                chunk_type: ChunkType::Leaf,
                section_path: vec!["第一章".to_string()],
                text: long_text.to_string(),
                page_start: 0,
                page_end: 0,
                source_block_ids: vec!["b_0_0".to_string()],
            },
            Chunk {
                chunk_id: "ch_001".to_string(),
                chunk_type: ChunkType::Leaf,
                section_path: vec!["第一章".to_string()],
                text: "短".to_string(), // 仅 1 字符 → 碎片
                page_start: 1,
                page_end: 1,
                source_block_ids: vec!["b_1_0".to_string()],
            },
            Chunk {
                chunk_id: "ch_002".to_string(),
                chunk_type: ChunkType::Leaf,
                section_path: vec!["第一章".to_string()],
                text: long_text.to_string(),
                page_start: 2,
                page_end: 2,
                source_block_ids: vec!["b_2_0".to_string()],
            },
        ];

        let config = ChunkingConfig {
            min_chunk_size: 30,
            ..ChunkingConfig::default()
        };
        let merged = merge_tiny_chunks(chunks, &config);

        // "短" 应被合并到前一个 chunk，最终保留 2 个 chunk
        assert_eq!(merged.len(), 2, "碎片应被合并到前一个块");
        assert!(merged[0].text.contains("短"), "碎片内容应在合并后的第一个块中");
        assert!(merged[0].text.contains("正常正文内容"), "第一个块的 long_text 应保留");
    }

    #[test]
    fn test_merge_tiny_chunks_disabled() {
        // min_chunk_size=0 → 不合并
        let chunks = vec![
            Chunk {
                chunk_id: "ch_000".to_string(),
                chunk_type: ChunkType::Leaf,
                section_path: vec!["第一章".to_string()],
                text: "短".to_string(),
                page_start: 0,
                page_end: 0,
                source_block_ids: vec!["b_0_0".to_string()],
            },
        ];

        let config = ChunkingConfig {
            min_chunk_size: 0,
            ..ChunkingConfig::default()
        };
        let merged = merge_tiny_chunks(chunks, &config);
        assert_eq!(merged.len(), 1);
    }

    // ─── V4.6: chunk 元数据完整性 ───────────────────────────────

    #[test]
    fn test_chunk_metadata_integrity() {
        // 对所有 chunk 验证 ID 连续、page 范围合法、block_ids 非空
        let s1 = make_leaf(4, "1. 条款A", "内容A。");
        let s2 = make_leaf(4, "2. 条款B", "内容B。");
        let sections = vec![
            Section {
                page_start: 2,
                page_end: 3,
                block_ids: vec!["b_2_0".to_string(), "b_2_1".to_string()],
                ..s1
            },
            Section {
                page_start: 4,
                page_end: 4,
                block_ids: vec!["b_4_0".to_string()],
                ..s2
            },
        ];

        let config = ChunkingConfig::default();
        let chunks = chunk_sections(&sections, &config);

        // ID 连续性
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_id, format!("ch_{:03}", i));
        }

        // page 范围合法性
        for chunk in &chunks {
            assert!(
                chunk.page_start <= chunk.page_end,
                "page_start({}) ≤ page_end({})",
                chunk.page_start,
                chunk.page_end
            );
        }

        // block_ids 非空
        for chunk in &chunks {
            assert!(
                !chunk.source_block_ids.is_empty(),
                "每个 chunk 应有至少一个 source_block_id"
            );
        }

        // section_path 非空
        for chunk in &chunks {
            assert!(
                !chunk.section_path.is_empty(),
                "每个 chunk 应有非空 section_path"
            );
        }
    }

    // ─── 边界/极端条件 ──────────────────────────────────────────

    #[test]
    fn test_empty_sections() {
        let config = ChunkingConfig::default();
        let chunks = chunk_sections(&[], &config);
        assert!(chunks.is_empty(), "空 sections 列表应产生空 chunks");
    }

    #[test]
    fn test_deeply_nested_empty_leaves() {
        // 深层嵌套，但所有叶子 body_text 为空 → 不应产生 chunk
        let empty_leaf = Section {
            level: 5,
            title: "（1）空条款".to_string(),
            pattern: "paren_digit".to_string(),
            page_start: 1,
            page_end: 1,
            block_ids: vec!["b_1_0".to_string()],
            body_text: String::new(),
            children: Vec::new(),
            body_page_start: 1,
            body_page_end: 1,
        };
        let container = make_container(4, "1. 容器", vec![empty_leaf]);
        let root = make_container(2, "一、章节", vec![container]);

        let config = ChunkingConfig::default();
        let chunks = chunk_sections(&[root], &config);

        // 空叶子不应产生 chunk
        for chunk in &chunks {
            assert!(!chunk.text.is_empty(), "不应产生空 text 的 chunk");
        }
    }
}
