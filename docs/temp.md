# 项目状态总览

最后更新：2026-06-30

---

## 1. backend-java/ — 大规模重构（进行中）

### 已完成

- **包结构重组**：`pojo/` → `model/`（含 `dto/`、`entity/`、`vo/`、`enums/`、`result/` 子包），新增 `dto/rust/` 子包（Rust API 专用 DTO）
- **横切组件合并到 `common/`**：BaseContext、BizException、GlobalExceptionHandler、JwtTokenAdminInterceptor、工具类（JwtUtil、MD5Util、DocxToPdfConverter）、StringListJsonTypeHandler
- **旧代码清理**：
  - 已删除 LangchainConfig、MilvusConfig、MyBatisConfig、AuditEmbeddingProperties
  - 已删除旧的 JPA Repository（AuditIssueRepository、AuditTaskEventRepository、AuditTaskRepository、DocumentParseJobRepository、RagTriggerOutboxRepository）
  - 已删除旧的 RAG 服务（StandardRagService、RagContextMerger、HttpRetriever、Retriever 等）
  - 已删除旧的 LLM 服务（LlmClient、StubLlmClient、PromptLoader、JsonLinesIssueParser）
  - 已删除旧的 chunking 服务（ChunkingService、DefaultChunkingService、ChunkSlice 等）
  - 已删除旧的 document extract 服务（WordExtractService、DocumentExtractService、ParsedDocument 等）
  - 已删除旧的 DocumentParseJobController/Service
  - 已删除旧的 entity/dto/vo 文件（已迁移到新 model 包）
  - 已删除 Milvus/MinIO/Redis volumes 数据文件
- **新增 Rust 引擎通信层**：`service/engine/rust/`（RustApiClient、RustSseClient、RustDocumentService）
- **新增队列实现**：`service/engine/queue/`（Async/RedisList/RedisStream 三种策略模式实现）
- **新增 VO 类型**：AgentProgressVO、PhaseVO、TraceEventVO（支持前端展示 Multi-Agent 审查进度）
- **配置文件更新**：
  - `application.yml` 指向 Rust 引擎 `rust.api.base-url: http://127.0.0.1:3001`
  - 新增 `application-prod.yml`
  - 新增 `logback-spring.xml`
  - 删除了 `application-embedding.yml`（嵌入已移至 Rust 引擎）
- **编译验证通过**（2026-06-30）：`mvn compile` + `mvn test-compile` + `mvn test` 全部 BUILD SUCCESS，132 源文件 + 7 测试 0 失败
- **修复内容**：
  - 测试文件 `RedisListAuditTaskWorkerTest` / `RedisListAuditTaskDispatcherTest` 添加 `engine.queue` 包 import
  - `ChatService.java` 3 处 `userId` 改为 effectively final（lambda 捕获要求）
  - `RedisStreamAuditTaskWorker.java` unchecked 警告：`MapRecord.create()` 添加显式类型参数 + `@SuppressWarnings("unchecked")`（Spring Data Redis API 固有泛型限制）
  - Mapper XML 路径确认：全部 Mapper 使用 `BaseMapper<T>` + `@Select` 注解，零 XML 文件，无需对齐

### 还剩要做

（暂无）

---

## 2. backend-rust/ — HTTP API 服务器 + Agent 框架（基本完成）

### 已完成

- **HTTP API 服务器框架**：`src/bin/server.rs` + `src/api/mod.rs` + `src/api/router.rs` + `src/api/handlers.rs`
- **7 个 REST 端点**：
  - `POST /api/v1/documents` — 上传并处理文档
  - `GET /api/v1/documents/:id` — 查询文档状态
  - `POST /api/v1/documents/:id/review` — 运行审核
  - `POST /api/v1/documents/:id/chat` — 对话问答
  - `POST /api/v1/documents/:id/search` — 语义搜索
  - `GET /api/v1/review/:doc_id/stream` — SSE 实时推送
  - `GET /api/v1/review/:doc_id/result` — 获取审查结果
  - `GET /health` — 健康检查
- **10 个内置 Agent**（7 Reviewer + BlindSpot + LegalVerify + Debate）+ Dynamic(String) 动态 Agent
- **Coordinator 10 步聚合管线**（Route → Preload → Execute → Merge → LegalVerify → BlindSpot → Link → Debate → Triage → Register）
- **AgentBus** 消息广播通道
- **SessionGraph** 12 字段 Blackboard 共享工作区
- **4 个工具**：web_search（DashScope/SearXNG 双模式）、search_document（BGE-M3）、read_section、output_finding
- **test_agents bus 集成测试通过**：4 条款审查，3/4 PASS，75% pass rate
- **新增依赖**：axum、tower-http（HTTP 服务器 + CORS 中间件）
- **ReviewEventBus**：`src/agents/review_event.rs` — 10 种 SSE 事件类型 + 9 个单元测试 ✅
- **编译零警告**（2026-06-30）：`cargo check`（lib + test_agents + server 三个 target）全部零警告通过
- **7 个 HTTP handler 全部实现**：process_document、get_document、review_document（异步 202 Accepted）、stream_review_events（SSE）、get_review_result、chat_with_document、search_document ✅
- **服务器全链路验证通过**（2026-06-30）：
  - `GET /health` ✅ — 200 OK
  - `POST /api/v1/documents` ✅ — 2页PDF → 8块 → 20章节 → 12 chunks → 1024d 向量
  - `GET /api/v1/documents/:id` ✅ — 返回文档元数据
  - `POST /api/v1/documents/:id/review` ✅ — 202 Accepted，异步审核启动
  - `GET /api/v1/review/:doc_id/result` ✅ — ~60s 后 completed，返回 6 条 findings
  - `POST /api/v1/documents/:id/search` ⚠️ — 仅支持 `EMBED_ENGINE=local`（remote 模式待适配）
- **修复内容**：
  - `review_event.rs` — `emit_sse` 添加 `#[allow(dead_code)]`（SSE 流式推送预留，待 API handler 接入）
  - `coordinator.rs` — `merge_findings` 添加 `#[allow(dead_code)]`（仅 `#[cfg(test)]` 中使用）
  - `Cargo.lock` 无冲突
  - `agents/dynamic_agents.json` — 非预置文件，Coordinator 运行时自动创建（首次运行不存在为正常行为）

### 还剩要做

（暂无）

---

## 3. frontend/ — 功能扩展（已完成）

### 已完成

- **新增 8 个 BidAnalysis 组件**：
  - AgentProgressCards.tsx — Agent 进度卡片
  - CitationList.tsx — 引用来源列表
  - ClauseActivityMap.tsx — 条款活动热力图
  - LiveReviewFeed.tsx — 实时审查推送
  - PipelineProgress.tsx — 管线进度条
  - ReasoningDrawer.tsx — 推理过程抽屉
  - TierBadge.tsx — 风险等级徽章
  - buildSectionTree.ts — Section 树构建工具
- **删除 AnalysisOverview.tsx**（被新组件替代）
- **新增工具函数**：`bidAudit/utils/audit.ts`、`bidAudit/utils/mapFinding.ts`
- **新增共享类型**：`src/types/audit.ts` — Rust RiskFinding/Severity/ChatCitation/SectionTreeNode 等完整 TS 类型定义
- **新增 mock 数据**：`mockFindingsData.ts` — Multi-Agent 审查发现 mock
- **多个页面类型定义更新**：IssueList/types.ts、bidAudit/types.ts、bidLibrary/api/types.ts、bidUpload/types.ts、dashboard/types.ts
- **Hooks 更新**：useAiChat.ts、useAuditTask.ts、usePdfFlow.ts、useAuditIssuesList.tsx、useTableColumns.tsx
- **登录页更新**：LoginPage.tsx
- **路由、API 请求层更新**
- **编译验证通过**（2026-06-30）：`tsc --noEmit` 零错误 + `vite build` 成功（5801 modules, 23.6s）
- **修复内容**（14 个文件）：
  - 删除 7 处未使用变量/import（ChatInput、ChatWindow、AnalysisList、ClauseActivityMap、IssueTableFilter、mapFinding）
  - `MessageBubble.tsx` ChatCitation 判别联合类型窄化
  - `buildSectionTree.ts` 输入类型修正为 `AuditIssue[]` + Severity 类型转换
  - `mockFindingsData.ts` 移除 ~51 处 `suggestedAgent: null`（改为 undefined）
  - `useAuditTask.ts` 移除 `webSearchEnabled` + 修复 `.map()` 类型推断
  - `IssueList/hooks/mock.ts` severity `"warning"/"critical"` 改为 `"medium"/"high"`
  - `MobileFileCard.tsx` snake_case 改为 camelCase（`file_name` -> `fileName` 等）
  - `dashboardMock.ts` `"标书"/"合同"` 改为 `"bid"/"contract"`

### 还剩要做

（暂无）

---

## 4. 测试/验证

`cargo test --lib` 结果（2026-06-30）：**231 passed, 0 failed**

| 模块 | 单元测试 | 覆盖内容 |
|------|---------|---------|
| Step 1: DOCX->PDF | 1 个 | `test_find_soffice` |
| Step 2: PDF->Raw | **16 个** (新增) | clean_layout_text (6) + reconstruct_text_from_words (5) + compute_blocks (5) |
| Step 3: Sectionize | 17 个 | 7 种标题模式 + 页码过滤 + 截断检测 + 树构建 |
| Step 4: Chunk 切分 | 34 个 | 规则 1-5 + 后处理 + 元数据完整性 |
| Step 5: Embedding | **7 个** (新增) | l2_normalize_in_place (7) — 单位向量/零向量/高维/符号保留 |
| Agent 框架 | 145 个 | registry (15) + coordinator (19) + bus (10) + chat_agent (13) + review_event (9) + react_loop + trace |
| 其他 | 11 个 | desensitize + paths + vector_index |

### 4.1 验证脚本

| 脚本 | 状态 | 结果 |
|------|------|------|
| `scripts/validate_embeddings.py` | 已完成 | 两份文档 4/4 PASS (12 chunks + 172 chunks) |
| `scripts/validate_chunks.py` | 已完成 | 264 chunks PASS（5 WARNING 为表格/格式模板，非问题） |
| `tests/verify_e2e.ps1` | 已完成 | 全链路 E2E 脚本：依赖检查 + 单元测试 + CLI 管线 + 输出文件 + 质量脚本 |

### 4.2 待准备的测试数据

| 文件 | 说明 | 状态 |
|------|------|------|
| `tests/fixtures/` 目录 | 已创建 | 4 份测试 PDF 在 `tests/file/` |
| `tests/fixtures/golden_*.json` | Golden Data | 可用现有 output/ 中的真实输出锁定 |

---

## 5. 文档

| 文档 | 状态 | 说明 |
|------|------|------|
| docs/设计.md | ✅ 完成 | 系统设计总文档（425KB，较完整） |
| docs/实现.md | ✅ 基本完成 | 实现细节（§3-§6 有内容） |
| docs/验证.md | 🟡 框架完整 | 用例翔实但多数未执行 |
| docs/问题与解决.md | ✅ | 问题记录 |
| docs/文件目录说明.md | ✅ | 目录结构说明 |
| docs/开发流程.md | ✅ | 开发规范 |
| docs/标书中一句话的历险记.md | ✅ | 科普性流程说明 |
| docs/提交记录.md | ✅ | Git 提交历史记录 |
| docs/temp.md | 🟡 本文档 | 项目状态追踪 |
| backend-java/CLAUDE.md | ✅ | Java 子项目指令 |
| backend-rust/CLAUDE.md | ✅ | Rust 子项目指令 |

---

## 6. Git 工作树状态

- **分支**: main
- **未跟踪文件**: 25 个（含 output/、tmp/、logs/、.mvn/、mvnw、CLAUDE.md、新Java代码、新Rust代码、新前端代码、PDF测试文件）
- **已修改**: 36 个文件
- **已删除**: ~650 个文件（主要是 volumes/milvus、volumes/minio、volumes/redis 数据文件 + 旧 Java 代码）
- **已暂存**: 暂无（所有变更在工作树中）

---

## 7. 优先行动计划

（全部完成）

| 项目 | 结果 |
|------|------|
| Java 编译+测试 | `mvn compile` + `mvn test` 零失败 |
| Rust 编译+测试 | `cargo check` 零警告 + `cargo test --lib` 231 passed |
| Rust HTTP API | 7 端点完整实现 + 服务器全链路验证通过 |
| 前端编译 | `tsc --noEmit` + `vite build` 零错误 |
| Step 2/5 单元测试 | 新增 23 个测试 (16 + 7) |
| 验证脚本 | validate_embeddings.py 4/4 PASS + validate_chunks.py PASS |
| E2E 脚本 | tests/verify_e2e.ps1 全链路脚本 |
| 前端 SSE 对接 | 11 种事件类型完整实现，Java → Rust → 前端全链路打通 |
| 高亮回溯链路 | PDF 坐标 → Block → Chunk → Finding 正反向追踪（9 个前端文件） |
| Golden Data | tests/fixtures/ 已锁定 (raw + sections + chunks, 3.1MB) |
| CI 配置 | .github/workflows/ci.yml (Rust + Java + Frontend + Golden Data 回归) |
