# Rust 引擎 API 参考（OpenAPI 3.0）

> **版本：** v1.0  
> **基准路径：** `http://127.0.0.1:3001`  
> **格式：** JSON，属性命名风格为 snake_case  
> **CORS：** 允许任意来源，任意方法和 Header  

---

## 目录

1. [概述](#概述)
2. [认证](#认证)
3. [通用错误格式](#通用错误格式)
4. [端点总览](#端点总览)
5. [健康检查](#1-健康检查)
6. [文档管理](#2-文档管理)
7. [智能审核（异步）](#3-智能审核异步)
8. [智能对话](#4-智能对话)
9. [语义搜索](#5-语义搜索)
10. [辅助接口](#6-辅助接口)
11. [事件流 SSE](#7-事件流-sse)
12. [数据类型参考](#8-数据类型参考)
13. [Multi-Agent 引擎](#9-multi-agent-引擎)

---

## 概述

Rust 引擎是 ai-bid 系统的智能核心，负责以下功能：

| 功能 | 说明 |
|---|---|
| 文档解析 | PDF/DOCX 解析、section 划分、chunk 分割 |
| 向量嵌入 | BGE-M3 本地推理 或 DashScope text-embedding-v4 远程 |
| Multi-Agent 审核 | 10 个专业 Agent + Coordinator 协调，三层审查 |
| RAG 对话 | 基于检索增强生成的文档问答 |
| 语义搜索 | 向量相似度搜索 |

### 环境变量

| 变量 | 说明 | 默认值 |
|---|---|---|
| `AIBID_DATA_DIR` | 数据根目录 | `.` |
| `DASHSCOPE_API_KEY` | DashScope API 密钥 | — |
| `EMBED_ENGINE` | 嵌入引擎：`local` / `remote` | `local` |
| `AIBID_AGENT` | 启用 Multi-Agent 模式 | `0` |
| `AIBID_COORDINATOR` | 启用 Coordinator 模式 | `0` |
| `AIBID_LLM_PROTOCOL` | LLM 协议：`dashscope` / `openai_compatible` | `dashscope` |
| `AIBID_SEARCH_BACKEND` | 搜索后端：`dashscope` / `searxng` | `dashscope` |

---

## 认证

当前版本不使用 API Key 或 Token 认证。Rust 引擎监听 `127.0.0.1:3001`，仅供内部服务调用。

---

## 通用错误格式

所有错误响应遵循统一结构：

```json
{
  "error": "NOT_FOUND",
  "detail": "Document 550e8400-e29b-41d4-a716-446655440000 not found"
}
```

| HTTP 状态码 | error 枚举值 | 含义 |
|---|---|---|
| 400 | `BAD_REQUEST` | 请求参数校验失败 |
| 404 | `NOT_FOUND` | 资源不存在 |
| 409 | `CONFLICT` | 资源冲突（如重复提交审核） |
| 500 | `INTERNAL_SERVER_ERROR` | 服务端内部错误 |

---

## 端点总览

| # | 方法 | 路径 | 说明 |
|---|------|------|------|
| 1 | `GET` | `/health` | 健康检查 |
| 2 | `POST` | `/api/v1/documents` | 上传并解析文档 |
| 3 | `GET` | `/api/v1/documents/{id}` | 获取文档信息 |
| 4 | `POST` | `/api/v1/documents/{id}/review` | 启动异步 Multi-Agent 审核 |
| 5 | `POST` | `/api/v1/documents/{id}/chat` | 与文档对话（非流式） |
| 6 | `POST` | `/api/v1/documents/{id}/chat/stream` | 与文档对话（SSE 流式） |
| 7 | `POST` | `/api/v1/documents/{id}/search` | 语义搜索 |
| 8 | `GET` | `/api/v1/documents/{id}/blocks` | 获取块边界框 |
| 9 | `GET` | `/api/v1/review/{doc_id}/stream` | 审核进度 SSE 流 |
| 10 | `GET` | `/api/v1/review/{doc_id}/result` | 获取审核结果 |

---

## 1. 健康检查

### `GET /health`

检查服务是否正常运行。

**请求示例**

```
GET /health HTTP/1.1
Host: 127.0.0.1:3001
```

**响应 `200 OK`**

```json
{
  "status": "ok"
}
```

---

## 2. 文档管理

### `POST /api/v1/documents` — 上传并解析文档

上传一个 PDF/DOCX/DOC 文件，执行完整处理流水线：
PDF 提取 → Section 划分 → Chunk 分割 → 向量嵌入。

**请求**

```
Content-Type: multipart/form-data
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `file` | File (binary) | 是 | 上传的文档文件 |

支持的扩展名：`.pdf`、`.docx`、`.doc`

**请求示例 (cURL)**

```bash
curl -X POST http://127.0.0.1:3001/api/v1/documents \
  -F "file=@招标文件.pdf"
```

**响应 `200 OK`**

```json
{
  "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "filename": "招标文件.pdf",
  "total_pages": 85,
  "total_blocks": 1240,
  "total_sections": 42,
  "total_chunks": 156,
  "avg_chunk_size": 512.3,
  "vector_count": 156,
  "vector_dimension": 1024
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `document_id` | string (UUID v4) | 文档唯一标识，后续所有接口都使用此 ID |
| `filename` | string | 原始文件名 |
| `total_pages` | usize | PDF 总页数 |
| `total_blocks` | usize | 解析出的文本块数 |
| `total_sections` | usize | 自动划分的章节数 |
| `total_chunks` | usize | RAG 用的 chunk 数量 |
| `avg_chunk_size` | f64 | 平均每个 chunk 的字符数 |
| `vector_count` | usize | 向量嵌入数量（= total_chunks） |
| `vector_dimension` | usize | 嵌入向量维度（BGE-M3 为 1024） |

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 400 | `BAD_REQUEST` | 上传文件为空 |
| 500 | `INTERNAL_SERVER_ERROR` | 临时目录创建失败、文件写入失败、DOCX→PDF 转换失败、PDF 提取失败、嵌入生成失败 |

---

### `GET /api/v1/documents/{id}` — 获取文档信息

查询已上传文档的处理状态和摘要信息。

**请求**

```
GET /api/v1/documents/a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

**响应 `200 OK`**

```json
{
  "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "filename": "招标文件.pdf",
  "total_pages": 85,
  "total_chunks": 156,
  "vector_count": 156
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `document_id` | string | 文档 UUID |
| `filename` | string | 原始文件名 |
| `total_pages` | usize | 总页数 |
| `total_chunks` | usize | chunk 数量 |
| `vector_count` | usize | 嵌入向量数量 |

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 404 | `NOT_FOUND` | 文档 ID 不存在（服务重启后内存数据丢失，需重新上传） |

---

## 3. 智能审核（异步）

### `POST /api/v1/documents/{id}/review` — 启动异步审核

提交文档进入 Multi-Agent 审核。审核在后台异步运行，通过 SSE 推送进度，最终结果通过 `/result` 接口获取。

> ⚠️ **重要：** 客户端应**先**连接 SSE 流（`GET /api/v1/review/{doc_id}/stream`），**再**调用此接口，以免错过早期事件。

**请求**

```
POST /api/v1/documents/a1b2c3d4-e5f6-7890-abcd-ef1234567890/review
Content-Type: application/json
```

```json
{
  "chunk_ids": [],
  "max_clauses": 200,
  "enabled_agents": ["FactCheckAgent", "RuleEngineAgent", "ScoringAgent"]
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `chunk_ids` | string[] | 否 | 指定审核的 chunk ID 列表，空数组表示全部 |
| `max_clauses` | usize | 否 | 最大审核条目数，默认 200 |
| `enabled_agents` | string[] | 否 | 启用的 Agent 名称列表，空数组表示全部启用 |

**响应 `202 Accepted`（首次提交）**

```json
{
  "status": "accepted",
  "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "message": "审核任务已提交，通过 SSE 获取实时进度"
}
```

**响应 `409 Conflict`（重复提交）**

```json
{
  "status": "conflict",
  "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "message": "该文档已有进行中的审核任务"
}
```

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 404 | `NOT_FOUND` | 文档 ID 不存在 |
| 409 | `CONFLICT` | 该文档已有审核任务在运行 |

---

### `GET /api/v1/review/{doc_id}/stream` — 审核进度 SSE 流

订阅审核的实时进度事件。

**请求**

```
GET /api/v1/review/a1b2c3d4-e5f6-7890-abcd-ef1234567890/stream
Accept: text/event-stream
```

**事件类型**

| 事件 | 说明 |
|---|---|
| `phase` | 审核阶段切换 |
| `agent_progress` | 各 Agent 处理进度 |
| `trace` | Agent 思考/调用 LLM 的详细轨迹 |
| `finding_added` | 新增一条风险发现 |
| `done` | 审核完成 |
| `error` | 审核异常终止 |

**SSE 数据示例**

```
event: phase
data: {"phase":"routing","phase_index":1,"total_phases":4}

event: agent_progress
data: {"agent_name":"FactCheckAgent","clauses_done":12,"total_clauses":45}

event: finding_added
data: {"risk_id":"R_012","severity":"high","risk_type":"地域歧视","clause_text":"投标人须为本省注册企业"}

event: done
data: {"session_id":"sess_abc123","total_findings":8,"high_risk":3,"duration_secs":45.2}

event: error
data: {"message":"LLM 调用超时","session_id":"sess_abc123"}
```

**连接生命周期**

- 连接建立后保持打开直到 `done` 或 `error` 事件
- 审核任务完成后，SSE 推送通道保留 **5 秒**后关闭
- 如果审核已完成时连接，会直接收到 `done` 事件（带缓存结果）

---

### `GET /api/v1/review/{doc_id}/result` — 获取审核结果

查询审核任务的最终结果。支持在审核进行中、已完成、或失败后任意时刻调用。

**请求**

```
GET /api/v1/review/a1b2c3d4-e5f6-7890-abcd-ef1234567890/result
```

**响应 `200 OK`（审核已完成）**

```json
{
  "status": "completed",
  "result": {
    "document_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "findings": [
      {
        "risk_id": "R_001",
        "clause_ids": ["c_3", "c_4"],
        "block_ids": ["b_5_2", "b_5_3"],
        "agent": "FactCheckAgent",
        "no_risk": false,
        "severity": "high",
        "risk_type": "地域歧视",
        "source_quote": "投标人须为本省注册企业",
        "legal_basis": ["《政府采购法实施条例》第20条", "《招标投标法》第18条"],
        "case_refs": ["(2019)最高法行再123号"],
        "reason": "要求投标人须为本省注册企业，构成以不合理的条件限制或者排斥潜在投标人...",
        "suggestion": "修改为'投标人须在中华人民共和国境内注册'，删除地域限制要求",
        "confidence": 0.92,
        "_initial_tier": "L1",
        "_final_tier": "L2",
        "_tier_escalated": true,
        "_truncated": false,
        "suggested_agent": {
          "agent_name": "LegalVerifyAgent",
          "agent_prompt": "请核实该条款是否违反政府采购法关于地域歧视的规定",
          "section_keywords": ["投标人资格", "注册要求"],
          "reason": "涉及地域限制条款，需要法律合规深度审查"
        },
        "citations": [
          {
            "title": "政府采购法实施条例释义",
            "url": "https://www.ccgp.gov.cn/...",
            "site_name": "中国政府采购网"
          }
        ],
        "page_number": 5,
        "section_path": ["第三章", "投标人资格要求"],
        "context": "3.2 投标人资格要求\n3.2.1 投标人须为本省注册企业，具有独立法人资格..."
      }
    ],
    "routing_summary": {
      "total_clauses": 200,
      "agent_clause_counts": {
        "FactCheckAgent": 45,
        "RuleEngineAgent": 60,
        "ScoringAgent": 30,
        "ProcedureAgent": 25,
        "DemandAgent": 20,
        "ContractAgent": 15,
        "SemanticRiskAgent": 5
      },
      "high_risk_count": 3,
      "legal_verify_count": 2,
      "blind_spot_findings": 1
    },
    "graph_snapshot": null
  },
  "error": null
}
```

**响应 `200 OK`（审核进行中）**

```json
{
  "status": "pending",
  "result": null,
  "error": null
}
```

**响应 `200 OK`（审核失败）**

```json
{
  "status": "failed",
  "result": null,
  "error": "LLM API call timeout after 3 retries"
}
```

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 404 | `NOT_FOUND` | 无该文档的审核记录（服务重启后内存数据丢失） |

---

## 4. 智能对话

### `POST /api/v1/documents/{id}/chat` — 与文档对话（非流式）

基于 RAG（检索增强生成）与文档进行问答。引擎搜索相关 chunks，结合 LLM 生成回答。

**请求**

```
POST /api/v1/documents/a1b2c3d4-e5f6-7890-abcd-ef1234567890/chat
Content-Type: application/json
```

```json
{
  "user_input": "第三章对投标人资质有哪些具体要求？",
  "selection": null,
  "history": [],
  "max_turns": 5
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `user_input` | string | **是** | 用户问题 |
| `selection` | object | 否 | PDF 选中文本上下文 |
| `selection.text` | string | 否 | 用户选中的文本 |
| `selection.block_ids` | string[] | 否 | 选中区域对应的 block ID |
| `selection.page` | usize | 否 | 当前页码（0-based） |
| `selection.bbox` | object | 否 | 选中区域边界框 `{x0, top, x1, bottom}` |
| `history` | Message[] | 否 | 对话历史 |
| `history[].role` | string | 否 | 角色：`system` / `assistant` / `user` |
| `history[].content` | string | 否 | 消息内容 |
| `history[].tool_call_id` | string | 否 | 工具调用 ID（用于 function calling 历史） |
| `max_turns` | usize | 否 | 最大 ReAct 循环轮次 |

**响应 `200 OK`**

```json
{
  "answer": "根据招标文件第三章，投标人需要满足以下资质要求：\n\n1. **基本资格**[b_5_2]：具有独立法人资格...\n2. **专业资质**[b_5_5]：...",
  "reasoning": [
    "搜索与'第三章 投标人资质'相关的条款...",
    "找到3条相关条款，正在分析..."
  ],
  "references": [
    {
      "block_id": "b_5_2",
      "quote": "3.1 投标人须具有独立法人资格，持有有效的营业执照",
      "snippet": "3.1 投标人须具有独立法人资格，持有有效的营业执照，注册资本不低于500万元人民币...",
      "page": 4
    }
  ],
  "knowledge_refs": [
    {
      "ref_type": "law",
      "title": "《招标投标法》第26条",
      "excerpt": "投标人应当具备承担招标项目的能力；国家有关规定对投标人资格条件或者招标文件对投标人资格条件有规定的，投标人应当具备规定的资格条件。",
      "source_url": "https://flk.npc.gov.cn/detail.html?..."
    }
  ],
  "confidence": 0.88,
  "suggested_actions": [
    "查看第三章完整内容",
    "对比第四章评分标准中的资质分值"
  ]
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `answer` | string | LLM 生成的回答，含 `[b_XXX]` 标记用于 PDF 高亮 |
| `reasoning` | string[] | 推理过程步骤 |
| `references` | Reference[] | 文档内引用的 chunks |
| `knowledge_refs` | KnowledgeRef[] | 外部知识库引用（法规、案例） |
| `confidence` | f32 | 置信度（仅合规判断类问题，0.0-1.0） |
| `suggested_actions` | string[] | 建议的后续操作 |

**Reference 结构**

| 字段 | 类型 | 说明 |
|---|---|---|
| `block_id` | string | 文档中的 block ID |
| `quote` | string | 精确引用文本 |
| `snippet` | string | 200 字符上下文 |
| `page` | usize | 页码（0-based） |

**KnowledgeRef 结构**

| 字段 | 类型 | 说明 |
|---|---|---|
| `ref_type` | string | 引用类型：`law` / `case` / `negative_list` |
| `title` | string | 标题，如"《政府采购法实施条例》第20条" |
| `excerpt` | string | 摘要 |
| `source_url` | string | 来源链接（可选） |

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 404 | `NOT_FOUND` | 文档 ID 不存在 |
| 500 | `INTERNAL_SERVER_ERROR` | LLM 客户端创建失败或对话执行异常 |

---

### `POST /api/v1/documents/{id}/chat/stream` — 与文档对话（SSE 流式）

功能同 `/chat`，但通过 SSE 逐 token 返回回答。

**请求**

请求体与 `POST /chat` 完全相同。

**响应**

```
Content-Type: text/event-stream
```

| 事件 | 说明 |
|---|---|
| `thinking` | 模型思考过程 |
| `tool_call` | ReAct 工具调用 |
| `answer` | 完整 `ChatResponse` JSON（流式累积） |
| `done` | 对话结束，携带最终 `ChatResponse` |
| `error` | 异常终止 |

```
event: thinking
data: {"message":"正在搜索文档中关于投标人资质的相关条款..."}

event: tool_call
data: {"name":"search_document","args":"{\"query\":\"投标人资质要求\"}"}

event: answer
data: {"answer":"根据招标文件第三章...","references":[...],"knowledge_refs":[...]}

event: done
data: {"answer":"...","reasoning":[...],"references":[...],"confidence":0.88}

event: error
data: {"message":"LLM API返回错误"}
```

---

## 5. 语义搜索

### `POST /api/v1/documents/{id}/search` — 语义搜索

使用向量相似度搜索文档中与查询语句最相关的 chunks。

**请求**

```
POST /api/v1/documents/a1b2c3d4-e5f6-7890-abcd-ef1234567890/search
Content-Type: application/json
```

```json
{
  "queries": ["投标保证金金额", "履约担保形式"],
  "top_k": 5
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `queries` | string[] | **是** | 搜索查询列表（支持批量搜索） |
| `top_k` | usize | 否 | 每个查询返回的最相关结果数，默认 5 |

**响应 `200 OK`**

```json
{
  "results": [
    {
      "query": "投标保证金金额",
      "hits": [
        {
          "chunk_id": "c_42",
          "title": "投标保证金",
          "score": 0.934,
          "snippet": "3.5 投标保证金 3.5.1 投标人应在投标截止时间前，按以下标准缴纳投标保证金...",
          "page_start": 12
        },
        {
          "chunk_id": "c_87",
          "title": "保证金退还",
          "score": 0.821,
          "snippet": "6.2 保证金退还 6.2.1 未中标人的投标保证金将在中标通知书发出后5个工作日内退还...",
          "page_start": 28
        }
      ]
    }
  ]
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `results[].query` | string | 原始查询文本 |
| `results[].hits[].chunk_id` | string | 匹配到的 chunk ID |
| `results[].hits[].title` | string | chunk 标题（通常是章节标题） |
| `results[].hits[].score` | f32 | 余弦相似度（0-1） |
| `results[].hits[].snippet` | string | 前 200 字符的内容预览 |
| `results[].hits[].page_start` | usize | 起始页码（0-based） |

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 404 | `NOT_FOUND` | 文档 ID 不存在 |
| 500 | `INTERNAL_SERVER_ERROR` | 查询向量编码失败、嵌入引擎未初始化 |

---

## 6. 辅助接口

### `GET /api/v1/documents/{id}/blocks` — 获取块边界框

获取指定 block 在 PDF 页面中的精确坐标，用于前端 PDF 高亮定位。

**请求**

```
GET /api/v1/documents/a1b2c3d4-e5f6-7890-abcd-ef1234567890/blocks?ids=b_5_2,b_5_3,b_12_1
```

| 参数 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `ids` | string (query) | **是** | 逗号分隔的 block ID 列表 |

**响应 `200 OK`**

```json
[
  {
    "block_id": "b_5_2",
    "page": 4,
    "bbox": {
      "x0": 72.0,
      "top": 156.3,
      "x1": 468.5,
      "bottom": 172.8
    },
    "page_width": 595.0
  },
  {
    "block_id": "b_5_3",
    "page": 4,
    "bbox": {
      "x0": 72.0,
      "top": 178.5,
      "x1": 480.2,
      "bottom": 194.0
    },
    "page_width": 595.0
  }
]
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `block_id` | string | block 标识符 |
| `page` | usize | 所在页码（0-based） |
| `bbox.x0` | f64 | 左边界（PDF points，原点在左上角） |
| `bbox.top` | f64 | 上边界 |
| `bbox.x1` | f64 | 右边界 |
| `bbox.bottom` | f64 | 下边界 |
| `page_width` | f64 | PDF 页面宽度（points），供前端计算缩放比例 |

**坐标系统说明**

- 原点位于页面**左上角**
- 单位为 PDF points（1 point = 1/72 英寸）
- 前端需要根据 `page_width` 和实际渲染宽度计算缩放比例

**错误**

| 状态码 | error | 说明 |
|---|---|---|
| 404 | `NOT_FOUND` | 文档 ID 不存在 |

---

## 7. 事件流 SSE

### 通用说明

Rust 引擎使用 **Server-Sent Events (SSE)** 进行实时通信。SSE 格式遵循标准规范：

```
event: <事件类型>
data: <JSON 数据>

```

- 每个事件以空行分隔
- `data` 字段始终为 JSON 字符串
- 客户端应使用 `EventSource` API 或手动解析

---

## 8. 数据类型参考

### RiskFinding（风险发现）

Multi-Agent 审核输出的核心数据结构。

| 字段 | 类型 | 说明 |
|---|---|---|
| `risk_id` | string | 风险编号，如 `R_001` |
| `clause_ids` | string[] | 关联的条款 ID |
| `block_ids` | string[] | 关联的 PDF block ID |
| `agent` | string | 发现该风险的 Agent 名称 |
| `no_risk` | bool | 是否为"无风险"标记 |
| `severity` | `"info"` \| `"low"` \| `"medium"` \| `"high"` | 风险严重程度 |
| `risk_type` | string | 风险类型中文标签，如"地域歧视"、"品牌指定"、"程序违规" |
| `source_quote` | string | 文档中的原文引用 |
| `legal_basis` | string[] | 法律依据列表 |
| `case_refs` | string[] | 相关案例引用 |
| `reason` | string | 完整的推理链 |
| `suggestion` | string | 修改建议 |
| `confidence` | f32 | 置信度（0.0-1.0） |
| `_initial_tier` | `"L1"` \| `"L2"` \| `"L3"` | 初始审查层级 |
| `_final_tier` | `"L1"` \| `"L2"` \| `"L3"` | 最终审查层级 |
| `_tier_escalated` | bool | 是否升级至更高级别审查 |
| `_truncated` | bool | 是否因长度限制被截断 |
| `suggested_agent` | object | 建议的后续 Agent |
| `citations` | Citation[] | 外部引用来源 |
| `page_number` | usize | 页码（0-based） |
| `section_path` | string[] | 章节路径 |
| `context` | string | 条款前 500 字符上下文 |

### ReviewResultResponse（审核结果包装）

| 字段 | 类型 | 说明 |
|---|---|---|
| `status` | `"pending"` \| `"completed"` \| `"failed"` | 审核状态 |
| `result` | ReviewResult \| null | 审核结果（仅 completed 时有值） |
| `error` | string \| null | 错误信息（仅 failed 时有值） |

### RoutingSummary（路由摘要）

| 字段 | 类型 | 说明 |
|---|---|---|
| `total_clauses` | usize | 总审核条款数 |
| `agent_clause_counts` | Map\<string, usize\> | 各 Agent 分配的条款数 |
| `high_risk_count` | usize | 高风险发现数 |
| `legal_verify_count` | usize | 需法律复核数 |
| `blind_spot_findings` | usize | 盲点发现数 |

### ChatRequest（对话请求）

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `user_input` | string | **是** | 用户问题 |
| `selection` | TextSelection \| null | 否 | PDF 选中文本 |
| `history` | Message[] | 否 | 对话历史 |
| `max_turns` | usize | 否 | 最大 ReAct 轮次 |

### ChatResponse（对话响应）

| 字段 | 类型 | 说明 |
|---|---|---|
| `answer` | string | 回答文本，含 `[b_XXX]` 高亮标记 |
| `reasoning` | string[] | 推理步骤 |
| `references` | Reference[] | 文档内引用 |
| `knowledge_refs` | KnowledgeRef[] | 外部知识引用 |
| `confidence` | f32 \| null | 合规判断置信度 |
| `suggested_actions` | string[] | 建议后续操作 |

### SearchRequest（搜索请求）

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `queries` | string[] | **是** | 查询文本列表 |
| `top_k` | usize | 否 | 每个查询返回数，默认 5 |

---

## 9. Multi-Agent 引擎

### 启用的 Agent 列表

| Agent 名称 | 职责 | 审查层级 |
|---|---|---|
| `FactCheckAgent` | 事实核查（资质、证书、业绩等） | L1 |
| `ProcedureAgent` | 程序合规检查（时间节点、流程） | L1 |
| `RuleEngineAgent` | 硬规则匹配（禁止性条款） | L1 |
| `SemanticRiskAgent` | 语义风险识别（隐性歧视、模糊表述） | L1 |
| `ScoringAgent` | 评分标准合理性分析 | L2 |
| `DemandAgent` | 需求合理性分析 | L2 |
| `ContractAgent` | 合同条款风险评估 | L2 |
| `BlindSpotAgent` | 盲点扫描（遗漏风险） | L3 |
| `LegalVerifyAgent` | 法律合规深度审查 | L3 |
| `DebateAgent` | 对抗辩论（正反方交叉验证） | L3 |

### 审查层级

```
L1（批量快筛）──→ L2（定向深审）──→ L3（对抗验证）
   200 条/次         ~20 条/次          ~5 条/次
   秒级响应          分钟级             深度审查
```

### Coordinator 模式

当 `AIBID_COORDINATOR=1` 时，启用 Coordinator 智能路由：
- 分析每个 clause 的特征
- 自动分配合适的 Agent 组合
- 聚合多 Agent 结果并去重

### 完整审核流水线

```
上传文档
  └→ PDF 解析 (pdfplumber/orbison)
     └→ Section 划分
        └→ Chunk 分割 (512-1024 字符)
           └→ 向量嵌入 (BGE-M3 1024d)
              └→ POST /review
                 └→ Coordinator 路由
                    ├→ L1 批量筛查 (6 Agent)
                    │   └→ 疑似风险 → L2
                    │       └→ 定向深审 (4 Agent)
                    │           └→ 高风险 → L3
                    │               └→ 对抗验证 (3 Agent)
                    └→ SSE 实时推送进度
                       └→ 最终结果 JSON
```
