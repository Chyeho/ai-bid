# 第3课教学计划：错误处理与 Trait

---

## 教学目标

学员从这节课起不再 `unwrap()` 一切，并能定义 trait 做多态。本课两个主题（Result + Trait）是 Agent 课程日常必需。

---

## 教学内容安排

### 0-5min：快速回顾第 2 课

- 展示文本搜索引擎的优秀作业

### 5-25min：错误处理（20min）

1. **Option**（5min）：match Some/None → unwrap_or → if let
2. **Result**（5min）：演示读文件失败 → match Ok/Err
3. **? 运算符**（10min，重点）：现场重构——先写嵌套 match 版本（难看）→ 用 ? 重写（清爽）
   - 强调：main 要改成 `-> anyhow::Result<()>` 才能用 ?
   - 强调：? 只能在返回 Result 的函数里用
4. **anyhow + context**（穿插）：`.with_context(|| format!(...))`

### 25-45min：Trait（20min）

1. **定义 + 实现**（10min）：现场写 Tool trait → Calculator 实现 → Clock 实现
2. **Box<dyn Trait>**（10min）：演示为什么 `Vec<Calculator>` 和 `Vec<Clock>` 不能放一起 → `Vec<Box<dyn Tool>>` 解决问题
   - 提前渗透：Agent 第 2 课的 ToolRegistry = `HashMap<String, Box<dyn Tool>>`

### 45-55min：作业说明

- 作业分两个 part 但互相关联
- Part 1（Config 加载）练 Result + ?
- Part 2（Command 系统）练 trait + Box<dyn>
- 时间够的话，两个 part 在 main 里串联

### 55-60min：预告第 4 课

---

## 常见坑

| 坑 | 应对 |
|---|---|
| ? 在 main 里报错 | main → `fn main() -> anyhow::Result<()>` |
| `Box<dyn Trait>` 编译报错 | 检查 trait 是否 import 了、方法签名是否有 `&self` |
| serde 反序列化类型不匹配 | 用 `dbg!(&raw_json)` 看实际结构 |
| context 闭包语法不会写 | 给模板 `|| format!(...)` |

---

## 评分

| 项 | 权重 |
|---|---|
| Config::from_file 正确 | 20% |
| 三个错误场景都能处理 | 15% |
| Command trait + 2 个实现 | 20% |
| CommandRegistry 正确 | 15% |
| 全用 anyhow::Result + ? | 10% |
| 编译 + clippy | 20% |
