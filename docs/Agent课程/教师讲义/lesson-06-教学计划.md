# 第6课教学计划：MCP 协议

---

## 教学目标

难度最高的一课。学员理解 MCP 作为"工具接入的统一标准"，实现 MCP Client 通过 stdio 管道与独立 MCP Server 进程通信。

---

## 教学内容安排

### 0-10min：上周回顾

### 10-25min：🦀 Rust 子进程通信（本课穿插）

> 之前从来没讲过 subprocess。今天花 15min 讲透，因为 MCP Client 全程依赖这个。

1. **启动子进程**（5min）：`Command::new()` → `.stdin(Stdio::piped())` → `.stdout(Stdio::piped())` → `.spawn()`
2. **管道读写**（5min）：`child.stdin.take()` → `writeln!` → `flush`；`child.stdout.take()` → `BufReader` → `read_line`
3. **清理子进程**（5min）：`Drop` trait → `child.kill()`，不杀子进程会变僵尸

### 25-45min：MCP 概念讲解

**用 USB 类比解释 MCP**（5min）：以前每种设备要专用接口（串口、并口、PS/2...），USB 统一了所有设备的接入方式。MCP 就是 AI Agent 世界的 USB——不管什么工具，实现 MCP Server 就能接入。

**重点讲 JSON-RPC 2.0**（15min）：学员可能没接触过 RPC 协议。演示一次完整的 tools/list 和 tools/call 的请求→响应过程。强调 `id` 字段用于匹配请求和响应。

### 30-50min：技术要点

- `std::process::Command` 启动子进程 + 管道配置
- **关键陷阱**：stdin/stdout 的读写顺序——必须先写请求再读响应，不能反过来
- 演示玩具 Server 的工作原理（代码走读 5 分钟）
- MCP vs 第2课 Tool trait 的对比：动态发现 vs 编译时确定、跨进程 vs 同进程

### 50-60min：作业说明

- 提供完整的玩具 MCP Server（`toy_mcp_server.rs`），学员不需要改
- 学员只写 Client 端：启动子进程 + initialize 握手 + list_tools + call_tool
- 玩具 Server 提供两个工具：`read_file(path)`、`list_directory(path)`

---

## 学员常见坑

| 坑 | 怎么帮 |
|---|---|
| 子进程启动后 stdin 写不进去 | 检查 `.stdin(Stdio::piped())` 配置 |
| 读 stdout 卡住 | `read_line` 是阻塞的，需要 Server 真的回复了 |
| JSON-RPC 格式错误 | 严格对照规范：jsonrpc/ method/ params/ id 四个字段缺一不可 |
| Server 进程没退出 | 实现 `Drop` trait，确保子进程被 kill |
| Windows 上 stdio 编码问题 | 玩具 Server 用 `writeln!` 确保换行，Client 用 `read_line` |

---

## 评分标准

| 项 | 权重 | 怎么检查 |
|---|---|---|
| initialize 握手成功 | 15% | 收到 Server 的 capabilities |
| list_tools 解析正确 | 20% | 发现 read_file 和 list_directory |
| call_tool 返回正确结果 | 25% | 读文件返回文件内容，列目录返回文件名 |
| 集成到 Agent 工具系统 | 20% | Agent 能通过 MCP 工具操作文件系统 |
| 思考题 | 10% | MCP vs Tool trait 的分析 |
| clippy | 10% | |

---

## 与项目的关联

项目目前没有 MCP 实现。这节课是**为项目未来接入 MCP 打基础**。如果团队决定用 MCP 接入外部法规数据库/案例库，学员已经理解了协议原理。
