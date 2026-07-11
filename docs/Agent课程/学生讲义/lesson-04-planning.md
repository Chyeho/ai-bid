# 第4课：Planning — 先计划后执行

> 复杂任务不能边走边看。Agent 学会先做规划，再一步步执行。

---

## 学习目标

1. 理解 Plan-and-Solve 和 ReAct 的区别
2. 用结构化 JSON 让 LLM 输出可解析的计划
3. 实现依赖管理：步骤之间有先后顺序

---

## 核心概念

### 为什么 ReAct 不够用了

ReAct（前两课的模式）对简单任务很高效：

```
用户："北京今天天气怎么样？"
Agent: Thought → Action: get_weather("北京") → 回答
```

但面对复杂任务，ReAct 容易跑偏：

```
用户："帮我规划北京3日游，预算3000"
Agent: 搜景点 → 哦还有个博物馆 → 也有个公园 → 还有什么来着？→ 忘了算预算 → 跑偏了
```

Plan-and-Solve 解决这个问题：

```
Phase 1: Plan  → LLM 输出 JSON：4个步骤，含依赖关系
Phase 2: Execute → 逐个执行步骤，满足依赖的可以并行
Phase 3: Solve  → 汇总所有步骤结果，生成最终答案
```

### 计划长什么样

```json
{
  "task": "规划北京3日游，预算3000元",
  "steps": [
    { "id": 1, "desc": "搜索热门景点", "tool": "search_document", "depends_on": [] },
    { "id": 2, "desc": "查3天天气",     "tool": "get_weather",    "depends_on": [] },
    { "id": 3, "desc": "排每日行程",     "tool": null,            "depends_on": [1, 2] },
    { "id": 4, "desc": "算总预算",       "tool": "calculator",    "depends_on": [3] }
  ]
}
```

注意步骤 3 和 4 的 `depends_on`：步骤 4 必须在 3 之后执行（先有行程才能算预算），步骤 1 和 2 可以并行（互不依赖）。

### Plan-and-Solve vs ReAct

| | ReAct | Plan-and-Solve |
|---|---|---|
| 决策方式 | 每步观察→思考→行动 | 先全局规划→再逐步执行 |
| 适合场景 | 简单任务、需要灵活应变 | 复杂任务、多步骤有依赖 |
| 优点 | 灵活 | 不会遗漏 |
| 缺点 | 容易跑偏 | 计划可能不准确 |

---

## 作业

### 基本要求

实现一个旅行规划 Agent：

1. **Plan 阶段**：用户输入旅行需求 → 调 LLM 生成 JSON 计划（至少 3 步）
2. **Execute 阶段**：按依赖顺序执行计划：
   - `tool` 不为 null → 调用对应工具执行
   - `tool` 为 null → 调 LLM 基于已完成步骤的结果做推理
   - 某步失败 → 标记 `success: false`，依赖它的步骤跳过
3. **Solve 阶段**：所有步骤完成后，调 LLM 汇总所有结果，生成最终旅行方案
4. **可用工具**（复用前几课的）：`search_document`、`get_weather`（选做）、`calculator`

### 进阶（选做）

- **RePlan**：某步骤失败或结果不理想时，调 LLM 重新生成后续计划
- **并行执行**：用 `tokio::join!` 并行执行不互相依赖的步骤
- **进度显示**：执行时打印 `[2/4] 搜索景点中...`

### 思考题

> Plan-and-Solve 和 ReAct 各有什么优缺点？在标书审核场景中，哪种模式更合适？为什么？

---

## 参考资料

- [Plan-and-Solve 论文 (arXiv:2305.04091)](https://arxiv.org/abs/2305.04091)
- [serde_json 解析 JSON](https://docs.rs/serde_json/latest/serde_json/)
