# 第4课教学计划：Planning

---

## 教学目标

学员理解为什么复杂任务需要 Plan-and-Solve，学会用结构化 JSON 做计划。

---

## 教学内容安排

### 0-10min：上周回顾

### 10-35min：概念讲解

**用"搬家"类比解释 Plan-and-Solve vs ReAct：**

- ReAct = 搬一件想一件："这个箱子搬去哪？哦厨房。这个呢？也是厨房。那这个？卧室。还剩什么？呃...忘了。"
- Plan-and-Solve = 先画平面图，标房号，再按房间分批搬

**核心演示**（20min）：同一个复杂问题"帮我规划北京3日游，预算3000"——
1. 先用 ReAct 跑一遍（观察：Agent 搜了好几轮，容易跑偏）
2. 再用 Plan-and-Solve 跑一遍（对比：有步骤、有依赖、不遗漏）

**三阶段讲解**（5min）：
- Plan = 调 LLM 输出 JSON 步骤列表
- Execute = 按依赖顺序执行（无依赖的并行，有依赖的串行）
- Solve = 汇总生成最终答案

### 35-50min：JSON 结构化输出的坑

- LLM 输出的 JSON 经常带 ```json 包裹 → 需要 clean 函数
- 尾部逗号、注释、字段缺失 → 需要 `serde_json` 的 `#[serde(default)]`
- 演示一次错误的 JSON → 展示怎么处理

### 50-60min：作业说明

---

## 学员常见坑

| 坑 | 怎么帮 |
|---|---|
| JSON 解析失败 | 提示写一个 `clean_json()` 去 ``` 包裹 |
| 依赖解析逻辑写错 | 提示用 HashSet 存已完成步骤 id |
| 步骤执行顺序不对 | 先收集所有无依赖步骤 → 执行 → 标记完成 → 收集下一批 |
| LLM 生成的计划太简单（只有 1-2 步） | prompt 中要求"至少 3 步" |

---

## 评分标准

| 项 | 权重 | 怎么检查 |
|---|---|---|
| Plan 阶段产出 JSON | 25% | JSON 含 task + steps，每步有 id/depends_on |
| Execute 正确处理依赖 | 25% | 依赖未完成的步骤被跳过 |
| Solve 汇总生成最终答案 | 20% | 答案引用了各步骤的结果 |
| 错误处理（某步失败） | 10% | 不 panic，标记失败并跳过依赖步骤 |
| 思考题 | 10% | ReAct vs Plan-Solve 在标书审核的分析 |
| clippy | 10% | |

---

## 与项目的关联

让学员看 `backend-rust/src/agents/coordinator.rs` 的 7 阶段流水线——这就是一个 Plan-and-Solve 的生产级实现。ROUTE = Plan、EXECUTE = Execute、MERGE+TRIAGE = Solve。
