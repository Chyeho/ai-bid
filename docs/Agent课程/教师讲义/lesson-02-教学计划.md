# 第2课教学计划：Tool Use

---

## 教学目标

让学员理解 Function Calling 的本质：它是一个多轮对话协议，不是魔法 API。学员自己设计 Tool trait、实现 tool definition 的 JSON Schema、处理 tool_calls 循环。

---

## 教学内容安排

### 0-15min：上周回顾

- 展示 2 份优秀作业（1 份基础档、1 份进阶档）
- 展示 1 个典型错误：只发了最后一条消息没发完整历史 → Agent 失忆

### 15-40min：概念讲解

**三个关键点：**

1. **Function Calling 不是 API，是协议**（15min）
   - 画时序图：Request(tools=...) → Response(tool_calls) → Execute → Tool Result → Request → Response(text)
   - 用生活化例子：你问助手"现在几点？"→ 助手说"我需要看表"→ 你看表告诉他"3:15"→ 助手说"现在下午 3:15"
   - 强调：LLM **不执行**工具，它只是**说要调用哪个**。执行是你的代码做的

2. **第 4 种消息角色：tool**（5min）
   - 对比四种角色在对话历史中的位置
   - 强调 `tool_call_id` 必须匹配，否则 LLM 报错

3. **Tool Definition 的 JSON Schema**（5min）
   - 展示 get_current_time 的 definition
   - 指出三个必须字段：name、description、parameters

### 40-55min：作业说明

- 学员在第 1 课代码基础上改，不需要新建项目
- 核心改动：Agent loop 里加一个 `if response.tool_calls { ... }` 分支
- 提示：先用 `dbg!` 看 LLM 返回的 tool_calls 长什么样，再写解析逻辑

---

## 学员常见坑

| 坑 | 原因 | 怎么帮 |
|---|---|---|
| LLM 不调工具，直接说"我不知道" | System Prompt 没说明有哪些工具 | 在 prompt 中列出工具名+用途 |
| tool_call_id 不匹配 | 学员自己生成了 id | 必须用 LLM 返回的 `tc.id` |
| 工具调用死循环 | 没限制轮数 | 加 `max_turns` 计数器 |
| `Tool` trait 编译报错 | 用了泛型而非 trait object | 提示 `Box<dyn Tool>` |
| LLM 返回的 JSON arguments 是字符串 | DashScope 字段类型不同 | 提示先 `as_str()` 再 `serde_json::from_str` |

---

## 评分标准

| 项 | 权重 | 怎么检查 |
|---|---|---|
| 编译 | 30% | `cargo check` |
| Tool trait 设计合理 | 15% | name/definition/execute 三个方法 |
| 2 个工具能正确执行 | 20% | get_time 返回当前时间，calculator 算出正确结果 |
| Agent 能自主选择工具 | 15% | 问"现在几点"→调 get_time；问"3+5"→调 calculator |
| 工具结果回到对话 | 10% | LLM 基于工具结果生成最终回复 |
| 思考题 | 10% | |

---

## 与项目的关联

让学员看 `backend-rust/src/agents/tools/mod.rs` 的 `AgentTool` trait 和 `ToolRegistry`。他们的 `Tool` trait 是简化版，项目的多了 `Send + Sync` 约束和异步 execute。
