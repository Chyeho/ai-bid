# 第3课：错误处理与 Trait

> Rust 没有 try-catch，没有 null。这节课学 ? 运算符和 trait——Agent 课程每天都要用的两个核心机制。

---

## 学习目标

1. 用 `Result<T, E>` 和 `?` 写出不 panic 的代码
2. 用 `anyhow` 快速处理"不关心具体错误类型"的场景
3. 定义和实现 trait，理解 `Box<dyn Trait>`

---

## 核心概念

### part A：Option — 没有 null 的世界

```rust
// get() 返回 Option<&V>——可能是 Some(值) 或 None
let mut scores = HashMap::from([("小明", 85)]);
let xiaohong = scores.get("小红");  // Option<&i32>

// 三种处理方式
match xiaohong {
    Some(score) => println!("{}", score),
    None => println!("查无此人"),
}
// 快捷写法
scores.get("小明").copied().unwrap_or(0);  // 没有就返回默认值
// if let
if let Some(score) = scores.get("小明") {
    println!("成绩: {}", score);
}
```

### part B：Result — 可能失败的操作

```rust
use std::fs;

// 读文件的返回值是 Result<String, Error>
match fs::read_to_string("data.txt") {
    Ok(content) => println!("内容: {}", content),
    Err(e) => println!("读取失败: {}", e),
}
```

### ? 运算符 — 错误传播利器

没有 `?` 时：

```rust
fn read_number_from_file() -> Result<i32, Box<dyn std::error::Error>> {
    let content = match fs::read_to_string("number.txt") {
        Ok(c) => c,
        Err(e) => return Err(Box::new(e)),
    };
    let num = match content.trim().parse() {
        Ok(n) => n,
        Err(e) => return Err(Box::new(e)),
    };
    Ok(num)
}
```

有 `?` 时：

```rust
fn read_number_from_file() -> anyhow::Result<i32> {
    let content = fs::read_to_string("number.txt")?;
    let num = content.trim().parse()?;
    Ok(num)
}
```

`?` 的意思是：Ok(v) → 取出 v 继续往下；Err(e) → 立刻从这里 return Err(e)。

### anyhow — 快速上手的错误处理

```rust
use anyhow::{Context, Result};

fn load_user(id: u32) -> anyhow::Result<User> {
    let path = format!("users/{}.txt", id);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("无法读取文件: {}", path))?;
    // parse_user 自己实现了错误处理，返回 anyhow::Result<User>
    let user = parse_user(&content)
        .context("用户数据格式错误")?;
    Ok(user)
}

// 自定义错误：用 bail! 和 ensure!
use anyhow::bail;

fn validate_age(age: u32) -> anyhow::Result<()> {
    if age == 0 {
        bail!("年龄不能为 0");
    }
    anyhow::ensure!(age < 150, "年龄 {} 超出合理范围", age);
    Ok(())
}
// Agent 课程中所有函数都返回 anyhow::Result
```

### 不要 panic

```rust
// ❌ 文件不存在就崩溃
let content = fs::read_to_string("data.txt").unwrap();

// ✅ 让调用方决定怎么处理
fn process() -> anyhow::Result<()> {
    let content = fs::read_to_string("data.txt")?;
    Ok(())
}
```

### part C：trait — 定义行为接口

```rust
// 定义一个 trait（别的语言叫 interface）
trait Tool {
    fn name(&self) -> &str;
    fn execute(&self, input: &str) -> String;
}

// 实现 trait
struct Calculator;
impl Tool for Calculator {
    fn name(&self) -> &str { "calculator" }
    fn execute(&self, input: &str) -> String {
        format!("TODO: 计算 {}", input)
    }
}

struct Clock;
impl Tool for Clock {
    fn name(&self) -> &str { "get_time" }
    fn execute(&self, _input: &str) -> String {
        "2026-07-11 14:30".to_string()
    }
}
```

### Box<dyn Trait> — 多态的钥匙

```rust
// 为什么需要 Box<dyn Tool>？
// 因为 Vec<Calculator> 和 Vec<Clock> 是不同的类型，不能放在同一个 Vec 里
// Box<dyn Tool> 把它们统一为同一种类型

let tools: Vec<Box<dyn Tool>> = vec![
    Box::new(Calculator),
    Box::new(Clock),
];

for tool in &tools {
    println!("工具: {}, 结果: {}", tool.name(), tool.execute("test"));
}
```

> Agent 第 2 课的 ToolRegistry 就是这样：`HashMap<String, Box<dyn Tool>>`。

---

## 作业

### 基本要求

实现一个带错误处理的配置加载器 + 命令执行器：

**part 1 — 配置加载**

1. 定义 `Config` 结构体：`api_key: String`、`model: String`、`max_tokens: u32`
2. 实现 `Config::from_file(path: &str) -> anyhow::Result<Self>`：
   - 读文件 → 按行解析（格式：`key=value`） → 验证字段（api_key 非空，max_tokens > 0）
   - 每步用 `?` 传播错误，用 `.context()` 附加信息，用 `bail!()` 报告验证失败

**part 2 — 可扩展的命令系统**

3. 定义 `Command` trait：`fn name(&self) -> &str` + `fn run(&self, args: &[String]) -> String`
4. 实现 2 个 Command：`EchoCommand`（拼接参数）、`UppercaseCommand`（转大写）
5. 实现 `CommandRegistry`：`HashMap<String, Box<dyn Command>>` + `register()` + `execute()`
6. 在 `main()` 中测试

### 进阶（选做）

- 配置加载同时支持从 `.env` 读取（如果文件不存在，fallback 到环境变量）
- 添加第 3 个 Command：`WordCount`（统计参数中的单词总数）

---

## 参考资料

- [Rust Book 第 9 章：错误处理](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Rust Book 第 10 章：Trait](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [anyhow crate](https://docs.rs/anyhow/latest/anyhow/)
