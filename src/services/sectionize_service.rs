//! 文档章节结构识别服务
//!
//! 本模块负责从 [`RawDocument`] 中识别章节层级结构。
//! 采用规则引擎方案：用正则匹配中文标书特有的标题编号模式，
//! 按模式类型确定层级，最终构建章节树。
//!
//! ## 标题模式 → 层级映射
//!
//! | 模式                          | 层级 | 示例                   |
//! |-------------------------------|------|------------------------|
//! | `第X章`                       | 1    | 第一章 磋商邀请        |
//! | `第X节`                       | 2    | 第一节 项目概况        |
//! | `一、二、三、`                 | 2    | 一、项目概述           |
//! | `（一）（二）`                 | 3    | （一）资格要求         |
//! | `1. 2. 3.` (短标题)           | 4    | 1. 供应商资格          |
//! | `(1) (2)` / `（1）（2）`       | 5    | （1）营业执照          |
//! | `第X条`                       | 4    | 第九条 工程支付        |
//!
//! ## 过滤规则
//!
//! - 纯数字行（页码）→ 剔除
//! - 匹配行过长（> 80 字）→ 降级（可能是条款正文而非标题）
//! - 同一行匹配多个模式 → 取层级最高的

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::domain::raw_document::RawDocument;

// ─── 标题模式定义 ─────────────────────────────────────────────

/// 标题匹配模式。patterns 按优先级排列，靠前的优先匹配。
static HEADING_PATTERNS: LazyLock<Vec<HeadingPattern>> = LazyLock::new(|| {
    vec![
        // Level 1: 第X部分（标书顶层结构）
        HeadingPattern {
            pattern_type: "part",
            level: 1,
            regex: Regex::new(r"^第[一二三四五六七八九十百千]+部分").expect("part regex"),
        },
        // Level 1: 第X章
        HeadingPattern {
            pattern_type: "chapter",
            level: 1,
            regex: Regex::new(r"^第[一二三四五六七八九十百千]+章").expect("chapter regex"),
        },
        // Level 2: 第X节
        HeadingPattern {
            pattern_type: "section",
            level: 2,
            regex: Regex::new(r"^第[一二三四五六七八九十百千]+节").expect("section regex"),
        },
        // Level 2: 中文序号标题 (一、二、三、...)
        HeadingPattern {
            pattern_type: "cjk_numbered",
            level: 2,
            regex: Regex::new(r"^[一二三四五六七八九十百千]+[、.．]\s*\S").expect("cjk_numbered regex"),
        },
        // Level 3: 括号中文序号 （一）（二）...
        HeadingPattern {
            pattern_type: "paren_cjk",
            level: 3,
            regex: Regex::new(r"^[（(][一二三四五六七八九十百千]+[）)]\s*\S").expect("paren_cjk regex"),
        },
        // Level 4: 数字序号 (1. 2、3) ...) — 要求后跟非空且标题短
        HeadingPattern {
            pattern_type: "digit_dot",
            level: 4,
            regex: Regex::new(r"^\d+[.、)）]\s*\S").expect("digit_dot regex"),
        },
        // Level 5: 括号数字 （1）（2）(1) (2) ...
        HeadingPattern {
            pattern_type: "paren_digit",
            level: 5,
            regex: Regex::new(r"^[（(]\d+[）)]\s*\S").expect("paren_digit regex"),
        },
        // Level 4: 第X条 (合同条款)
        HeadingPattern {
            pattern_type: "article",
            level: 4,
            regex: Regex::new(r"^第[一二三四五六七八九十百千]+条").expect("article regex"),
        },
    ]
});

struct HeadingPattern {
    pattern_type: &'static str,
    level: u8,
    regex: Regex,
}

// ─── 输出数据结构 ─────────────────────────────────────────────

/// 一个章节节点，可嵌套子节点形成树。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// 层级深度 (1=章, 2=节, 3=小节, ...)
    pub level: u8,
    /// 标题文本（已清洗）
    pub title: String,
    /// 匹配的标题模式类型
    pub pattern: String,
    /// 起始页码 (0-based)
    pub page_start: usize,
    /// 结束页码 (0-based，包含)
    pub page_end: usize,
    /// 本节包含的所有 block ID（用于回溯高亮）
    pub block_ids: Vec<String>,
    /// 本节的主体文本内容（不含标题行本身），从关联 blocks 中提取。
    /// 包含子章节标题行，但不包含子章节标题之下的正文。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body_text: String,
    /// 子章节
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Section>,
}

/// sectionize 的完整输出。
#[derive(Debug, Serialize, Deserialize)]
pub struct SectionizeOutput {
    pub document_id: String,
    pub source_path: String,
    /// 顶层章节列表
    pub sections: Vec<Section>,
    /// 统计信息
    pub stats: SectionizeStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SectionizeStats {
    /// 总章节数（含嵌套）
    pub total_sections: usize,
    /// 各级别数量
    pub level_counts: std::collections::HashMap<u8, usize>,
    /// 未归属到任何 section 的 block 数量
    pub orphan_blocks: usize,
}

// ─── 内部候选结构 ─────────────────────────────────────────────

/// 扫描到的标题候选（中间数据）。
#[derive(Debug, Clone)]
struct HeadingCandidate {
    /// 层级
    level: u8,
    /// 匹配的标题文本行
    title: String,
    /// 标题模式类型
    pattern: &'static str,
    /// 所在页码 (0-based)
    page: usize,
    /// 所在 block 的 ID
    block_id: String,
}

// ─── 主入口 ──────────────────────────────────────────────────

/// 从 RawDocument 中提取章节树。
pub fn sectionize(raw: &RawDocument) -> SectionizeOutput {
    // 1. 收集所有 block 及其所属页面
    let all_blocks: Vec<(&crate::domain::raw_document::RawBlock, usize)> = raw
        .pages
        .iter()
        .flat_map(|page| page.blocks.iter().map(move |b| (b, page.page_index)))
        .collect();

    if all_blocks.is_empty() {
        return SectionizeOutput {
            document_id: raw.document_id.clone(),
            source_path: raw.source_path.clone(),
            sections: Vec::new(),
            stats: SectionizeStats {
                total_sections: 0,
                level_counts: std::collections::HashMap::new(),
                orphan_blocks: 0,
            },
        };
    }

    // 2. 扫描所有 block 的文本行，提取标题候选
    let mut candidates: Vec<HeadingCandidate> = Vec::new();

    for (block, page_idx) in &all_blocks {
        // 取 block 文本的第一行进行匹配
        // 同时检查每一行（某些 block 可能包含多个内嵌标题）
        for (_line_idx, line) in block.text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // 过滤 PDF 噪声行（页码、"第X页共Y页"、控制字符）
            if is_page_noise(line) {
                continue;
            }

            // 尝试匹配所有模式，取第一个命中的。
            // 若原始行无法匹配，尝试去除强调符号（★▲● 等）后再次匹配，
            // 此时标题仍取原始行，以保留重要性标记。
            let stripped = strip_emphasis_prefix(line);
            let try_lines: [(&str, bool); 2] = [
                (line, false),                          // 原始行，匹配位置即标题起始
                (stripped, stripped.len() < line.len()), // 去符号版，命中则用原始行整体作标题
            ];

            let mut found = false;
            for (test_line, use_original_as_title) in &try_lines {
                if found {
                    break;
                }
                for pattern in HEADING_PATTERNS.iter() {
                    if let Some(mat) = pattern.regex.find(test_line) {
                        let title = if *use_original_as_title {
                            // 去符号版本匹配成功 → 取原始行整体（保留 ★ 等标记）
                            line.to_string()
                        } else {
                            test_line[mat.start()..].to_string()
                        };

                        // 标题长度上限过滤：过长的"标题"大概率是正文误匹配
                        // 层级越高（数字越小）标题应越短
                        let max_title_len = match pattern.level {
                            1 => 40,  // 章/部分标题 ≤ 40 字符
                            2 => {
                                // cjk_numbered 易将法律条款长句误匹配为标题
                                // （如 "一、《深圳经济特区政府采购条例》第五十七条..."）
                                // 真实的中文序号标题（"一、技术要求"）均 ≤ 25 字
                                if pattern.pattern_type == "cjk_numbered" { 25 } else { 40 }
                            }
                            3 => 60,  // 括号中文序号 ≤ 60 字符
                            _ => 40,  // Level 4+ 数字/条款序号 ≤ 40 字符
                        };
                        if title.chars().count() > max_title_len {
                            continue;
                        }

                        // 规则 A：句末标点排除 — Level 4 digit_dot 标题含 。！？ → 跳过
                        // 中文完整句子必然以句号结尾，而真实标题不会。
                        // 精确打击被误匹配的完整句子（如 "1.1本招标文件适用于..."）
                        if pattern.pattern_type == "digit_dot" && title.contains(['。', '！', '？']) {
                            continue;
                        }

                        candidates.push(HeadingCandidate {
                            level: pattern.level,
                            title,
                            pattern: pattern.pattern_type,
                            page: *page_idx,
                            block_id: block.id.clone(),
                        });
                        found = true;
                        break; // 一行只匹配一个模式
                    }
                }
            }
        }
    }

    // 3. 构建章节树
    let (sections, orphan_blocks) = build_section_tree(&candidates, &all_blocks);

    // 5. 统计
    let total_sections = count_sections(&sections);
    let mut level_counts: std::collections::HashMap<u8, usize> = std::collections::HashMap::new();
    count_levels(&sections, &mut level_counts);

    SectionizeOutput {
        document_id: raw.document_id.clone(),
        source_path: raw.source_path.clone(),
        sections,
        stats: SectionizeStats {
            total_sections,
            level_counts,
            orphan_blocks,
        },
    }
}

// ─── 辅助函数 ────────────────────────────────────────────────

/// 判断一行是否为 PDF 噪声（页码行、私有区控制字符等）。
///
/// 过滤三类噪声：
/// 1. 纯数字短行（原有逻辑，如 "1"、"92"）
/// 2. "第X页共Y页" 格式的页码行（含残缺变体如 "78第72页共页"）
/// 3. 含 Unicode 私有区字符（U+E000–U+F8FF）的行，这些是 PDF 渲染产生的控制字符（如 ）
fn is_page_noise(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }

    // 1. 纯数字短行（页码），长度 ≤ 3 且全为 ASCII 数字/空格
    if trimmed.len() <= 3 && trimmed.chars().all(|c| c.is_ascii_digit() || c == ' ') {
        return true;
    }

    // 2. "第X页共Y页" 格式页码行
    //    匹配模式: [可选前缀数字] "第" 数字 "页共" [可选数字] "页" [可选后缀]
    //    使用简单的子串匹配：含 "第" + "页" + "共" + "页" 的结构
    if trimmed.contains('第') && trimmed.contains('页') && trimmed.contains("共") {
        // 进一步确认非正文：行长度 ≤ 20 字符（正常页码行 < 15 字符）
        if trimmed.chars().count() <= 20 {
            return true;
        }
    }

    // 3. 含 Unicode 私有区字符（U+E000–U+F8FF），如 PDF bullet 符号 
    if trimmed.contains(|c: char| ('\u{E000}'..='\u{F8FF}').contains(&c)) {
        return true;
    }

    false
}

/// 去除行首的强调符号（★▲●■ 等），用于标题模式匹配前的归一化。
///
/// 中文标书中常以这些符号标记重点条目，它们会干扰 `digit_dot` 等正则。
/// 去除后返回剩余文本；若无前缀符号则原样返回。
fn strip_emphasis_prefix(s: &str) -> &str {
    // 标书常见重点标注符号
    const EMPHASIS: &[char] = &[
        '★', '▲', '●', '■', '◆', '◎', '☆',
        '△', '▽', '◁', '▷', '◇', '□', '○',
        '✔', '☑', '❗', '✓',
    ];
    let trimmed = s.trim();
    match trimmed.chars().next() {
        Some(c) if EMPHASIS.contains(&c) => trimmed[c.len_utf8()..].trim_start(),
        _ => trimmed,
    }
}

/// 检查一行文本是否匹配任何标题模式（用于在正文提取中识别子标题边界）。
/// 使用与主扫描一致的过滤规则：标题过长视为正文误匹配。
fn matches_heading_pattern(line: &str) -> bool {
    for pattern in HEADING_PATTERNS.iter() {
        if let Some(mat) = pattern.regex.find(line) {
            let title = &line[mat.start()..];
            let max_title_len = match pattern.level {
                1 => 40,
                2 => {
                    if pattern.pattern_type == "cjk_numbered" { 25 } else { 40 }
                }
                3 => 60,
                _ => 40,
            };
            if title.chars().count() > max_title_len {
                continue;
            }
            // 规则 A：句末标点排除 — 完整句子误匹配的精确打击
            if pattern.pattern_type == "digit_dot" && title.contains(['。', '！', '？']) {
                continue;
            }
            return true;
        }
    }
    false
}

/// 从候选列表构建章节树。
///
/// 两阶段：先建扁平列表 + 父子索引关系，再递归组装树。
/// 返回 (root_sections, orphan_block_count)。
fn build_section_tree(
    candidates: &[HeadingCandidate],
    all_blocks: &[(&crate::domain::raw_document::RawBlock, usize)],
) -> (Vec<Section>, usize) {
    if candidates.is_empty() {
        return (Vec::new(), all_blocks.len());
    }

    // Phase 1: 创建所有 section 并记录父子关系
    let mut sections: Vec<Section> = Vec::new();
    let mut parent_of: Vec<Option<usize>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // 祖先链索引
    let mut assigned_blocks: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (i, candidate) in candidates.iter().enumerate() {
        let next_boundary = find_next_boundary(candidates, i);
        let block_ids = collect_blocks_between(
            all_blocks,
            candidate,
            next_boundary,
            &mut assigned_blocks,
        );
        let page_end = block_ids
            .iter()
            .filter_map(|bid| find_block_page(all_blocks, bid))
            .max()
            .unwrap_or(candidate.page);

        let body_text = extract_section_body(candidate, next_boundary, all_blocks, &block_ids);
        // Level 1-2（章/节标题）本身即为完整标题，不检测截断
        let title_truncated = candidate.level >= 3 && is_title_truncated(&candidate.title, &body_text);

        // 如果标题被 PDF 折行截断，将续接正文合并回标题，
        // 避免"标题 + 正文"的人为割裂。
        let (final_title, final_body_text) = if title_truncated {
            merge_truncated_title(&candidate.title, &body_text)
        } else {
            (candidate.title.clone(), body_text)
        };

        let section = Section {
            level: candidate.level,
            title: final_title,
            pattern: candidate.pattern.to_string(),
            page_start: candidate.page,
            page_end,
            block_ids,
            body_text: final_body_text,
            children: Vec::new(),
        };

        // 弹出栈中所有 level >= 当前 level 的祖先
        while let Some(&top_idx) = stack.last() {
            if sections[top_idx].level >= candidate.level {
                stack.pop();
            } else {
                break;
            }
        }

        let my_idx = sections.len();
        let my_parent = stack.last().copied();
        sections.push(section);
        parent_of.push(my_parent);
        stack.push(my_idx);
    }

    // Phase 2: 收集每个节点的子节点索引
    let n = sections.len();
    let mut children_of: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut root_indices: Vec<usize> = Vec::new();
    for idx in 0..n {
        match parent_of[idx] {
            Some(p) => children_of[p].push(idx),
            None => root_indices.push(idx),
        }
    }

    // Phase 3: 递归构建树（从 Option<Section> 中 take）
    let mut opt_sections: Vec<Option<Section>> = sections.into_iter().map(Some).collect();
    let orphan_blocks = all_blocks.len() - assigned_blocks.len();

    let root_sections: Vec<Section> = root_indices
        .into_iter()
        .map(|root_idx| take_section_from_flat(&mut opt_sections, &children_of, root_idx))
        .collect();

    (root_sections, orphan_blocks)
}

/// 递归从扁平 Option 数组中取出 section 及其所有子孙。
fn take_section_from_flat(
    flat: &mut [Option<Section>],
    children_of: &[Vec<usize>],
    idx: usize,
) -> Section {
    let mut section = flat[idx].take().expect("section already taken");
    for &child_idx in &children_of[idx] {
        section
            .children
            .push(take_section_from_flat(flat, children_of, child_idx));
    }
    section
}

/// 找到下一个层级 ≤ 当前候选层级的标题候选（同级或更高级）。
/// 这标记了当前 section 的结束位置。
fn find_next_boundary(candidates: &[HeadingCandidate], current_idx: usize) -> Option<&HeadingCandidate> {
    let current = &candidates[current_idx];
    candidates[current_idx + 1..]
        .iter()
        .find(|c| c.level <= current.level)
}

/// 收集在两个候选之间的所有 block_ids。
fn collect_blocks_between(
    all_blocks: &[(&crate::domain::raw_document::RawBlock, usize)],
    start: &HeadingCandidate,
    end: Option<&HeadingCandidate>,
    assigned: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    let mut started = false;

    for (block, _page) in all_blocks {
        if block.id == start.block_id {
            started = true;
        }

        if started {
            ids.push(block.id.clone());
            assigned.insert(block.id.clone());
        }

        if let Some(end_candidate) = end {
            if block.id == end_candidate.block_id {
                // 只有当 end 与 start 不在同一个 block 时，才弹出 end block
                // （end block 属于下一个 section）。
                // 如果 start 和 end 共享同一个 block，说明该 block 内存在多个标题，
                // 当前 section 的内容仍然包含在该 block 中，因此保留。
                if end_candidate.block_id != start.block_id {
                    ids.pop();
                    assigned.remove(&end_candidate.block_id);
                }
                break;
            }
        }
    }

    ids
}

/// 从当前 section 关联的 blocks 中提取正文文本。
///
/// 从 section 标题行之后开始收集，到下一个同级/上级标题出现时停止。
/// 正文中会保留子章节的标题行，但跳过页码等噪声行。
fn extract_section_body(
    candidate: &HeadingCandidate,
    next_boundary: Option<&HeadingCandidate>,
    all_blocks: &[(&crate::domain::raw_document::RawBlock, usize)],
    block_ids: &[String],
) -> String {
    let block_id_set: std::collections::HashSet<&str> =
        block_ids.iter().map(|s| s.as_str()).collect();

    if block_id_set.is_empty() {
        return String::new();
    }

    let mut body_lines: Vec<String> = Vec::new();
    let mut found_title = false;
    let mut done = false;

    for (block, _) in all_blocks {
        if done {
            break;
        }
        if !block_id_set.contains(block.id.as_str()) {
            continue;
        }

        for line in block.text.lines() {
            if done {
                break;
            }
            let trimmed = line.trim();

            // 跳过空行和 PDF 噪声（中文 PDF 块内换行均为物理折行伪影，不保留）
            if trimmed.is_empty() || is_page_noise(trimmed) {
                continue;
            }

            // 定位到当前 section 的标题行
            if !found_title {
                if trimmed == candidate.title.trim() {
                    found_title = true;
                }
                continue;
            }

            // 遇到任意子标题模式 → 停止（防止父 section 吞并子 section 正文）
            if matches_heading_pattern(trimmed) {
                done = true;
                break;
            }

            // 遇到下一边界标题 → 停止
            if let Some(end) = next_boundary {
                if trimmed == end.title.trim() {
                    done = true;
                    break;
                }
            }

            body_lines.push(trimmed.to_string());
        }
    }

    // 去除尾部空行
    while body_lines.last().map_or(false, |l| l.is_empty()) {
        body_lines.pop();
    }

    body_lines.join("")
}

/// 检测标题是否因 PDF 物理折行而被截断。
///
/// 调用方应仅对 Level >= 3 调用此函数（章/节标题本身即为完整标题）。
///
/// 判定条件：
/// 1. 标题不以句号（。！？）、冒号（：）、逗号（，）或括号结尾
/// 2. body_text 存在且首字符是 CJK 统一汉字或小写英文字母（续接特征）
fn is_title_truncated(title: &str, body_text: &str) -> bool {
    if body_text.is_empty() {
        return false;
    }

    // 短标题（< 15 字符）通常是完整标题，不需要续接检测
    if title.chars().count() < 15 {
        return false;
    }

    let title_last = title.chars().last().unwrap_or('\0');
    // 标题以这些字符结尾 → 大概率是完整句子，未截断
    // 注意：逗号（，,）不是句子结束标点，标题以逗号结尾通常是 PDF 折行截断
    if matches!(title_last, '。' | '！' | '？' | '）' | ')' | '：' | ':') {
        return false;
    }

    // 跳过 body_text 开头的空白字符
    let body_first = body_text.chars().find(|c| !c.is_whitespace()).unwrap_or('\0');
    if body_first == '\0' {
        return false;
    }
    // body 首字符是 CJK 汉字 或 小写字母 → 续接
    body_first >= '\u{4E00}' && body_first <= '\u{9FFF}'
        || body_first >= '\u{3400}' && body_first <= '\u{4DBF}'
        || body_first.is_ascii_lowercase()
}

/// 将被 PDF 物理折行截断的标题与正文首段进行合并。
///
/// 当标题行不以句子结束标点（。！？）结尾，且 body_text 首字符为 CJK
/// 续接内容时，将 body_text 中的续接行合并回标题，直到遇到：
/// 1. 句子结束标点
/// 2. 另一个标题模式匹配
/// 3. body_text 耗尽
///
/// 返回 `(merged_title, remaining_body_text)`。
fn merge_truncated_title(title: &str, body_text: &str) -> (String, String) {
    if body_text.is_empty() {
        return (title.to_string(), String::new());
    }

    let mut merged = title.to_string();
    let mut remaining: Vec<&str> = Vec::new();
    let mut merge_done = false;

    for line in body_text.lines() {
        if merge_done {
            remaining.push(line);
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // 遇到另一个标题模式 → 停止合并，该行留给剩余正文
        if matches_heading_pattern(trimmed) {
            remaining.push(line);
            merge_done = true;
            continue;
        }

        merged.push_str(line);

        // 句子结束标点 → 当前行合并完成，后续行留给 body_text
        if trimmed.ends_with('。') || trimmed.ends_with('！') || trimmed.ends_with('？') {
            merge_done = true;
        }
    }

    (merged, remaining.join(""))
}

/// 根据 block_id 查找页码。
fn find_block_page(
    all_blocks: &[(&crate::domain::raw_document::RawBlock, usize)],
    target_id: &str,
) -> Option<usize> {
    all_blocks
        .iter()
        .find(|(b, _)| b.id == target_id)
        .map(|(_, page)| *page)
}

/// 递归统计 section 总数。
fn count_sections(sections: &[Section]) -> usize {
    sections
        .iter()
        .map(|s| 1 + count_sections(&s.children))
        .sum()
}

/// 递归统计各级别数量。
fn count_levels(sections: &[Section], counts: &mut std::collections::HashMap<u8, usize>) {
    for s in sections {
        *counts.entry(s.level).or_insert(0) += 1;
        count_levels(&s.children, counts);
    }
}

// ─── 测试 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_page_noise() {
        // 纯数字页码
        assert!(is_page_noise("1"));
        assert!(is_page_noise("92"));
        assert!(is_page_noise(" 5 "));
        // "第X页共Y页" 格式
        assert!(is_page_noise("第1页共78页"));
        assert!(is_page_noise("第11页共78页"));
        assert!(is_page_noise("第2页共78页温馨提示")); // 短后缀也过滤
        assert!(is_page_noise("78第72页共页")); // 残缺变体
        // Unicode 私有区控制字符 (U+F06E)
        assert!(is_page_noise("系统架构要求应用系统采用浏览器/服务器架构，如无特殊原因，禁止要求终端用户安装客\u{F06E}"));
        // 非噪声
        assert!(!is_page_noise("第一章"));
        assert!(!is_page_noise("1. 供应商资格"));
        assert!(!is_page_noise("1234")); // 纯数字但超过3位
        assert!(!is_page_noise("供应商应具备以下条件：")); // 正常正文
    }

    #[test]
    fn test_part_pattern() {
        let pat = &HEADING_PATTERNS[0];
        assert!(pat.regex.is_match("第一部分投标邀请函"));
        assert!(pat.regex.is_match("第五部分投标文件格式"));
        assert!(!pat.regex.is_match("第一章磋商邀请"));
        assert!(!pat.regex.is_match("一、项目概况"));
    }

    #[test]
    fn test_chapter_pattern() {
        let pat = &HEADING_PATTERNS[1];
        assert!(pat.regex.is_match("第一章磋商邀请"));
        assert!(pat.regex.is_match("第五章合同文本"));
        assert!(!pat.regex.is_match("一、项目概况"));
        assert!(!pat.regex.is_match("1. 供应商资格"));
    }

    #[test]
    fn test_cjk_numbered_pattern() {
        let pat = &HEADING_PATTERNS[3];
        assert!(pat.regex.is_match("一、项目概况"));
        assert!(pat.regex.is_match("二.供应商的资格要求"));
        assert!(!pat.regex.is_match("第一章"));
    }

    #[test]
    fn test_paren_cjk_pattern() {
        let pat = &HEADING_PATTERNS[4];
        assert!(pat.regex.is_match("（一）资格要求"));
        assert!(pat.regex.is_match("(二) 评审标准"));
    }

    #[test]
    fn test_digit_dot_pattern() {
        let pat = &HEADING_PATTERNS[5];
        assert!(pat.regex.is_match("1. 供应商资格"));
        assert!(pat.regex.is_match("2、项目概况"));
        assert!(pat.regex.is_match("3)其他要求"));
    }

    #[test]
    fn test_paren_digit_pattern() {
        let pat = &HEADING_PATTERNS[6];
        assert!(pat.regex.is_match("（1）营业执照副本"));
        assert!(pat.regex.is_match("(2) 法定代表人证明"));
    }

    #[test]
    fn test_article_pattern() {
        let pat = &HEADING_PATTERNS[7];
        assert!(pat.regex.is_match("第九条工程的支付、结算"));
    }
}
