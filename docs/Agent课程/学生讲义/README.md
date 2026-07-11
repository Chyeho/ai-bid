# Rust AI Agent 训练营

> 7 周学习 + 2 周大作业，从零写出一个标书审核 Agent。

---

## 你需要准备什么

| 要求 | 说明 |
|---|---|
| Rust 基础 | 会用 struct / enum / trait / async / Result |
| 一个编辑器 | VSCode + rust-analyzer |
| DashScope API Key | 团队统一下发 |
| 好奇心 | 想知道 Agent 到底是怎么工作的 |

### 验证环境

在 `backend-rust/` 下执行，确认能连上 LLM：

```powershell
cargo run --bin test_llm
# 输出 "LLM 连接正常" 即就绪
```

---

## 你会学到什么

每周一节课，每节课一个可运行的 Rust 项目。

| 周次 | 课程 | 你会写出什么 |
|---|---|---|
| W1 | Hello Agent | 一个命令行对话机器人 |
| W2 | Tool Use | 机器人学会用工具（计算器、查时间） |
| W3 | Agentic RAG | 机器人能搜索本地文档再回答 |
| W4 | Planning | 旅行规划师——先计划后执行 |
| W5 | Multi-Agent | 代码审查双 Agent——一个找 bug、一个修 bug |
| W6 | MCP 协议 | Agent 通过标准协议接入外部工具 |
| W7 | Memory | Agent 记住你的偏好，跨对话持久化 |
| **W8-9** | **🎓 毕业设计** | **Mini 标书审核 Agent — [详见 final-project.md](final-project.md)** |

---

## 怎么学

```
周一     发布讲义 + 作业要求
周中     你自己读讲义 + 写代码（约 2-4 小时）
周四     交作业（提 PR）
周五     答疑 + 发参考答案
```

每节课的作业是一个 Cargo 项目。不提供封装好的框架，你自己从零写。

---

## 代码怎么写

**基础设施你自己造。** Rust 大作业里你写了 `ai-client` crate，Agent 课程全程用它调 LLM。你只关注 Agent 的逻辑——循环、历史、工具、协作。

---

## 参考资源

- 你的 `ai-client` crate — LLM 调用全部走它
- [Datawhale hello-agents 中文教程](https://datawhalechina.github.io/hello-agents/#/) — 概念讲解（Python 代码可忽略）
- [Microsoft AI Agents for Beginners 中文版](https://microsoft.github.io/ai-agents-for-beginners/translations/zh-CN/) — 概念框架
