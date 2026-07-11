# 教师讲义 — Rust 前置课程教学指南

> 2 天、4 课。只教 Agent 课程必需的 Rust。broadcast、Arc、子进程等高阶知识放到 Agent 课里穿插。

---

## 课程定位

这不是一门完整的 Rust 入门课。它是一张**最小可行技能清单**——学完这 4 课，学员有能力开始 Agent 第 1 课。

Agent 课里需要的更深 Rust 知识（`Box<dyn Tool>` 的 Send+Sync、`tokio::spawn`、`broadcast::channel`、`std::process::Command`），在对应的 Agent 课里花 10-15 分钟穿插讲解。**脱离 Agent 场景讲这些太抽象，放到具体需求里讲效果好得多。**

---

## 与 Agent 课程的 Rust 知识分布

```
Rust 课程
  ├─ 第0课（课前自学）：环境搭建 + cargo run
  ├─ 第1课：类型、struct、enum、所有权、#[derive]
  ├─ 第2课：Vec、String、HashMap、迭代器
  ├─ 第3课：Result、?、anyhow、trait、Box<dyn>
  ├─ 第4课：async/await + reqwest + serde + clap
  └─ 大作业（1天）：ai-client crate ── Agent 第1课 直接 import

Agent 课程内穿插
  ├─ Agent 第2课：复习 Box<dyn Tool>（第 3 课已学过）
  ├─ Agent 第3课：学员已具备 Vec/HashMap + 迭代器能力（第 2 课）
  ├─ Agent 第5课：新讲 tokio::spawn + broadcast + Arc
  └─ Agent 第6课：新讲 Command + stdio pipe
```

---

## 教学节奏

2 天 4 课密集训练营：

| 时间 | 内容 |
|---|---|
| 开课前 3 天 | 发布第 0 课（环境搭建），学员自学完成 |
| 第 1 天上午 | 第 1 课（类型、结构体、所有权） |
| 第 1 天下午 | 第 2 课（集合、迭代器） |
| 第 2 天上午 | 第 3 课（错误处理、Trait） |
| 第 2 天下午 | 第 4 课（异步 + HTTP 实战） |
| 第 3 天 | **大作业**：ai-client crate |
| 第 4 天 | **Agent 第 1 课开始** |
| 之后 | 全部作业陆续批改 |

---

## 第 3 课特别注意事项

第 3 课把错误处理和 trait 放在一起，因为 Agent 课程每天都要用这两个机制：
- Result + ? + anyhow：不用深入，能用就行
- trait + Box<dyn>：重点。Agent 第 2 课的 ToolRegistry = HashMap<String, Box<dyn Tool>>
- 作业分两个 part 但互相关联——Config 加载练 Result，Command 系统练 trait

## 第 4 课特别注意事项

第 4 课是 Rust 课程的 climax——学员第一次看到 LLM 回复出现在自己写的终端里：
- async/await：不讲 Future 原理，只讲用法
- reqwest/serde：现场演示调 DashScope
- clap：给模板，学员复制改参数名
- 本课作业（命令行 AI 助手）做完后，Rust 大作业就是把同样的代码打包成 ai-client crate

> 学员做完第 4 课 + 大作业，Agent 第 1 课只需关注"Agent 的逻辑"。

---

## 学员常见坑（全 4 课汇总）

| 坑 | 应对 |
|---|---|
| String vs &str | 口诀：存就用 String，读就用 &str |
| move 后用旧变量 | 教会读编译器报错 |
| unwrap 一切 | 第 2 课后禁止 unwrap，必须用 ? |
| `Box<dyn Trait>` 编译报错 | 检查 trait 是否在 scope 内（`use`） |
| .await 忘写 | 编译器报 future，99% 是没 await |
| .env 文件位置不对 | dotenv 从当前工作目录找，不是 src/ |

---

## 作业评分

| 项 | 权重 |
|---|---|
| 编译通过 | 30% |
| 功能正确 | 40% |
| clippy 0 warning | 20% |
| 代码可读性 | 10% |
