# ai-bid — 智能标书审核系统

前后端分离的 monorepo 项目。

## 目录结构

| 目录 | 说明 |
|---|---|
| `backend-rust/` | Rust 后端：CLI 工具 + Multi-Agent 审核引擎 |
| `backend-java/` | Java 后端：Spring Boot 业务平台（认证/CRUD/文件管理） |
| `frontend/` | React 前端（Vite + TypeScript） |
| `docs/` | 项目文档（中文） |
| `models/` | ONNX 嵌入模型文件（~568MB，git ignored） |

## 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust 2024, Tokio, Reqwest |
| LLM | DashScope (qwen-plus) 或 OpenAI 兼容接口 |
| 嵌入 | BGE-M3 ONNX 本地推理 或 远程 DashScope text-embedding-v4 |
| 搜索 | DashScope 联网搜索 或 SearXNG 自托管 |
| 前端 | React 18 + TypeScript + Vite |

## 环境变量（.env）

Rust 后端依赖以下环境变量（详见 `backend-rust/src/main.rs`）：

- `DASHSCOPE_API_KEY` — 阿里云 DashScope API 密钥
- `AIBID_LLM_PROTOCOL=dashscope` — LLM 协议（dashscope / openai_compatible）
- `OPENAI_API_KEY` / `OPENAI_BASE_URL` — OpenAI 兼容接口
- `AIBID_SEARCH_BACKEND=dashscope` — 搜索后端（dashscope / searxng）
- `EMBED_ENGINE=local` — 嵌入引擎（local / remote）
- `AIBID_AGENT=1` — 启用 Multi-Agent 模式
- `AIBID_COORDINATOR=1` — 启用 Coordinator 模式
- `AIBID_DATA_DIR` — 数据根目录，默认 `.`。从 `backend-rust/` 目录运行时设为 `AIBID_DATA_DIR=..` 可将路径解析到项目根目录

## 前后端通信

- 开发环境：前端 `localhost:5173` → 后端 `localhost:3000/api`
- API 约定：RESTful，JSON 格式

## 工作约定

- 跨子项目修改时，先看对应的 `CLAUDE.md`
- 后端用 `cargo check` / `cargo test` 验证
- 前端用 `pnpm dev` / `pnpm build` 验证
- 文档在 `docs/` 下，中文撰写
