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

use crate::domain::raw_document::{BBox, BlockType, RawDocument, RawTable};
#[cfg(test)]
use crate::paths::data_path_str;

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

// ─── 行内标题拆分 ─────────────────────────────────────────────

/// 行内标题拆分正则：检测右括号后紧跟的数字编号。
///
/// 匹配 `)` 或 `）` 后紧跟的 `数字.` / `数字、` / `数字)` / `数字）` 模式。
/// 用于处理 PDF 提取中标题与前文被合并到同一行的情况。
///
/// # 示例
///
/// ```text
/// 输入: "采购包1（...二期））1.主要商务要求"
/// 输出: ["采购包1（...二期））", "1.主要商务要求"]
/// ```
static INLINE_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[)）](\d+[.、)）])").expect("inline heading regex")
});

/// 将行内标题从前置内容中拆分出来。
///
/// 遍历行中每个 `)数字.` / `)数字、` 模式，在右括号后将行切开。
/// 拆分出的标题行（以数字编号开头）会被后续的 [`HEADING_PATTERNS`] 正常匹配。
///
/// 无匹配时返回原行（单元素 Vec）。
fn split_inline_headings(line: &str) -> Vec<String> {
    let matches: Vec<(usize, usize)> = INLINE_HEADING_RE
        .find_iter(line)
        .map(|m| {
            // 右括号的字节位置（也是 split 点之后的位置）
            let paren_byte_start = m.start();
            let paren_char = line[paren_byte_start..].chars().next().unwrap();
            let paren_byte_end = paren_byte_start + paren_char.len_utf8();
            // 标题数字的起始位置 = 右括号之后
            let heading_byte_start = paren_byte_end;
            (paren_byte_end, heading_byte_start)
        })
        .collect();

    if matches.is_empty() {
        return vec![line.to_string()];
    }

    let mut result = Vec::new();
    let mut last_start = 0;

    for (prefix_end, heading_start) in &matches {
        // 前缀部分（含右括号）
        if *prefix_end > last_start {
            let prefix = line[last_start..*prefix_end].to_string();
            if !prefix.trim().is_empty() {
                result.push(prefix);
            }
        }
        last_start = *heading_start;
    }

    // 最后一个标题（从 heading_start 到行尾）
    if last_start < line.len() {
        let remainder = line[last_start..].trim().to_string();
        if !remainder.is_empty() {
            result.push(remainder);
        }
    }

    if result.is_empty() {
        vec![line.to_string()]
    } else {
        result
    }
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
    /// 起始页码 (0-based) — 标题所在页
    pub page_start: usize,
    /// 结束页码 (0-based，包含) — section 子树涵盖的最大页码
    pub page_end: usize,
    /// 本节包含的所有 block ID（用于回溯高亮）
    pub block_ids: Vec<String>,
    /// body_text 实际来源的起始页 (0-based)。
    /// 对于叶子 section，通常等于 page_start；
    /// 对于容器 section，是引文/说明文字的实际起始页，
    /// 可远小于 page_end（子节点页码范围）。
    #[serde(default)]
    pub body_page_start: usize,
    /// body_text 实际来源的结束页 (0-based)。
    #[serde(default)]
    pub body_page_end: usize,
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
        // ★ P1: 行内标题预拆分 — 将 "）1.主要商务要求" 拆为独立行
        let expanded_lines: Vec<String> = block
            .text
            .lines()
            .flat_map(|l| split_inline_headings(l))
            .collect();

        let mut block_has_candidate = false;

        for line in &expanded_lines {
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
                        block_has_candidate = true;
                        break; // 一行只匹配一个模式
                    }
                }
            }
        }

        // ★ A2: 无编号标题 — 利用 PDF 提取器的 block type 信号
        // 如果 block 被标注为 heading 但所有行均未匹配任何编号标题模式，
        // 将首行短文本作为 plain_heading 候选。
        // 典型场景："付款方式""验收要求" 等无编号的表格列标题。
        if !block_has_candidate && block.block_type == BlockType::Heading {
            if let Some(first_line) = block
                .text
                .lines()
                .map(|l| l.trim())
                .find(|l| !l.is_empty() && !is_page_noise(l))
            {
                let char_count = first_line.chars().count();
                // 仅接受短文本（≤ 30 字符），避免将长段落误判为标题
                if char_count >= 2 && char_count <= 30 {
                    candidates.push(HeadingCandidate {
                        level: 5, // 最低层，挂到最近的上级 section 下
                        title: first_line.to_string(),
                        pattern: "plain_heading",
                        page: *page_idx,
                        block_id: block.id.clone(),
                    });
                }
            }
        }
    }

    // 2.5 启发式过滤：排除封面法律条款/提示列表伪装成的伪章节
    //     标书封面后常出现《采购条例》等法律条文列举（如"（一）...；"），
    //     其编号模式与真实章节标题相同但语义是列表项而非章节结构。
    let candidates = filter_pseudo_section_candidates(candidates);

    // 2.6 链式验证：按编号家族+深度分组，验证编号连续性。
    //     真实章节标题（如 "1. 供应商资格"、"2. 项目概况"）形成连续编号链；
    //     正文编号内容（如 "1.项目编号：0724-..."）是孤立项，移除。
    //     参考 Oracle 专利 US 11468346 的"链式验证"方法。
    let candidates = validate_numbering_chains(candidates);

    // 2.7 TOC 目录页检测：同一页出现 ≥3 个 level-1 候选且均无 body_text
    //     → 判定为目录页 → 移除这些候选，避免产生幽灵 Section。
    //     参考 LlamaIndex 的"层级密度检测"方法。
    let candidates = filter_toc_page_candidates(candidates, &all_blocks);

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

/// 启发式过滤：排除封面法律条款/提示列表伪装成的伪章节。
///
/// 标书封面后常出现《采购条例》等法律条文列举（如"（一）在采购活动中应当回避而未回避的；"），
/// 以及"温馨提示"列表（如"二、为避免因迟到而失去投标资格，请适当提前到达。"）。
/// 这些文本的编号模式与真实章节标题相同，但语义是列表项而非章节结构。
///
/// 过滤规则（仅在第一个 Level-1 候选出现之前生效）：
/// 1. 连续 ≥3 个 `paren_cjk`，全部以 `；` 或 `。` 结尾 → 法律条款列举 → 移除
/// 2. 单个 `cjk_numbered` 以 `。！？` 结尾 → 完整句子伪标题 → 移除
fn filter_pseudo_section_candidates(
    mut candidates: Vec<HeadingCandidate>,
) -> Vec<HeadingCandidate> {
    if candidates.is_empty() {
        return candidates;
    }

    // 找到第一个 level-1 候选的索引（"第X部分"/"第X章" 等）
    let first_l1 = match candidates.iter().position(|c| c.level == 1) {
        Some(idx) => idx,
        None => return candidates, // 无 level-1 章节，保守不执行过滤
    };

    // 仅对第一个 level-1 之前的候选执行过滤
    if first_l1 == 0 {
        return candidates;
    }

    let mut remove_indices: Vec<usize> = Vec::new();

    // ── 规则 1: 连续 paren_cjk 组（≥3 个），全部以 ；或。 结尾 → 法律条款枚举 ──
    let mut i = 0;
    while i < first_l1 {
        if candidates[i].pattern == "paren_cjk" {
            let group_start = i;
            let mut group_end = i;
            while group_end < first_l1 && candidates[group_end].pattern == "paren_cjk" {
                group_end += 1;
            }
            let group_size = group_end - group_start;
            if group_size >= 3 {
                // 比例匹配而非全量匹配：PDF 提取可能导致个别长标题被截断，
                // 丢失结尾的 ；或。，因此用 ≥70% 阈值容忍提取噪声。
                let clause_count = candidates[group_start..group_end]
                    .iter()
                    .filter(|c| {
                        let t = c.title.trim();
                        t.ends_with('；') || t.ends_with('。')
                    })
                    .count();
                let ratio = clause_count as f64 / group_size as f64;
                if ratio >= 0.7 {
                    for j in group_start..group_end {
                        remove_indices.push(j);
                    }
                }
            }
            i = group_end;
        } else {
            i += 1;
        }
    }

    // ── 规则 2: cjk_numbered 以 。！？ 结尾 → 完整句子，非章节标题 ──
    //           （真实标题如 "一、技术要求" 不会以句末标点结尾）
    for idx in 0..first_l1 {
        if candidates[idx].pattern == "cjk_numbered" {
            let t = candidates[idx].title.trim();
            if t.ends_with('。') || t.ends_with('！') || t.ends_with('？') {
                remove_indices.push(idx);
            }
        }
    }

    // 按索引降序安全移除
    remove_indices.sort_unstable();
    remove_indices.dedup();
    for &idx in remove_indices.iter().rev() {
        candidates.remove(idx);
    }

    candidates
}

// ─── 链式验证（Oracle 专利 US 11468346 方法）────────────────────

/// 对 `digit_dot` 和 `paren_digit` 候选按编号家族+深度分组，
/// 验证编号连续性。孤立或断链的低置信度候选 → 移除。
///
/// # 核心思想
///
/// 真实章节标题形成跨页连续编号链（"1.", "2.", "3." ...），
/// 而正文编号（"1.项目编号：0724-..."）是孤立的、不成链的。
///
/// # 算法
///
/// 1. 按 (pattern_type, rank) 分组
/// 2. 每组内按文档位置排序
/// 3. 检测连续性：成员数 ≥2 且编号递增 → 保留组；孤立成员 → 移除
/// 4. 额外信号：标题过长（>35 chars for digit_dot / >50 for paren_digit）的孤立候选
///    → 确认为正文内容泄漏 → 移除
fn validate_numbering_chains(
    mut candidates: Vec<HeadingCandidate>,
) -> Vec<HeadingCandidate> {
    if candidates.is_empty() {
        return candidates;
    }

    // 只对 digit_dot (level 4) 和 paren_digit (level 5) 做链式验证
    // 更高层级的 pattern (part/chapter/cjk_numbered/paren_cjk) 由其他过滤器处理
    let target_patterns: &[&str] = &["digit_dot", "paren_digit"];

    // 为每个候选分配全局索引（用于后续稳定排序和移除）
    // 提取编号序列和 rank
    #[derive(Debug, Clone)]
    struct IndexedCandidate {
        global_idx: usize,
        rank: usize,        // 编号深度: "1." = 1, "1.1" = 2
        num_prefix: Vec<u32>, // 编号序列: "1.2.3" → [1, 2, 3]
        title_len: usize,   // 标题字符数
    }

    let mut indexed: Vec<IndexedCandidate> = Vec::new();
    for (idx, c) in candidates.iter().enumerate() {
        if !target_patterns.contains(&c.pattern) {
            continue;
        }
        let (rank, num_prefix) = extract_numbering_info(&c.title, c.pattern);
        indexed.push(IndexedCandidate {
            global_idx: idx,
            rank,
            num_prefix,
            title_len: c.title.chars().count(),
        });
    }

    if indexed.is_empty() {
        return candidates;
    }

    // 按 (pattern_type, rank) 分组
    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new(); // key → Vec<index_into_indexed>
    for (i, ic) in indexed.iter().enumerate() {
        let key = format!("{}_{}", candidates[ic.global_idx].pattern, ic.rank);
        groups.entry(key).or_default().push(i);
    }

    let mut remove_indices: Vec<usize> = Vec::new();

    for (_key, member_indices) in &groups {
        if member_indices.len() < 2 {
            // 孤立候选（该 pattern+rank 组只有 1 个成员）→ 额外检查
            for &mi in member_indices {
                let ic = &indexed[mi];
                let c = &candidates[ic.global_idx];

                // 信号1：标题过长 → 大概率是正文内容
                let long_title = match c.pattern {
                    "digit_dot" => ic.title_len > 35,
                    "paren_digit" => ic.title_len > 50,
                    _ => false,
                };

                // 信号2：标题含冒号 → 正文特征（如 "1.项目编号：..."）
                let has_colon = c.title.contains('：') || c.title.contains(':');

                if long_title || has_colon {
                    remove_indices.push(ic.global_idx);
                }
            }
            continue;
        }

        // 组内按编号序列排序
        let mut sorted_members: Vec<usize> = member_indices.clone();
        sorted_members.sort_by_key(|&mi| &indexed[mi].num_prefix);

        // 验证编号连续性：检查相邻成员的编号是否递增
        let mut chain_breaks: Vec<usize> = Vec::new(); // 断链成员的 global_idx
        for w in sorted_members.windows(2) {
            let prev = &indexed[w[0]];
            let curr = &indexed[w[1]];

            // 检查 prev.num_prefix < curr.num_prefix (字典序)
            if prev.num_prefix >= curr.num_prefix {
                // 编号不连续或重复 → mark curr as suspicious
                chain_breaks.push(curr.global_idx);
            }
        }

        // 对断链成员做二次判断
        for &gb_idx in &chain_breaks {
            let ic = indexed.iter().find(|x| x.global_idx == gb_idx).unwrap();
            let c = &candidates[gb_idx];

            // 以句末标点结尾 → 完整句子，确认为正文泄漏
            let ends_with_sentence = {
                let t = c.title.trim();
                t.ends_with('。') || t.ends_with('！') || t.ends_with('？')
            };

            // 标题过长 → 大概率正文
            let too_long = match c.pattern {
                "digit_dot" => ic.title_len > 40,
                "paren_digit" => ic.title_len > 55,
                _ => false,
            };

            if ends_with_sentence || too_long {
                remove_indices.push(gb_idx);
            }
        }
    }

    // 安全移除（降序）
    remove_indices.sort_unstable();
    remove_indices.dedup();
    for &idx in remove_indices.iter().rev() {
        candidates.remove(idx);
    }

    candidates
}

/// 从标题文本中提取编号信息：(rank, number_sequence)。
///
/// - `digit_dot`: "1." → rank=1, [1]; "1.2.3" → rank=3, [1,2,3]
/// - `paren_digit`: "（1）" → rank=2, [1]; "(2)" → rank=2, [2]
fn extract_numbering_info(title: &str, pattern: &str) -> (usize, Vec<u32>) {
    match pattern {
        "digit_dot" => {
            // 匹配开头的数字序列: "1.2.3" → [1,2,3], "1、" → [1]
            let prefix = title
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect::<String>();
            let nums: Vec<u32> = prefix
                .split('.')
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            let rank = nums.len();
            (rank, nums)
        }
        "paren_digit" => {
            // 匹配括号内的数字: "（1）" → [1], "(2)" → [2]
            let inner: String = title
                .chars()
                .skip_while(|c| *c != '（' && *c != '(')
                .skip(1)
                .take_while(|c| c.is_ascii_digit())
                .collect();
            let num = inner.parse::<u32>().unwrap_or(0);
            (2, vec![num]) // paren_digit 默认 rank=2（嵌套在 digit_dot 之下）
        }
        _ => (1, vec![0]),
    }
}

// ─── TOC 目录页检测（LlamaIndex 层级密度检测方法）───────────────

/// 检测目录页产生的伪标题候选并移除。
///
/// # 判定条件
///
/// 同一页内，如果 level=1 的候选密度 ≥ 3 个，且这些候选的 block 均为
/// Heading 类型且该 block 是页面上唯一的 Heading（无 body text 跟随）→ TOC 页。
///
/// # 原理
///
/// 标书目录页将文档所有 "第X部分" 集中列在一页上，每个都是独立的 Heading block，
/// 不含正文。而真实章节标题分布在不同页，每页最多 1-2 个 level-1 标题，
/// 且标题后有 body text。
fn filter_toc_page_candidates(
    mut candidates: Vec<HeadingCandidate>,
    all_blocks: &[(&crate::domain::raw_document::RawBlock, usize)],
) -> Vec<HeadingCandidate> {
    if candidates.is_empty() {
        return candidates;
    }

    // 统计每页的 level-1 候选数量
    use std::collections::HashMap;
    let mut l1_per_page: HashMap<usize, Vec<usize>> = HashMap::new(); // page → Vec<candidate_idx>
    for (idx, c) in candidates.iter().enumerate() {
        if c.level == 1 {
            l1_per_page.entry(c.page).or_default().push(idx);
        }
    }

    let mut remove_indices: Vec<usize> = Vec::new();

    for (page, cand_indices) in &l1_per_page {
        // 阈值：同一页 ≥ 3 个 level-1 候选 → 疑似目录页
        if cand_indices.len() < 3 {
            continue;
        }

        // 进一步确认：检查这些候选所在的 block 周围是否有 body text
        // 目录页的 level-1 block 通常孤立（页面内无 body text 跟随）
        let page_block_ids: Vec<&str> = all_blocks
            .iter()
            .filter(|(_, p)| *p == *page)
            .map(|(b, _)| b.id.as_str())
            .collect();

        let toc_l1_count = cand_indices
            .iter()
            .filter(|&&ci| {
                let c = &candidates[ci];
                // 检查该候选的 block 在页面内是否是孤立的 Heading
                // （后续 block 都不是同级的 Paragraph body）
                let block_pos = page_block_ids.iter().position(|&id| id == c.block_id);
                match block_pos {
                    Some(pos) => {
                        // 该 block 之后还有 block → 检查是否有紧邻的 Paragraph
                        let has_body_after = page_block_ids[pos..].iter().any(|&id| {
                            all_blocks
                                .iter()
                                .any(|(b, _)| {
                                    b.id == id
                                        && b.block_type
                                            == crate::domain::raw_document::BlockType::Paragraph
                                })
                        });
                        // 目录页 level-1 标题后不应有紧邻的 Paragraph body
                        !has_body_after
                    }
                    None => true,
                }
            })
            .count();

        // 如果 ≥3 个 level-1 都是孤立的（无 body 跟随）→ 确认是目录页
        if toc_l1_count >= 3 {
            for &ci in cand_indices {
                remove_indices.push(ci);
            }
        }
    }

    // 安全移除（降序）
    remove_indices.sort_unstable();
    remove_indices.dedup();
    for &idx in remove_indices.iter().rev() {
        candidates.remove(idx);
    }

    candidates
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

        let (body_text, body_page_start, body_page_end) =
            extract_section_body(candidate, next_boundary, all_blocks, &block_ids);
        // Level 1-2（章/节标题）本身即为完整标题，不检测截断
        let title_truncated = candidate.level >= 3 && is_title_truncated(&candidate.title, &body_text);

        // 如果标题被 PDF 折行截断，将续接正文合并回标题，
        // 避免"标题 + 正文"的人为割裂。
        let (final_title, final_body_text) = if title_truncated {
            let (merged_title, remaining_body) =
                merge_truncated_title(&candidate.title, &body_text);
            // 二次防御：合并后标题若含句末标点（。！？），说明"标题"
            // 实际是完整句子的前半段（如 "5）参加采购活动前3年内..." +
            // "。重大违法记录是指..."），而非真实被截断的标题。
            // 此时回退合并，保留原标题和完整 body_text。
            if merged_title.contains(['。', '！', '？']) {
                (candidate.title.clone(), body_text)
            } else {
                (merged_title, remaining_body)
            }
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
            body_page_start,
            body_page_end,
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

/// 从当前 section 关联的 blocks 中提取正文文本，同时追踪正文的实际页码范围。
///
/// 从 section 标题行之后开始收集，到下一个同级/上级标题出现时停止。
/// 正文中会保留子章节的标题行，但跳过页码等噪声行。
///
/// # 返回值
/// - `String`: 提取的正文文本（已 smart join）
/// - `usize`: body_text 实际起始页 (0-based)
/// - `usize`: body_text 实际结束页 (0-based)
fn extract_section_body(
    candidate: &HeadingCandidate,
    next_boundary: Option<&HeadingCandidate>,
    all_blocks: &[(&crate::domain::raw_document::RawBlock, usize)],
    block_ids: &[String],
) -> (String, usize, usize) {
    let block_id_set: std::collections::HashSet<&str> =
        block_ids.iter().map(|s| s.as_str()).collect();

    if block_id_set.is_empty() {
        return (String::new(), candidate.page, candidate.page);
    }

    let mut body_lines: Vec<String> = Vec::new();
    let mut body_page_start: usize = candidate.page;
    let mut body_page_end: usize = candidate.page;
    let mut found_title = false;
    let mut done = false;
    let mut first_body_page_set = false;

    for (block, page) in all_blocks {
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

            // 追踪正文来源的页码范围
            if !first_body_page_set {
                body_page_start = *page;
                first_body_page_set = true;
            }
            body_page_end = *page;

            body_lines.push(trimmed.to_string());
        }
    }

    // 去除尾部空行
    while body_lines.last().map_or(false, |l| l.is_empty()) {
        body_lines.pop();
    }

    let body_text = smart_join_body(&body_lines);
    (body_text, body_page_start, body_page_end)
}

/// 智能拼接 body lines。
///
/// 核心思路：用通用的"行首语义单元检测" + "行尾句子边界检测"，
/// 而非依赖特定关键词。使得方正排版 PDF 和标准 PDF 的表格/条目结构
/// 都能被正确保留，同时正常段落的 PDF 物理折行仍被无缝拼接。
///
/// 断行条件（满足任一即断）：
/// 1. 当前行是新的语义单元起始（数字开头、括号编号、特殊符号）
/// 2. 前一行以句子结束标点（。！？）结尾
///
/// 默认：PDF 物理折行，用 "" 无缝拼接。
fn smart_join_body(lines: &[String]) -> String {
    let mut result = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 && should_break_before(&lines[i - 1], line) {
            result.push('\n');
        }
        result.push_str(line);
    }
    result
}

/// 判断从前一行到当前行是否需要插入换行符。
///
/// 两套规则（满足任一即断行）：
/// - 规则1：当前行以新语义单元标记开头 → 独立条目/表行/编号条款
/// - 规则2：前一行以句子结束标点结尾 → 新句子开始
fn should_break_before(prev: &str, curr: &str) -> bool {
    let curr = curr.trim();
    if curr.is_empty() {
        return false;
    }

    // 规则1：新语义单元起始符
    if is_new_semantic_unit(curr) {
        return true;
    }

    // 规则2：前一行以句末标点结尾（仅 。！？ 三类无歧义标点；
    // 不包含 ：；，（因为它们常出现在标签-值对或从句中）
    let prev = prev.trim();
    if prev.ends_with(['。', '！', '？']) {
        return true;
    }

    // 默认：PDF 物理折行，无缝拼接
    false
}

/// 检测一行是否为"新语义单元"的起始。
///
/// 三类标记：
/// 1. ASCII 数字开头（覆盖 `digit_sep` 如 "1、" "2." 和 `digit_bare`
///    如 "1PVC管材" "1-1教育用房"）
/// 2. 括号编号开头（如 "（1）" "(2)" "（一）"）
/// 3. 特殊符号标记（▲ ★ ● ■ ◆）
fn is_new_semantic_unit(s: &str) -> bool {
    let c0 = s.chars().next().unwrap_or('\0');

    // 1. 数字开头 → 新编号条目 / 表格行
    if c0.is_ascii_digit() {
        return true;
    }

    // 2. 括号编号 → （1）（2）(1) (2)（一）（二）
    if c0 == '\u{FF08}' || c0 == '(' {
        // fullwidth left parenthesis （ 或 halfwidth (
        return true;
    }

    // 3. 特殊符号标记 → ▲ ★ ● ■ ◆
    if matches!(c0, '\u{25B2}' | '\u{2605}' | '\u{25CF}' | '\u{25A0}' | '\u{25C6}') {
        return true;
    }

    false
}

/// 检测一行是否以 ASCII 数字紧接 CJK 汉字开头（如 "4驻场要求"、"7质量保证"）。
/// 这是标书 PDF 提取中常见的独立编号条款模式。
/// 注意：此函数已不再被 `smart_join_body` 直接调用（改用通用的
/// `is_new_semantic_unit`），但保留作为独立检测工具供其他模块使用。
fn is_digit_cjk_start(s: &str) -> bool {
    let chars: Vec<char> = s.chars().take(2).collect();
    chars.len() == 2
        && chars[0].is_ascii_digit()
        && chars[1] >= '\u{4E00}'
        && chars[1] <= '\u{9FFF}'
}

/// 检测标题是否因 PDF 物理折行而被截断。
///
/// 调用方应仅对 Level >= 3 调用此函数（章/节标题本身即为完整标题）。
///
/// 判定条件：
/// 1. 标题不以句号（。！？）、冒号（：）、逗号（，）或括号结尾
/// 2. body_text 存在且首字符是 CJK 统一汉字或小写英文字母（续接特征）
/// 3. 新增：如果 title + body_text 第一个句子能拼成以 。！？ 结尾的完整句
///    （≤ 120 chars），说明 title 是完整句子的前半段而非被截断的标题 → 返回 false
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

    // ── 新增：句子完整性预判 ──
    // 如果 title + body_text 的第一个句子能拼成以 。！？ 结尾的完整句，
    // 说明 title 是完整句子的前半段（如 "5）参加采购活动前3年内..."），
    // 而非被 PDF 折行截断的真实标题 → 不触发 merge
    if let Some(end_byte) = body_text.find(['。', '！', '？']) {
        // 取到第一个句末标点（含）为止
        let end_char_len = body_text[end_byte..]
            .chars()
            .next()
            .map_or(0, |c| c.len_utf8());
        let first_sentence_end = end_byte + end_char_len;
        let combined_len = title.chars().count()
            + body_text[..first_sentence_end].chars().count();
        // 组合后不超过 120 字符且以句号结尾 → title 是句子前半段
        if combined_len <= 120 {
            return false;
        }
    }

    // 跳过 body_text 开头的空白字符
    let body_first = body_text.chars().find(|c| !c.is_whitespace()).unwrap_or('\0');
    if body_first == '\0' {
        return false;
    }
    // body 首字符是 CJK 汉字、小写字母或 ASCII 数字 → 续接
    // ASCII 数字捕获 "4驻场要求"、"7质量保证" 等标书常见模式
    body_first >= '\u{4E00}' && body_first <= '\u{9FFF}'
        || body_first >= '\u{3400}' && body_first <= '\u{4DBF}'
        || body_first.is_ascii_lowercase()
        || body_first.is_ascii_digit()
}

/// 将被 PDF 物理折行截断的标题与正文首段进行合并。
///
/// 当标题行不以句子结束标点（。！？）结尾，且 body_text 首字符为 CJK
/// 续接内容时，将 body_text 中的续接行合并回标题，直到遇到：
/// 1. 句子结束标点（仅合并到第一个 。！？ 为止，不吞整行）
/// 2. 另一个标题模式匹配
/// 3. 独立编号条款（digit+CJK 开头，如 "4驻场要求"）
/// 4. 合并字符数超过上限（60 字符）
/// 5. body_text 耗尽
///
/// 返回 `(merged_title, remaining_body_text)`。
fn merge_truncated_title(title: &str, body_text: &str) -> (String, String) {
    if body_text.is_empty() {
        return (title.to_string(), String::new());
    }

    const MAX_MERGE_CHARS: usize = 60; // 合并字符上限

    let mut merged = title.to_string();
    let title_len = merged.chars().count();
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

        // 遇到独立编号条款（digit+CJK，如 "4驻场要求"）→ 停止合并
        // 这是新的语义单元，不是标题续文
        if is_digit_cjk_start(trimmed) {
            remaining.push(line);
            merge_done = true;
            continue;
        }

        // 统一 merge cap：所有 push_str 不得超过此配额
        let cap = title_len + MAX_MERGE_CHARS - merged.chars().count();
        if cap == 0 {
            remaining.push(line);
            merge_done = true;
            continue;
        }

        // 句子级合并：只合并到第一个句末标点为止，且不超过 cap。
        // 解决 smart join 将多句话合并成一行后，push_str(line) 一次吞入全部的问题。
        if let Some(byte_pos) = trimmed.find(['。', '！', '？']) {
            let char_len = trimmed[byte_pos..].chars().next().unwrap().len_utf8();
            let end = byte_pos + char_len;
            let merge_text = &trimmed[..end]; // 到第一个句末标点（含）
            let take_count = merge_text.chars().count().min(cap);
            let split_byte: usize = merge_text.char_indices()
                .nth(take_count)
                .map(|(i, _)| i)
                .unwrap_or(merge_text.len());
            merged.push_str(&merge_text[..split_byte]);
            // 未合并完的部分（如果有）留在 body
            if take_count < merge_text.chars().count() {
                remaining.push(merge_text[split_byte..].trim());
            }
            // 同一行中句号之后的内容留在 body
            let rest = trimmed[end..].trim();
            if !rest.is_empty() {
                remaining.push(rest);
            }
            merge_done = true;
            continue;
        }

        // 无句末标点的行：仅合并不超过 cap 的字符数
        let take_count = line.chars().count().min(cap);
        if take_count < line.chars().count() {
            let split_byte: usize = line.char_indices()
                .nth(take_count)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            merged.push_str(&line[..split_byte]);
            let rest = line[split_byte..].trim();
            if !rest.is_empty() {
                remaining.push(rest);
            }
            merge_done = true;
        } else {
            merged.push_str(line);
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

// ─── 启发式表格检测（方案五）─────────────────────────────────

/// 从 blocks 中启发式检测纯文本型表格（`|` 分隔），补充到 raw_doc 的 tables 中。
///
/// 对每个页面，扫描 blocks 中连续含 `|` 分隔符且列数一致的段落组，
/// 每组构造一个 RawTable，追加到对应 RawPage.tables。
///
/// # 检测条件
///
/// - 至少连续 2 行含 `|` 分隔符
/// - 相邻行 `|` 分隔出的列数相同（≥2）
/// - block 类型非 heading
///
/// # 返回
///
/// 检测到的伪表格数量（用于日志输出）。
pub fn detect_pipe_tables(raw_doc: &mut RawDocument) -> usize {
    let mut total_detected = 0;

    for page in &mut raw_doc.pages {
        let mut i = 0;
        while i < page.blocks.len() {
            let block = &page.blocks[i];

            // 跳过非 paragraph 类型（降低误判）
            if block.block_type != BlockType::Paragraph {
                i += 1;
                continue;
            }

            // 检查是否含 `|` 分隔符
            let cols = block.text.split('|').count();
            if cols < 2 {
                i += 1;
                continue;
            }

            // 收集连续且列数一致的 blocks
            let mut table_block_indices: Vec<usize> = Vec::new();
            let mut j = i;
            while j < page.blocks.len() {
                let next = &page.blocks[j];
                if next.block_type != BlockType::Paragraph {
                    break;
                }
                let next_cols = next.text.split('|').count();
                if next_cols != cols {
                    break;
                }
                table_block_indices.push(j);
                j += 1;
            }

            // 至少 2 行才认为是表格
            if table_block_indices.len() < 2 {
                i = j;
                continue;
            }

            // 构造 RawTable
            let rows: Vec<Vec<Option<String>>> = table_block_indices
                .iter()
                .map(|&idx| {
                    page.blocks[idx]
                        .text
                        .split('|')
                        .map(|cell| {
                            let trimmed = cell.trim().to_string();
                            if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed)
                            }
                        })
                        .collect()
                })
                .collect();

            let table_id = format!("t_{}_{}", page.page_index, page.tables.len());
            let table = RawTable {
                id: table_id,
                bbox: None, // 启发式表格无精确定位 bbox
                rows,
            };
            page.tables.push(table);
            total_detected += 1;

            i = j; // 跳过已消费的 blocks
        }
    }

    total_detected
}

// ─── 跨页表格合并 ─────────────────────────────────────────────

/// 合并跨页断裂的表格。
///
/// PDF 提取器按页提取表格，同一逻辑表格跨页时会被拆成多个独立 RawTable。
/// 此函数检测连续页面上结构一致（列数相同、首列不重复表头）的表格并合并。
///
/// # 合并条件
///
/// 1. 连续两页（N, N+1）各有至少一张表
/// 2. 页 N 的最后一张表与页 N+1 的第一张表**列数相同**
/// 3. 页 N+1 的表首行首单元格 ≠ 页 N 的表首行首单元格（否则是重复表头，非延续）
///
/// # 行为
///
/// - 将页 N+1 的首表行追加到页 N 的末表
/// - 更新页 N 末表的 bbox 为两张表的并集
/// - 从页 N+1 删除已合并的表
/// - 递归尝试，直到无法再合并
pub fn merge_cross_page_tables(raw_doc: &mut RawDocument) -> usize {
    if raw_doc.pages.len() < 2 {
        return 0;
    }

    let mut merge_count = 0;
    let page_count = raw_doc.pages.len();

    // 遍历所有连续页面对
    for n in 0..(page_count - 1) {
        // 需要安全地同时借用 pages[n] 和 pages[n+1]
        // 使用 split_at_mut 实现
        let (left_pages, right_pages) = raw_doc.pages.split_at_mut(n + 1);
        let page_n = &mut left_pages[n];
        let page_n1 = &mut right_pages[0];

        if page_n.tables.is_empty() || page_n1.tables.is_empty() {
            continue;
        }

        let last_idx = page_n.tables.len() - 1;

        // 先提取比较所需的信息（不可变借用）
        let cols_n = page_n.tables[last_idx].rows.first().map(|r| r.len()).unwrap_or(0);
        let cols_n1 = page_n1.tables[0].rows.first().map(|r| r.len()).unwrap_or(0);
        if cols_n == 0 || cols_n != cols_n1 {
            continue;
        }

        let first_cell_n = page_n.tables[last_idx].rows.first()
            .and_then(|r| r.first())
            .and_then(|c| c.as_deref())
            .unwrap_or("")
            .to_string();
        let first_cell_n1 = page_n1.tables[0].rows.first()
            .and_then(|r| r.first())
            .and_then(|c| c.as_deref())
            .unwrap_or("")
            .to_string();
        if first_cell_n == first_cell_n1 && !first_cell_n.is_empty() {
            continue;
        }

        // 提取 n1 表的行（take 避免 clone）；bbox 引用读取
        let n1_rows = std::mem::take(&mut page_n1.tables[0].rows);
        let n1_bbox_ref = &page_n1.tables[0].bbox; // 借用，不移出

        // 更新 bbox 为两张表的并集
        {
            let bbox_n = &page_n.tables[last_idx].bbox;
            if let (Some(bb_n), Some(bb_n1)) = (bbox_n, n1_bbox_ref) {
                page_n.tables[last_idx].bbox = Some(BBox {
                    x0: bb_n.x0.min(bb_n1.x0),
                    top: bb_n.top.min(bb_n1.top),
                    x1: bb_n.x1.max(bb_n1.x1),
                    bottom: bb_n.bottom.max(bb_n1.bottom),
                });
            }
        }

        page_n.tables[last_idx].rows.extend(n1_rows);
        // n1 表已被清空（rows taken），移除它
        page_n1.tables.remove(0);
        merge_count += 1;
    }

    merge_count
}

// ─── 表格内容注入（方案二）─────────────────────────────────────

/// 将 RawDocument 中的表格内容注入到 Section 树的 body_text 中。
///
/// 对每个 Section，查找其 **body 实际页码范围**（body_page_start..=body_page_end）
/// 覆盖的页面上的表格，将表格格式化为 Markdown 表格文本，追加到 body_text 末尾。
/// 同时将表格 ID 追加到 block_ids 以便回溯。
///
/// # 去重策略
///
/// 使用全局 `visited_tables: HashSet<String>` 确保每张表格只注入一次——
/// 注入到**最先遇到**的 Section（递归深度优先，即最深层级最精确的 Section）。
/// 祖先 Section 不会重复注入已被子孙 Section 消费的表格。
///
/// # Markdown 表格格式
///
/// ```markdown
/// | 品目号 | 品目名称 | 采购标的 | 数量 | 是否允许进口 |
/// |--------|----------|----------|------|-------------|
/// | 1-1    | 教育用房施工 | 东莞理工学院... | 1(项) | 否 |
/// ```
///
/// - 单元格内的换行符替换为空格
/// - 空单元格输出为空字符串
/// - 表格之间用 `\n\n` 分隔
pub fn inject_tables_into_sections(
    sections: &mut [Section],
    raw_doc: &RawDocument,
) {
    // 构建 page → tables 索引（只读，一次扫描）
    let page_tables: std::collections::HashMap<usize, &[RawTable]> = raw_doc
        .pages
        .iter()
        .map(|p| (p.page_index, p.tables.as_slice()))
        .collect();

    let mut visited_tables: std::collections::HashSet<String> = std::collections::HashSet::new();
    inject_tables_recursive(sections, &page_tables, &mut visited_tables);
}

fn inject_tables_recursive(
    sections: &mut [Section],
    page_tables: &std::collections::HashMap<usize, &[RawTable]>,
    visited_tables: &mut std::collections::HashSet<String>,
) {
    for section in sections.iter_mut() {
        // 先对子节点按 body_page_start 排序，确保页码小的 Section
        // 优先认领边界页表格，行为确定化。
        section.children.sort_by_key(|c| c.body_page_start);

        // 先递归处理子节点（深度优先），确保表格优先归属到最深层 Section
        inject_tables_recursive(&mut section.children, page_tables, visited_tables);

        // 收集该 Section **body 实际页码范围**内的表格
        // 使用 body_page_start..=body_page_end 而非 page_start..=page_end：
        // 容器 Section 的 page_start..=page_end 涵盖所有子孙节点页面，
        // 若用此范围会吞并属于子节点的表格。body_page 范围只反映 Section
        // 自身正文的实际页面，精确归属。
        let mut table_texts: Vec<String> = Vec::new();

        for page_idx in section.body_page_start..=section.body_page_end {
            if let Some(tables) = page_tables.get(&page_idx) {
                for table in *tables {
                    // 去重：跳过已被更深层 Section 消费的表格
                    if visited_tables.contains(&table.id) {
                        continue;
                    }
                    if let Some(md) = format_table_as_markdown(table) {
                        table_texts.push(md);
                        visited_tables.insert(table.id.clone());
                        // 将 table ID 加入追溯链
                        if !section.block_ids.contains(&table.id) {
                            section.block_ids.push(table.id.clone());
                        }
                    }
                }
            }
        }

        // ── Fallback: 纯容器 Section 的 page span 扫描 ──────────
        // 纯容器 Section（无 body_text，body_page_start/end 通常为 0）
        // 其子节点覆盖的页面范围可能存在间隙（如子节点 A 覆盖 30-35 页、
        // 子节点 B 覆盖 40-60 页，第 36-39 页为容器过渡页）。
        // 这些间隙页上的表格不被任何子节点认领，也不会被 body_page 扫描
        // （body_page 为 0..=0），导致彻底丢失。
        //
        // 此处使用容器的完整 page_start..=page_end 范围做一次兜底扫描，
        // 拾取子节点遗漏的表格。由于 visited_tables 已被子节点消费过，
        // 不会造成重复注入。
        if !section.children.is_empty() && section.body_text.is_empty() {
            for page_idx in section.page_start..=section.page_end {
                if let Some(tables) = page_tables.get(&page_idx) {
                    for table in *tables {
                        if visited_tables.contains(&table.id) {
                            continue;
                        }
                        if let Some(md) = format_table_as_markdown(table) {
                            table_texts.push(md);
                            visited_tables.insert(table.id.clone());
                            if !section.block_ids.contains(&table.id) {
                                section.block_ids.push(table.id.clone());
                            }
                        }
                    }
                }
            }
        }

        if !table_texts.is_empty() {
            let table_section = table_texts.join("\n\n");
            if section.body_text.is_empty() {
                section.body_text = table_section;
            } else {
                section.body_text = format!("{}\n\n{}", section.body_text, table_section);
            }
        }
    }
}

/// 将 RawTable 格式化为 Markdown 表格字符串。
///
/// 返回 None 如果表格为空（无行或无列）。
fn format_table_as_markdown(table: &RawTable) -> Option<String> {
    if table.rows.is_empty() {
        return None;
    }

    // 计算列数（取最大行宽）
    let col_count = table.rows.iter()
        .map(|row| row.len())
        .max()
        .unwrap_or(0);

    if col_count == 0 {
        return None;
    }

    let mut lines: Vec<String> = Vec::new();

    for (row_idx, row) in table.rows.iter().enumerate() {
        let cells: Vec<String> = (0..col_count)
            .map(|col| {
                row.get(col)
                    .and_then(|opt| opt.as_ref())
                    .map(|s| s.replace('\n', " ").trim().to_string())
                    .unwrap_or_default()
            })
            .collect();

        lines.push(format!("| {} |", cells.join(" | ")));

        // 表头后添加分隔行
        if row_idx == 0 {
            let sep: Vec<String> = (0..col_count).map(|_| "---".to_string()).collect();
            lines.push(format!("| {} |", sep.join(" | ")));
        }
    }

    Some(lines.join("\n"))
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

    // ─── A1: split_inline_headings 测试 ─────────────────────────

    #[test]
    fn test_split_inline_headings_no_match() {
        // 普通行无右括号+数字标题模式 → 原样返回
        let result = split_inline_headings("普通的正文内容");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "普通的正文内容");
    }

    #[test]
    fn test_split_inline_headings_basic() {
        // 采购包名后紧跟 "1.主要商务要求"
        let result = split_inline_headings(
            "采购包1（东莞理工学院松山湖校区智慧教室环境改造工程（二期））1.主要商务要求",
        );
        assert_eq!(result.len(), 2, "应拆分为前缀和标题两部分，实际: {:?}", result);
        assert!(result[0].contains("（二期））"), "前缀应包含右括号，实际: {}", result[0]);
        assert_eq!(result[1], "1.主要商务要求", "标题应从数字开始，实际: {}", result[1]);
    }

    #[test]
    fn test_split_inline_headings_heading_already_at_start() {
        // 标题已在行首 → 不应被拆分（没有前置右括号）
        let result = split_inline_headings("1.具有良好的商业信誉和健全的财务会计制度；");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "1.具有良好的商业信誉和健全的财务会计制度；");
    }

    #[test]
    fn test_split_inline_headings_plain_text_no_paren() {
        // 行中包含数字编号但前面不是右括号（是冒号）
        let result = split_inline_headings("条件包括：1.具有良好的商业信誉");
        assert_eq!(result.len(), 1, "冒号不是右括号，不应拆分");
    }

    #[test]
    fn test_split_inline_headings_empty_line() {
        let result = split_inline_headings("");
        assert_eq!(result.len(), 1);
        assert!(result[0].is_empty());
    }

    // ─── A1 + A2: sectionize 集成测试 ───────────────────────────

    /// 构造包含内联标题的 RawDocument 用于集成验证。
    fn make_raw_doc_with_inline_heading() -> RawDocument {
        use crate::domain::raw_document::{BBox, RawBlock, RawPage};
        RawDocument {
            document_id: "test_inline".to_string(),
            source_path: String::new(),
            pages: vec![RawPage {
                page_index: 0,
                width: 595.0,
                height: 842.0,
                text: String::new(),
                words: vec![],
                blocks: vec![
                    RawBlock {
                        id: "b_0_0".to_string(),
                        block_type: BlockType::Heading,
                        text: "六、《资格条件承诺函》格式".to_string(),
                        bbox: BBox { x0: 90.0, top: 75.0, x1: 350.0, bottom: 100.0 },
                    },
                    RawBlock {
                        id: "b_0_1".to_string(),
                        block_type: BlockType::Paragraph,
                        text: "采购包1（东莞理工学院）1.主要商务要求".to_string(),
                        bbox: BBox { x0: 90.0, top: 560.0, x1: 500.0, bottom: 580.0 },
                    },
                ],
                tables: vec![],
                lines: vec![],
                rects: vec![],
            }],
        }
    }

    #[test]
    fn test_sectionize_detects_inline_heading() {
        let doc = make_raw_doc_with_inline_heading();
        let output = sectionize(&doc);

        // 应检测到 2 个标题：六、... 和 1.主要商务要求
        // 遍历树查找 "1.主要商务要求"
        let titles: Vec<String> = collect_all_titles(&output.sections);
        assert!(
            titles.iter().any(|t| t.contains("1.主要商务要求")),
            "应检测到内联标题 '1.主要商务要求'，实际标题: {:?}",
            titles
        );
    }

    #[test]
    fn test_sectionize_detects_plain_heading() {
        let doc = make_raw_doc_with_inline_heading();
        let output = sectionize(&doc);

        let orphans = output.stats.orphan_blocks;
        // "1.主要商务要求" 应被正确识别，不应有过多孤儿 block
        assert!(
            orphans <= 1,
            "孤儿 block 不应超过 1 个（可能有页码噪声），实际: {}",
            orphans
        );
    }

    /// 递归收集所有 section 的 title。
    fn collect_all_titles(sections: &[Section]) -> Vec<String> {
        let mut titles = Vec::new();
        for s in sections {
            titles.push(s.title.clone());
            titles.extend(collect_all_titles(&s.children));
        }
        titles
    }

    /// 递归收集所有 section 的 (pattern, title) 对，用于调试。
    fn collect_pattern_titles(sections: &[Section]) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for s in sections {
            result.push((s.pattern.clone(), s.title.clone()));
            result.extend(collect_pattern_titles(&s.children));
        }
        result
    }

    // ─── 端到端回归测试：使用实际 PDF 的 RawDocument ─────────────

    /// 加载已有的 raw JSON，运行 sectionize，验证关键标题被正确识别。
    /// 此测试依赖 `output/raw_json/智慧教室环境改造工程_raw.json`。
    #[test]
    fn test_real_pdf_detects_inline_and_plain_headings() {
        let raw_path = data_path_str("output/raw_json/智慧教室环境改造工程_raw.json");
        let raw_json = match std::fs::read_to_string(raw_path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("跳过: raw JSON 文件不存在 ({})", raw_path);
                return;
            }
        };
        let doc: RawDocument = match serde_json::from_str(&raw_json) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("跳过: raw JSON 解析失败: {}", e);
                return;
            }
        };

        let output = sectionize(&doc);
        let titles = collect_all_titles(&output.sections);
        let pattern_titles = collect_pattern_titles(&output.sections);

        // ★ 验证 A1: "1.主要商务要求" 被检测为独立 section
        let has_business_req = titles.iter().any(|t| t.contains("1.主要商务要求"));
        assert!(
            has_business_req,
            "A1 失败: 未检测到 '1.主要商务要求'。\n\
             实际标题列表 (共 {} 个):\n{:#?}\n\
             完整 pattern+title:\n{:#?}",
            titles.len(),
            titles,
            pattern_titles,
        );

        // ★ 验证 A2: "付款方式" 被检测为 plain_heading section
        let has_payment = titles.iter().any(|t| t == "付款方式");
        assert!(
            has_payment,
            "A2 失败: 未检测到 '付款方式' (plain_heading)。\n\
             实际标题列表:\n{:#?}",
            titles,
        );

        // ★ 验证层级关系: "付款方式" 应在 "1.主要商务要求" 下
        // （"付款方式" 的 section path 祖先应包含 "1.主要商务要求"）
        if has_business_req && has_payment {
            let payment_under_business = verify_child_of(&output.sections, "1.主要商务要求", "付款方式");
            assert!(
                payment_under_business,
                "层级关系错误: '付款方式' 应位于 '1.主要商务要求' 下"
            );
        }

        println!(
            "✅ 端到端验证通过: {} 个 section, {} 个孤儿 block",
            output.stats.total_sections, output.stats.orphan_blocks
        );
    }

    /// 验证 `child_title` 是否在 `parent_title_contains` 的子树中。
    fn verify_child_of(sections: &[Section], parent_contains: &str, child_title: &str) -> bool {
        for s in sections {
            if s.title.contains(parent_contains) {
                let children_titles = collect_all_titles(&s.children);
                return children_titles.iter().any(|t| t == child_title);
            }
            if verify_child_of(&s.children, parent_contains, child_title) {
                return true;
            }
        }
        false
    }

    /// 验证跨页表格合并：t_9_0（标的提供时间/地点）+ t_10_0（付款方式/验收要求）
    /// 是同一张"主要商务要求"表格，合并后 t_9_0 应包含全部 4 行。
    #[test]
    fn test_merge_cross_page_tables_real_pdf() {
        let raw_path = data_path_str("output/raw_json/智慧教室环境改造工程_raw.json");
        let raw_json = match std::fs::read_to_string(raw_path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("跳过: raw JSON 文件不存在 ({})", raw_path);
                return;
            }
        };
        let mut doc: RawDocument = match serde_json::from_str(&raw_json) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("跳过: raw JSON 解析失败: {}", e);
                return;
            }
        };

        // 记录合并前各页表格数
        let tables_before: Vec<usize> = doc.pages.iter().map(|p| p.tables.len()).collect();

        let merged = merge_cross_page_tables(&mut doc);

        let tables_after: Vec<usize> = doc.pages.iter().map(|p| p.tables.len()).collect();

        // 验证：应该有合并发生（page 9 + page 10 的表格被合并）
        assert!(merged > 0,
            "应至少合并 1 组跨页表格。\n\
             合并前各页表格数: {:?}\n\
             合并后各页表格数: {:?}",
            tables_before, tables_after);

        // 验证 page 9 (index 9) 的最后一个表格包含了"付款方式"和"验收要求"
        let page_9 = &doc.pages[9];
        let last_table = page_9.tables.last().expect("page 9 应有表格");
        let all_cells: Vec<String> = last_table.rows.iter()
            .flat_map(|r| r.iter())
            .filter_map(|c| c.as_deref())
            .map(|s| s.chars().take(50).collect())
            .collect();

        let has_payment = all_cells.iter().any(|c| c.contains("付款方式"));
        let has_acceptance = all_cells.iter().any(|c| c.contains("验收要求"));
        let has_delivery_time = all_cells.iter().any(|c| c.contains("标的提供的时间"));

        assert!(has_delivery_time,
            "合并后表格应包含 '标的提供的时间'（来自原 t_9_0）。\n单元格: {:?}", all_cells);
        assert!(has_payment,
            "合并后表格应包含 '付款方式'（来自原 t_10_0）。\n单元格: {:?}", all_cells);
        assert!(has_acceptance,
            "合并后表格应包含 '验收要求'（来自原 t_10_0）。\n单元格: {:?}", all_cells);

        println!(
            "✅ 跨页表格合并测试通过: {} 组合并，合并前后表格数 {:?} → {:?}",
            merged, tables_before, tables_after
        );
    }
}
