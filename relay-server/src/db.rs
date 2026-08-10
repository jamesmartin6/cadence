use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{DocRecord, OpRecord};

pub async fn create_doc(pool: &PgPool, title: Option<String>) -> sqlx::Result<DocRecord> {
    sqlx::query_as::<_, DocRecord>(
        "INSERT INTO docs (title) VALUES ($1) RETURNING id, title, created_at",
    )
    .bind(title)
    .fetch_one(pool)
    .await
}

pub async fn get_doc(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<DocRecord>> {
    sqlx::query_as::<_, DocRecord>("SELECT id, title, created_at FROM docs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn list_docs(pool: &PgPool) -> sqlx::Result<Vec<DocRecord>> {
    sqlx::query_as::<_, DocRecord>(
        "SELECT id, title, created_at FROM docs ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn list_ops(pool: &PgPool, doc_id: Uuid) -> sqlx::Result<Vec<OpRecord>> {
    sqlx::query_as::<_, OpRecord>("SELECT payload FROM ops WHERE doc_id = $1 ORDER BY seq ASC")
        .bind(doc_id)
        .fetch_all(pool)
        .await
}

/// Append an op to the durable log, assigning the next per-doc `seq` atomically.
///
/// Locks the parent `docs` row for the duration of the transaction so concurrent writers
/// targeting the *same* document serialize (no duplicate/racing seq numbers), while
/// writers on *different* documents don't block each other at all.
pub async fn append_op(
    pool: &PgPool,
    doc_id: Uuid,
    site_id: Option<i64>,
    payload: &serde_json::Value,
) -> sqlx::Result<i64> {
    let mut tx = pool.begin().await?;

    sqlx::query("SELECT id FROM docs WHERE id = $1 FOR UPDATE")
        .bind(doc_id)
        .fetch_one(&mut *tx)
        .await?;

    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO ops (doc_id, seq, site_id, payload)
         VALUES ($1, (SELECT COALESCE(MAX(seq), 0) + 1 FROM ops WHERE doc_id = $1), $2, $3)
         RETURNING seq",
    )
    .bind(doc_id)
    .bind(site_id)
    .bind(payload)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(seq)
}
