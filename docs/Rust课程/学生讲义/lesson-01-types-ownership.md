# 第1课：类型、结构体、所有权

> Rust 最核心的三个概念。学完你能定义自己的数据类型，理解 String 和 &str 的区别。

---

## 学习目标

1. 用 `struct` 和 `enum` 定义自己的类型
2. 理解所有权（ownership）的基本规则
3. 区分 `String` 和 `&str`，知道什么时候用哪个

---

## 核心概念

### struct：自定义数据组合

```rust
// 定义一个结构体
struct ChatMessage {
    role: String,      // "system" / "user" / "assistant"
    content: String,   // 消息内容
}

// 创建实例
let msg = ChatMessage {
    role: String::from("user"),
    content: String::from("你好"),
};

// 访问字段
println!("角色: {}, 内容: {}", msg.role, msg.content);
```

### enum：多种可能性的类型

```rust
// Agent 课程第 1 课就会用到的 enum
enum Role {
    System,
    User,
    Assistant,
}

// 携带数据的 enum
enum ChatMessage2 {
    System { content: String },
    User { content: String },
    Assistant { content: String },
}
```

> 思考：上面两种定义方式，哪种更好？为什么？（提示：想想 match 怎么用）

### 所有权：Rust 最独特的规则

三条核心规则：

```rust
// 规则1：每个值有且只有一个 owner
let s1 = String::from("hello");
let s2 = s1;              // s1 的所有权移给了 s2
// println!("{}", s1);   // ❌ 编译错误！s1 已经失效

// 规则2：引用（borrow）不转移所有权
let s1 = String::from("hello");
let s2 = &s1;             // s2 借用了 s1，s1 仍然有效
println!("{}", s1);       // ✅ 可以

// 规则3：可变引用同一时刻只能有一个
let mut s = String::from("hello");
let r1 = &mut s;
// let r2 = &mut s;      // ❌ 不能同时有两个可变引用
```

### String vs &str

```rust
// String：拥有数据，在堆上分配，可以修改
let mut owned = String::from("hello");
owned.push_str(" world");  // ✅ 可以修改

// &str：借用数据，不可修改，通常用于函数参数
fn print_message(text: &str) {  // 接收 &str，不获取所有权
    println!("{}", text);
}
print_message(&owned);     // String 可以借为 &str
print_message("字面量");    // 字符串字面量本身就是 &str
```

> 一个简单的判断准则：如果你要**存储**这个字符串（放在 struct 里），用 String。如果你只是**读取**它（函数参数），用 &str。

---

### #[derive] — 一行代码自动生成 trait 实现

```rust
// 加上 #[derive(Debug)]，就能用 println!("{:?}", s) 打印结构体
#[derive(Debug)]
struct Student {
    name: String,
    scores: Vec<u32>,
}

let s = Student { name: "小明".into(), scores: vec![85, 92] };
println!("{:?}", s);  // ✅ 自动生成的 Debug 实现
// 不加 #[derive(Debug)] 的话，上面这行会编译报错

// 同样，之后的课程会用到 #[derive(Serialize, Deserialize)] 自动生成 JSON 转换代码
```

> `#[derive(...)]` 告诉编译器"帮我把这个 trait 自动实现出来"。Debug、Clone、Serialize 是几个最常用的。Agent 课程会大量用到它。

---

## 作业

### 基本要求

实现一个简单的学生成绩管理系统：

1. 定义 `Student` 结构体：`name: String`、`scores: Vec<u32>`
2. 定义 `Grade` 枚举：`Excellent`、`Good`、`Pass`、`Fail`
3. 给 `Student` 实现方法：
   - `average(&self) -> f64`：计算平均分
   - `grade(&self) -> Grade`：根据平均分返回等级（≥90 Excellent，≥75 Good，≥60 Pass，<60 Fail）
4. 在 `main()` 中创建 3 个学生，打印每人的姓名、平均分和等级

### 进阶（选做）

- 添加 `Subject` 枚举，让每个分数关联科目名
- 尝试故意写一个所有权错误（如 use after move），读懂编译器的报错信息

---

## 参考资料

- [Rust Book 第 4 章：所有权](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rust Book 第 5 章：结构体](https://doc.rust-lang.org/book/ch05-00-structs.html)
- [Rust Book 第 6 章：枚举与匹配](https://doc.rust-lang.org/book/ch06-00-enums.html)
