# Rust 快速上手 — 为 Agent 开发做准备

> 2 天、4 节课 + 1 天大作业。只学 Agent 开发必需的 Rust 知识。不学用不到的。

---

## 你需要准备什么

| 要求 | 说明 |
|---|---|
| 编程基础 | 学过任意一门语言（Python/C/Java 都行） |
| Rust 已安装 | `rustc --version` ≥ 1.96 |
| 一个编辑器 | VSCode + rust-analyzer 插件 |

---

## 你会学到什么

2 周密集学完 Agent 课程必需的基础。更深入的 Rust 知识（broadcast channel、Arc、子进程通信）在 Agent 课里遇到时穿插讲解。

| 课次 | 内容 | 解决什么问题 |
|---|---|---|
| **第0课** | **环境搭建** | **cargo run 能跑通（开课前自学完成）** |
| 第1课 | 类型、结构体、枚举、所有权 | 能定义 ChatMessage、理解 String vs &str |
| 第2课 | 集合、迭代器 | 操作消息历史、搜索结果排序 |
| 第3课 | 错误处理、Trait | 用 ? 代替 unwrap、定义 Tool trait |
| 第4课 | 异步、HTTP 实战 | 调 DashScope API——Agent 第 1 课直接开始 |

---

## 怎么学

```
第0课     开课前自学完成（环境搭建 + 验证脚本）
第1-2课   第一天：上午学习 + 下午学习
第3-4课   第二天：上午学习 + 下午学习
大作业     第三天：ai-client crate（1 天完成）
Agent 第1课 第四天：Agent 课程正式开始 🚀
```

---

## 和 Agent 课程的关系

```
第1-2天  Rust 前置（4 课）── 基本语法 + HTTP + JSON + async
第3天    Rust 大作业（ai-client crate）
第4天起  Agent 课程（7 周）── Rust 进阶知识在用到时穿插讲
最后2周  Agent 大作业（Mini 标书审核）
```

Rust 进阶知识（trait object、broadcast channel、Arc、子进程 stdio）不放在前置课程里——脱离 Agent 场景讲这些太抽象。在 Agent 课里用到时，花 10 分钟现场讲 Rust 概念，学员立刻用，效果好得多。

---

## 学完你能做什么

- 定义 struct/enum 建模业务数据
- 用 Vec/HashMap 操作集合，用迭代器做数据转换
- 用 Result + ? + anyhow 写出不 panic 的代码
- 用 reqwest + serde + dotenv 调 HTTP API
- 用 tokio 写 async 函数并发请求
- 定义并实现 trait
