# Phase 1 分组任务安排（第 1-2 周）

> 最后更新：2026-07-11

## 什么是 Phase

本项目分 4 个阶段交付，每个阶段称为一个 Phase：

| 阶段 | 周次 | 主题 | 核心交付 |
|---|---|---|---|
| **Phase 1** | W1-W2 | 基础建设 | 环境搭建 + 接口契约锁定 + Mock 数据 |
| **Phase 2** | W3-W5 | MVP 审招标文件 | 单文档合规审核全链路跑通 |
| **Phase 3** | W6-W8 | 产品扩展 | +审投标文件 + 知识飞轮闭环 |
| **Phase 4** | W9-W10 | 打磨上线 | 招投标对照审核 + 部署 |

**Phase 1 目标**：锁定接口契约 → 各组产出 Mock 数据 → 下游拿到就能独立开发。

---

## 全局时间线

| 节点 | 事件 |
|---|---|
| W1 周一 | 全员环境搭建完成（见《环境搭建指南》） |
| W1 周三 | G1 产出 Schema 初稿，G2/G3 开始适配 |
| W1 周五 | 各组组长汇报进度 |
| W2 周三 | 全部 Schema 存入 `docs/schemas/`，Mock 存入 `docs/mocks/` |
| W2 周五 | Phase 1 终检 |

---

## G1 文档解析（3人）— 阻塞 G2/G3

> G1 不交付 Schema，G2/G3 无法开工。优先配最强的人。

1. 找 3 份学校采购招标文件 PDF（办公设备 / 信息化 / 物业各一），作为后续全组共用测试样本
2. 研究 Docling + MinerU，跑通一份 PDF 的完整解析流程，记录遇到的坑
3. 定义 `parsed-document.schema.json`：章节层级 → 条款文本 → 表格结构 → 页码坐标 → 元数据。参考 [项目架构设计 §3.2](../项目架构设计.md) 中的 Schema 模板
4. 用 3 份 PDF 跑解析，产出 3 份 Mock JSON → `docs/mocks/sample-parsed-document-*.json`
5. W1 周三前交出 Schema 初稿给 G2/G3；W2 周三前 3 份 Mock 全部就绪

**人员**：1 人 PDF 解析引擎（Docling/MinerU） + 1 人 Schema 设计 + 1 人数据准备与验证。Python 为主。

---

## G2 规则匹配（2人）— 依赖 G1 Schema

> Phase 1 不写代码，做领域建模。规则引擎的难点是把法规转化为可执行规则，不是技术实现。

1. 阅读 [项目架构设计 §3.3](../项目架构设计.md) 中的规则 YAML 格式，理解 `industry` / `source` / `conditions` 三个核心字段的设计意图
2. 建立学校采购行业分类树（8 大类 → 子类）：
   - 办公设备 / 信息化建设 / 物业管理 / 教材图书
   - 实验室设备 / 食堂采购 / 校园工程 / 安保服务
3. 梳理核心法规清单（≥ 20 部）：法规全名 + 版本/修订年份 + 生效日期 + 官方来源 URL
4. 从法规中挑 5 条"可检查的条款"（如《政府采购法》第 5 条：不得以地域限制排斥潜在供应商），逐条写成 YAML 规则
5. 用 G1 的 Mock 数据手动验证这 5 条规则能否命中
6. 产出 `docs/schemas/rule-match.schema.json`

**人员**：1 人政策/法规研究 + 1 人 Rust/YAML。G1 Schema 出来前先用假设的数据结构开工。

---

## G3 知识检索（2人）— 依赖 G1 Schema

1. 启动向量库：docker-compose 中的 Milvus 已就绪，或另搭 Qdrant
2. 下载政府采购法 + 实施条例全文 → 按条款切片（200-800 字/片，保留条款编号+所属法规名）
3. 跑通 BGE-M3 向量化：先用 `EMBED_ENGINE=remote`（DashScope API）快速跑通，后续切换 `local`（ONNX）
4. 200 条切片向量化入库
5. 定义 `docs/schemas/evidence-set.schema.json`
6. 用 G1 Mock 数据的某条条款手动构造查询 → 验证检索能命中相关法条
7. 产出 1 份 Mock → `docs/mocks/sample-evidence-set.json`

**人员**：1 人向量库/嵌入模型 + 1 人数据抓取/清洗/切片。

---

## G4 Agent 智能（3人）— 不依赖任何人

> 现有 8000+ 行 Rust Agent 代码是你们的实验平台。Phase 1 目标：吃透代码，画出地图。

1. 跑通 `cargo test --bin test_agents -- --test all`，确认现有 Agent 框架能正常工作
2. 按调用链顺序通读代码，边读边画图：
   - `coordinator.rs` — 7 阶段 Pipeline 入口，先看懂 `review()` 的主链路
   - `react_loop.rs` — 单个 Agent 的 ReAct 循环（读条款→调工具→输出结论）
   - `session_graph.rs` — Blackboard 共享状态（条款→风险→法条的图关系）
   - `bus.rs` — Agent 间消息广播
   - `prompts.rs` + `types.rs` — 11 个 Agent 的 Prompt 和全部类型定义
3. 画出三张图：调用链路图（函数级别）、数据流图（ParsedDocument → RiskFindings）、Agent 生命周期图
4. 基于代码理解，写精简方案：11 Agent → 3 Agent（RuleAgent / JudgeAgent / CriticAgent），7 阶段 → 4 阶段（Route → Execute → Merge → Triage），明确砍掉 Scout / Debate / BlindSpot
5. 与 G5 对齐 `AgentTool` trait 签名（你不需要知道工具怎么实现，G5 不需要知道 Agent 怎么调度）

**人员**：3 人都需要强 Rust 能力（Tokio + async）。建议分工：1 人主攻 coordinator + bus、1 人主攻 react_loop + session_graph、1 人主攻 prompts + types + 画图。

---

## G5 领域工具（3人）— 与 G4 并行

> G5 让 Agent"懂行"。工具是 Agent 的手，Prompt 是 Agent 的脑。

1. 通读 `backend-rust/src/agents/tools/` 下全部工具代码，逐个理解用途和调用方式
2. 与 G4 对齐 `AgentTool` trait 签名，锁定接口即解耦
3. 筛选 MVP 工具清单：
   - **Phase 2 保留**：`search_knowledge` / `read_section` / `output_finding` / `validate_calculation`
   - **砍到 Phase 3**：`check_cross_reference` / `calculate_timeline` / `compare_with_template` / `extract_obligations` / `search_contradiction` / `search_document`
   - 每个砍掉的工具写一句话理由
4. 为 4 个保留工具各写 3 个场景用例（正常/边界/异常），在现有代码中跑通
5. 写 JudgeAgent System Prompt v1（针对学校采购招标文件）：
   - 角色定义 + 审查维度 + 证据使用规则 + 输出格式 + 2 个 Few-shot 示例
   - 存入 `backend-rust/src/agents/prompts/`

**人员**：1 人 Prompt 工程 + 1 人工具开发/测试 + 1 人领域知识（学校采购法规）。

---

## G6 知识沉淀（2人）— MVP 不进主链路

> Phase 1 只出设计，不写代码。

1. 阅读 [项目架构设计 §3.7](../项目架构设计.md) 中 Curator Pipeline 的完整设计
2. 设计 Neo4j 图 Schema：5 种节点（Law/Article/Case/Risk/ProhibitionRule）+ 6 种边（has_article/cited_in/found_in/exemplifies/similar_to/amended_by），输出 ER 图 + 节点属性表
3. 写 Curator Pipeline 技术方案：
   - Step 1 CuratorDedupAgent — 去重清洗策略
   - Step 2 CuratorGraphAgent + CuratorFreshAgent — 建图 + 保鲜策略
   - Step 3 CuratorEmbedAgent — 向量化写入策略
   - 触发方式（Cron）、容错机制、回滚策略
4. 与 G8 对齐 `session_snapshots` 表结构

**人员**：1 人图数据库设计 + 1 人 Pipeline 方案。

---

## G7 推理研究（1-2人）— 研究线，非阻塞

1. 从以下方向中选 1-2 个：
   - 模型量化（GPTQ/AWQ 4bit → 降低显存 70%）
   - KV Cache 复用（同文档多条款共享 KV）
   - 批量推理（Continuous Batching）
   - 模型蒸馏（Qwen-7B → 3B 标书专用 + LoRA）
2. 申请 GPU 资源（学校审批或云租用，RTX 4090 / A100）
3. 搭建 PyTorch + transformers + vLLM 实验环境
4. W2 末产出第一份技术调研报告（2000 字 + 实验数据）

**人员**：Python + PyTorch，有模型推理/微调经验优先。

---

## G8 SaaS 平台（3-4人）— 与 G9 并行

1. 确认已有 Spring Boot 项目能启动：`mvn spring-boot:run`，`/actuator/health` 返回 UP
2. 用已有代码跑通认证链路：注册 → 登录 → JWT 签发 → 拦截器验证 → 获取当前用户
3. 审查并补全数据库 Schema（已有部分表），输出 ER 图：
   - 用户 `sys_user` / 项目 `project` / 文件 `bid_document`
   - 任务 `audit_task` / 事件 `audit_task_event` / 问题 `audit_issue` / 报告 `audit_report`
4. 设计 REST API（结合已有 Controller 补全）：
   - 认证：`POST /api/auth/register` `/login` `/refresh`
   - 项目：`POST/GET /api/projects` `GET /api/projects/{id}`
   - 文件：`POST /api/files/upload` `GET /api/files/{id}`
   - 审核：`POST /api/audit-tasks` `GET /api/audit-tasks/{id}` `GET /api/audit-tasks/{id}/stream`
5. 配置 SpringDoc → `localhost:8086/swagger-ui.html` 可访问
6. W1 末给 G9 一份可调用的 Mock API（/auth/login 和 /projects 至少能用）

**人员**：Java 17 + Spring Boot 3 + MyBatis-Plus。至少 1 人主攻数据库设计。

---

## G9 前端（3-4人）— 与 G8 并行

1. `pnpm install && pnpm dev` 跑通现有前端
2. 搭建路由结构：
   - `/login` `/register` — 登录注册
   - `/dashboard` — 工作台首页
   - `/projects` `/projects/:id` — 项目列表/详情
   - `/projects/:id/audit/:tid` — 审核工作台（核心页面）
   - `/projects/:id/report/:rid` — 审核报告
3. 画审核工作台原型（最核心的页面）：
   - 左栏：PDF 文档预览区（先用 react-pdf 渲染示例 PDF）
   - 右栏：问题列表（Ant Design Table + 展开/折叠 + 按严重度筛选）
   - 左右联动：点击问题 → 左栏高亮对应位置
4. 跟 G8 对齐 API 格式 → 用 Mock 数据渲染页面（先不连真实后端）
5. 封装 SSE 客户端 Hook（`useAuditStream`），为 Phase 2 审核实时进度做准备

**人员**：React 18 + TypeScript + Ant Design 5。至少 1 人熟悉前端工程化（路由/状态管理/构建）。

---

## 组间依赖速查

```
G1 ──Schema──→ G2（并行）
G1 ──Schema──→ G3（并行，不等待 G2）
G4 ←──trait──→ G5（完全并行）
G8 ←──OpenAPI──→ G9（完全并行）
G6、G7 → 独立，不进主链路
```

## 人员分配

| 方案 | G1 | G2 | G3 | G4 | G5 | G6 | G7 | G8 | G9 | 合计 |
|---|---|---|---|---|---|---|---|---|---|---|
| 22 人 | 3 | 2 | 2 | 3 | 3 | 2 | 1 | 3 | 3 | 22 |
| 15 人 | 2 | 1 | G3+G6=2 | G4+G5=4 | — | 0 | 3 | 3 | 15 |
