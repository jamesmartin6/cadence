//! Integration tests exercising the real HTTP + WebSocket surface against a real
//! Postgres, satisfying Phase 3's definition of done:
//!   - two WebSocket clients on the same doc converge on each other's ops in real time
//!   - a late joiner receives the full op log and reconstructs the current document
//!   - a relay-server restart doesn't lose data (a fresh pool/app reads ops back from
//!     the same Postgres, exactly like restarting the process against a durable DB
//!     that kept running -- which is the real-world scenario this matters for)
//!
//! Uses `postgresql_embedded` to spin up a real, disposable Postgres instance with no
//! system install or Docker required (see progress.md for why: this dev machine has
//! neither available). Production code path is unaffected -- it's still a plain
//! `sqlx::PgPool` against `DATABASE_URL`.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use postgresql_embedded::{PostgreSQL, Settings};
use relay_server::{app_router, AppState};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

struct TestPg {
    // Kept alive for the duration of the test -- dropping it tears down the temporary
    // instance and deletes its data directory.
    instance: PostgreSQL,
    database_name: &'static str,
}

impl TestPg {
    async fn start() -> Self {
        let settings = Settings::default();
        let mut instance = PostgreSQL::new(settings);
        instance.setup().await.expect("postgres setup failed");
        instance.start().await.expect("postgres start failed");
        let database_name = "cadence_test";
        instance
            .create_database(database_name)
            .await
            .expect("failed to create test database");
        Self {
            instance,
            database_name,
        }
    }

    fn url(&self) -> String {
        self.instance.settings().url(self.database_name)
    }
}

async fn spawn_app(database_url: &str) -> SocketAddr {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .expect("failed to connect to test postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let app = app_router(AppState::new(pool));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

async fn create_doc(base_url: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/docs"))
        .json(&json!({}))
        .send()
        .await
        .expect("POST /docs failed");
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);
    let body: serde_json::Value = resp.json().await.unwrap();
    body["id"].as_str().unwrap().to_string()
}

async fn connect_ws(
    ws_base: &str,
    doc_id: &str,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = format!("{ws_base}/ws/docs/{doc_id}");
    let (stream, _resp) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket connect failed");
    stream
}

async fn next_json(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> serde_json::Value {
    loop {
        let msg = timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("timed out waiting for websocket message")
            .expect("stream ended unexpectedly")
            .expect("websocket error");
        if let Message::Text(text) = msg {
            return serde_json::from_str(&text).expect("invalid JSON from server");
        }
    }
}

#[tokio::test]
async fn two_clients_converge_and_late_joiner_reconstructs_history() {
    let pg = TestPg::start().await;
    let addr = spawn_app(&pg.url()).await;
    let http_base = format!("http://{addr}");
    let ws_base = format!("ws://{addr}");

    let doc_id = create_doc(&http_base).await;

    // Client A connects first.
    let mut client_a = connect_ws(&ws_base, &doc_id).await;
    let history_a = next_json(&mut client_a).await;
    assert_eq!(history_a["kind"], "history");
    assert_eq!(
        history_a["ops"].as_array().unwrap().len(),
        0,
        "brand new doc has empty history"
    );

    // Client A sends an insert op.
    let op_payload =
        json!({"type": "Insert", "id": {"site_id": 1, "counter": 0}, "after": null, "char": "h"});
    client_a
        .send(Message::Text(
            json!({"kind": "op", "payload": op_payload, "site_id": 1}).to_string(),
        ))
        .await
        .unwrap();

    // Client B connects *after* A's op was sent -- it must see it in its history replay
    // (this is the "late joiner reconstructs the document" requirement).
    // Give the server a moment to persist A's op before B connects.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let mut client_b = connect_ws(&ws_base, &doc_id).await;
    let history_b = next_json(&mut client_b).await;
    assert_eq!(history_b["kind"], "history");
    let ops = history_b["ops"].as_array().unwrap();
    assert_eq!(
        ops.len(),
        1,
        "late joiner must receive the already-persisted op"
    );
    assert_eq!(ops[0]["char"], "h");

    // Now B sends its own op; A (already connected) must receive it live, in real time.
    let op_payload_b =
        json!({"type": "Insert", "id": {"site_id": 2, "counter": 0}, "after": null, "char": "i"});
    client_b
        .send(Message::Text(
            json!({"kind": "op", "payload": op_payload_b, "site_id": 2}).to_string(),
        ))
        .await
        .unwrap();

    let relayed_to_a = next_json(&mut client_a).await;
    assert_eq!(relayed_to_a["kind"], "op");
    assert_eq!(relayed_to_a["payload"]["char"], "i");

    // And B must NOT receive its own op echoed back.
    let echo_check = timeout(Duration::from_millis(500), client_b.next()).await;
    assert!(
        echo_check.is_err(),
        "server must not echo a client's own op back to itself"
    );
}

#[tokio::test]
async fn server_restart_does_not_lose_data() {
    let pg = TestPg::start().await;

    // "First run" of the relay server.
    let addr1 = spawn_app(&pg.url()).await;
    let http_base1 = format!("http://{addr1}");
    let ws_base1 = format!("ws://{addr1}");

    let doc_id = create_doc(&http_base1).await;
    let mut client = connect_ws(&ws_base1, &doc_id).await;
    let _initial_history = next_json(&mut client).await;

    let op_payload =
        json!({"type": "Insert", "id": {"site_id": 1, "counter": 0}, "after": null, "char": "z"});
    client
        .send(Message::Text(
            json!({"kind": "op", "payload": op_payload, "site_id": 1}).to_string(),
        ))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    drop(client);

    // "Restart": a brand new pool + app + rooms registry (nothing in memory carries
    // over), pointed at the *same* Postgres instance -- exactly what happens when only
    // the relay-server process restarts while its Postgres container keeps running.
    let addr2 = spawn_app(&pg.url()).await;
    let ws_base2 = format!("ws://{addr2}");
    let mut client2 = connect_ws(&ws_base2, &doc_id).await;
    let history_after_restart = next_json(&mut client2).await;
    let ops = history_after_restart["ops"].as_array().unwrap();
    assert_eq!(ops.len(), 1, "op survives a full relay-server restart");
    assert_eq!(ops[0]["char"], "z");
}
