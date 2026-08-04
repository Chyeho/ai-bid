# 知识库组-法规知识库入库功能 + 共享 Qdrant 模块交付总结

> 本文档面向后续接入的**知识库组**（读侧）与 **Java 组**（入库触发），说明已交付的共享 Qdrant 实例如何接入，以及入库功能的完整契约。本文档为最终交付说明，不依赖其他执行文档。

## 1. 改动清单

### 新增 3 个文件（backend-rust）

| 文件 | 职责 |
|---|---|
| `src/services/qdrant_store.rs` | 共享 Qdrant 接入层（入库写 + 检索读），定义字段契约 `KnowledgePayload` |
| `src/services/knowledge_ingest_service.rs` | 入库服务：PDF提取 → 章节化 → 切分 → 嵌入 → upsert |
| `src/api/knowledge_handlers.rs` | `POST /api/v1/knowledge/ingest` 接口 |

### 修改 6 处

| 文件 | 修改 |
|---|---|
| `backend-rust/Cargo.toml` | 新增 `qdrant-client = "1"`；uuid 启用 `v5` 特性 |
| `backend-rust/src/services/mod.rs` | 注册 `qdrant_store` / `knowledge_ingest_service` |
| `backend-rust/src/api/mod.rs` | 注册 `knowledge_handlers` |
| `backend-rust/src/api/router.rs` | 挂载 `/api/v1/knowledge/ingest` 路由 |
| `backend-java/.../docker-compose.yml` | Milvus 4 服务 → Qdrant（MySQL/Redis 保留，doc-converter 注释） |
| `backend-java/.../sql/smart_tender.sql` | `knowledge_file` 表新增 `applicable_scope` 列（对齐实体） |

> 红线约束：未改动 `frontend/`、`backend-java` 代码、`agents/`、既有 `handlers.rs` 逻辑、既有 services/domain/metrics/paths/server。本任务只做"新增"，未改动既有功能。

## 2. 共享 Qdrant 实例（知识库组必读）

### 连接信息

| 项 | 值 |
|---|---|
| collection 名 | `legal_kb`（常量 `KB_COLLECTION`） |
| 向量维度 | `1024`（BGE-M3 / text-embedding-v4，常量 `KB_VECTOR_DIM`） |
| 距离 | Cosine |
| gRPC 地址 | `http://localhost:6334`（`QDRANT_URL` 环境变量可覆盖） |
| REST/Dashboard | `http://localhost:6333` |

### 字段契约 `KnowledgePayload`（入库与检索共用的 11 个字段，一个都不能少）

| 字段 | 类型 | 说明 |
|---|---|---|
| `document_id` | String | 入库生成的 UUID（Java 侧应回写 MySQL） |
| `document_name` | String | 文件名 |
| `category` | String | `regulation` / `price` / `supplier` / `contract` / `case` / `other` |
| `applicable_scope` | String | `procurement` / `engineering` / `general` |
| `chunk_id` | String | 分块 ID，格式 `ch_042`（切分模块生成） |
| `section_path` | Vec\<String\> | 章节层级路径（根到当前节点的标题链，摘要回显用） |
| `embed_text` | String | 携带章节层级前缀的嵌入文本（搜索时作摘要返回） |
| `text_len` | usize | 文本长度 |
| `page_start` / `page_end` | usize | 起止页码（0-based） |
| `ingested_at` | String | RFC3339 入库时间 |

### Point ID 规则（重要）

- 由 `(document_id, chunk_id)` 用 **UUID v5** 确定性派生，全局唯一、幂等可重试。
- **不要**用 `{document_id}_{chunk_id}` 拼接格式 —— qdrant-client 1.x 会把字符串直接当 UUID 发送，Qdrant 会报 "Unable to parse UUID"。
- 检索侧无需关心 Point ID，按 payload 字段过滤即可。

### 检索用法（`QdrantStore::search`）

```rust
use crate::services::qdrant_store::{QdrantStore, KB_COLLECTION};

let store = QdrantStore::from_env()?;
store.ensure_collection().await?; // 幂等，检索前可安全调用
// query_vector 必须已 L2 归一化（复用 EmbeddingClient::encode_queries，与入库侧同源）
let hits = store
    .search(query_vector, top_k, Some("regulation"), Some("engineering"))
    .await?;
// hits: Vec<(f32 score, KnowledgePayload)>，score 为余弦相似度
```

- 可选过滤 `category` / `applicable_scope` 传 `None` 即不过滤。
- 按 `document_id` 删除向量：`store.delete_by_document(document_id).await?`。
- 集合向量总数：`store.count().await?`（测试/监控用）。

### 检索工具命名建议（知识库组成员 D2）

- 工具名建议 **`search_regulation`**，description 写清"检索已入库的法规/标准库文件"。
- 参数：`query / top_k / category? / applicable_scope?`（后两个直接透传 `search` 的过滤参数）。

### 其他共享方法（`QdrantStore`）

| 方法 | 签名 | 用途 |
|---|---|---|
| `from_env` | `() -> Result<Self>` | 按 `QDRANT_URL` 建客户端 |
| `ensure_collection` | `async () -> Result<()>` | 幂等创建 `legal_kb` |
| `upsert_chunks` | `async (Vec<KnowledgePayload>, Vec<Vec<f32>>) -> Result<()>` | 批量写入（每批 128 条） |
| `search` | 见上 | 语义检索 |
| `delete_by_document` | `async (&str) -> Result<()>` | 按文档删除 |
| `count` | `async () -> Result<u64>` | 向量总数 |

## 3. 入库管线（复用现有模块，未重新实现）

`ingest_bytes(file_bytes, filename, category, applicable_scope) -> Result<IngestResult>` 逐段复用既有能力：

1. **落盘临时文件**（`data_path_str("tmp")`，UUID 命名）
2. **DOCX/DOC → PDF**：复用 `docx_convert_service::convert_docx_to_pdf`（需本机 LibreOffice）
3. **PDF → RawDocument**：复用 `pdf_extract_service::extract_pdf_to_raw_json`（Rust 主解析），失败自动切 `extract_with_python`（Python 兜底）
4. **章节化 + 表格处理**：复用 `sectionize_service`（sectionize / detect_pipe_tables / merge_cross_page_tables / inject_tables_into_sections），含 orphan 块按连续页码分组兜底
5. **切分**：复用 `chunking_service::chunk_sections` + `populate_bbox_refs`（`ChunkingConfig::default()`）
6. **嵌入**：按 `EMBED_ENGINE` 切换 —— `remote` 用 `embedding_api_client` + `embed_chunks_remote`（**内部已自动脱敏**：先 `desensitize_service::desensitize` 再调 DashScope）；`local` 用 `embed_chunks_parallel`
7. **写库**：组装 `KnowledgePayload` → `store.upsert_chunks`（替代标书链路的 save_index 落盘）

## 4. 入库 HTTP 接口（Java 组必读）

### 接口定义

```
POST http://127.0.0.1:3001/api/v1/knowledge/ingest
Content-Type: multipart/form-data
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `file` | 是 | 法规文件（PDF / DOCX / DOC） |
| `category` | 否 | 默认 `regulation` |
| `applicable_scope` | 否 | 默认 `general` |
| `document_name` | 否 | 显示名，缺省用文件名 |

### 成功返回（200）

```json
{
  "document_id": "634bb42d-...",
  "document_name": "某法规.pdf",
  "chunk_count": 264,
  "dimension": 1024,
  "collection": "legal_kb",
  "elapsed_ms": 22334,
  "message": "入库成功"
}
```

### 失败返回

- **400**（缺文件等参数错误）：
  ```json
  { "error": "上传文件为空", "detail": "上传文件为空" }
  ```
- **500**（解析/嵌入/Qdrant 任一环节失败）：
  ```json
  { "error": "入库失败: {详细原因}", "detail": "入库失败: {详细原因}" }
  ```

### curl 手动验证

```bash
curl -X POST http://localhost:3001/api/v1/knowledge/ingest \
  -F "file=@某法规.pdf" -F "category=regulation" -F "applicable_scope=general"
```

### 建议（Java 组待接线，不在本次交付范围）

1. `KnowledgeFileServiceImpl.uploadFile` 落库后，**异步**（不要阻塞上传响应）调用上面接口触发入库；Rust 端是同步模式（单次约 20s，取决于文件大小与嵌入 API），Java 侧建议用 `@Async` / 消息队列，并让上传接口先返回"文件已保存"。
2. 把返回的 `document_id` 回写 `knowledge_file` 表（需新增 `vector_id` / `vector_status` 字段，由 Java 组自建）。
3. 删除标准库文件时，需同步删除 Qdrant 向量 —— Rust 已提供 `delete_by_document`，但**目前没有 HTTP 接口包装**，需 Java 组与 Rust 组协商补一个 DELETE 接口。
4. 检索侧如需 HTTP 接口（如语义搜索法规），同样尚未包装，由知识库组/Java 组后续添加。

## 5. 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `QDRANT_URL` | `http://localhost:6334` | Qdrant gRPC 地址（**必须是 6334 gRPC，不是 6333 REST**） |
| `EMBED_ENGINE` | `local` | 嵌入引擎；本机推荐 `remote`（DashScope，需 `DASHSCOPE_API_KEY`），local 需下载 BGE-M3 ONNX 模型 |
| `DASHSCOPE_API_KEY` | — | remote 嵌入必需 |
| `AIBID_DATA_DIR` | `.` | 数据根目录；从 `backend-rust/` 运行时临时设为 `..` |

> ⚠️ **EMBED_ENGINE 入库与检索两侧必须一致**（都 `remote` 或都 `local`），否则入库向量与检索 query 落在不同语义空间，召回会全部错乱。

## 6. 测试与验收

### 新增单元测试（14 个）

| 文件 | 用例数 |
|---|---|
| `qdrant_store.rs` | 4（Point ID 合法性/确定性、payload JSON 往返、payload 必含字段、字段契约完整） |
| `knowledge_ingest_service.rs` | 4（IngestResult 字段完整、空 chunk 正常返回、扩展名解析、递归 block_id 收集） |
| `knowledge_handlers.rs` | 6（multipart 默认值/缺文件/默认文件名、bad_request/server_error 结构、IngestResponse 组装） |

### 验收结果

| 验收标准 | 结果 |
|---|---|
| `cargo build` 无错误 | ✅ |
| `cargo test` 全部通过（新增 ≥6） | ✅ 新增 14 个全绿；完整跑 261 passed，仅 1 个既有失败 `test_find_soffice`（本机未装 LibreOffice，与本次改动无关） |
| curl 导入返回 `chunk_count > 0` | ✅ 实测 264 |
| Qdrant Dashboard 可见 `legal_kb` 及向量 | ✅ |
| scroll payload 字段完整（document_id / chunk_id / section_path / embed_text / category / applicable_scope 等 11 字段） | ✅ |
| 重复导入同一文件生成不同 `document_id`（不覆盖） | ✅ |
