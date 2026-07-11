# 第1课教学计划：类型、结构体、所有权

---

## 教学目标

学员能定义 struct 和 enum，理解所有权三条规则。不深入生命周期。

---

## 教学内容安排

### 0-10min：开场

- 自我介绍 + 课程介绍
- 展示本课最终效果：一个能运行的成绩管理系统
- 问学员："你们觉得 Rust 最难的是什么？"（预热）

### 10-35min：概念讲解（现场写代码）

**顺序很重要**：先 struct/enum（像其他语言的东西，建立信心），再所有权（Rust 特有的）。

1. **struct 和 enum**（10min）：现场写 Student + Grade，演示 match
2. **方法**（5min）：`impl Student { fn average(&self) -> f64 }`——注意 `&self` 是借用
3. **所有权**（10min）：三条规则，用白板画图解释 move → 用代码演示编译错误

### 35-55min：作业说明

- 带学员创建一个新的 Cargo 项目
- 演示一遍"定义 struct → 写 impl → 在 main 里创建实例"的完整流程
- 强调：用 `String` 存数据，用 `&str` 接收参数

### 55-60min：教他们读编译器错误

找一个典型的所有权错误（use after move），带学员一起读编译器的报错信息。这项技能比什么都重要。

---

## 常见坑

| 坑 | 应对 |
|---|---|
| `struct Student { name: &str }` 编译失败 | 解释：struct 字段用 `&str` 需要生命周期标注，初学直接用 `String` |
| `s1.average()` 后 s1 不能用了 | 检查是否 `fn average(self)` 而不是 `&self` |
| String 和 &str 到处转换 | 给口诀："拿进来用 &str，存起来用 String" |

---

## 评分

| 项 | 权重 |
|---|---|
| struct + enum 定义正确 | 25% |
| average() 和 grade() 正确 | 30% |
| 创建 3 个学生并打印 | 25% |
| 编译 + clippy | 20% |
