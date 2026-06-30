//! HTTP 请求处理函数 — 薄胶水层。
//!
//! 不写业务逻辑，只负责：
//! 1. 解析 HTTP 请求（JSON / multipart）
//! 2. 调用现有核心函数（services / agents）
//! 3. 格式化 HTTP 响应（JSON + 状态码）

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};
use uuid::Uuid;

use crate::agents::bus::AgentBus;
use crate::agents::chat_agent::ChatAgent;
use crate::agents::coordinator::Coordinator;
use crate::agents::react_loop::{ChatMessage, LlmClient};
use crate::agents::registry::AgentRegistry;
use crate::agents::review_event::ReviewEventBus;
use crate::agents::session_graph::SessionGraph;
use crate::agents::tools::{
    answer_user::AnswerUserTool,
    output_finding::OutputFindingTool,
    read_section::ReadSectionTool,
    search_document::SearchDocumentTool,
    search_knowledge::{DashScopeSearchBackend, SearchKnowledgeTool},
    ToolRegistry,
};
use crate::agents::trace::TraceLog;
use crate::agents::types::{
    AgentId, ChatAgentConfig, ChatResponse, ChatStreamEvent, CoordinatorConfig, CoordinatorOutput,
    ReviewClause, TextSelection,
};
use crate::domain::chunk::{Chunk, ChunkingConfig};
use crate::domain::raw_document::RawDocument;
use crate::domain::vector_index::DocumentVectorIndex;
use crate::paths::data_path_str;
use crate::services::chunking_service::chunk_sections;
use crate::services::docx_convert_service::convert_docx_to_pdf;
use crate::services::embedding_service::EmbeddingClient;
use crate::services::llm_client::create_llm_client;
use crate::services::pdf_extract_service::{extract_pdf_to_raw_json, extract_with_python};
use crate::services::sectionize_service::{self, Section};

// ─── 应用状态 ───────────────────────────────────────────────────────

/// 服务全局共享状态。
#[derive(Clone)]
pub struct AppState {
    /// 文档缓存：document_id → 已处理文档
    pub documents: Arc<TokioRwLock<HashMap<String, Arc<DocumentState>>>>,
    /// 嵌入客户端（BGE-M3，启动时加载一次）
    pub embed_client: Arc<StdMutex<Option<Arc<EmbeddingClient>>>>,
    /// DashScope 联网搜索
    pub dashscope_search: Option<Arc<DashScopeSearchBackend>>,
    /// 搜索后端类型（dashscope / searxng）
    pub search_backend: String,
    /// 嵌入引擎类型（local / remote）
    pub embed_engine: String,
    /// SSE 实时推送通道：doc_id → ReviewEventBus
    pub review_event_buses: Arc<TokioMutex<HashMap<String, Arc<ReviewEventBus>>>>,
    /// 异步审查结果缓存：doc_id → CoordinatorOutput
    pub review_results: Arc<TokioMutex<HashMap<String, CoordinatorOutput>>>,
    /// 异步审查失败信息：doc_id → 错误消息
    pub review_errors: Arc<TokioMutex<HashMap<String, String>>>,
    /// 正在执行的审核任务：doc_id（用于并发控制，防止重复提交）
    pub active_reviews: Arc<TokioMutex<HashSet<String>>>,
}

use std::sync::Mutex as StdMutex;

/// 单个文档的处理状态。
pub struct DocumentState {
    pub id: String,
    pub filename: String,
    pub stem: String,
    pub raw_doc: RawDocument,
    pub sections: Vec<Section>,
    pub chunks: Vec<Chunk>,
    pub chunk_map: Arc<HashMap<String, Chunk>>,
    pub chunk_order: Arc<Vec<String>>,
    pub doc_index: Arc<DocumentVectorIndex>,
}

impl AppState {
    /// 初始化全局状态。
    pub async fn init() -> anyhow::Result<Self> {
        let embed_engine =
            std::env::var("EMBED_ENGINE").unwrap_or_else(|_| "local".to_string());

        let embed_client = {
            let client = EmbeddingClient::from_env()?;
            Some(Arc::new(client))
        };

        let search_backend =
            std::env::var("AIBID_SEARCH_BACKEND").unwrap_or_else(|_| "dashscope".to_string());

        let dashscope_search = if search_backend == "dashscope" {
            DashScopeSearchBackend::from_env().map(Arc::new).ok()
        } else {
            None
        };

        Ok(Self {
            documents: Arc::new(TokioRwLock::new(HashMap::new())),
            embed_client: Arc::new(StdMutex::new(embed_client)),
            dashscope_search,
            search_backend,
            embed_engine,
            review_event_buses: Arc::new(TokioMutex::new(HashMap::new())),
            review_results: Arc::new(TokioMutex::new(HashMap::new())),
            review_errors: Arc::new(TokioMutex::new(HashMap::new())),
            active_reviews: Arc::new(TokioMutex::new(HashSet::new())),
        })
    }
}

// ─── 请求/响应 DTO ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReviewRequest {
    #[serde(default)]
    pub chunk_ids: Vec<String>,
    #[serde(default)]
    pub max_clauses: Option<usize>,
    #[serde(default)]
    pub enabled_agents: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub user_input: String,
    #[serde(default)]
    pub selection: Option<TextSelection>,
    #[serde(default)]
    pub history: Option<Vec<ChatMessageDto>>,
    #[serde(default)]
    pub max_turns: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub queries: Vec<String>,
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ProcessResponse {
    pub document_id: String,
    pub filename: String,
    pub total_pages: usize,
    pub total_blocks: usize,
    pub total_sections: usize,
    pub total_chunks: usize,
    pub avg_chunk_size: f64,
    pub vector_count: usize,
    pub vector_dimension: usize,
}

#[derive(Debug, Serialize)]
pub struct DocumentInfo {
    pub document_id: String,
    pub filename: String,
    pub total_pages: usize,
    pub total_chunks: usize,
    pub vector_count: usize,
}

#[derive(Debug, Serialize)]
pub struct ReviewAccepted {
    pub status: String,
    pub document_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewResponse {
    pub document_id: String,
    pub findings: Vec<crate::agents::types::RiskFinding>,
    pub routing_summary: crate::agents::types::RoutingSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_snapshot: Option<crate::agents::types::GraphSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewResultResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ReviewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultGroup>,
}

#[derive(Debug, Serialize)]
pub struct SearchResultGroup {
    pub query: String,
    pub hits: Vec<SearchHitDto>,
}

#[derive(Debug, Serialize)]
pub struct SearchHitDto {
    pub chunk_id: String,
    pub title: String,
    pub score: f32,
    pub snippet: String,
    pub page_start: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub detail: String,
}

// ─── Handlers ───────────────────────────────────────────────────────

/// GET /health
pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

/// POST /api/v1/documents
pub async fn process_document(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ProcessResponse>, (StatusCode, Json<ErrorResponse>)> {
    println!("[REQ] 收到文件上传请求，开始解析 multipart...");

    let mut file_data: Vec<u8> = Vec::new();
    let mut filename = String::from("upload.pdf");

    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(name) = field.file_name() {
            filename = name.to_string();
        }
        if let Ok(data) = field.bytes().await {
            file_data = data.to_vec();
        }
    }

    if file_data.is_empty() {
        return Err(bad_request("上传文件为空"));
    }

    println!("[REQ] 收到文件上传: filename={}, size={} bytes", filename, file_data.len());

    let tmp_dir = data_path_str("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| {
        server_error("创建临时目录失败", e)
    })?;
    let stem = Uuid::new_v4().to_string();
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("pdf");
    let tmp_path = format!("{}/{}.{}", tmp_dir, stem, ext);
    std::fs::write(&tmp_path, &file_data).map_err(|e| {
        server_error("写入临时文件失败", e)
    })?;

    // DOCX → PDF 转换（对齐 CLI 行为）
    let pdf_path = if ext == "docx" || ext == "doc" {
        println!("[STAGE] DOCX → PDF 转换...");
        convert_docx_to_pdf(&tmp_path, &tmp_dir).map_err(|e| {
            server_error("DOCX 转 PDF 失败", e)
        })?
    } else {
        std::path::PathBuf::from(&tmp_path)
    };

    // 阶段 1: PDF → RawDocument（Rust 主路径 + Python 兜底）
    println!("[STAGE] PDF 文本提取...");
    let pdf_path_str = pdf_path.to_str().unwrap_or(&tmp_path).to_string();
    let raw_doc: RawDocument = match extract_pdf_to_raw_json(&pdf_path_str) {
        Ok(doc) => {
            println!("Rust pdfplumber 解析成功");
            doc
        }
        Err(e) => {
            println!("Rust pdfplumber 失败: {}", e);
            println!("切换到 Python pdfplumber 兜底提取...");
            let fallback_json = format!("{}/{}_python_fallback_raw.json", tmp_dir, stem);
            extract_with_python(&pdf_path_str, &fallback_json).map_err(|e2| {
                server_error("PDF 解析失败（Rust 和 Python 均失败）", e2)
            })?;
            let json_str = std::fs::read_to_string(&fallback_json).map_err(|e2| {
                server_error("读取 Python 兜底 JSON 失败", e2)
            })?;
            serde_json::from_str(&json_str).map_err(|e2| {
                server_error("解析 Python 兜底 JSON 失败", e2)
            })?
        }
    };
    println!("[STAGE] 提取完成: {} 页, {} 个文本块", raw_doc.pages.len(), raw_doc.pages.iter().map(|p| p.blocks.len()).sum::<usize>());

    // 构建磁盘输出用的 stem：{原始文件名}_{uuid前8位}
    let file_stem = std::path::Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let disk_stem = format!("{}_{}", file_stem, &stem[..8.min(stem.len())]);

    // ── 写盘：raw_json ──
    {
        let dir = data_path_str("output/raw_json");
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{}/{}_raw.json", dir, disk_stem);
        if let Ok(json) = serde_json::to_string_pretty(&raw_doc) {
            let _ = std::fs::write(&path, json);
            println!("[DISK] raw_json → {}", path);
        }
    }

    // 阶段 2: RawDocument → Sections
    let sections_output = sectionize_service::sectionize(&raw_doc);
    let mut raw_doc_mut = {
        // Re-serialize and deserialize to get a mutable copy
        // (RawDocument doesn't implement Clone)
        let json = serde_json::to_value(&raw_doc).map_err(|e| {
            server_error("序列化 RawDocument 失败", e)
        })?;
        serde_json::from_value(json).map_err(|e| {
            server_error("反序列化 RawDocument 失败", e)
        })?
    };
    sectionize_service::detect_pipe_tables(&mut raw_doc_mut);

    let assigned: HashSet<&str> = sections_output
        .sections
        .iter()
        .flat_map(|s| collect_all_block_ids(s))
        .collect();
    let orphan_blocks: Vec<&crate::domain::raw_document::RawBlock> = raw_doc_mut
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .filter(|b| !assigned.contains(b.id.as_str()))
        .collect();

    let mut all_sections = sections_output.sections.clone();
    if !orphan_blocks.is_empty() {
        let block_page: HashMap<&str, usize> = raw_doc_mut
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter().map(move |b| (b.id.as_str(), p.page_index as usize)))
            .collect();
        let mut page_to_blocks: BTreeMap<usize, Vec<&crate::domain::raw_document::RawBlock>> =
            BTreeMap::new();
        for block in &orphan_blocks {
            if let Some(&page_idx) = block_page.get(block.id.as_str()) {
                page_to_blocks.entry(page_idx).or_default().push(*block);
            }
        }
        let sorted_pages: Vec<usize> = page_to_blocks.keys().copied().collect();
        let mut page_groups: Vec<Vec<usize>> = Vec::new();
        let mut current_group: Vec<usize> = Vec::new();
        for &p in &sorted_pages {
            if current_group.is_empty() || p == current_group.last().unwrap() + 1 {
                current_group.push(p);
            } else {
                page_groups.push(std::mem::take(&mut current_group));
                current_group.push(p);
            }
        }
        if !current_group.is_empty() {
            page_groups.push(current_group);
        }
        for group in &page_groups {
            let group_start = *group.first().unwrap();
            let group_end = *group.last().unwrap();
            let group_blocks: Vec<&&crate::domain::raw_document::RawBlock> = group
                .iter()
                .flat_map(|p| page_to_blocks[p].iter())
                .collect();
            let orphan_ids: Vec<String> = group_blocks.iter().map(|b| b.id.clone()).collect();
            let orphan_text = group_blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            all_sections.push(Section {
                level: 0,
                title: format!("未归类内容 (第{}-{}页)", group_start + 1, group_end + 1),
                pattern: "orphan".to_string(),
                page_start: group_start,
                page_end: group_end,
                block_ids: orphan_ids,
                body_text: orphan_text,
                children: Vec::new(),
                body_page_start: group_start,
                body_page_end: group_end,
            });
        }
    }

    sectionize_service::merge_cross_page_tables(&mut raw_doc_mut);
    sectionize_service::inject_tables_into_sections(&mut all_sections, &raw_doc_mut);

    // ── 写盘：sections ──
    {
        let dir = data_path_str("output/sections");
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{}/{}_sections.json", dir, disk_stem);
        if let Ok(json) = serde_json::to_string_pretty(&all_sections) {
            let _ = std::fs::write(&path, json);
            println!("[DISK] sections → {}", path);
        }
    }

    // 阶段 3: Sections → Chunks
    println!("[STAGE] 章节 → 条款切分 ({} 个章节)...", all_sections.len());
    let chunking_config = ChunkingConfig::default();
    let mut chunks = chunk_sections(&all_sections, &chunking_config);
    crate::services::chunking_service::populate_bbox_refs(&mut chunks, &raw_doc);
    println!("[STAGE] 切分完成: {} 个条款块", chunks.len());

    // ── 写盘：chunks ──
    {
        let dir = data_path_str("output/chunks");
        let _ = std::fs::create_dir_all(&dir);
        let path = format!("{}/{}_chunks.json", dir, disk_stem);
        if let Ok(json) = serde_json::to_string_pretty(&chunks) {
            let _ = std::fs::write(&path, json);
            println!("[DISK] chunks → {}", path);
        }
    }

    // 阶段 4: Chunks → Embeddings
    println!("[STAGE] 生成嵌入向量 (引擎: {})...", state.embed_engine);
    let doc_index = if state.embed_engine == "remote" {
        let api_client = crate::services::embedding_api_client::EmbeddingApiClient::from_env()
            .map_err(|e| server_error("嵌入 API 客户端初始化失败", e))?;
        crate::services::embedding_service::embed_chunks_remote(
            &chunks,
            &chunking_config,
            &sections_output.document_id,
            &api_client,
        )
    } else {
        crate::services::embedding_service::embed_chunks_parallel(
            &chunks,
            &chunking_config,
            &sections_output.document_id,
            2,
        )
    }
    .map_err(|e| server_error("嵌入生成失败", e))?;

    let vector_count = doc_index.len();
    let vector_dimension = doc_index.embeddings.first().map(|v| v.len()).unwrap_or(0);

    // ── 写盘：embeddings ──
    {
        let dir = data_path_str("output/embeddings");
        if let Err(e) = crate::services::embedding_service::save_index(&doc_index, &dir, &disk_stem) {
            eprintln!("[DISK] embeddings 写入失败: {}", e);
        } else {
            println!("[DISK] embeddings → {}/{}_embedding_index/", dir, disk_stem);
        }
    }

    let chunk_map: HashMap<String, Chunk> = chunks
        .iter()
        .map(|c| (c.chunk_id.clone(), c.clone()))
        .collect();
    let chunk_order: Vec<String> = chunks.iter().map(|c| c.chunk_id.clone()).collect();

    let total_pages = raw_doc.pages.len();
    let total_blocks: usize = raw_doc.pages.iter().map(|p| p.blocks.len()).sum();
    let total_chars: usize = chunks.iter().map(|c| c.text.len()).sum();

    let doc_id = stem.clone();
    let doc_state = Arc::new(DocumentState {
        id: doc_id.clone(),
        filename: filename.clone(),
        stem,
        raw_doc,
        sections: all_sections,
        chunks: chunks.clone(),
        chunk_map: Arc::new(chunk_map),
        chunk_order: Arc::new(chunk_order),
        doc_index: Arc::new(doc_index),
    });
    state.documents.write().await.insert(doc_id.clone(), doc_state);

    let _ = std::fs::remove_file(&tmp_path);

    println!("[OK] 文档处理完成: doc_id={}, pages={}, chunks={}, vectors={}d",
        doc_id, total_pages, chunks.len(), vector_dimension);

    Ok(Json(ProcessResponse {
        document_id: doc_id,
        filename,
        total_pages,
        total_blocks,
        total_sections: sections_output.stats.total_sections,
        total_chunks: chunks.len(),
        avg_chunk_size: if chunks.is_empty() { 0.0 } else { total_chars as f64 / chunks.len() as f64 },
        vector_count,
        vector_dimension,
    }))
}

/// GET /api/v1/documents/:id
pub async fn get_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<DocumentInfo>, (StatusCode, Json<ErrorResponse>)> {
    let docs = state.documents.read().await;
    let doc = docs.get(&doc_id).ok_or_else(|| {
        not_found(&format!("文档不存在: {}", doc_id))
    })?;
    Ok(Json(DocumentInfo {
        document_id: doc.id.clone(),
        filename: doc.filename.clone(),
        total_pages: doc.raw_doc.pages.len(),
        total_chunks: doc.chunks.len(),
        vector_count: doc.doc_index.len(),
    }))
}

/// POST /api/v1/documents/:id/review
///
/// 启动异步 Multi-Agent 审查管线，立即返回 202 Accepted。
/// 审查在后台 Tokio task 中执行，通过 SSE (`GET /review/:doc_id/stream`)
/// 实时推送进度事件，完成后通过 `GET /review/:doc_id/result` 获取结果。
#[axum::debug_handler]
pub async fn review_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> Result<(StatusCode, Json<ReviewAccepted>), (StatusCode, Json<ErrorResponse>)> {
    let docs = state.documents.read().await;
    let doc = docs.get(&doc_id).ok_or_else(|| {
        not_found(&format!("文档不存在: {}", doc_id))
    })?.clone();
    drop(docs);

    println!("[REQ] 启动异步审核: doc_id={}, filename={}", doc_id, doc.filename);

    // 并发控制：检查是否已有进行中的审核（用 active_reviews 标记而非 bus 存在性）
    {
        let mut active = state.active_reviews.lock().await;
        if active.contains(&doc_id) {
            return Ok((
                StatusCode::CONFLICT,
                Json(ReviewAccepted {
                    status: "conflict".to_string(),
                    document_id: doc_id,
                    message: "该文档已有进行中的审核任务".to_string(),
                }),
            ));
        }
        active.insert(doc_id.clone());
    }

    // 创建或获取 ReviewEventBus（SSE 客户端可能已提前连接）
    let review_events = {
        let mut buses = state.review_event_buses.lock().await;
        buses
            .entry(doc_id.clone())
            .or_insert_with(|| Arc::new(ReviewEventBus::new(256)))
            .clone()
    };

    // 准备 clause 列表
    let chunking_config = ChunkingConfig::default();
    let max_clauses = req.max_clauses.unwrap_or(200);
    let review_clauses: Vec<ReviewClause> = doc
        .chunks
        .iter()
        .take(max_clauses)
        .map(|c| ReviewClause::from_chunk(c, chunking_config.embed_ctx_depth, chunking_config.embed_path_max_len))
        .collect();

    println!(
        "[REQ] 审核条款数: {} (上限 {}), 启用 Agent: {:?}",
        review_clauses.len(),
        max_clauses,
        req.enabled_agents
    );

    // 提取后台任务所需数据（脱离 doc 引用）
    let enabled_agents = req.enabled_agents.clone();
    let chunk_map = doc.chunk_map.clone();
    let doc_index = doc.doc_index.clone();
    let chunk_order = doc.chunk_order.clone();
    let dashscope_search = state.dashscope_search.clone();
    let search_backend = state.search_backend.clone();
    let embed_client_for_tools = {
        let ec = state.embed_client.lock().unwrap();
        ec.clone()
    };

    // 后台执行管线
    let state_for_task = state.clone();
    let doc_id_for_task = doc_id.clone();
    tokio::spawn(async move {
        run_review_pipeline(
            state_for_task,
            doc_id_for_task,
            review_clauses,
            enabled_agents,
            chunk_map,
            doc_index,
            chunk_order,
            dashscope_search,
            search_backend,
            embed_client_for_tools,
            review_events,
        )
        .await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(ReviewAccepted {
            status: "accepted".to_string(),
            document_id: doc_id,
            message: "审核任务已提交，通过 SSE 获取实时进度".to_string(),
        }),
    ))
}

/// 后台执行 Multi-Agent 审核管线。
///
/// 成功时：存储结果到 `review_results`，发送 Done SSE 事件。
/// 失败时：存储错误到 `review_errors`，发送 Error SSE 事件。
#[allow(clippy::too_many_arguments)]
async fn run_review_pipeline(
    state: AppState,
    doc_id: String,
    review_clauses: Vec<ReviewClause>,
    enabled_agents: Option<Vec<String>>,
    chunk_map: Arc<HashMap<String, Chunk>>,
    doc_index: Arc<DocumentVectorIndex>,
    chunk_order: Arc<Vec<String>>,
    dashscope_search: Option<Arc<DashScopeSearchBackend>>,
    search_backend: String,
    embed_client_for_tools: Option<Arc<EmbeddingClient>>,
    review_events: Arc<ReviewEventBus>,
) {
    let start_time = std::time::Instant::now();

    let bus = Arc::new(AgentBus::new(32));
    let graph = Arc::new(SessionGraph::new());
    let trace = Arc::new(TokioMutex::new(TraceLog::new()));

    let mut coord_config = CoordinatorConfig::default();
    if let Some(ref agent_names) = enabled_agents {
        coord_config.enabled_agents = agent_names
            .iter()
            .filter_map(|s| AgentId::from_str(s))
            .collect();
    }

    let llm_factory = Arc::new(move || create_llm_client().expect("创建 LLM 客户端失败"));

    let doc_index_for_tools = doc_index.clone();
    let chunk_map_for_tools = chunk_map.clone();
    let chunk_order_for_tools = chunk_order.clone();
    let ds_search = dashscope_search.clone();
    let sb = search_backend.clone();
    let ec_for_tools = embed_client_for_tools.clone();

    let tools_factory = Arc::new(move || {
        let mut registry = ToolRegistry::new();
        if let Some(ref ec) = ec_for_tools {
            registry.register(Box::new(SearchDocumentTool::new(
                doc_index_for_tools.clone(),
                ec.clone(),
            )));
        }
        registry.register(Box::new(ReadSectionTool::new(
            chunk_map_for_tools.clone(),
            chunk_order_for_tools.clone(),
        )));
        if sb == "dashscope" {
            if let Some(ref ds) = ds_search {
                registry.register(Box::new(SearchKnowledgeTool::with_dashscope(ds.clone())));
            }
        }
        registry.register(Box::new(OutputFindingTool));
        registry
    });

    let registry = AgentRegistry::builtin();
    let coordinator = Coordinator::new(
        coord_config,
        registry,
        llm_factory,
        tools_factory,
        bus,
        graph,
        trace,
    )
    .with_review_events(review_events.clone());

    println!("[STAGE] Multi-Agent 审核中 (async)...");
    match coordinator.review(&review_clauses).await {
        Ok(mut output) => {
            let duration_secs = start_time.elapsed().as_secs_f64();
            println!("[OK] 审核完成: {} 条风险发现, 耗时 {:.1}s", output.findings.len(), duration_secs);

            // 填充 location 字段 + block_ids（用于前端 bbox-based PDF 高亮）
            for finding in &mut output.findings {
                if let Some(first_clause_id) = finding.clause_ids.first() {
                    if let Some(chunk) = chunk_map.get(first_clause_id) {
                        finding.page_number = Some(chunk.page_start);
                        finding.section_path = Some(chunk.section_path.clone());
                        finding.context = Some(chunk.text.chars().take(500).collect());
                        finding.block_ids = chunk.source_block_ids.clone();
                    }
                }
            }
            let findings_with_blocks = output.findings.iter().filter(|f| !f.block_ids.is_empty()).count();
            println!("[OK] block_ids 已填充: {}/{} 条 finding 携带 block 引用",
                findings_with_blocks, output.findings.len());

            let high_risk_count = output
                .findings
                .iter()
                .filter(|f| f.severity == crate::agents::types::RiskSeverity::High)
                .count();

            // ── 写盘：findings ──
            {
                let disk_stem = {
                    let docs = state.documents.read().await;
                    if let Some(doc) = docs.get(&doc_id) {
                        let file_stem = std::path::Path::new(&doc.filename)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("document");
                        format!("{}_{}", file_stem, &doc_id[..8.min(doc_id.len())])
                    } else {
                        format!("doc_{}", &doc_id[..8.min(doc_id.len())])
                    }
                };
                let dir = data_path_str("output/findings");
                let _ = std::fs::create_dir_all(&dir);
                let findings_path = format!("{}/{}_findings.json", dir, disk_stem);
                if let Ok(json) = serde_json::to_string_pretty(&output.findings) {
                    let _ = std::fs::write(&findings_path, json);
                    println!("[DISK] findings → {}", findings_path);
                }
                let summary_path = format!("{}/{}_routing_summary.json", dir, disk_stem);
                if let Ok(json) = serde_json::to_string_pretty(&output.routing_summary) {
                    let _ = std::fs::write(&summary_path, json);
                }
                if let Some(ref snap) = output.graph_snapshot {
                    let snap_path = format!("{}/{}_graph_snapshot.json", dir, disk_stem);
                    if let Ok(json) = serde_json::to_string_pretty(snap) {
                        let _ = std::fs::write(&snap_path, json);
                    }
                }
            }

            // 存入 review_results 供 GET /result 查询
            {
                let mut results = state.review_results.lock().await;
                results.insert(doc_id.clone(), output.clone());
            }

            // 写盘: {doc_id}_result.json — 重启后磁盘 fallback
            {
                let dir = data_path_str("output/findings");
                let _ = std::fs::create_dir_all(&dir);
                let result_path = format!("{}/{}_result.json", dir, doc_id);
                let persisted = ReviewResultResponse {
                    status: "completed".to_string(),
                    result: Some(ReviewResponse {
                        document_id: doc_id.clone(),
                        findings: output.findings.clone(),
                        routing_summary: output.routing_summary.clone(),
                        graph_snapshot: output.graph_snapshot.clone(),
                    }),
                    error: None,
                };
                if let Ok(json) = serde_json::to_string_pretty(&persisted) {
                    let _ = std::fs::write(&result_path, json);
                    println!("[DISK] result → {}", result_path);
                }
            }

            // 发送 Done 事件
            review_events.emit(&crate::agents::review_event::ReviewEvent::Done {
                total_findings: output.findings.len(),
                high_risk: high_risk_count,
                session_id: doc_id.clone(),
                duration_secs,
            });
        }
        Err(e) => {
            let msg = format!("审核引擎执行失败: {}", e);
            eprintln!("[ERROR] async review failed: doc_id={}, {}", doc_id, msg);

            // 存入 review_errors
            {
                let mut errors = state.review_errors.lock().await;
                errors.insert(doc_id.clone(), msg.clone());
            }

            // 发送 Error 事件
            review_events.emit(&crate::agents::review_event::ReviewEvent::Error {
                message: msg,
                session_id: doc_id.clone(),
            });
        }
    }

    // 延迟清理 ReviewEventBus 和 active_reviews
    // （给 SSE 客户端时间接收 Done/Error 事件）
    let cleanup_doc_id = doc_id.clone();
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let mut buses = cleanup_state.review_event_buses.lock().await;
        buses.remove(&cleanup_doc_id);
        let mut active = cleanup_state.active_reviews.lock().await;
        active.remove(&cleanup_doc_id);
    });
}

/// GET /api/v1/review/:doc_id/stream
///
/// SSE 端点：实时推送审查进度事件。
/// 客户端应**先连接此端点**，再调用 POST /review 触发审查，
/// 以确保不丢失早期事件。
///
/// 事件格式（标准 SSE）：
/// ```text
/// event: phase
/// data: {"phase":"execute","phase_index":2,...}
///
/// event: agent_progress
/// data: {"agent_id":"...","clauses_done":23,...}
///
/// event: trace
/// data: {"event_type":"agent_thought","summary":"...",...}
///
/// event: finding_added
/// data: {"risk_id":"R_001","severity":"high",...}
///
/// event: done
/// data: {"total_findings":8,"high_risk":3,...}
/// ```
pub async fn stream_review_events(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> axum::response::Sse<impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use axum::response::sse::Event;

    // 创建或获取 ReviewEventBus（如果 POST /review 尚未创建）
    let review_events = {
        let mut buses = state.review_event_buses.lock().await;
        buses
            .entry(doc_id.clone())
            .or_insert_with(|| Arc::new(ReviewEventBus::new(256)))
            .clone()
    };

    let mut rx = review_events.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    // msg 格式: event:{event_type}\n{json} 或 纯 JSON
                    let (event_type, data) = if let Some(rest) = msg.strip_prefix("event:") {
                        if let Some((etype, body)) = rest.split_once('\n') {
                            (etype.to_string(), body.to_string())
                        } else {
                            ("message".to_string(), rest.to_string())
                        }
                    } else {
                        // 旧格式（直接从 emit() 发送的纯 JSON）
                        ("message".to_string(), msg.clone())
                    };

                    // tagged JSON（来自 emit()）: {"event":"phase","data":{...}}
                    // 提取 event 类型 → SSE event type，提取内层 data → SSE data
                    let (final_event_type, final_data) = if event_type == "message" {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
                            let etype = parsed.get("event")
                                .and_then(|v| v.as_str())
                                .unwrap_or("message")
                                .to_string();
                            // ★ 解包内层 data 字段，避免双重包装
                            let inner = parsed.get("data")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| data.clone());
                            (etype, inner)
                        } else {
                            ("message".to_string(), data)
                        }
                    } else {
                        // SSE 前缀格式（来自 emit_sse()）: event:phase\n{json}
                        // 已正确分离 event_type 和 data
                        (event_type, data)
                    };

                    yield Ok(Event::default()
                        .event(final_event_type)
                        .data(final_data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    let _ = n;
                    yield Ok(Event::default()
                        .event("error")
                        .data(r#"{"message":"SSE lagged, some events were dropped"}"#));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    axum::response::Sse::new(stream)
}

/// GET /api/v1/review/:doc_id/result
///
/// 查询异步审查的最终结果。返回状态：
/// - `"completed"` → 200 + result
/// - `"pending"` → 200 + `{ status: "pending" }`（审查仍在进行）
/// - `"failed"` → 200 + `{ status: "failed", error: "..." }`
/// - `"not_found"` → 404（无审查记录）
pub async fn get_review_result(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<ReviewResultResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 1. 检查内存中已完成的结果（不移除，允许多次查询）
    {
        let results = state.review_results.lock().await;
        if let Some(output) = results.get(&doc_id) {
            return Ok(Json(ReviewResultResponse {
                status: "completed".to_string(),
                result: Some(ReviewResponse {
                    document_id: doc_id,
                    findings: output.findings.clone(),
                    routing_summary: output.routing_summary.clone(),
                    graph_snapshot: output.graph_snapshot.clone(),
                }),
                error: None,
            }));
        }
    }

    // 2. 检查失败信息
    {
        let errors = state.review_errors.lock().await;
        if let Some(msg) = errors.get(&doc_id) {
            return Ok(Json(ReviewResultResponse {
                status: "failed".to_string(),
                result: None,
                error: Some(msg.clone()),
            }));
        }
    }

    // 3. 检查是否仍在进行中
    {
        let buses = state.review_event_buses.lock().await;
        if buses.contains_key(&doc_id) {
            return Ok(Json(ReviewResultResponse {
                status: "pending".to_string(),
                result: None,
                error: None,
            }));
        }
    }

    // 4. 磁盘 fallback — 重启后内存为空，从 JSON 文件恢复
    {
        let dir = data_path_str("output/findings");
        let result_path = format!("{}/{}_result.json", dir, doc_id);
        if let Ok(json) = std::fs::read_to_string(&result_path) {
            if let Ok(result) = serde_json::from_str::<ReviewResultResponse>(&json) {
                println!("[DISK] result loaded from disk: {}", result_path);
                return Ok(Json(result));
            }
        }
    }

    Err(not_found(&format!(
        "审查结果不存在: {}",
        doc_id
    )))
}

/// POST /api/v1/documents/:id/chat
pub async fn chat_with_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let docs = state.documents.read().await;
    let doc = docs.get(&doc_id).ok_or_else(|| {
        not_found(&format!("文档不存在: {}", doc_id))
    })?.clone();
    drop(docs);

    let llm: Arc<dyn LlmClient> = Arc::from(create_llm_client()
        .map_err(|e| server_error("创建 Chat LLM 客户端失败", e))?);

    let embed_client = {
        let ec = state.embed_client.lock().unwrap();
        ec.clone()
    };

    let mut chat_tools = ToolRegistry::new();
    if let Some(ref ec) = embed_client {
        chat_tools.register(Box::new(SearchDocumentTool::new(
            doc.doc_index.clone(),
            ec.clone(),
        )));
    }
    chat_tools.register(Box::new(ReadSectionTool::new(
        doc.chunk_map.clone(),
        doc.chunk_order.clone(),
    )));
    if let Some(ref ds) = state.dashscope_search {
        chat_tools.register(Box::new(SearchKnowledgeTool::with_dashscope(ds.clone())));
    }
    chat_tools.register(Box::new(AnswerUserTool));

    let chat_config = ChatAgentConfig::default();
    let chat_agent = ChatAgent::new(
        chat_config,
        llm,
        chat_tools,
        Some(doc.doc_index.clone()),
        embed_client,
        Some(doc.chunk_map.clone()),
    ).map_err(|e| server_error("创建 ChatAgent 失败", e))?;

    // DTO history → ChatMessage
    let history = req.history.map(|h| {
        h.into_iter().map(|m| {
            match m.role.as_str() {
                "system" => ChatMessage::System {
                    content: m.content.unwrap_or_default(),
                },
                "assistant" => ChatMessage::Assistant {
                    content: m.content,
                    tool_calls: None,
                },
                _ => ChatMessage::User {
                    content: m.content.unwrap_or_default(),
                },
            }
        }).collect()
    });

    let response = chat_agent.chat(req.selection, &req.user_input, history).await
        .map_err(|e| server_error("对话执行失败", e))?;

    Ok(Json(response))
}

/// POST /api/v1/documents/:id/chat/stream
///
/// SSE streaming endpoint for ChatAgent. Clients receive incremental
/// `thinking` / `tool_call` / `answer` / `done` events as the agent
/// processes the query.
pub async fn chat_with_document_stream(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Json(req): Json<ChatRequest>,
) -> axum::response::Sse<impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use axum::response::sse::Event;

    // All setup + streaming in a single async_stream block
    // (each async_stream::stream! creates a unique type — can't have early returns)
    let stream = async_stream::stream! {
        // ── Setup (inside stream to avoid type mismatch) ──
        let docs = state.documents.read().await;
        let doc = match docs.get(&doc_id) {
            Some(d) => d.clone(),
            None => {
                yield Ok(Event::default()
                    .event("error")
                    .data(r#"{"message":"文档不存在"}"#));
                return;
            }
        };
        drop(docs);

        let llm: Arc<dyn LlmClient> = match create_llm_client() {
            Ok(client) => Arc::from(client),
            Err(e) => {
                yield Ok(Event::default()
                    .event("error")
                    .data(format!(r#"{{"message":"{}"}}"#, e)));
                return;
            }
        };

        let embed_client = {
            let ec = state.embed_client.lock().unwrap();
            ec.clone()
        };

        let mut chat_tools = ToolRegistry::new();
        if let Some(ref ec) = embed_client {
            chat_tools.register(Box::new(SearchDocumentTool::new(
                doc.doc_index.clone(),
                ec.clone(),
            )));
        }
        chat_tools.register(Box::new(ReadSectionTool::new(
            doc.chunk_map.clone(),
            doc.chunk_order.clone(),
        )));
        if let Some(ref ds) = state.dashscope_search {
            chat_tools.register(Box::new(SearchKnowledgeTool::with_dashscope(ds.clone())));
        }
        chat_tools.register(Box::new(AnswerUserTool));

        let chat_config = ChatAgentConfig::default();
        let chat_agent = match ChatAgent::new(
            chat_config,
            llm,
            chat_tools,
            Some(doc.doc_index.clone()),
            embed_client,
            Some(doc.chunk_map.clone()),
        ) {
            Ok(agent) => agent,
            Err(e) => {
                yield Ok(Event::default()
                    .event("error")
                    .data(format!(r#"{{"message":"创建 ChatAgent 失败: {}"}}"#, e)));
                return;
            }
        };

        let history = req.history.map(|h| {
            h.into_iter().map(|m| match m.role.as_str() {
                "system" => ChatMessage::System {
                    content: m.content.unwrap_or_default(),
                },
                "assistant" => ChatMessage::Assistant {
                    content: m.content,
                    tool_calls: None,
                },
                _ => ChatMessage::User {
                    content: m.content.unwrap_or_default(),
                },
            }).collect()
        });

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ChatStreamEvent>();

        // Spawn ChatAgent in background
        tokio::spawn(async move {
            let _ = chat_agent.chat_stream(req.selection, &req.user_input, history, tx).await;
        });

        // ── Relay events from agent ──
        while let Some(event) = rx.recv().await {
            let (event_type, data) = match &event {
                ChatStreamEvent::Thinking { message } =>
                    ("thinking", format!(r#"{{"message":"{}"}}"#, message)),
                ChatStreamEvent::ToolCall { name, args } =>
                    ("tool_call", format!(r#"{{"name":"{}","args":"{}"}}"#, name, args)),
                ChatStreamEvent::Answer(resp) =>
                    ("answer", serde_json::to_string(resp).unwrap_or_default()),
                ChatStreamEvent::Done(resp) =>
                    ("done", serde_json::to_string(resp).unwrap_or_default()),
                ChatStreamEvent::Error(msg) =>
                    ("error", format!(r#"{{"message":"{}"}}"#, msg)),
            };
            let is_terminal = matches!(event, ChatStreamEvent::Done(_) | ChatStreamEvent::Error(_));
            yield Ok(Event::default().event(event_type).data(data));
            if is_terminal {
                break;
            }
        }
    };

    axum::response::Sse::new(stream)
}

/// POST /api/v1/documents/:id/search
pub async fn search_document(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let docs = state.documents.read().await;
    let doc = docs.get(&doc_id).ok_or_else(|| {
        not_found(&format!("文档不存在: {}", doc_id))
    })?.clone();
    drop(docs);

    let embed_client = {
        let ec = state.embed_client.lock().unwrap();
        ec.clone()
    };

    let query_texts: Vec<&str> = req.queries.iter().map(|s| s.as_str()).collect();
    let query_embs = if let Some(ref ec) = embed_client {
        ec.encode_queries(&query_texts).map_err(|e| server_error("查询编码失败", e))?
    } else {
        return Err(server_error_fmt("嵌入客户端未初始化"));
    };

    let top_k = req.top_k.unwrap_or(5);
    let mut results = Vec::new();
    for (i, query) in req.queries.iter().enumerate() {
        let hits = doc.doc_index.search(&query_embs[i], top_k);
        let hit_dtos: Vec<SearchHitDto> = hits.iter().map(|h| SearchHitDto {
            chunk_id: h.chunk_id.clone(),
            title: h.title.clone(),
            score: h.score,
            snippet: h.snippet.chars().take(200).collect(),
            page_start: h.page_start,
        }).collect();
        results.push(SearchResultGroup {
            query: query.clone(),
            hits: hit_dtos,
        });
    }

    Ok(Json(SearchResponse { results }))
}

// ─── Block BBox 查询 ─────────────────────────────────────────────

/// 请求参数：ids 为逗号分隔的 block_id 列表
#[derive(Debug, Deserialize)]
pub struct BlockQuery {
    pub ids: String,
}

/// BBox 坐标 DTO
#[derive(Debug, Serialize)]
pub struct BBoxDto {
    pub x0: f64,
    pub top: f64,
    pub x1: f64,
    pub bottom: f64,
}

/// 单个 block 的 BBox 响应
#[derive(Debug, Serialize)]
pub struct BlockBBoxResponse {
    pub block_id: String,
    /// 所在页码 (0-based)
    pub page: usize,
    /// 包围盒坐标（PDF points）
    pub bbox: BBoxDto,
    /// 原始 PDF 页面宽度 (pt)，用于前端 scale = renderedWidth / pageWidth
    pub page_width: f64,
}

/// GET /api/v1/documents/:id/blocks?ids=b_5_2,b_5_3
///
/// 返回指定 block_id 的 BBox 坐标，用于前端 bbox-based PDF 精确高亮。
pub async fn get_block_bboxes(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<BlockQuery>,
) -> Result<Json<Vec<BlockBBoxResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let docs = state.documents.read().await;
    let doc = docs
        .get(&doc_id)
        .ok_or_else(|| not_found(&format!("文档不存在: {}", doc_id)))?;

    let requested_ids: Vec<&str> = params.ids.split(',').map(|s| s.trim()).collect();
    println!("[BLOCKS] 查询 block BBox: doc={}, ids={:?}", doc_id, requested_ids);
    let mut results: Vec<BlockBBoxResponse> = Vec::new();

    for page in &doc.raw_doc.pages {
        for block in &page.blocks {
            if requested_ids.contains(&block.id.as_str()) {
                results.push(BlockBBoxResponse {
                    block_id: block.id.clone(),
                    page: page.page_index,
                    bbox: BBoxDto {
                        x0: block.bbox.x0,
                        top: block.bbox.top,
                        x1: block.bbox.x1,
                        bottom: block.bbox.bottom,
                    },
                    page_width: page.width,
                });
            }
        }
    }

    println!("[BLOCKS] 返回 {} 条 BBox 坐标 (请求 {} 个 block)", results.len(), requested_ids.len());
    Ok(Json(results))
}

// ─── Helper ────────────────────────────────────────────────────────

fn collect_all_block_ids(section: &Section) -> Vec<&str> {
    let mut ids: Vec<&str> = section.block_ids.iter().map(|s| s.as_str()).collect();
    for child in &section.children {
        ids.extend(collect_all_block_ids(child));
    }
    ids
}

fn bad_request(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "BAD_REQUEST".to_string(),
            detail: msg.to_string(),
        }),
    )
}

fn server_error(msg: &str, e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    let detail = format!("{}: {}", msg, e);
    eprintln!("[ERROR] {}", detail);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: msg.to_string(),
            detail,
        }),
    )
}

fn server_error_fmt(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: msg.to_string(),
            detail: msg.to_string(),
        }),
    )
}

fn not_found(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: "NOT_FOUND".to_string(),
            detail: msg.to_string(),
        }),
    )
}