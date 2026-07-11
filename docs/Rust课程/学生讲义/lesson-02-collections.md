# 第2课：集合与迭代器

> 程序 = 数据结构 + 算法。这节课学 Agent 开发中最常用的三种集合和链式操作。

---

## 学习目标

1. 熟练使用 `Vec`、`String`、`HashMap`
2. 用迭代器链式处理数据（`map`、`filter`、`collect`、`sort_by`）
3. 理解 `iter()` vs `into_iter()` vs `iter_mut()`

---

## 核心概念

### 三种最常用的集合

```rust
use std::collections::HashMap;

// Vec<T>：有序、可重复、可索引
let mut messages: Vec<String> = Vec::new();
messages.push("hello".to_string());
messages.push("world".to_string());
println!("第一条: {}", messages[0]);
println!("共 {} 条", messages.len());

// String：UTF-8 文本，支持拼接、切片、搜索
let mut text = String::from("你好，世界");
text.push('！');
text.push_str(" 今天天气不错。");
if text.contains("天气") {
    println!("提到了天气");
}
// 拆词（按空白和标点分）
for word in text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()) {
    if word.chars().count() >= 2 {  // 过滤单字
        println!("词: {}", word);
    }
}

// HashMap<K, V>：键值对，快速查找
let mut scores = HashMap::new();
scores.insert("小明", 85);
scores.insert("小红", 92);
// Agent 第 2 课的 ToolRegistry 就是这个结构
if let Some(score) = scores.get("小明") {
    println!("{}: {}", "小明", score);
}
// 遍历
for (name, score) in &scores {
    println!("{} → {}", name, score);
}
```

### 迭代器：数据流水线

```rust
let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// filter → map → collect 链式操作
let even_squares: Vec<i32> = numbers
    .iter()
    .filter(|&x| x % 2 == 0)  // 保留偶数
    .map(|&x| x * x)           // 平方
    .collect();                 // 收集为 Vec
println!("{:?}", even_squares);  // [4, 16, 36, 64, 100]

// 搜索：找到第一个满足条件的
let first_big = numbers.iter().find(|&x| *x > 7);
println!("{}", first_big.unwrap());  // 8

// 判断：是否全部/任意满足
let all_positive = numbers.iter().all(|&x| x > 0);  // true
let any_big = numbers.iter().any(|&x| x > 20);       // false
```

### 三种迭代器的区别

```rust
let mut v = vec![1, 2, 3];

// iter()：只读借用
for x in v.iter() { println!("{}", x); }
println!("{:?}", v);  // ✅ v 还能用——iter 不获取所有权

// iter_mut()：可变借用，可修改元素
for x in v.iter_mut() { *x *= 2; }
println!("{:?}", v);  // ✅ [2, 4, 6]

// into_iter()：获取所有权，原始数据被消费
for x in v.into_iter() { println!("{}", x); }
// println!("{:?}", v);  // ❌ v 已被移动
```

### 实战：搜索结果排序

```rust
struct SearchResult {
    title: String,
    snippet: String,
    score: f64,
}

// 按 score 降序排列
results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

// 取 Top-5
let top5: Vec<&SearchResult> = results.iter().take(5).collect();
```

---

## 作业

### 基本要求

实现一个简单的文本搜索引擎：

1. 硬编码 3 段文本，每段给一个标题（如"政府采购法 第22条"、"招标投标法 第20条"等）
2. 定义 `Document` 结构体：`title: String`、`content: String`
3. 实现搜索函数：
   ```rust
   fn search(query: &str, documents: &[Document]) -> Vec<Document>
   // 返回的 Document 可以只保留 title + 命中的 snippet（clone 即可，不用引用）
   ```
4. 搜索逻辑：query 拆词 → 去掉长度 < 2 的词 → 找到命中任意关键词的文档
5. 按命中关键词数量降序排列
6. `main()` 中测试几个搜索词，打印匹配的文档标题

### 进阶（选做）

- 不止返回文档，返回匹配的具体段落（按双换行切段，返回命中段落的前 200 字）
- 搜索结果同时打印命中分数

---

## 参考资料

- [Rust Book 第 8 章：集合](https://doc.rust-lang.org/book/ch08-00-common-collections.html)
- [Rust Book 第 13 章：迭代器](https://doc.rust-lang.org/book/ch13-00-functional-features.html)
