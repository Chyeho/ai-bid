# G8 SaaS 平台 — Phase 1-4 完整推进

> 3-4 人 | Java 17 + Spring Boot 3 + MyBatis-Plus + PostgreSQL + Redis
> 服务 G9（前端）| 调用 G1（文档解析队列）| 接收 G4（审核回调）

---

## Phase 1：基础设施+认证模块（W1-W2）

1. 确认已有 Spring Boot 项目能启动：`mvn spring-boot:run`，`/actuator/health` 返回 UP
2. 用已有代码跑通认证链路：注册 -> 登录 -> JWT 签发 -> 拦截器验证 -> 获取当前用户
3. 审查并补全数据库 Schema（已有部分表），输出 ER 图：
   - 用户 `sys_user` / 项目 `project` / 文件 `bid_document`
   - 任务 `audit_task` / 事件 `audit_task_event`
   - 问题 `audit_issue` / 报告 `audit_report`
   - 知识 `knowledge_file` / `knowledge_chunk`
   - 聊天 `chat_message`
4. 设计 REST API（结合已有 Controller 补全）：
   - 认证：`POST /api/auth/register` `/login` `/refresh`
   - 项目：`POST/GET /api/projects` `GET /api/projects/{id}`
   - 文件：`POST /api/files/upload` `GET /api/files/{id}`
   - 审核：`POST /api/audit-tasks` `GET /api/audit-tasks/{id}` `GET /api/audit-tasks/{id}/stream`
5. 配置 SpringDoc -> `localhost:8086/swagger-ui.html` 可访问
6. W1 末给 G9 一份可调用的 Mock API

**Phase 1 交付物**：
- 数据库 ER 图 + 建表 SQL
- OpenAPI 文档
- 认证模块可运行

---

## Phase 2：审核任务全流程（W3-W5）

1. 文件上传：支持 PDF/Word，上传完成 -> MinIO 存储 -> 返回文件 ID
2. 审核任务 CRUD：创建任务 -> 推入 Redis Streams -> G1 消费解析 -> G4 消费审核
3. 结果回调：G4 审核完成 -> G8 接收回调 -> 更新任务状态 -> 通知前端
4. Rust 引擎代理：`RustApiClient` 同步调用 + `RustSseClient` SSE 流转发
5. Session 快照表：与 G6 协作建 `session_snapshots` 表，审核完成后 G4 写入 curation 候选数据
6. W3 末 API 就绪，供 G9 对接

**Phase 2 交付物**：
- 文件上传 + MinIO 集成
- 审核任务 CRUD + Redis Streams 队列
- Rust 引擎代理（SSE 转发）
- Session 快照表就绪

---

## Phase 3：多租户+产品化（W6-W8）

1. 多租户隔离：租户注册 -> 项目空间 -> 成员邀请/角色/权限
2. 用量统计：页数统计 / 审核次数 / 存储用量
3. 报告导出 API：PDF/Word 格式导出
4. 开放 API：API Key 管理 / 速率限制 / 调用统计（P2）
5. 订阅计费：套餐定义 / 订阅管理（P1，如时间允许）

**Phase 3 交付物**：
- 多租户完整流程
- 报告导出 API
- 用量统计
- 开放 API（P2）

---

## Phase 4：部署上线（W9-W10）

1. 端到端测试：50 并发用户注册->上传->审核->查看报告
2. 性能优化：API P99 < 500ms，50 并发不降级
3. 部署文档 + 运维手册
4. Docker Compose 生产环境编排
5. 健康检查 + 日志 + 监控 + 告警

**Phase 4 交付物**：
- 产品部署到服务器
- 部署文档 + 运维手册
- 性能达标

---

## 关键接口（供 G9 和后端对接）

```
POST   /api/auth/register          # 注册
POST   /api/auth/login             # 登录
POST   /api/auth/refresh           # Token 刷新
POST   /api/projects               # 创建项目
GET    /api/projects               # 项目列表
GET    /api/projects/{id}          # 项目详情
POST   /api/projects/{id}/members  # 邀请成员
POST   /api/files/upload           # 上传文件
GET    /api/files/{id}             # 文件信息
GET    /api/files/{id}/download    # 下载文件
POST   /api/audit-tasks            # 创建审核任务
GET    /api/audit-tasks/{id}       # 任务状态
GET    /api/audit-tasks/{id}/report  # 审核报告
GET    /api/audit-tasks/{id}/stream  # SSE 进度推送
```

---

## 人员分工建议

| 角色 | Phase 1-2 | Phase 3-4 |
|---|---|---|
| 数据库+认证 | Schema 设计 + JWT | 多租户隔离 |
| 文件+任务 | 文件上传 + MinIO + 审核 CRUD | 报告导出 + 用量统计 |
| Rust 代理 | RustApiClient + SSE 转发 | 性能优化 + 开放 API |
| （4 人时）部署 | Redis Streams 队列 | Docker 编排 + 运维文档 |
