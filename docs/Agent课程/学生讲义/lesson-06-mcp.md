# 第6课：MCP — 用标准协议接入外部工具

> 前几课你的工具都在自己的进程里。这节课 Agent 通过 MCP 协议调用**另一个独立进程**里的工具。

---

## 前置：Rust 子进程通信（本课新知识）

之前没讲过怎么启动子进程并用管道通信，这里补上。

### 启动子进程

```rust
use std::process::{Command, Stdio};

let mut child = Command::new("./target/debug/toy_mcp_server")
    .stdin(Stdio::piped())   // 子进程的标准输入 → 我们可以往里写
    .stdout(Stdio::piped())  // 子进程的标准输出 → 我们可以读
    .spawn()                 // 启动
    .expect("启动 MCP Server 失败");
```

### 通过管道读写

```rust
use std::io::{BufRead, BufReader, Write};

// 拿到子进程的 stdin（我们可以写的）
let mut stdin = child.stdin.take().unwrap();
// 拿到子进程的 stdout（我们可以读的）
let mut reader = BufReader::new(child.stdout.take().unwrap());

// 写一行 JSON-RPC 请求到子进程
writeln!(stdin, "{}", json_string)?;
stdin.flush()?;  // 确保数据发出去

// 从子进程读一行 JSON-RPC 响应
let mut line = String::new();
reader.read_line(&mut line)?;
let response: serde_json::Value = serde_json::from_str(&line)?;
```

### 别忘了清理

```rust
// 程序退出时杀掉子进程（否则会变成僵尸进程）
impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
```

> 这三个概念是本课作业的核心。你不需要理解进程/管道的底层实现，会用就行。

---

## 学习目标

1. 理解 MCP（Model Context Protocol）解决什么问题
2. 理解 JSON-RPC 2.0 的请求/响应格式
3. 实现 MCP Client，通过 stdio 与 MCP Server 通信

---

## 核心概念

### 为什么需要 MCP

```
没有 MCP：
  每个工具都要单独适配
  → 工具 A 是 HTTP API → 写一套适配代码
  → 工具 B 是命令行   → 写另一套适配代码
  → 工具 C 是 gRPC    → 再写一套
  → 每接入一个新工具，Agent 代码都要改

有了 MCP：
  所有工具提供者 → 实现一个 MCP Server
  Agent → 通过 MCP Client 发现和调用工具
  → 新工具？启动一个 MCP Server 就行，Agent 代码一行不改
```

MCP 就是一个**工具接入的统一标准**，像 USB 一样——不管什么设备，插上就能用。

### MCP 的核心操作

Agent（MCP Client）和工具（MCP Server）之间通过 JSON-RPC 2.0 通信：

| 操作 | 方向 | 含义 |
|---|---|---|
| `initialize` | Client → Server | 握手，确认协议版本和能力 |
| `tools/list` | Client → Server | "你有哪些工具？" |
| `tools/call` | Client → Server | "帮我执行这个工具" |

### JSON-RPC 2.0 消息格式

请求（Client → Server）：
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "read_file",
    "arguments": { "path": "/tmp/test.txt" }
  }
}
```

响应（Server → Client）：
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{ "type": "text", "text": "文件内容在这里..." }]
  }
}
```

### 本课架构

```
main.rs (你的 Agent + MCP Client)
  │ 通过 stdin/stdout 管道通信
  ▼
toy_mcp_server.rs (玩具 MCP Server，讲师提供)
  提供两个工具：read_file(path)、list_directory(path)
```

讲师会给一个完整的玩具 Server。你的任务是写 Client 端：启动 Server 子进程，通过管道发 JSON-RPC 请求，解析响应。

---

## 作业

### 基本要求

1. **启动 MCP Server 子进程**：用 `std::process::Command` 启动玩具 Server，配置 `stdin/stdout` 为管道
2. **实现 `initialize` 握手**：发 JSON-RPC 请求，验证 Server 返回了正确的协议版本
3. **实现 `list_tools`**：向 Server 请求工具列表，解析 JSON → 得到 `[read_file, list_directory]`
4. **实现 `call_tool`**：发送 `tools/call` 请求，接收并解析执行结果
5. **集成到 Agent**：从 MCP Server 发现的工具，注册为 Agent 的 Tool，Agent 调用时通过 MCP Client 转发

### 进阶（选做）

- 处理并发请求：多个 `tools/call` 同时发出时，用 `id` 字段区分回包
- 处理 Server 崩溃：检测子进程退出 → 自动重启
- 扩展玩具 Server：添加 `write_file` 工具（带简单的路径安全检查）
- 把玩具 Server 用 HTTP 替代 stdio 通信

### 思考题

> MCP 和第 2 课你写的 `Tool` trait 本质上都是"让 LLM 调用外部函数"。两者的核心区别是什么？什么时候选 MCP，什么时候直接实现 `Tool` trait？

---

## 参考资料

- [MCP 官方规范](https://modelcontextprotocol.io/specification/2024-11-05/)
- [JSON-RPC 2.0 规范](https://www.jsonrpc.org/specification)
- [std::process::Command](https://doc.rust-lang.org/std/process/struct.Command.html)
