# Rust 多 Agent 标书审核系统 — 性能瓶颈分析

## 核心问题：LLM API 调用次数爆炸

以一个典型标书文档（约 100 条条款）为例，Coordinator 模式下的 LLM 调用估算：

| 阶段 | 计算 | 估算调用次数 |
|---|---|---|
| EXECUTE | 7 Agent × 15~40 条条款 × 5~12 轮 ReAct | 2000 ~ 4000 |
| LEGAL_VERIFY | 有法条引用的 finding × 6 轮 | 100 ~ 500 |
| BLINDSPOT | 50 条候选条款 × 10 轮 + Sweep 2 轮 | 200 ~ 600 |
| DEBATE | High + 低置信度 finding × 8 轮 | 20 ~ 100 |

**总计：约 2500 ~ 5000 次 LLM API 调用，每次 2~10 秒。**

---

## 问题清单（按时间浪费从大到小排列）

### 1. BlindSpot 盲点扫描阻塞输出 — 最大浪费（~36 分钟，占管线 25~30%）

**位置**：`coordinator.rs:1031-1246` `blind_spot_scan()`

**当前行为**：EXECUTE 完成后，BlindSpotAgent 启动完整 ReAct 循环，扫描所有未被充分审查的条款（上限 50 条 × 10 turns），之后还有一个 Sweep 二次扫描（2 turns）。全部完成后才输出结果。

**为什么是问题**：
- BlindSpot 定位是"兜底"，发现的大多是 Info/Low 级别的"可能遗漏"，不需要阻塞用户看到主要结果
- ReAct 产出率极低——候选 50 条条款通常只有 0~3 条新发现
- Sweep 二次扫描是"对兜底的兜底"——用 LLM 确认"LLM 确实没发现问题"
- BlindSpot 虽然有纯规则 fallback（不调 LLM），但 fallback 只在 ReAct 失败时触发，而不是主动选择

**管线串行加剧问题**（`coordinator.rs:170-320`）：7 个阶段严格顺序执行，LEGAL_VERIFY / BLINDSPOT / DEBATE 都必须等前一阶段完全结束，总耗时 = 各阶段之和，无法重叠。

**建议**：
```
当前: EXECUTE → MERGE → LEGAL_VERIFY → BLINDSPOT → DEBATE → TRIAGE → 输出
                          ↑ 用户等 6-14 分钟后才看到结果

改为: EXECUTE → MERGE → TRIAGE → 🟢 立即输出主结果
                          ↓ 后台异步
                       LEGAL_VERIFY → BLINDSPOT → 补充推送 SSE FindingAdded
```

**✅ 已解决**（2026-07）：

解决方案分两步：

1. **BlindSpot 移出主流程** — `coordinator.rs:152` 注释明确"BlindSpot 不在此主流程中"。`review()` 管线从原来的 7 阶段（含 BLINDSPOT）缩减为 6 阶段（Route → Preload → Execute → Merge → LegalVerify → Debate → Triage），用户不再需要等 BlindSpot 完成才能看到审核结果。

2. **BlindSpot 改为后台异步经验沉淀** — 新增独立的 `run_blind_spot()` 方法（`coordinator.rs:429`），在 `review()` 返回之后调用，不阻塞主结果输出。BlindSpot 的发现**不再追加到本次审核结果**，而是提取 `suggested_agent` 建议写入 `dynamic_agents.json`，为**下一次**审核积累经验（自动生成新的检测维度）。这一定位调整使 BlindSpot 从"阻塞输出的兜底扫描"转变为"不阻塞的增量学习"。

---

### 2. LegalVerify 每条 finding 独立跑 ReAct — 设计冗余（~25 分钟，占管线 15~20%）

**位置**：`coordinator.rs:718-804` `legal_verify()`

**当前行为**：把每条有 legal_basis 的 finding 拆成独立 ReviewClause（`chunk_id = "legal_verify_R_001"`），每条跑一个完整 ReAct（max_turns=6）。

**为什么是问题**：
- 30 条 findings with legal_basis → 30 个独立 ReAct → 180 次 LLM 调用
- 这些验证任务之间完全独立，但共享同一个 system prompt 和工具集
- 6 轮的原因是"搜索可能返回垃圾结果需绕路"，但大多数验证第 2-3 轮就能完成
- LegalVerify 本质上做的是同一件事：验证法条是否真实存在且适用。一次 LLM 调用可以批量验证多条法条引用——当前的"一条一个 ReAct"是原子化过度的表现

**✅ 已解决**（2026-07）：

方案：**按法律领域分组 → 批量 + 分层判断**

```
MERGE 之后的 LegalVerify:

┌─ Step A: LegalDomain 自动分类（纯规则，<1ms）──────────────┐
│  对所有有 legal_basis 的 finding 做关键词匹配                │
│  → 打上 LegalDomain 标签                                    │
│  8 个领域: ProcurementProcedure, SupplierQualification,     │
│  GeographicRestriction, BrandDesignation, ScoringEvaluation,│
│  ContractTerms, BidBondTimeline, TechnicalRequirements      │
└────────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
┌─ procurement ─┐ ┌─ geographic ─┐ ┌─ scoring ─┐
│ procedure (8条)│ │ restriction  │ │ evaluation │
│                │ │ (5条)        │ │ (6条)      │
│ ┌────────────┐ │ │ ┌──────────┐ │ │ ┌────────┐ │
│ │ 规则预筛    │ │ │ │ 规则预筛  │ │ │ │规则预筛 │ │
│ │ known_laws  │ │ │ │ known_laws│ │ │ │known_   │ │
│ │ 条款号范围  │ │ │ │ 条款号范围│ │ │ │laws     │ │
│ │ ✅ 7条通过 │ │ │ │ ✅ 4条   │ │ │ │ ✅ 3条  │ │
│ └────────────┘ │ │ └──────────┘ │ │ └────────┘ │
│                │ │               │ │             │
│ 只有 1 条模糊  │ │ 只有 1 条模糊  │ │ 3 条模糊   │
│ → 批量 LLM     │ │ → 批量 LLM    │ │ → 批量 LLM │
│ (1 次 ReAct,   │ │ (1 次 ReAct,  │ │ (1 次, 4轮) │
│  4 轮)         │ │  4 轮)        │ │             │
└────────────────┘ └───────────────┘ └─────────────┘
         ↑ 规则直通 ~70% 的 finding 零 LLM 调用
```

**三层结构**：

| 层 | 机制 | 触发条件 | LLM 调用 |
|---|---|---|---|
| **规则预筛** | 已知法规库匹配（法规名 + 条款号合法性范围） | 法规在 `known_laws` 中 + 条款号合法 + confidence ≥ 0.7 | 0 |
| **批量 LLM** | 按 LegalDomain 分组，一组一条 prompt 含全部待验证条目 | 规则无法判断（模糊引用、非标准法规） | 每组 1 个 ReAct (4 轮) |
| **逐条 fallback** | 保留旧版逐条模式 | Other 领域（无法归类的法条） | 每条 1 个 ReAct (3 轮) |

**新增工具**：`output_verification_batch` — 批量验证的终端工具，一次性输出该组所有验证结论（替代逐条的 `output_finding`）。

**新增类型**：
- `LegalDomain` 枚举（8 个领域 + Other）— `types.rs:454-520`
- `BatchVerificationEntry` / `BatchVerificationOutput` — `types.rs:524-541`
- `OutputVerificationBatchTool` — `tools/output_verification_batch.rs`
- `LlmResponse::has_output_verification_batch()` / `get_verification_batch()` — `react_loop.rs:97-110`
- `Coordinator::rule_based_law_check()` — `coordinator.rs` 规则预筛函数
- `Coordinator::format_batch_legal_verify_task()` — `coordinator.rs` 批量 prompt 构建

**EXP-001 vs EXP-002 实测对比**（2页测试文件）：

| 指标 | EXP-001 (逐条) | EXP-002 (分组批量+分层) | 变化 |
|---|---|---|---|
| 总耗时 | 197.9s | 87.8s | -55.6% |
| LegalVerify 耗时 | ~20s (ReAct 6轮) | <1s (规则直通) | -95% |
| LegalVerify LLM 调用 | 1条 → 1 ReAct | 1条 → 规则直通 0 LLM | -100% |
| 审核结果一致性 | 1条 Medium | 1条 Medium | ✅ 一致 |

**预估 30 条 legal_basis 的效果**：

| 指标 | 旧版 | 新版 |
|---|---|---|
| 规则直通 (~70%) | 21 个独立 ReAct (126 LLM调用) | 0 次 LLM |
| 批量 LLM (~30%) | 9 个独立 ReAct (54 LLM调用) | 3-4 个分组 ReAct (12-16 LLM调用) |
| LegalVerify 总计 | ~180 次 LLM 调用 | ~12-16 次 LLM 调用，**节省 ~90%** |

**参考社区方案**：
- EMNLP 2025 "Batched Self-Consistency" — 批量评估精度**高于**逐条（+4.5 NDCG@10），因为同领域共享法律上下文
- Harvey (2025) — 三层流水线（规则 → embedding → LLM）实现 >95% 法条引用准确率
- 关键设计原则：规则预筛处理 70% 的确定性案例，LLM 只处理模糊边界

---


### 3. Agent max_turns 设置过高（多 20~30% LLM 调用）

**位置**：`types.rs:37-44` `RiskTier::max_turns()` + `registry.rs` 各 Agent 的 `default_max_turns`

**当前 vs 建议**：

| Agent | prompt 自述 | 当前 max_turns | 建议 max_turns | 理由 |
|---|---|---|---|---|
| FactCheckAgent | "目标 3~4 轮完成" | 10 | 6 | prompt 自己说 3-4 轮 |
| RuleEngineAgent | 硬规则匹配 | 14 | 8 | 规则匹配不需要多轮搜索 |
| ScoringAgent | — | 10 | 6 | 评分因素审查是结构化任务 |
| ContractAgent | — | 12 | 8 | 合同条款有明确法条参照 |
| ProcedureAgent | — | 12 | 8 | 程序合规有明确期限/金额阈值 |
| DemandAgent | — | 12 | 8 | 技术参数对照不需要深度搜索 |
| SemanticRiskAgent | 深度审查 | 14 | 10 | 隐性风险确实需要深度搜索 |

**条款级 RiskTier 也过高**：

| Tier | 当前 max_turns | 建议 |
|---|---|---|
| L1 (格式条款) | 8 | 3 — 格式条款 1-2 轮就够了 |
| L2 (标准审查) | 12 | 6 |
| L3 (高风险) | 14 | 9 |

**更根本的问题**：条款级 ReAct 循环是严格串行的（`react_loop.rs:350`），第 2 轮必须等第 1 轮的 LLM 响应 + 工具执行完成。一条条款 10 轮 × (3-5s LLM + 2-3s 工具) = 50-80 秒。

虽然 `review_clauses_parallel()` 用了 Semaphore（`react_loop.rs:1388`），但默认 `max_parallel_clauses=5`（`types.rs:618`），一个 Agent 有 30 条条款就需要 6 个串行批次。

**建议**：降低 max_turns + 提高 `max_parallel_clauses` 到 15~20（LLM 调用是 IO-bound，不需要严格限制并发）。

---

### 4. 多 Agent 重复审查同一条款（~30~40% LLM 调用浪费）

**位置**：`coordinator.rs:362-426` `route_clauses()`

**当前行为**：路由是一对多的，一条条款匹配多个 Agent 的关键词就分配给多个 Agent。例如含"技术"和"评分"的条款，可能被 Demand + Scoring + FactCheck 三个 Agent 各审一遍。

**为什么是问题**：
- 约 30~40% 的 Agent-条款对最终输出 `no_risk=true`——纯浪费
- 很多条款是纯信息性的（如"项目编号：XXX-2024-001"），不需要 SemanticRisk + Demand + Contract + Scoring 全部审一遍
- 关键词路由粒度太粗。比如含"付款"就路由到 ContractAgent，但"付款方式见合同附件"不需要深度审查

**建议**：
- 每条 clause 最多分配给 2 个 Agent（选关键词匹配度最高的）
- 或引入快速预判：ROUTE 阶段加一个轻量级 LLM 判断（单次调用，非 ReAct）："这条条款是否需要 [AgentName] 审查？"
- 或使用向量相似度做更精细的路由：条款 embedding vs Agent description embedding，只路由到相似度 > 阈值的 Agent

---

### 5. 搜索工具调用慢且被滥用

**位置**：`react_loop.rs:1005-1075` + `tools/search_knowledge.rs`

**为什么是问题**：
- LLM 经常在同一轮中多次调用 `search_knowledge`，每次走 DashScope 联网搜索或 SearXNG，耗时 2~5 秒
- 虽然有 5 次硬上限，但 5 次 × 2~5 秒 = 额外 10~25 秒/条款
- 多个 Agent 同时搜索触发下游引擎限流（SearchBuffer 串行化请求，但让等待时间更长）
- 多个 Agent 搜索同一法规时各自独立调用，没有跨 Agent 结果共享

**建议**：
- 搜索结果跨 Agent 共享缓存（SearchBuffer 目前只串行化，应增加跨 Agent 缓存层）
- 降低搜索硬上限从 5 次到 3 次
- 考虑预设常用法规知识库，减少实时搜索需求

---

### 6. 无 LLM 请求缓存/批处理

**为什么是问题**：
- **无 prompt caching**：每次 LLM 调用都发送完整 system prompt + conversation history。一个 Agent 审 30 条条款 = system prompt 重复发送 150~360 次
- **无结果缓存**：两个 Agent 搜索同一个法规，各自独立调用搜索 API
- **工具内部缓存太小**：`search_cache`（`react_loop.rs:234`）只缓存搜索 query，不缓存 LLM 对类似条款的推理
- Conversation 越来越长，每次 LLM API 请求 body 越来越大，增加网络传输时间和首 token 延迟

**建议**：
- 如果 DashScope 支持 prompt caching，缓存 system prompt 部分
- 跨 Agent 共享搜索缓存
- 对相似条款的推理结果做简单缓存（如同一条款文本 hash → 之前的结果）

---

### 7. Debate 高风险辩论 — 成本高、收益低（~1~3 分钟）

**位置**：`coordinator.rs:841-943` `debate_high_risk()`

**当前行为**：对 High + confidence ≤ 0.85 的发现，启动 DebateAgent 做 Defender → Challenger → Arbiter 三角色辩论，每个候选发现独立跑一个 ReAct（max_turns=8）。

**为什么是问题**：
- 三角色辩论在同一个 LLM 调用中模拟——LLM 自己扮演三个角色对话，不是真正的多模型对抗。学术界已证明单模型自我辩论效果有限
- 5 个 High + low-confidence 发现 = 5 × 8 = 40 次额外 LLM 调用
- 辩论结果通常只是微调 severity 和 confidence，很少推翻原始发现
- 真正需要辩论的场景（两个 Agent 对同一问题有冲突判断）在 MERGE 去重阶段已用"保留高 confidence"处理

**建议**：
- 降级为可选功能（环境变量 `AIBID_DEBATE=1` 才启用），默认关闭
- 或改为简单规则：High + confidence < 0.7 → 降级为 Medium，不跑 LLM 辩论

---

### 8. SessionGraph 每 turn 重复查询（~10 秒纯锁开销）

**位置**：`react_loop.rs:402-460` `query_clause_context()`

**当前行为**：每 turn 开始时调用 `graph.query_clause_context(&clause.chunk_id)`，查询已知风险、已审查 Agent、关联条款、矛盾边。

**为什么是低效**：
- 同一 Agent 审同一 clause 的多个 turn 之间，context 几乎不变（其他 Agent 恰好在同一两秒内写入新数据的概率极低）
- 涉及 RwLock 获取 + HashMap 多次查找 + 字符串拼接
- 150 clauses × 7 agents × 5 turns × 2ms ≈ 10 秒纯锁开销

**建议**：缓存 clause context。同一个 (agent, clause) 对的 context 在审查期间基本不变，仅在 AgentBus 收到新消息时 invalidate。

---

### 9. AgentBus 双重轮询（Step 0b + Step 2.5）— 影响小但体现过度防御

**位置**：`react_loop.rs:462-497`（Step 0b）+ `react_loop.rs:622-659`（Step 2.5）

**当前行为**：每个 turn 做两次 AgentBus poll——LLM 调用前（Step 0b）+ LLM 返回后、output_finding 前（Step 2.5）。

**为什么是问题**：
- Step 2.5 的注释说"如果其他 Agent 的广播恰好在 LLM 调用期间到达，Step 0b 会错过"——概率极低。LLM 调用期间其他 Agent 也在等自己的 LLM 响应，大家同步等待
- 即使错过了，下一条条款的 Step 0b 会收到
- 每次 poll 涉及 `Mutex<broadcast::Receiver>` 锁 + 遍历所有排队消息

**建议**：去掉 Step 2.5，单次 poll 足够。担心时效性可用 `tokio::sync::Notify` 而非双重 polling。

---

## 优化优先级总览

| 优先级 | 改动 | 预期提速 | 改动范围 |
|---|---|---|---|
| **P0** | 降低 max_turns（Agent + RiskTier 都降） | 减少 20-30% LLM 调用 | `types.rs` + `registry.rs` |
| **P0** | 提高 max_parallel_clauses 5→15 | 2-3x | `types.rs` 默认值 |
| **P1** | BlindSpot + LegalVerify 改为后台异步 | 用户感知提速 25-30% | `coordinator.rs` |
| **P1** | 限制每个 clause 最多 2 个 Agent | 减少 30-40% 调用 | `coordinator.rs` `route_clauses()` |
| **P1** | LegalVerify 改为批量模式 | 减少 80% LegalVerify 调用 | `coordinator.rs` `legal_verify()` |
| **P1** | L1 条款快速通道（纯规则，不走 ReAct） | 减少 15-30% 调用 | 新增 |
| **P2** | 搜索结果跨 Agent 共享缓存 | 减少搜索耗时 30-50% | `search_knowledge.rs` |
| **P2** | Debate 默认关闭 | 减少 1-3 分钟 | `types.rs` + 环境变量 |
| **P3** | SessionGraph context 缓存 | 减少 ~10 秒锁开销 | `session_graph.rs` |
| **P3** | 去掉 AgentBus Step 2.5 双重 poll | 微小 | `react_loop.rs` |
| **P3** | LLM prompt caching（需 DashScope 支持） | 减少每次调用 token 成本 | `llm_client.rs` |

---

## V2 问题清单（基于 Run 实验实测数据分析）

> 数据来源：`output/runs/` 下 4 次实验的 RunMetrics JSON（`20260703T163540`, `20260703T224011`, `20260704T223709`, `20260704T225721`）。
> 测试文档：研究生院智慧校园项目招标测试文件（2页，12 chunks，1395 字符）。配置：7 Agent / Coordinator / qwen-plus / dashscope 搜索。

---

### V2-1：LLM 浪费率实测 73-75%（V1 问题 #3 的实证量化）

**数据**（最新 Run `20260704T225721`）：

| Agent | LLM 调用 | 产出 Finding | 浪费调用 | 浪费率 |
|---|---|---|---|---|
| DemandAgent | 27 | 7 | 20 | **74%** |
| ProcedureAgent | 11 | 2 | 9 | **82%** |
| RuleEngineAgent | 11 | 2 | 9 | **82%** |
| FactCheckAgent | 8 | 4 | 4 | **50%** |
| **合计** | **55** | **15** | **40** | **73%** |

浪费成本：¥0.27 总成本中 ¥0.20 是浪费的（73%）。

**根因**：每个 Agent 的 Turn 1 必定是 `read_section`（读原文，不产出 finding），Turn 2-3 必定是 `web_search`（搜法规，不产出 finding），直到 Turn 3-7 才产出 `output_finding`。前 2-3 轮是固定的"热身消耗"。

**关联 V1 问题**：#3（max_turns 过高）、#4（多 Agent 重复审查）、#5（搜索被滥用）。

**建议**：
- 将 `read_section` 从 ReAct 循环中剥离——条款原文在 Agent 启动前预注入 context，省掉每条款的首轮 LLM 调用
- 降低 max_turns（V1 #3 的建议值仍然有效）
- 引入早停：连续 2 轮 `web_search` 无新信息 → 强制 `output_finding`

---

### V2-2：Coordinator LegalVerify + Debate 阶段实际零产出，但耗时占比 73%

**数据**（最新 Run `20260704T225721`）：

```
AgentReview 总耗时: 308.8s
├── Scout:         26.7s  (8.6%)
├── Route+Preload:  0.0s  (0%)
├── Execute:       53.5s  (17.3%)
├── Merge+Link:     0.0s  (0%)
├── LegalVerify:  111.6s  (36.1%)  ← 零产出
├── Debate:       116.9s  (37.9%)  ← 零产出
└── Triage:         0.0s  (0%)
```

但 Coordinator 质量统计全部为零：

```json
"coordinator": {
  "debate_triggered": 0,
  "debate_changed_verdict": 0,
  "blindspot_extra_findings": 0,
  "cross_agent_links": 0,
  "legal_verify_count": 0
}
```

**关键发现**：
- LegalVerify 耗时 111.6s 但 `legal_verify_count=0`——说明 LegalVerify 阶段在空转（可能走了 ReAct 循环但没有有效的法条需要验证）
- Debate 耗时 116.9s 但 `debate_triggered=0`——同样在空转
- 两个阶段合计 **228.5 秒（3.8 分钟）完全浪费**

**与 V1 "已解决"标记的矛盾**：V1 问题 #1（BlindSpot）和 #2（LegalVerify）标记为"✅ 已解决（2026-07）"，但实测数据表明：
- LegalVerify 的"规则预筛 + 批量 LLM"方案虽然实现了代码，但在 2 页测试文档上规则直通了所有 finding（legal_verify_count=0），而 LegalVerify 阶段本身仍然被调用并消耗了 111.6s——可能是空 ReAct 循环或等待超时
- Debate 同样——没有 finding 满足辩论条件，但仍然跑了 116.9s

**建议**：
- LegalVerify 和 Debate 阶段加 **前置快速判断**：如果没有任何 finding 需要验证/辩论，直接跳过（<1ms），不启动 ReAct
- 或者按 V1 方案：这两个阶段完全移出主流程，改为后台异步（"已解决"方案未完全落地）

---

### V2-3：Agent 负载严重不均衡 —— DemandAgent 垄断 49% 资源

**数据**（4 次实验一致）：

| Agent | LLM 调用占比 | Token 占比 | Finding 数 | 单个 Finding 成本 |
|---|---|---|---|---|
| **DemandAgent** | **49%** | **51%** | 7 (47%) | 21.8K tokens/finding |
| RuleEngineAgent | 20% | 18% | 2 (13%) | 27.2K tokens/finding |
| ProcedureAgent | 20% | 21% | 2 (13%) | 31.3K tokens/finding |
| FactCheckAgent | 15% | 10% | 4 (27%) | **7.8K tokens/finding** |
| 其余 3 Agent | 0% | 0% | 0 | N/A |

**根因**：
- 配置 7 个 Agent，但条款路由只匹配到 4 个——其余 3 个 Agent（SemanticRiskAgent / LegalAgent / ComplianceAgent?）从未被分配工作
- DemandAgent 的审查范围（技术需求倾向性/排他性）天然覆盖大部分条款，且它的 ReAct 循环最长（27 次调用）
- FactCheckAgent 效率最高（7.8K tokens/finding），但分配到的条款少

**建议**：
- 检查 ROUTE 阶段的关键词匹配逻辑——为什么 3 个 Agent 从未被分配任务
- 为 DemandAgent 设置 `max_clauses` 上限，超出部分降级给其他 Agent
- 或者将 DemandAgent 的审查职责拆分给多个 Agent（如将"平台指定"和"品牌指定"拆为独立 Agent）

---

### V2-4：Agent 间无共享知识缓存 —— 同一法规被 3 个 Agent 各自搜索

**实测证据**（`call_log` 中提取）：

同一实验中对 "87号令第二十条关于实质性条款/★号废标" 的独立搜索：

| 时间线 | Agent | 搜索内容 |
|---|---|---|
| Turn 2 | ProcedureAgent | "87号令第二十条关于实质性条款和否决投标的规定原文是什么？" |
| Turn 2 | RuleEngineAgent | "87号令第二十条关于否决投标的规定，是否禁止将非实质性技术参数设为'一项不符合即投标无效'？" |
| Turn 2 | DemandAgent | "87号令第二十条关于实质性条款和否决投标的关系是如何规定的？" |
| Turn 3 | ProcedureAgent | "实施条例第二十条关于'以不合理的条件对供应商实行差别待遇'的具体情形有哪些？" |
| Turn 3 | RuleEngineAgent | "实施条例第二十条关于'不得以不合理的条件对供应商实行差别待遇或者歧视待遇'..." |
| Turn 3 | DemandAgent | "实施条例第二十条关于'以不合理的条件对供应商实行差别待遇或者歧视待遇'如何适用？" |

3 个 Agent 对同一法规做了 **10+ 次独立搜索**，每次 1.5-2.5s。如果有跨 Agent 搜索结果缓存，可以省掉 6-8 次搜索。

**同样的问题**：同一条款 ch_005 被 ProcedureAgent、RuleEngineAgent、DemandAgent 各 `read_section` 一次。

**建议**：
- `SessionGraph` 增加搜索结果缓存层（key = 搜索 query 的语义 hash，跨 Agent 共享）
- `read_section` 结果缓存在 SessionGraph 中，后续 Agent 直接从 graph 读取
- 这对应 V1 问题 #5 和 #6，但实测数据提供了具体的浪费量化

---

### V2-5：2 页文档消耗 55 次 LLM 调用、5 分钟 —— 规模化不可行

**效率比**：

| 指标 | 数值 |
|---|---|
| 源文本 | 1,395 字符 / 12 chunks / 2 页 |
| LLM 调用 | 55 次 |
| Input tokens | 301,107 |
| Output tokens | 12,809 |
| 总耗时 | 311.4 秒（5.2 分钟） |
| **Input tokens / 源字符** | **216 : 1** |

每个源文字符平均被发送给 LLM **216 次**（嵌入在 system prompt + conversation history 中重复发送）。

**外推到典型生产文档**（100 页 / ~600 chunks）：
- LLM 调用：~2,750 次
- 耗时：~4.3 小时
- 成本：~¥13.50
- 这在生产环境中完全不可接受

**建议**：这是 V1 全部 9 个优化项的综合效果指标。优先落地 P0 项（降低 max_turns + 提高并发），预期将效率比从 216:1 降到 ~50:1。

---

### V2-6：4 次实验配置完全相同 —— 缺乏 A/B 对比实验

| Run | 日期 | 耗时 | LLM 调用 | 浪费率 | 配置 |
|---|---|---|---|---|---|
| 1 | 07/03 16:35 | ~364s | ~55 | ≈73% | 7A / Coordinator / qwen-plus |
| 2 | 07/03 22:40 | 364s | 59 | 75% | 同上 |
| 3 | 07/04 22:37 | 240s | 59 | 75% | 同上 |
| 4 | 07/04 22:57 | 311s | 55 | 73% | 同上 |

4 次实验完全相同的配置，同一份 2 页 PDF，没有任何对比变量。

**应该做的 A/B 实验**：
- Coordinator ON vs OFF（对比单 Agent 直接审核的效果和成本）
- Agent 数量：7 → 4 → 2（验证 AgentDropout 思路）
- 模型切换：qwen-plus → qwen-turbo（测试便宜模型是否够用）
- max_parallel_clauses：10 → 20 → 30
- max_turns 减半实验

**建议**：建立实验记录规范——每次 Run 的 `meta.notes` 字段应记录实验目的和改动说明，而非留空。

---

### V2-7：median_confidence 始终为 0 —— 数据采集 Bug

**位置**：`backend-rust/src/metrics/collector.rs:493`

```rust
median_confidence: 0.0,  // 硬编码占位值，从未计算
```

4 次实验的 `avg_confidence` 均为 0.93，但 `median_confidence` 始终为 0.0。中位数对发现"是否存在大量低置信度 finding"很有价值，但当前未实现计算逻辑。

**修复**：在 `build_quality_report()` 中收集所有 finding 的 confidence 值，排序后取中位数。

---

### V2 问题与 V1 问题对应关系

| V2 问题 | 对应 V1 问题 | 关系 |
|---|---|---|
| V2-1 (浪费率 73%) | #3, #4, #5 | V1 的**理论估算**被实测数据**量化验证** |
| V2-2 (LegalVerify+Debate 零产出) | #1, #2, #7 | V1 标记"已解决"，但**实测证明未完全落地** |
| V2-3 (DemandAgent 垄断) | #4 | V1 建议"限制每 clause 最多 2 Agent"的**新证据** |
| V2-4 (无共享缓存) | #5, #6 | V1 理论分析，V2 提供**具体重复搜索证据** |
| V2-5 (规模化不可行) | 全部 | V1 问题的**综合量化** |
| V2-6 (无 A/B 实验) | 新增 | 流程/方法论层面 |
| V2-7 (median=0 Bug) | 新增 | 代码 Bug |

---

## 社区方案对照（2024-2025 研究 & 工程实践）

以下将我们的 9 个问题逐一映射到社区已有的解决方案，标注来源和核心思想。

---

### 对照 1：BlindSpot + 管线串行 → 流式增量输出 + 主结果优先

**对标实践**：
- **Anthropic Orchestrator-Worker 模式**（已被 Microsoft AutoGen v0.4 采纳为默认模式）：Orchestrator 分类意图后立即返回主结果，Worker 后台异步补充。减少 39% token、70% 延迟改善。
- **Ellipsis 代码审查系统**：主审查结果立即返回，补充检查（hallucination filter、cross-reference validation）在后台管道异步执行，不在关键路径上。

**核心思想**：用户不需要等所有验证完成才看到结果。主路径极简快速 → 结果先返回 → 补充验证异步进行。

**来源**：Anthropic 工程博客；AutoGen v0.4 架构文档；Ellipsis (ZenML LLMOps Database)

---

### 对照 2：LegalVerify 逐条 ReAct → Batch Judge（批量评估）

**对标论文**：Korikov et al., *"Batched Self-Consistency Improves LLM Relevance Assessment and Ranking"*, EMNLP 2025

**核心发现**：

| 方式 | 质量 | 延迟 |
|---|---|---|
| 逐条 pointwise（当前做法） | Baseline | 最慢 |
| 批量 pointwise（多条放一个 prompt） | **更好** | **~15× 更快** |
| 批量 + self-consistency (15 次调用) | **最好** (NDCG@10 51.3% vs 46.8%) | 仍远快于逐条 |

**关键洞察**：批量处理让模型能在条款间做**联合比较**，质量反而更高——因为模型能看到全局而非孤立判断。

**另外的对照**：Baseten *"The Bitter Lesson of LLM Evals"* (2025) 提出相反观点——把大 prompt 拆成多个小检查更可靠。两种路线各有适用场景：
- 同质任务（如验证法条引用）→ 批量模式更好
- 异质任务（如不同维度的审查）→ 分解为独立检查更好

**来源**：EMNLP 2025；Baseten Blog (July 2025)；RouteJudge (OpenReview 2025)

---

### 对照 3：max_turns 过高 → Utility-Guided Orchestration + Focused ReAct + ReWOO

**方案 A：Utility-Guided Orchestration（显式步进门控）**

**对标论文**：*"Utility-Guided Agent Orchestration for Efficient LLM Tool Use"*, arXiv:2603.19896 (2025)

不靠 prompt 暗示"该停了"，而是在每步显式计算四个分数：
- **预估收益**：再跑一轮能提升多少？
- **步成本**：这一轮花多少 token？
- **不确定性**：当前证据是否足够？
- **冗余度**：是否在重复之前的动作？

最高分的动作被执行。这让"何时停止"成为**可控制、可分析的策略**，而非依赖 LLM 自觉。

**方案 B：Focused ReAct（Reiterate + Early Stop）**

**对标论文**：Li et al., *"Focused ReAct"* (2024, 更新于 Dec 2025)

两个零训练的 prompt 级改进：

| 机制 | 做法 | 效果 |
|---|---|---|
| **Reiterate** | 每步开头重复原始问题 | 防止上下文漂移 |
| **Early Stop** | 检测完全相同的重复动作 → 强制终止 | Loop 率从 38% → 5% |

在 HotPotQA 上准确率从 2.0% → 12.6%，loop 频率从 38% → 5%，runtime 下降 ~34%。

**方案 C：ReWOO — 先规划、并行执行**

**对标论文**：2025 年多篇对比（Cohorte, Nutrient.io, Red Hat）

| 维度 | ReAct（当前） | ReWOO（社区推荐） |
|---|---|---|
| Token 用量 | 高（~5×） | 低 |
| 延迟 | 串行，慢 | 并行 I/O，快 |
| 适应性 | 可中途调整 | 规划固定 |
| 错误恢复 | 可自适应 | 通常硬失败 |

**2025 年最佳实践 — 混合模式**：ReWOO 主路径 + ReAct 兜底。先尝试一次性规划所有工具调用并并行执行，只有 worker 失败时才退回到 ReAct 逐轮模式。

**对我们系统的含义**：降低 max_turns 的数值只是治标。治本方案是引入 Utility-Guided 的早停机制 + Focused ReAct 的重复检测 + 对简单条款用 ReWOO 替代 ReAct。

**来源**：arXiv:2603.19896；Focused ReAct (EmergentMind)；Cohorte/Nutrient.io/Red Hat 2025 对比

---

### 对照 4：多 Agent 重复审查 → AgentDropout（动态 Agent 消除）

**对标论文**：Wang et al., *"AgentDropout: Dynamic Agent Elimination for Token-Efficient and High-Performance LLM-Based Multi-Agent Collaboration"*, ACL 2025

**核心机制**：不是所有任务都需要所有 Agent。训练一个通信图，每轮动态淘汰贡献度最低的 Agent 节点和边。

| 任务复杂度 | 使用 Agent 数 |
|---|---|
| 极轻 | 仅 1 个 Agent（或跳过 LLM） |
| 轻 | 0-1 个专家 |
| 中 | 2-3 个专家 |
| 重 | 5+ 个专家 |

**量化效果**（Llama3-8B / Qwen2.5-72B / DeepSeek-V3）：
- Prompt token 减少 **21.6%**
- Completion token 减少 **18.4%**
- 任务性能同时提升 **+1.14 分**

**关键属性**：
- **领域可迁移**：在一个数据集上学的通信拓扑，可直接迁移到同领域其他任务，无需重新训练
- **对初始拓扑鲁棒**：不管初始是全连接 / 分层 / 随机，都能稳定收敛

**对我们系统的含义**：当前每条条款最多被 7 个 Agent 审查。借鉴 AgentDropout 的思路，ROUTE 阶段不做"匹配就分配"，而是按条款复杂度动态决定派几个 Agent：
- L1 格式条款 → 1 个 Agent（FactCheck）
- L2 标准条款 → 2-3 个 Agent
- L3 高风险条款 → 全部 Agent

不需要训练通信图（那是 AgentDropout 论文做的），简单的复杂度 → Agent 数量映射就足够。

**来源**：ACL 2025 (aclanthology.org/2025.acl-long.1170)；arXiv:2503.18891；GitHub: wangzx1219/AgentDropout

---

### 对照 5：搜索慢 → Semantic Caching + 跨 Agent 共享

**对标实践**：

**方案 A：Semantic Caching（语义缓存）**

来自 Financial Document Processing benchmark 的规模化实践：
- 对重复/相似的文档片段做语义缓存，避免重复搜索同一个法规
- SHA hash 作为 chunk ID，增量处理时只处理变化的片段
- Ellipsis 报告：大多数 commit 只影响少于 5% 的 chunks，增量同步只需"几秒钟"

**方案 B：Context Compression（上下文压缩）**

来自 Adaptive Focus Memory (AFM) 和 LangGraph RFC #6617：
- **AFM**：按相关性和时间衰减给每条历史消息打分，分配保真度级别（完整 / 压缩摘要 / 占位符 stub），达到 ~65% token 减少；对比 naive replay 减少 70-80%
- **Token Budget 管理**（LangGraph 提案）：`trim_oldest`、`trim_middle`、`summarize` 策略，保持最近 N 轮完整、更早轮次做摘要
- **Artifact 寻址压缩**：工具调用结果保持引用而非塞入上下文，模型按 ID 获取

**对我们系统的含义**：
- 当前 `search_cache` 只缓存 query → result 映射 → 升级为跨 Agent 共享的语义缓存
- 当前 conversation 线性增长到 10K+ token → 引入 token budget + 中间轮次摘要
- 常用法规（招标投标法、政府采购法等）→ 预设知识库，不走实时搜索

**来源**：Financial Document Processing Benchmark (arXiv:2603.22651)；LangGraph RFC #6617；AFM 论文 (2025)

---

### 对照 6：无缓存 → Prompt Caching + Progressive Context Loading

**方案 A：Prompt Caching**

来自 Anthropic 和 KVFlow (NeurIPS 2025)：
- System prompt + tool definitions 在同一 session 的多次调用中保持不变，应被缓存
- **KVFlow**：用 Agent Step Graph 预测 KV cache 的未来复用模式，主动 prefetch。在并发工作流上实现 **1.83-2.19× 加速**

**方案 B：Progressive Context Loading（渐进式上下文加载）**

社区标准 5 层策略：

| 层 | 内容 | Token |
|---|---|---|
| 0 | Bootstrap（意图识别） | ~50 |
| 1 | 意图分析 | +100 |
| 2 | 选择性上下文 | 500-3K |
| 3 | 深度上下文（复杂任务） | 10-20K |
| 4 | 外部研究（新发现） | 20-50K |

**对我们系统的含义**：
- System prompt 在 session 内只发一次（需 DashScope 支持 prompt caching）
- SessionGraph 查询从"每 turn 全量"改为"首 turn 全量 + 后续增量更新"
- 只在新消息到达时才 invalidate 缓存并重新查询

**来源**：KVFlow (NeurIPS 2025)；Anthropic Prompt Caching 文档；SuperClaude Framework Token Efficiency 研究

---

### 对照 7：Debate 单模型自我辩论 → Verdict Ensemble + Cross-Vendor Review

**对标实践**：

**方案 A：Ensemble of Cheap Judges 替代单模型自我辩论**

来自 Haize Labs **Verdict** 框架 (2025)：
- **不要**用一个昂贵模型自己和自己辩论（当前做法：Defender → Challenger → Arbiter 同一模型）
- **改用** 3-5 个便宜模型各审一次，多数投票。效果 ≥ 单个顶尖模型，成本更低、延迟更短

**方案 B：Cross-Vendor Review**

来自 `dualpass` 库的实践：
- **"对抗自评偏差的最强手段是用不同模型提供商做审查"**
- 例如：生成用 DashScope(qwen-plus)，审查用另一个模型

**方案 C：RouteJudge — 难度路由**

来自 RouteJudge (OpenReview 2025)：
- 不是所有高风险 finding 都需要深度辩论
- Low-confidence → 简单规则降级（confidence < 0.7 → 直接降为 Medium）
- Medium-confidence → 1 个专家复查
- High-confidence → 跳过辩论

**对我们系统的建议**：Debate 默认关闭。真的需要时，用规则降级 + 单模型复查，不要三角色自我辩论。

**来源**：Haize Labs Verdict (GitHub 2025)；RouteJudge (OpenReview 2025)；dualpass (PyPI)

---

### 对照 8-9：SessionGraph + AgentBus 开销 → Lazy Context + 单次 Poll

**对标实践**：

**Progressive Context Loading**（同对照 6 的方案 B）：只在首次审查该条款时加载 context，后续 turns 在 AgentBus 有新消息时才增量更新。

**Single poll + Notify 模式**：社区共识是单次 poll 足够。如果真有时效性要求，用 `tokio::sync::Notify` 或条件变量唤醒，而非双重轮询。

**来源**：SuperClaude Framework 研究；LangGraph RFC #6617

---

### 跨问题：整体架构模式选择

**对标论文**：*"Benchmarking Multi-Agent LLM Architectures for Financial Document Processing"*, arXiv:2603.22651 (2025)

四种架构的生产环境量化对比：

| 架构 | 准确率 | 成本 | 适用场景 |
|---|---|---|---|
| **Sequential Pipeline**（当前） | Baseline | 1× | 简单、线性流程 |
| **Parallel Fan-Out + Merge** | 高 | 1.2× | 独立审查维度 ⬅️ **我们最适用** |
| **Hierarchical Supervisor** | 最高 | 1.6× | 成本敏感的规模化 |
| **Reflexive Self-Correcting** | 最高 (F1 0.943) | 2.3× | 高风险审计 |

**Scale-up 策略**（10K→100K docs/day）：
1. **Semantic Caching**：缓存重复/相似文档片段的审查结果
2. **Model Routing**：简单文档 → 便宜快速模型，复杂文档 → 前线模型
3. **Adaptive Retry**：只对低置信度字段重新提取（阈值 0.85）
4. **Hybrid Configuration**：Caching + Routing + Adaptive Retry 组合 → 达到 89% 准确率，仅 1.15× 基线成本

**对我们系统的最佳匹配**：**Parallel Fan-Out + Merge**。标书的审查维度（事实核查、程序合规、隐性风险、技术需求等）相互独立，天然适合并行 Fan-Out。当前 Coordinator 已经做了 Agent 间并行，但 LEGAL_VERIFY / BLINDSPOT / DEBATE 又把管线拉回串行——把这三个阶段移到关键路径之外即可恢复并行的优势。

**来源**：arXiv:2603.22651 (Financial Document Processing Benchmark, 2025)；dualpass (PyPI)

---

### 最终建议：三阶段改造路线

| 阶段 | 目标 | 改动 | 参考方案 |
|---|---|---|---|
| **Phase 1** (立即) | 30 分钟 → 8 分钟 | 降低 max_turns + 提高 max_parallel_clauses + 限制每 clause 最多 2 Agent | Focused ReAct + AgentDropout 思路 |
| **Phase 2** (本周) | 用户感知 8 分钟 → 3 分钟 | 主结果立即返回 + BlindSpot/LegalVerify/Debate 转后台异步 | Anthropic Orchestrator-Worker 模式 |
| **Phase 3** (本月) | 质量不降 + 更快 | ReWOO 混合模式 + 语义缓存 + Prompt Caching + 批量 LegalVerify | ReWOO+ReAct 混合 + EMNLP 2025 Batch Judge |
