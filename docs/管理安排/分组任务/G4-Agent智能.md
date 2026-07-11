# G4 Agent 智能 — Phase 1-4 完整推进

> 3 人 | Rust（Tokio + async）
> 不依赖任何人（Phase 1）| 依赖 G2/G3（Phase 2 起）

---

## Phase 1：代码理解 + 精简方案（W1-W2）

现有 8000+ 行 Rust Agent 代码是实验平台。Phase 1 目标：吃透代码，画出地图。

1. 跑通 `cargo test --bin test_agents -- --test all`，确认现有 Agent 框架能正常工作
2. 按调用链顺序通读代码，边读边画图：
   - `coordinator.rs` — 7 阶段 Pipeline 入口，先看懂 `review()` 的主链路
   - `react_loop.rs` — 单个 Agent 的 ReAct 循环（读条款 -> 调工具 -> 输出结论）
   - `session_graph.rs` — Blackboard 共享状态（条款 -> 风险 -> 法条的图关系）
   - `bus.rs` — Agent 间消息广播
   - `prompts.rs` + `types.rs` — 11 个 Agent 的 Prompt 和全部类型定义
3. 画出三张图：调用链路图（函数级别）、数据流图（ParsedDocument -> RiskFindings）、Agent 生命周期图
4. 基于代码理解，写精简方案：
   - 11 Agent -> 3 Agent（RuleAgent / JudgeAgent / CriticAgent）
   - 7 阶段 -> 4 阶段（Route -> Execute -> Merge -> Triage）
   - 明确砍掉：Scout / Debate / BlindSpot / Agent 并发 / AgentBus
5. 与 G5 对齐 `AgentTool` trait 签名

**Phase 1 交付物**：
- 代码理解文档（内部，供后续 G4 成员参考）
- 精简方案初稿

---

## Phase 2：精简 Pipeline 跑通（W3-W5）

1. W3：编写精简版 Coordinator，config 中只启用 3 个 Agent
   - 砍掉 `scout_phase()` / `debate_high_risk()` / `run_blind_spot()`
   - 砍掉 Agent 并发（`tokio::spawn`），改为串行
   - 保留核心 4 阶段：Route -> Execute -> Merge -> Triage
2. W4：集成 G2 规则引擎 HTTP API（RuleAgent 调用 G2 获取规则匹配结果）+ 集成 G3 知识检索 HTTP API（JudgeAgent 调用 G3 获取证据）
3. W4：注入 G5 提供的新 Prompt（RuleAgent / JudgeAgent / CriticAgent 三个）
4. W5：全链路联调（G1 -> G2 -> G3 -> G4 -> G5 -> G8 -> G9）
5. 用真实学校采购招标文件端到端调试：解决输出格式不一致、工具调用失败、幻觉引用等实际问题

**Phase 2 交付物**：
- 可运行的精简 Pipeline（3 Agent 串行）
- G2/G3 HTTP 集成通过
- 端到端审核报告产出

---

## Phase 3：恢复高级特性（W6-W8）

1. W6：恢复 Agent 并发执行（3 Agent 并行 via `tokio::spawn`）
2. W7：恢复 AgentBus（高风险发现实时广播）
3. W7-8：新增投标文件审核模式
4. 成本-质量控制：限制 LLM API 消耗（每文档 token 预算）、处理超时重试、部分失败降级
5. 开始构建评测基准（50 份人工标注文档）

**Phase 3 交付物**：
- Agent 并发执行恢复
- AgentBus 恢复
- 投标文件审核模式可用

---

## Phase 4：辩论+人在回路+对照审核（W9-W10）

1. 恢复 Debate 阶段（高风险发现正反方辩论）
2. 实现 HumanReviewAgent 三节点（确认审查计划 / 实时预警 / 人工复核）
3. 招投标对照审核模式：招标要求 vs 投标响应逐条比对
4. 性能达标：百页文档端到端 < 5 分钟
5. 评测 F1 > 0.80，Critical Recall > 95%

**Phase 4 交付物**：
- Debate + HumanReview 就绪
- 对照审核模式可用
- 评测指标达标

---

## 不做的事

- 不从零搭建 Agent 框架（已有）
- 不设计新的 Pipeline 架构（已有）
- 不实现新的消息总线（已有）
- 不写 Agent 的 System Prompt（那是 G5 的工作）
- 不实现新的审查工具（那是 G5 的工作）

---

## 人员分工建议

| 角色 | Phase 1 | Phase 2-4 |
|---|---|---|
| 框架/调度 | coordinator + bus 代码阅读 | 精简配置 + Pipeline 调试 |
| ReAct/状态 | react_loop + session_graph 代码阅读 | HTTP 集成 + 性能优化 |
| 类型/评测 | prompts + types 阅读 + 画图 | 评测框架搭建 + 指标追踪 |
