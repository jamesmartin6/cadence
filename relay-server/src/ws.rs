use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::db;
use crate::models::WsMessage;
use crate::rooms::Envelope;
use crate::AppState;

pub async fn ws_handler(
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    match db::get_doc(&state.pool, doc_id).await {
        Ok(Some(_)) => ws.on_upgrade(move |socket| handle_socket(socket, doc_id, state)),
        Ok(None) => (StatusCode::NOT_FOUND, "document not found").into_response(),
        Err(err) => {
            warn!(?err, "failed to look up document before websocket upgrade");
            (StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response()
        }
    }
}

async fn handle_socket(socket: WebSocket, doc_id: Uuid, state: AppState) {
    let conn_id = Uuid::new_v4();
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Send the full op history first, so the client can reconstruct the current document
    // before it starts receiving/sending live ops.
    let history = match db::list_ops(&state.pool, doc_id).await {
        Ok(ops) => ops.into_iter().map(|op| op.payload).collect(),
        Err(err) => {
            warn!(?err, %doc_id, "failed to load op history for new connection");
            Vec::new()
        }
    };
    if send_json(&mut ws_tx, &WsMessage::History { ops: history })
        .await
        .is_err()
    {
        return;
    }

    let (room_tx, mut room_rx) = state.rooms.join(doc_id);

    let mut forward_task = tokio::spawn(async move {
        while let Ok(envelope) = room_rx.recv().await {
            if envelope.sender == conn_id {
                continue; // never echo a client's own message back to itself
            }
            if send_json(&mut ws_tx, &envelope.message).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            let text = match msg {
                Message::Text(text) => text,
                Message::Close(_) => break,
                _ => continue,
            };

            let parsed: WsMessage = match serde_json::from_str(&text) {
                Ok(m) => m,
                Err(err) => {
                    debug!(?err, "ignoring malformed client message");
                    continue;
                }
            };

            match parsed {
                WsMessage::Op { payload, site_id } => {
                    if let Err(err) = db::append_op(&state.pool, doc_id, site_id, &payload).await {
                        warn!(?err, %doc_id, "failed to persist op");
                        continue;
                    }
                    let _ = room_tx.send(Envelope {
                        sender: conn_id,
                        message: WsMessage::Op { payload, site_id },
                    });
                }
                WsMessage::Cursor { user_id, index } => {
                    let _ = room_tx.send(Envelope {
                        sender: conn_id,
                        message: WsMessage::Cursor { user_id, index },
                    });
                }
                WsMessage::History { .. } => {
                    debug!("ignoring client-sent History message (server -> client only)");
                }
            }
        }
    });

    // Whichever direction ends first (client disconnects, or we can no longer write to
    // them), tear down the other half too instead of leaking a half-open task.
    tokio::select! {
        _ = &mut forward_task => recv_task.abort(),
        _ = &mut recv_task => forward_task.abort(),
    }

    state.rooms.cleanup_if_empty(doc_id);
}

async fn send_json(
    ws_tx: &mut SplitSink<WebSocket, Message>,
    msg: &WsMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("WsMessage serialization cannot fail");
    ws_tx.send(Message::Text(text.into())).await
}
