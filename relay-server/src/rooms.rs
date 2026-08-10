use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::models::WsMessage;

const ROOM_CHANNEL_CAPACITY: usize = 256;

/// One message traveling through a room's broadcast channel, tagged with the connection
/// that produced it so that connection can skip re-forwarding its own message back to
/// itself (every connection is both a publisher and a subscriber of the same channel).
#[derive(Debug, Clone)]
pub struct Envelope {
    pub sender: Uuid,
    pub message: WsMessage,
}

/// In-memory registry of active document rooms: one broadcast channel per currently-open
/// document, fanning out ops and presence messages to every connected client. This is
/// purely a live-relay mechanism -- the durable source of truth is Postgres (see `db.rs`);
/// a room here just disappears (and gets recreated empty) if the server restarts.
#[derive(Default)]
pub struct Rooms {
    inner: Mutex<HashMap<Uuid, broadcast::Sender<Envelope>>>,
}

impl Rooms {
    pub fn new() -> Self {
        Self::default()
    }

    /// Join (creating if necessary) the room for `doc_id`, returning a sender for
    /// publishing to it and a fresh receiver for this connection's own subscription.
    pub fn join(
        &self,
        doc_id: Uuid,
    ) -> (broadcast::Sender<Envelope>, broadcast::Receiver<Envelope>) {
        let mut rooms = self.inner.lock().expect("rooms mutex poisoned");
        let sender = rooms
            .entry(doc_id)
            .or_insert_with(|| broadcast::channel(ROOM_CHANNEL_CAPACITY).0)
            .clone();
        let receiver = sender.subscribe();
        (sender, receiver)
    }

    /// Drop the room's channel if nobody is subscribed to it anymore, so the registry
    /// doesn't grow forever as documents are opened and closed over the server's lifetime.
    pub fn cleanup_if_empty(&self, doc_id: Uuid) {
        let mut rooms = self.inner.lock().expect("rooms mutex poisoned");
        if let Some(sender) = rooms.get(&doc_id) {
            if sender.receiver_count() == 0 {
                rooms.remove(&doc_id);
            }
        }
    }
}
