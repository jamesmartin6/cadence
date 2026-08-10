pub mod db;
pub mod models;
pub mod rooms;
pub mod ws;

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use models::CreateDocRequest;
use rooms::Rooms;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub rooms: Arc<Rooms>,
}

impl AppState {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            pool,
            rooms: Arc::new(Rooms::new()),
        }
    }
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/docs", post(create_doc).get(list_docs))
        .route("/ws/docs/{doc_id}", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn create_doc(
    State(state): State<AppState>,
    Json(body): Json<CreateDocRequest>,
) -> impl IntoResponse {
    match db::create_doc(&state.pool, body.title).await {
        Ok(doc) => (StatusCode::CREATED, Json(doc)).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to create doc");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to create document",
            )
                .into_response()
        }
    }
}

async fn list_docs(State(state): State<AppState>) -> impl IntoResponse {
    match db::list_docs(&state.pool).await {
        Ok(docs) => Json(docs).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to list docs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to list documents",
            )
                .into_response()
        }
    }
}
