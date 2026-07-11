# 第4课教学计划：异步与 HTTP 实战

> Rust 前置最后一课。学员能调 DashScope API。Agent 第 1 课从这里直接开始。

---

## 教学目标

1. 理解 async/await（不讲 Future 原理）
2. 用 reqwest + serde + dotenv 调 HTTP API
3. 用 clap 解析子命令
4. 把 4 课技能组装成 CLI 工具

---

## 教学内容安排

### 0-5min：回顾第 3 课

- 展示优秀的 Command 系统作业

### 5-20min：async/await（15min）

1. "拿号等餐"类比——同步 = 排队，异步 = 拿号等叫号
2. 现场写 `async fn` → `.await` → `#[tokio::main]`
3. 演示忘写 `.await` 的编译错误（学员一定遇到）
4. 不讲 Future、Pin、Waker——只讲用法

### 20-45min：调 DashScope API（25min，本课高潮）

从零开始，全场看 LLM 回复出现在终端：

1. `cargo init` → 加依赖
2. 写 `.env` + `dotenv`
3. 构造 JSON body → POST → 检查状态码 → 解析响应
4. 强调 `if !resp.status().is_success()`——不讲的话学员 401 只会 panic

### 45-55min：clap（10min）

- 给子命令模板，学员复制改参数名
- 不讲高级功能

### 55-60min：作业说明 + 预告 Agent

- "你刚写的 ask 函数，加一个 while 循环和 Vec<ChatMessage>，就是 Agent"
- 展示 Agent 第 1 课效果

---

## 常见坑

| 坑 | 应对 |
|---|---|
| 忘写 .await | 编译器报 future，99% 是没 await |
| .env 位置 | dotenv 从工作目录找，不是 src/ |
| DashScope 401 | Key 错或 .env 位置不对 |
| serde 路径写错 | `dbg!(&result)` 看实际结构 |

---

## 评分

| 项 | 权重 |
|---|---|
| ask 子命令正确 | 25% |
| translate 子命令正确 | 25% |
| API 失败不 panic | 20% |
| System Prompt 合理 | 10% |
| 编译 + clippy | 20% |
