use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use super::handlers::{self, AppState};

/// 构建完整的 API Router。
pub fn build(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/documents", post(handlers::process_document))
        .route("/documents/:id", get(handlers::get_document))
        .route("/documents/:id/review", post(handlers::review_document))
        .route("/documents/:id/chat", post(handlers::chat_with_document))
        .route("/documents/:id/chat/stream", post(handlers::chat_with_document_stream))
        .route("/documents/:id/search", post(handlers::search_document))
        .route("/documents/:id/blocks", get(handlers::get_block_bboxes))
        // SSE 实时推送 + 异步审查结果
        .route("/review/:doc_id/stream", get(handlers::stream_review_events))
        .route("/review/:doc_id/result", get(handlers::get_review_result));

    Router::new()
        .route("/health", get(handlers::health))
        .nest("/api/v1", api)
        .layer(cors)
        .with_state(state)
}