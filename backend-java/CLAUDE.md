# backend-java — Smart Tender 业务网关

Spring Boot 3.x 轻量业务网关，Java 17+，MyBatis-Plus ORM，MySQL 8，Redis。

## 架构定位

Java 层是**薄网关**，不承载 AI 逻辑：
- 认证/授权、CRUD、文件管理 → Java
- AI 审核、RAG 对话、语义搜索 → 透明代理到 Rust 引擎 (:3001)

## 包结构（7 个顶级包）

```
com.ithsd.smart_tender/
├── config/          # Spring 配置（5 文件）
│   ├── AsyncConfig, MybatisPlusConfig, MyMetaObjectHandler
│   ├── RustApiProperties, WebMvcConfiguration
├── common/          # 横切组件（合并了 context/exception/interceptor/handler/util）
│   ├── BaseContext          # ThreadLocal 用户 ID
│   ├── BizException         # 统一业务异常
│   ├── GlobalExceptionHandler  # @RestControllerAdvice
│   ├── JwtTokenAdminInterceptor
│   ├── typehandler/         # MyBatis TypeHandler（JSON List）
│   └── util/                # JwtUtil, MD5Util, DocxToPdfConverter
├── controller/      # REST 控制器（9 个）
├── mapper/          # MyBatis-Plus Mapper（10 个）
├── model/           # 领域模型
│   ├── dto/         # 请求 DTO（+ rust/ 子包：Rust API 专用 DTO）
│   ├── entity/      # 数据库实体（全部 MyBatis-Plus）
│   ├── enums/       # 枚举
│   ├── result/      # Result<T>, PageResult<T>
│   └── vo/          # 响应 VO
├── service/         # 业务逻辑
│   ├── *Service         # 接口（10 个）+ ChatService（具体类）
│   ├── impl/            # 实现（10 个）
│   └── engine/          # Rust 代理 + 任务队列
│       ├── queue/       # 审核任务调度（Redis List/Stream/Async）
│       └── rust/        # RustApiClient, RustSseClient, RustDocumentService
└── sse/             # SSE 实时推送（5 文件）
    ├── SseHub             # ConcurrentHashMap<SseEmitter>
    ├── AuditTaskEventService  # 事件持久化 + replay
    ├── RedisSseConnectionStateStore
    ├── ReplaySseEvent
    └── AuditSseProperties
```

## 关键流程

### 1. 认证链路
```
JwtTokenAdminInterceptor.preHandle()
  → 解析 Authorization: Bearer <token>
  → BaseContext.setCurrentId(userId)
  → afterCompletion() 清理 ThreadLocal
```
排除路径：`/api/auth/login`, `/api/auth/register`, `/api/audit-tasks/callback`

### 2. 审核任务生命周期
```
POST /api/audit-tasks {bidId, enabledAgents}
  → AuditTaskServiceImpl.createTask()
    → INSERT audit_task (status=PENDING)
    → afterCommit: AuditTaskDispatcher.dispatch(taskId)
      → [Async/Redis] AuditEngineServiceImpl.start(taskId)
        → Stage 1: upload doc to Rust (幂等)
        → Stage 2: RustSseClient.connect() → POST /review (202 Accepted)
        → Stage 3: relay SSE events → SseHub → frontend
        → Stage 4: on "done" → GET /result → emit COMPLETE SSE
```

### 3. Rust 透明代理
- `RustApiClient`: JDK HttpClient，同步调用 Rust API
- `RustSseClient`: 监听 Rust SSE 流，回调 BiConsumer
- `RustDocumentService`: 管理 Java Tender ↔ Rust document_id 映射

### 4. 任务队列（Strategy 模式）
- `AuditTaskDispatcher` 接口，3 种实现：
  - `AsyncAuditTaskDispatcher`（默认，@Async 线程池）
  - `RedisListAuditTaskDispatcher` + `RedisListAuditTaskWorker`（BLPOP 轮询）
  - `RedisStreamAuditTaskDispatcher` + `RedisStreamAuditTaskWorker`（Consumer Group + DLQ）

## 数据库

- ORM: 纯 MyBatis-Plus（JPA 已移除）
- 连接池: Druid
- 迁移: Flyway（默认禁用）
- 关键表: `sys_user`, `project`, `bid_document`, `audit_task`, `audit_task_event`, `knowledge_file`, `knowledge_chunk`, `chat_message`, `audit_issue`, `audit_report`
- JSON 列（`audit_task.enabled_checks`, `audit_task.failed_stages`）由 `StringListJsonTypeHandler` 处理
- 乐观锁: `audit_task.version` + MyBatis-Plus `@Version`

## 构建运行

```bash
# 编译
mvn clean package -DskipTests

# 运行
java -jar target/smart_tender-0.0.1-SNAPSHOT.jar

# 前提
MySQL 8 (smart_tender_system) + Redis (db 10) + Rust 引擎 (:3001)
```

## 编码约定

- **接口/实现**: 所有 Service 遵循 `XxxService` 接口 + `impl/XxxServiceImpl` 模式（ChatService 例外：叶子节点，无多态需求）
- **统一响应**: Controller 返回 `Result<T>` 或 `PageResult<T>`
- **异常处理**: 业务异常抛 `BizException(code, msg)`，由 `GlobalExceptionHandler` 统一处理
- **Entity**: Lombok `@Data @Builder @NoArgsConstructor @AllArgsConstructor`
- **单文件不单独开包**: 不够 3 个文件的共性，不配独立包
