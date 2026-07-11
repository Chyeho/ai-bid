# 第2课教学计划：集合与迭代器

---

## 教学目标

学员熟练使用 Vec、String、HashMap，掌握迭代器链式操作。作业（文本搜索）是 Agent 第 3 课 RAG 的前奏。

---

## 教学内容安排

### 0-5min：回顾第 1 课

- 展示典型所有权错误 + 编译器报错 → 演示怎么修

### 5-35min：概念讲解（现场写代码）

1. **Vec 和 String**（10min）：push/pop/索引/String 方法（contains/split/push_str）
2. **HashMap**（5min）：insert/get/遍历
3. **迭代器**（15min，重点）：
   - `iter()` → `filter` → `map` → `collect` 流水线
   - `find`、`any`、`all`
   - 关键对比：`iter()` vs `into_iter()` vs `iter_mut()`——各演示一遍
   - `sort_by` + `partial_cmp` 模板

### 35-55min：作业说明

- 文本搜索引擎 = Agent 第 3 课 search_document 工具的雏形
- 演示：拆词 → filter 去短词 → 命中检测 → 排序

---

## 常见坑

| 坑 | 应对 |
|---|---|
| filter 闭包参数解引用混乱 | 编译器会自动 deref，先写 `|x|` 试试 |
| sort_by 不会写 | 给模板 `a.xxx.partial_cmp(&b.xxx).unwrap()` |
| 中文拆词困难 | 不要求分词库，用 `split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())` |

---

## 评分

| 项 | 权重 |
|---|---|
| Document + search 函数 | 30% |
| 搜索逻辑正确 | 30% |
| 排序 + Top-N 输出 | 20% |
| 编译 + clippy | 20% |
