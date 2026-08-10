use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocRecord {
    pub id: Uuid,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OpRecord {
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateDocRequest {
    #[serde(default)]
    pub title: Option<String>,
}

/// The WebSocket wire protocol. The relay server never interprets `payload` — it's an
/// opaque, already-serialized CRDT [`crdt_engine::Operation`] as far as the server is
/// concerned (relay-server intentionally does not depend on crdt-engine at all: it's a
/// dumb relay + durable log, exactly per the build plan). Presence/cursor messages are
/// relayed the same way but never persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WsMessage {
    /// Client -> server: a local edit to persist and broadcast.
    /// Server -> other clients: a relayed edit from someone else.
    Op {
        payload: serde_json::Value,
        #[serde(default)]
        site_id: Option<i64>,
    },
    /// Either direction: a live cursor/selection position. Never persisted.
    Cursor { user_id: String, index: u32 },
    /// Server -> client only, sent once immediately after a successful connection:
    /// the full ordered op log so the client can reconstruct the document.
    History { ops: Vec<serde_json::Value> },
}
