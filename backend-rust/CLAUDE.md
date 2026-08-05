# Backend — Rust CLI + Multi-Agent 审核引擎

智能标书审核系统的后端服务。

## 构建与运行

```powershell
# 编译检查
cargo check

# 构建
cargo build --release

# 运行（需要 .env 中配置 API 密钥）
cargo run -- <投标文件.pdf>
cargo run -- --chat <投标文件.pdf>    # 交互式 ChatAgent 模式

# 测试
cargo test
cargo test --bin test_agents -- --test all    # Agent 集成测试

# 代码检查
cargo clippy -- -D warnings
```

## 环境变量

所有环境变量在根级 `.env` 中配置。详细列表见根级 `CLAUDE.md`。

如果从 `backend-rust/` 目录运行，设置 `AIBID_DATA_DIR=..` 让路径解析到项目根目录：

```powershell
$env:AIBID_DATA_DIR=".."
cargo run -- tests/file/xxx.pdf
```

## 架构概览

```
src/
├── main.rs              # CLI 入口，6 阶段管线编排
├── lib.rs               # 模块导出
├── paths.rs             # 统一路径解析（AIBID_DATA_DIR）
├── domain/              # 核心数据结构
│   ├── chunk.rs         # Chunk 类型与配置
│   ├── raw_document.rs  # PDF 解析中间表示
│   └── vector_index.rs  # 向量索引
├── services/            # 业务服务层
│   ├── chunking_service.rs    # 智能 Chunk 切分
│   ├── sectionize_service.rs  # 章节结构识别
│   ├── pdf_extract_service.rs # PDF 文本提取（双引擎）
│   ├── docx_convert_service.rs# DOCX→PDF 转换
│   ├── embedding_service.rs   # 嵌入生成（本地/远程）
│   ├── embedding_api_client.rs# 远程嵌入 API
│   ├── llm_client.rs          # LLM 客户端
│   └── desensitize_service.rs # 数据脱敏
├── agents/              # Multi-Agent 审核框架
│   ├── coordinator.rs   # 协调器（7 阶段管线）
│   ├── react_loop.rs    # ReAct 循环
│   ├── chat_agent.rs    # 交互式对话 Agent
│   ├── session_graph.rs # 会话状态图（Blackboard）
│   ├── registry.rs      # Agent 注册表（10 个内置 Agent）
│   ├── bus.rs           # Agent 消息总线（Broadcast）
│   ├── trace.rs         # 审查追溯日志（JSONL）
│   ├── review_event.rs  # SSE 实时事件推送
│   ├── prompts.rs       # 11 个系统提示词
│   ├── types.rs         # 核心类型定义
│   ├── testing.rs       # 集成测试基础设施
│   ├── fact_check.rs    #   FactCheck Agent 工厂
│   ├── procedure.rs     #   Procedure Agent 工厂
│   ├── semantic_risk.rs #   SemanticRisk Agent 工厂
│   └── tools/           # Agent 工具集（11 个已实现 + 2 个 V2 规划）
├── api/                 # HTTP API（Axum）
│   ├── router.rs        # 8 个 REST 端点 + SSE 流
│   └── ...
└── bin/
    ├── test_agents.rs   # Agent 集成测试
    ├── test_llm.rs      # LLM 连接测试
    └── server.rs        # HTTP API 服务器入口 (:3001)
```

## 6 阶段管线

1. **PDF → RawDocument** — Rust pdfplumber 主路径 + Python 兜底
2. **RawDocument → Sections** — 章节结构识别 + 表格注入 + 孤儿块兜底
3. **Sections → Chunks** — 条款级智能切分（Leaf / Merged / Split）
4. **Chunks → Embedding** — BGE-M3 本地或远程嵌入
5. **语义搜索验证** — 5 条预设查询验证检索质量
6. **Multi-Agent 审核** — Coordinator / 单 Agent 合规审查

## 路径约定

所有文件系统路径通过 `src/paths.rs` 的 `data_path()` / `data_path_str()` 解析，统一由 `AIBID_DATA_DIR` 环境变量控制。不要硬编码相对路径。

## 模型文件

BGE-M3 ONNX 模型（~568MB）缓存在 `models/` 目录，默认在 gitignore 中。首次运行 `EMBED_ENGINE=local` 时自动下载。
