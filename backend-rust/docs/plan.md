# 方案：Scout-then-Specialize (STS) 两阶段审查架构（修订版 v3）

> **v3 修订要点**：
> - **问题 1（搜索预算）**：Scout 改为零搜索，基于 Anthropic Orchestrator-Worker 模式——Orchestrator 只规划不搜索，Worker 做实际搜索验证。Scout 输出 `verification_required` 引导 Phase 2 精准搜索。
> - **问题 2（MERGE 键冲突）**：改为文本相似度去重（Jaccard char trigram ≥ 0.7），**加 clause_ids 保护**——不同条款的发现绝不合并。
> - **问题 3（隐性映射）**：删除 "不在你的领域内" 硬编码映射。Agent prompt 只正向描述自身职责，LLM 自行判断相关性。
> - **问题 4（merge 函数未定义）**：补充 `merge_contributors()` 和 `combine_reasons()` 伪代码。
> - **问题 5-7（API 缺口/枚举/prompt 膨胀）**：一并修正。
> - **v3.1 修订**：`risk_summary()` 追加 `verification_required` + `legal_basis` 展示（确保 Phase 2 Agent 能看到 Scout 的搜索引导）；`merge_findings_v3()` 加 clause_ids 保护。

---

## 一、问题诊断（不变）

| # | 根因 | 位置 |
|---|------|------|
| 1 | **并发竞态** — Agent 同时 spawn，SessionGraph 在审查开始时为空 | `coordinator.rs:572-695`, `react_loop.rs:487-544` |
| 2 | **延迟写入** — Agent 完成全部审查后才批量写 SessionGraph | `coordinator.rs:671-684` |
| 3 | **关键词一对多路由** — 一条 clause 命中多个 Agent 关键词 | `coordinator.rs:463-497` |
| 4 | **MERGE key 含 agent** — 不同 Agent 的同类发现不去重 | `coordinator.rs:725-728` |
| 5 | **search_cache 私有** — 跨 Agent 搜索无法共享 | `react_loop.rs:307` |
| 6 | **Prompt 无"跳过"指令** | `prompts.rs` |

---

## 二、业界方案参考（问题 1 调研）

### Anthropic Orchestrator-Worker 模式（2025.06 生产验证）

Anthropic 的 [多 Agent 研究系统](https://www.zenml.io/llmops-database/building-production-multi-agent-research-systems-with-claude) 是整个行业最相关的参考：

| 角色 | 职责 | 搜索？ |
|------|------|--------|
| **LeadResearcher (Orchestrator)** | 分析问题、分解任务、分配 Worker、合成结果 | **不做搜索** |
| **Subagent (Worker)** | 执行分配的子任务、搜索、验证 | 做搜索 |

关键设计原则：
- **"Orchestrator 不搜索，Worker 才搜索"** — Orchestrator 的价值是规划和分工，不是半吊子验证
- **子任务边界显式声明** — 每个 Worker 知道自己负责什么、不负责什么（正向描述，非负向排除）
- **研究预算前置声明** — Worker 启动时估算 tool call 数量，简单任务 ≤5 次，困难任务 ≤15 次
- **"当不再发现新信息时，立即停止"** — 硬性递减收益检测
- **"Start broad, then narrow"** — 先宽泛短 query（<5 词），评估后再收窄
- **Token 占性能方差的 80%** — 预算控制是核心架构决策，不是事后优化

### SCOUT-RAG（2025.02 学术方案）

[SCOUT-RAG](https://ar5iv.labs.arxiv.org/html/2602.08400) 的核心是 **训练无关的领域相关性估计器**——在完整检索前，用轻量级 scout 判断哪些领域值得深度遍历，实现 **4× token 减少**。

### 对我们的启示

> **Scout 不应该搜索。Scout 的价值是"快速分类 + 引导搜索"，不是"半吊子验证"。**

对应到我们的架构：
- **Scout = Anthropic 的 LeadResearcher**：快速扫描全部 clauses，输出"这条条款可能有什么风险、需要查什么法规"，但不自己查
- **Phase 2 Agents = Anthropic 的 Subagents**：根据 Scout 的引导，在自己的领域内做精准搜索验证
- **搜索预算前置**：Scout 在 prompt 中估算验证难度，Phase 2 Agent 据此决定搜索次数

---

## 三、方案概述（v3）

```
Phase 0: Scout（串行 mini-batch，零搜索，~60s）
  ┌──────────────────────────────────────────────────┐
  │ 工具: 只有 read_section + output_finding          │
  │ ★ 无 web_search — Scout 只规划，不搜索             │
  │ 产出: Hypothesis (confidence=0.4-0.5)             │
  │   + knowledge_source: "training_knowledge"        │
  │   + verification_required: ["法规名1", "法规名2"]  │
  │     ↑ 引导 Phase 2 Agent 搜索什么                  │
  │ 每 clause 最多 1 条 Hypothesis                    │
  │ 写入: add_hypothesis() (轻量，不创建 Law 节点)     │
  └──────────────────────┬───────────────────────────┘
                         │ SessionGraph 已有 Hypothesis
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
   ┌─ Demand ──┐  ┌─ Procedure ──┐  ┌─ RuleEngine ──┐
   │ [Session] │  │ [Session]    │  │ [Session]     │
   │ Hypothesis│  │ Hypothesis   │  │ Hypothesis    │
   │ "程序违规"│  │ "程序违规"   │  │ "程序违规"    │
   │ verify:   │  │ verify:      │  │ verify:       │
   │ 87号令,   │  │ 87号令,      │  │ 87号令,       │
   │ 采购法     │  │ 采购法        │  │ 采购法         │
   │           │  │              │  │               │
   │ 我的职责: │  │ 我的职责:    │  │ 我的职责:     │
   │ 技术需求  │  │ 采购程序     │  │ 硬性规则      │
   │ 审查      │  │ 合规审查     │  │ 合规审查      │
   │           │  │              │  │               │
   │→程序违规  │  │→程序违规     │  │→程序违规      │
   │ 不在我    │  │ 在我领域内   │  │ 在我领域内    │
   │ 领域内    │  │ → 验证!      │  │ → 验证!       │
   │ → skip    │  │              │  │               │
   └───────────┘  └──────────────┘  └───────────────┘
```

关键差异 vs v2：
- **Scout 零搜索**（不是 1 次/条款）。搜索由 Phase 2 Agent 按需执行
- **Scout 输出 `verification_required`**：明确告诉 Phase 2 Agent "需要查什么法规"
- **Phase 2 Agent 搜索次数受 Scout 引导而减少**：不再"搜索所有可能相关的法规"，而是"搜索 Scout 指出的具体法规"
- **路由保持关键词**（不变）。Agent 通过 prompt **正向判断**相关性（不是硬编码负向排除表）

---

## 四、详细设计

### 4.1 ScoutAgent（~100 行新建）

**文件**: `src/agents/scout.rs` (新建)  
**注册**: `registry.rs` 新增 `AgentId::Scout`  
**类型**: `types.rs` 新增 `FindingRole::Hypothesis`, `knowledge_source`, `verification_required`

**AgentConfig**:
```rust
AgentConfig {
    name: "ScoutAgent",
    default_max_turns: 3,  // read_section ×2 + output_finding ×1, 不搜索
    tool_names: ["read_section", "output_finding"],  // ★ 无 web_search
}
```

**System Prompt 核心**（~40 行）:
```markdown
你是标书审查的"初筛员"（Scout）。你的角色等同于研究团队中的 Lead Researcher——
你负责快速分类和规划，但**不做搜索验证**。搜索验证是下游专业 Agent 的工作。

## 你的工具（只有 2 个）
- read_section: 精读条款原文
- output_finding: 输出你的假设

## 你的任务
对每条条款做快速分类（最多 3 turns）：

1. **判断有无风险**：如无明显风险，output_finding(no_risk=true)
2. **如有风险，输出**：
   - risk_type: 风险类型标签（如"程序违规""品牌指定""排他条款""资质门槛"等）
   - legal_basis: 你**推测**可能涉及的法规名（基于训练知识，不需要条款号精确）
   - verification_required: ["法规名1", "法规名2"] — 需要 Phase 2 Agent 搜索验证的法规列表
   - knowledge_source: 固定填 "training_knowledge"（表示未搜索验证）
   - confidence: 0.4-0.5（低置信度，表示"我只是根据训练知识猜测"）
   - reason: 你为什么认为这里有风险（2-3句话即可，不需要引用法条）
   - **不要写 suggestion**（留给专业 Agent）

3. **每个 clause 只输出 1 条 finding**（选最重要的那个风险维度）

## 搜索预算
你**没有** web_search 工具。这是故意的——你的价值是快速分类和规划，
不是半吊子验证。如果你不确定某法规是否存在，仍然可以列在
verification_required 中，让 Phase 2 Agent 去验证。

## 关键原则
- 宁可漏报（Phase 2 还有 Agent 会覆盖）也不要过度猜测
- 如果条款看起来是纯信息性的（项目编号、日期、格式要求），直接 no_risk=true
- 你的 Hypothesis 会作为 Phase 2 Agent 的输入，所以只标记你**相对确定**的问题
```

### 4.2 类型系统（~25 行修改）

**文件**: `src/agents/types.rs`

```rust
// ── 新增枚举 ──

/// 发现的角色：区分初筛假设和已验证结论。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum FindingRole {
    /// Scout 产出: 待验证假设（不进最终 findings, 不参与 LegalVerify/Debate/Triage）
    Hypothesis,
    /// 专业 Agent 产出: 已验证结论
    #[default]
    Verified,
}

// ── RiskFinding 新增字段 ──

pub struct RiskFinding {
    // ... 现有字段保持不变 ...

    /// 此发现的角色
    #[serde(default)]
    pub finding_role: FindingRole,

    /// 知识来源（Scout 填 "training_knowledge"，Phase 2 填 "search_verified"）
    #[serde(default)]
    pub knowledge_source: String,

    /// Scout 标记的待验证法规列表（引导 Phase 2 Agent 搜索）
    #[serde(default)]
    pub verification_required: Vec<String>,

    /// 哪些 Agent 参与了初筛
    #[serde(default)]
    pub hypothesized_by: Vec<String>,

    /// 哪些 Agent 参与了验证
    #[serde(default)]
    pub verified_by: Vec<String>,
}

// ── AgentId 新增变体 ──

#[derive(Debug, Clone, ...)]
pub enum AgentId {
    // ... 现有变体 ...
    Scout,  // ← 新增
}
```

### 4.3 SessionGraph 改动（~35 行修改）

**文件**: `src/agents/session_graph.rs`

#### 4.3.1 `add_hypothesis()` — 轻量写入

```rust
/// 写入 Hypothesis（轻量版 add_risk_with_edges）。
///
/// 与 add_risk_with_edges 的区别:
/// - 不创建 Law 节点（Hypothesis 的法规名未验证，可能是幻觉）
/// - 不触发 same_law 推导（避免未验证信息污染图拓扑）
/// - 只写入 Risk 节点 + has_risk 边 + cites 边（单向）
pub fn add_hypothesis(&self, risk: RiskNode, chunk_id: &str) {
    let risk_id = risk.finding.risk_id.clone();
    let law_refs = risk.finding.legal_basis.clone();

    // 1. Risk 节点
    if let Ok(mut risks) = self.risks.write() {
        risks.insert(risk_id.clone(), risk);
    }

    // 2. has_risk 边
    if let Ok(mut edges) = self.has_risk.write() {
        edges.entry(chunk_id.to_string()).or_default().push(risk_id.clone());
    }

    // 3. cites 边（仅单向，不建 cited_by 反向索引，不触发 same_law）
    if let Ok(mut cites) = self.cites.write() {
        cites.entry(risk_id).or_default().extend(law_refs);
    }
}

/// 查询所有 Hypothesis（BlindSpot 用）。
pub fn get_hypotheses(&self) -> Vec<RiskFinding> {
    self.risks.read().ok()
        .map(|m| m.values()
            .filter(|r| r.finding.finding_role == FindingRole::Hypothesis)
            .map(|r| r.finding.clone())
            .collect())
        .unwrap_or_default()
}

/// Scout 阶段是否已完成。
pub fn is_scout_complete(&self) -> bool {
    self.scout_complete.load(std::sync::atomic::Ordering::Acquire)
}

pub fn mark_scout_complete(&self) {
    self.scout_complete.store(true, std::sync::atomic::Ordering::Release);
}
```

SessionGraph 新增字段:
```rust
scout_complete: AtomicBool,
```

#### 4.3.2 `ClauseContext::risk_summary()` 增强

直接修改现有的 `risk_summary()` 方法（当前只有一个调用点 `react_loop.rs:506`），追加:
- **角色标注**: Hypothesis → `[Scout 假设, 待验证]`，Verified → `[已验证]`
- **verification_required**: Scout 产出的"建议搜索验证"列表（Phase 2 Agent 的搜索起点）
- **legal_basis**: 已有法规依据（无论是推测还是已验证）

```rust
// 修改 ClauseContext::risk_summary()（原实现见 types.rs:867-881）
pub fn risk_summary(&self) -> String {
    if self.risks.is_empty() {
        return "（无）".to_string();
    }
    self.risks.iter().map(|r| {
        let role_label = if r.finding_role == FindingRole::Hypothesis {
            "[Scout 假设, 待验证]"
        } else {
            "[已验证]"
        };
        let mut line = format!("- {} {} (confidence={:.2}): {}",
            role_label, r.risk_type, r.confidence, r.reason);

        // Scout Hypothesis: 展示待验证法规作为 Phase 2 Agent 的搜索起点
        if r.finding_role == FindingRole::Hypothesis && !r.verification_required.is_empty() {
            line.push_str(&format!(
                "\n  🔍 建议搜索验证: {}",
                r.verification_required.join(", ")
            ));
        }

        // 展示法规依据（Hypothesis 是推测，Verified 是确认）
        if !r.legal_basis.is_empty() {
            line.push_str(&format!(
                "\n  法规依据: {}",
                r.legal_basis.join(", ")
            ));
        }
        line
    }).collect::<Vec<_>>().join("\n")
}
```

Phase 2 Agent 在每轮 ReAct 的 `[Session 记忆]` 中将看到：
```
已知风险:
- [Scout 假设, 待验证] 程序违规 (confidence=0.45): 招标公告发布日期距投标截止日仅15日...
  🔍 建议搜索验证: 87号令, 政府采购法
  法规依据: 《招标投标法》第24条
- [已验证] 资质门槛 (confidence=0.82): 要求投标人同时具备三种稀缺资质...
  法规依据: 《政府采购法》第22条, 《招标投标法》第18条
```

这样 §4.5 Prompt 中的指令 `Scout 的 legal_basis 和 verification_required 可作为搜索起点` 才有实际效果。

### 4.4 Coordinator 管线调整（~120 行修改）

**文件**: `src/agents/coordinator.rs`

#### 4.4.1 `review()` 主管线

```rust
pub async fn review(&self, clauses: &[ReviewClause]) -> Result<CoordinatorOutput> {
    self.preload_chunks(clauses);
    self.preload_agents();

    // [0] SCOUT — 串行 mini-batch（零搜索）
    self.scout_phase(clauses).await;
    self.graph.mark_scout_complete();

    // [1] ROUTE — 关键词路由（不改！）
    let routing = self.route_clauses(clauses);

    // [2] EXECUTE — 并行执行（Agent 从 SessionGraph 读到 Scout Hypothesis）
    let all_findings = self.execute_agents(&routing).await;

    // [3] MERGE — 文本相似度去重（跳过 Hypothesis）
    let merge_result = self.merge_findings_v3(all_findings, &emit);
    let mut merged = merge_result.retained;

    // [4] LINK — 跨 Agent 关联（不变）
    self.derive_cross_agent_links(&merged);

    // [5] LEGAL_VERIFY — 只处理 Verified
    let lv_count = if self.config.enable_legal_verify {
        self.legal_verify_only_verified(&mut merged).await
    } else { 0 };

    // [6] DEBATE — 只处理 Verified + High + confidence ≤ 0.85
    self.debate_only_verified(&mut merged).await;

    // [7] TRIAGE — 过滤 Hypothesis，排序 Verified
    let findings = self.triage_verified(merged);

    Ok(CoordinatorOutput { findings, routing_summary, graph_snapshot })
}
```

#### 4.4.2 `scout_phase()` 实现

```rust
/// Phase 0: Scout 初筛。Mini-batch 并行（3 clauses/批），零搜索。
async fn scout_phase(&self, clauses: &[ReviewClause]) {
    let scout_def = match self.registry.get(AgentId::Scout) {
        Some(d) => d,
        None => { eprintln!("  [SCOUT] ScoutAgent 未注册，跳过初筛"); return; }
    };

    let mut config = scout_def.to_agent_config(); // max_turns=3, tool_names=["read_section","output_finding"]
    // 确保没有 web_search 工具（Scout 不应该搜索）
    config.tool_names.retain(|t| t != "web_search");

    const BATCH_SIZE: usize = 3;
    let total = clauses.len();

    for (batch_idx, batch) in clauses.chunks(BATCH_SIZE).enumerate() {
        let mut handles = vec![];
        for clause in batch {
            let llm = (self.llm_factory)();
            let mut tools = (self.tools_factory)();
            tools.retain_only(&["read_section", "output_finding"]);

            let risk_id = self.graph.next_risk_id();
            let clause = clause.clone();
            let graph = self.graph.clone();
            let print_lock = self.print_lock.clone();
            let config = config.clone();

            handles.push(tokio::spawn(async move {
                let agent = ReActLoop::new(config, llm, tools)
                    .with_print_lock(print_lock);
                // NOTE: Scout 不需要 .with_graph() — SessionGraph 此时只有 Chunk 节点
                let mut finding = agent.review_single(&clause, &risk_id).await;

                if !finding.no_risk {
                    finding.finding_role = FindingRole::Hypothesis;
                    finding.knowledge_source = "training_knowledge".into();
                    finding.hypothesized_by = vec!["ScoutAgent".into()];
                    graph.add_hypothesis(
                        RiskNode { finding: finding.clone(), law_refs: finding.legal_basis.clone() },
                        &clause.chunk_id,
                    );
                }
                graph.add_reviewed_by(&clause.chunk_id, AgentId::Scout);
                eprintln!("  [SCOUT] {}: risk_type={}, no_risk={}",
                    clause.chunk_id,
                    finding.risk_type,
                    finding.no_risk,
                );
                finding
            }));
        }

        // 等待本批完成再启动下一批
        for h in handles { let _ = h.await; }
        eprintln!("  [SCOUT] 批次 {}/{} 完成", batch_idx + 1,
            (total + BATCH_SIZE - 1) / BATCH_SIZE);
    }
    eprintln!("  [SCOUT] 全部完成: {} clauses 已初筛", total);
}
```

#### 4.4.3 `merge_findings_v3()` — 文本相似度去重

```rust
/// MERGE v3: Hypothesis 不进 merge。
/// Verified 之间用 reason 文本的 Jaccard 相似度去重（阈值 0.7）。
///
/// **关键约束**: 不同 clause 的发现绝不合并——即使 reason 高度相似，
/// 也必须保留为独立发现（用户需要知道每个问题出现在哪个条款中）。
fn merge_findings_v3(&self, findings: Vec<RiskFinding>, emit: &dyn Fn(&ReviewEvent)) -> MergeResult {
    let mut retained: Vec<RiskFinding> = Vec::new();

    for mut f in findings {
        // Hypothesis 不进最终输出
        if f.finding_role == FindingRole::Hypothesis {
            continue;
        }

        // 与已保留的 finding 比较文本相似度
        let mut merged = false;
        for existing in retained.iter_mut() {
            // ★ 不同 clause 的发现绝不合并
            let same_clauses = existing.clause_ids.iter().any(|c| f.clause_ids.contains(c));
            if !same_clauses {
                continue;  // 不同条款 → 保留为独立发现
            }

            let sim = text_similarity(&f.reason, &existing.reason);
            if sim >= 0.70 {
                // 合并 contributors
                merge_contributors(existing, &f);
                // 合并 clause_ids（可能 f 有额外的关联 clause）
                for cid in &f.clause_ids {
                    if !existing.clause_ids.contains(cid) {
                        existing.clause_ids.push(cid.clone());
                    }
                }
                if f.confidence > existing.confidence {
                    existing.reason = combine_reasons(existing, &f);
                    existing.suggestion = f.suggestion.clone();
                    existing.legal_basis = dedup_legal_basis(&existing.legal_basis, &f.legal_basis);
                }
                existing.confidence = existing.confidence.max(f.confidence);
                emit(&ReviewEvent::FindingRemoved {
                    risk_id: f.risk_id.clone(),
                    reason: format!("文本相似度合并 (sim={:.2})", sim),
                    merged_into: Some(existing.risk_id.clone()),
                });
                merged = true;
                break;
            }
        }

        if !merged {
            retained.push(f);
        }
    }

    MergeResult { retained }
}

/// 计算两个 reason 文本的 Jaccard 相似度（基于字符 trigram）。
fn text_similarity(a: &str, b: &str) -> f64 {
    fn trigrams(s: &str) -> std::collections::HashSet<[char; 3]> {
        let cleaned: Vec<char> = s.chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        cleaned.windows(3)
            .filter_map(|w| <[char; 3]>::try_from(w).ok())
            .collect()
    }
    let ta = trigrams(a);
    let tb = trigrams(b);
    if ta.is_empty() || tb.is_empty() { return 0.0; }
    let intersection = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    intersection as f64 / union as f64
}

/// 合并 contributors：追加 hypothesized_by 和 verified_by，去重。
fn merge_contributors(existing: &mut RiskFinding, new: &RiskFinding) {
    for h in &new.hypothesized_by {
        if !existing.hypothesized_by.contains(h) {
            existing.hypothesized_by.push(h.clone());
        }
    }
    for v in &new.verified_by {
        if !existing.verified_by.contains(v) {
            existing.verified_by.push(v.clone());
        }
    }
}

/// 合并 reason 文本：existing 在前，new 在后（截断 800 字符防止膨胀）。
fn combine_reasons(existing: &RiskFinding, new: &RiskFinding) -> String {
    let existing_reason = existing.reason.trim();
    let new_reason = new.reason.trim();
    let combined = format!("{}\n\n[补充验证 — {}]: {}", existing_reason, new.agent, new_reason);
    // 截断到 800 字符
    if combined.chars().count() > 800 {
        format!("{}…", combined.chars().take(797).collect::<String>())
    } else {
        combined
    }
}

/// 合并 legal_basis 列表，去重。
fn dedup_legal_basis(a: &[String], b: &[String]) -> Vec<String> {
    let mut result: Vec<String> = a.to_vec();
    for item in b {
        if !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}
```

### 4.5 Agent Prompt 增强（~15 行修改，不含冗余映射）

**文件**: `src/agents/prompts.rs`

仅在 7 个 Reviewer Agent 的 system prompt 末尾追加**通用指令**（约 12 行）：

```markdown
## ⚠️ Session 记忆与 Scout 初筛结果

审查开始前，Scout 已完成对全部条款的初筛。每轮 [Session 记忆] 中可能包含
Scout Hypothesis（标注为 "[Scout 假设, 待验证]"）。

### 处理规则
1. 阅读 Hypothesis，**自行判断**它是否在你的审查职责范围内。
   你的职责: {各Agent已有的职责描述，无需修改}
2. 如果在职责范围内 → **验证并深化**。必须做至少 1 次独立搜索/细读验证。
   Scout 的 legal_basis 和 verification_required 可作为搜索起点。
3. 如果不在职责范围内 → output_finding(no_risk=true,
   reason="Scout Hypothesis [risk_type] 不在本 Agent 审查范围内")
4. **禁止**不经独立验证就引用 Scout 的结论作为最终输出。
```

**关键**：不再有 "以下风险类型不在你的领域内" 的硬编码映射表。每个 Agent 只知道自己负责什么（已有职责描述），LLM 自行判断 Hypothesis 是否相关。

### 4.6 工具注册表补充（~12 行修改）

**文件**: `src/agents/tools/mod.rs`

```rust
impl ToolRegistry {
    // ... 现有方法 ...

    /// 只保留指定名称的工具，删除其余。
    /// 用于 Scout 等精简工具集的 Agent。
    pub fn retain_only(&mut self, names: &[&str]) {
        self.tools.retain(|t| names.contains(&t.name()));
    }
}
```

---

## 五、改动清单（最终版）

| # | 文件 | 改动 | 行数 | 说明 |
|---|------|------|------|------|
| 1 | `src/agents/scout.rs` | **新建** | ~100 | ScoutAgent: system_prompt + max_turns=3 + 零搜索 |
| 2 | `src/agents/types.rs` | 修改 | +25 | `FindingRole` 枚举 + `knowledge_source` + `verification_required` + `hypothesized_by`/`verified_by` + `AgentId::Scout` |
| 3 | `src/agents/session_graph.rs` | 修改 | +40 | `add_hypothesis()` + `get_hypotheses()` + `scout_complete` + 增强 `risk_summary()`（角色标注+verification_required+legal_basis） |
| 4 | `src/agents/coordinator.rs` | 修改 | +140/-15 | `scout_phase()` + `merge_findings_v3()`(文本相似度去重+clause_ids保护) + LEGAL_VERIFY/DEBATE/TRIAGE/BlindSpot 适配 + `AgentId::Scout` match 臂 |
| 5 | `src/agents/registry.rs` | 修改 | +15 | 注册 `AgentId::Scout` |
| 6 | `src/agents/prompts.rs` | 修改 | +15 | 7 个 Agent 追加通用 Hypothesis 处理规则（无硬编码映射） |
| 7 | `src/agents/tools/mod.rs` | 修改 | +12 | `ToolRegistry::retain_only()` |

**总计：约 342 行新代码，15 行删除。不引入 SharedSearchCache、不引入 risk_type_to_agent()。**

---

## 六、v1→v2→v3 演进对照

| 问题 | v1 | v2 | v3 |
|------|-----|-----|-----|
| 搜索缓存 | 🔴 SharedSearchCache | ✅ 删除 | ✅ 删除 |
| Scout 搜索 | 零搜索 | 🟡 1次/clause | ✅ **零搜索**（Anthropic 模式） |
| Scout 产出 | Hypothesis | Hypothesis + 搜索法规 | Hypothesis + `verification_required` 引导 |
| 路由 | 🔴 risk_type_to_agent() | 🟡 关键词路由 + prompt 映射 | ✅ 关键词路由 + LLM 自行判断 |
| MERGE 去重 | risk_type+clause | risk_type+clause | ✅ **reason 文本相似度 ≥ 0.7** |
| Agent 负向排除表 | 无 | 🔴 "不在你领域内" 硬编码 | ✅ 删除（正向职责 + 自行判断） |
| Agent 偷懒 | 允许直接引用 | 强制 1 次独立验证 | ✅ 保持不变 |
| LEGAL_VERIFY/DEBATE/TRIAGE | 未适配 | ✅ 已适配 | ✅ 已适配 |
| merge 函数未定义 | — | 🟡 | ✅ 已补充伪代码 |
| Prompt 膨胀 | 40行/Agent | 40行/Agent | ✅ **~12行/Agent** |
| API 缺口 | — | 🟡 tools.filter_to() | ✅ tools.retain_only() |

---

## 七、风险 & 缓解

| 风险 | 缓解 |
|------|------|
| Scout 零搜索 → 法规名幻觉 | `add_hypothesis()` 不创建 Law 节点。Phase 2 Agent 独立搜索验证（Prompt 强制）。Scout 仍然可以列出 `verification_required`——Phase 2 Agent 会去验证 |
| Scout 遗漏关键风险 | 路由仍用关键词（不改）。即使 Scout 对某 clause 无 Hypothesis，该 clause 仍被路由到匹配的 Agent |
| LLM 对 Hypothesis 相关性判断不准 | 7 个 Agent 的职责描述已经存在且清晰。如果判断失误导致 Agent 全部跳过 → 路由兜底（FactCheck 至少审每条 clause） |
| 文本相似度去重误合并 | 阈值 0.7（Jaccard char trigram）。两个发现如果 reason 文本 >70% 重叠，它们大概率是同一发现。误合并概率低，且 confidence 取 max + reason 拼接保留信息 |
| Scout 增加延迟 | Scout 零搜索（每 clause 约 2×read + 1×output ≈ 4-6s），12 clauses × 3 并发 ≈ 24-36s。Phase 2 节省的搜索时间远超此开销 |

---

## 八、验证方案

### 单元测试
- `scout.rs`: `test_scout_produces_hypothesis_role()`, `test_scout_no_web_search_tool()`, `test_scout_outputs_verification_required()`
- `session_graph.rs`: `test_add_hypothesis_no_law_node()`, `test_get_hypotheses_filters_verified()`
- `coordinator.rs`: `test_text_similarity_identical()`, `test_text_similarity_different()`, `test_merge_v3_skips_hypothesis()`, `test_merge_v3_dedup_by_similarity()`

### 集成测试（A/B 对比）
用 2 页测试文件跑完整管线，通过 TraceLog + Metrics 记录：
- **LLM 总调用次数**：目标减少 25-40%
- **搜索次数**：对比 Scout 引入前后（Scout 0 次 + Phase 2 减少的搜索 vs 旧版 3 Agent 各搜索）
- **同一 (risk_type, clause) 的 findings 数量**：目标从 3+ → 1
- **总耗时**：目标持平或减少

### 手工验证
- `cargo test` 全部通过
- `cargo check` 无 warning
- `cargo clippy -- -D warnings` 通过
