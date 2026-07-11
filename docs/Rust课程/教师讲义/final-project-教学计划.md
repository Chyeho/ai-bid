# Rust 大作业教学计划：ai-client

---

## 设计意图

大作业不是"又布置一个练习题"，而是让学员**为 Agent 课程造轮子**。写完这个 crate，Agent 第 1 课只需 `use ai_client::LlmClient`，7 周的 LLM 调用全部复用这 200 行代码。

### 什么时候做

Rust 第 4 课结束后，Agent 第 1 课开始前。给 1 天时间。

---

## 验收方式

### 自动化检查

```powershell
# 1. 编译
cd 学员的 ai-client 目录
cargo check

# 2. 代码质量
cargo clippy -- -D warnings

# 3. 测试（如果学员写了测试）
cargo test
```

### 手动验收

让学员运行 `cargo run --example demo`，检查三个场景：

| 场景 | 期望 |
|---|---|
| 普通对话 | 终端打印 LLM 的文本回复 |
| 工具调用 | 终端打印 LLM 想调用的工具名和参数 |
| 错误处理 | 故意改错 Key → 打印人类可读的错误信息，**不 panic** |

---

## 常见问题预案

| 问题 | 预案 |
|---|---|
| 学员说"这就是把第 4 课代码搬过来" | 对，大作业的目的就是打包——把散落在 main.rs 里的代码组织成可复用的 crate |
| ChatMessage 序列化格式不对 | 提示对照 DashScope 文档检查 JSON 字段名（role/content/tool_calls） |
| Tool 消息的 tool_call_id 不知道怎么处理 | 提示看 DashScope 文档 tool_calls 的 id 字段 |
| 学员想加更多功能（流式、重试） | 鼓励但不要求。基础功能够 Agent 课程用就行 |

---

## 评分

| 项 | 权重 |
|---|---|
| 编译通过 | 30% |
| 基本对话场景 | 25% |
| 工具调用场景 | 25% |
| 错误处理不 panic | 10% |
| clippy 0 warning | 10% |

---

## 大作业之后的衔接

Agent 第 1 课开场，让学员打开自己写的 `ai-client` crate，讲师说：

> "你们上周写的这个 crate，接下来 7 周每天都要用。Agent 第 1 课的作业不是重新写 HTTP 请求，而是在 `LlmClient::chat()` 外面包一层 while 循环——这就是 Agent。"

这比"今天我们来学 Agent"有效得多。学员不是从零开始，而是在自己造的轮子上搭房子。
