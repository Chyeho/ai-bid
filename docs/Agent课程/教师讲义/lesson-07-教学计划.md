# 第7课教学计划：Memory

---

## 教学目标

最后一课，在第 5 课的多 Agent 系统基础上增加记忆系统。学员理解三层记忆模型，实现滑动窗口摘要和跨对话偏好持久化。

---

## 教学内容安排

### 0-10min：上周回顾

### 10-30min：概念讲解

**用一个贯穿的例子讲三层记忆**（20min）：

```
第 1 次对话：
  你："我喜欢详细的解释，带代码示例。Rust 的 async 怎么用？"
  Agent：[详细解释，带代码示例]

  工作记忆：存了 4 条消息（sys + user + assistant + user）
  短期记忆：（还没触发，消息不够多）
  长期记忆：LLM 提取偏好 → 写入 memory/default_user.json
          {"preferences": [{"key": "回答风格", "value": "详细，带代码示例"}]}

（关闭程序）

第 2 次对话：
  你："介绍一下 Tokio"
  Agent 启动 → 加载 memory/default_user.json
  → System Prompt 注入："[用户偏好] 回答风格：详细，带代码示例"
  → Agent 自动用详细风格回答

（聊了很久，消息超过 20 条）

  短期记忆触发 → 前半部分消息调 LLM 摘要
  → "[对话历史摘要] 用户询问了 Tokio 的基本概念和 spawn 用法，
      已经介绍了 Runtime 和 task 的概念。用户似乎对并发模型感兴趣。"
  → 原始消息删除 → 摘要替代 → 继续对话
```

### 30-45min：技术要点

- 摘要 prompt 怎么写："用 2-3 句话总结以下对话的关键信息"
- 偏好提取 prompt："从对话中提取用户明确表达的偏好，只输出 JSON"
- JSON 文件存储：`memory/{user_id}.json`，`serde_json::to_string_pretty`
- 注入 System Prompt：在 system 消息末尾追加记忆文本

### 45-60min：课程总结

- 7 周回顾：从 ai-client → Tool Use → RAG → Planning → Multi-Agent → MCP → Memory
- 映射到项目：学员现在能看懂 `react_loop.rs`、`coordinator.rs`、`bus.rs`、`session_graph.rs`
- 下一步：开始参与各组开发

---

## 学员常见坑

| 坑 | 怎么帮 |
|---|---|
| 摘要太长了（500 字+） | prompt 中加字数限制"2-3 句话" |
| 偏好提取了幻觉信息（LLM 推测的） | prompt 强调"只提取用户明确表达的内容" |
| memory.json 文件权限问题 | 检查工作目录，确保有写入权限 |
| 注入 System Prompt 后对话质量下降 | 记忆文本不要太长，控制在 200 字内 |
| drain 后索引乱了 | 用 `drain(0..half)` 而非手动 pop |

---

## 评分标准

| 项 | 权重 | 怎么检查 |
|---|---|---|
| 短时记忆：超 20 条触发摘要 | 25% | 连续发 20+ 条消息后有摘要生成 |
| 摘要后对话正常继续 | 15% | Agent 还能回答之前聊过的话题 |
| 长时记忆：偏好写入文件 | 20% | memory/ 下出现 JSON 文件 |
| 新对话加载偏好 | 20% | 关了重开，Agent 自动应用之前记录的偏好 |
| 思考题 | 10% | 结构化记忆 vs 对话记忆的分析 |
| clippy | 10% | |

---

## 与项目的关联

让学员看 `backend-rust/src/agents/session_graph.rs`。他们的 `UserMemory` 是"对话级别的非结构化记忆"，项目的 SessionGraph 是"审查任务级别的结构化图记忆"——按 chunk → risk → agent 的关系建模，支持图查询。
