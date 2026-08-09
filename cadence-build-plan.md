# Cadence — Real-Time Collaborative Text Editor (CRDT-based)

## Project Overview

A collaborative text editor where multiple users can edit the same document simultaneously — including while offline — with all edits merging back together automatically and deterministically, with no central "last write wins" and no lost keystrokes. The core merge logic is a CRDT (Conflict-free Replicated Data Type) implemented from scratch in Rust, compiled to WebAssembly, and run directly in the browser.

**Goal:** Two or more browser tabs editing the same document, typing concurrently, with instant sync over WebSockets. Disconnecting a tab, typing offline, and reconnecting merges cleanly with no conflicts and no data loss. Live multi-cursor presence shows where other users are editing.

**Tech stack:**
- CRDT engine: Rust, compiled to WebAssembly (`wasm-bindgen`, `wasm-pack`)
- Relay/persistence server: Rust, Axum (HTTP + WebSocket), PostgreSQL (via `sqlx`)
- Frontend: React + TypeScript + Vite, calling into the WASM module directly
- Testing: Rust's built-in test framework + `proptest` for property-based CRDT convergence tests; Playwright (or manual) for multi-client e2e verification
- Orchestration: Docker Compose (Postgres + relay server; frontend served separately in dev via Vite, built as static assets for prod)

---

## Repo Structure

```
cadence/
├── docker-compose.yml
├── README.md
├── crdt-engine/                    # Rust crate, compiles to WASM
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                  # wasm-bindgen exported API
│   │   ├── rga.rs                  # the core sequence CRDT (Replicated Growable Array)
│   │   ├── op.rs                   # Operation types: Insert, Delete
│   │   ├── id.rs                   # Unique operation ID (site_id, counter)
│   │   └── doc.rs                  # Document: applies ops, materializes to a string
│   └── tests/
│       ├── convergence.rs          # property-based: any op ordering converges to the same result
│       └── basic_ops.rs
├── relay-server/                   # Rust, Axum
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── ws.rs                   # WebSocket handler: join doc room, broadcast ops
│   │   ├── rooms.rs                # in-memory registry of active document rooms
│   │   ├── db.rs                   # Postgres: persist op log + periodic snapshots
│   │   └── models.rs
│   └── migrations/
│       └── 0001_init.sql
└── frontend/
    ├── package.json
    ├── vite.config.ts
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx
    │   ├── wasm/                   # generated bindings from wasm-pack build, imported here
    │   ├── hooks/
    │   │   ├── useCrdtDoc.ts       # wraps the WASM doc instance + local edit application
    │   │   └── useDocSocket.ts     # WebSocket connection, sends/receives ops
    │   ├── components/
    │   │   ├── Editor.tsx          # contenteditable or textarea-based editor surface
    │   │   ├── PresenceCursors.tsx # renders other users' live cursor positions
    │   │   └── ConnectionStatus.tsx # online/offline/reconnecting indicator
    │   └── lib/
    │       └── api.ts
    └── public/
```

---

## Core Concept Primer (for context, not to skip)

A **sequence CRDT** for text assigns every inserted character a globally unique, immutably-ordered ID (not just its position, which changes as the document edits). When two clients insert concurrently, both operations carry enough information (each character's ID plus a reference to what it was inserted after) that any node — regardless of the order operations arrive in — can deterministically compute the same final character ordering. Deletes are typically "tombstones" (mark as deleted, don't physically remove) so concurrent operations that reference a deleted character still resolve correctly.

This project implements an RGA (Replicated Growable Array) — one of the more approachable and well-documented sequence CRDT algorithms, and a reasonable middle ground between naive approaches and production algorithms like Yjs's YATA.

---

## Phase 1 — CRDT Engine Core (Rust, no WASM yet)

**Deliverable:** A pure-Rust library implementing the RGA algorithm, fully tested, with no browser/WASM concerns yet.

**Spec:**
- `id.rs`: `OpId { site_id: u32, counter: u64 }` — globally unique, totally ordered (compare by counter, then site_id as tiebreaker) per-character identifier.
- `op.rs`: `Operation::Insert { id: OpId, after: Option<OpId>, char: char }` and `Operation::Delete { target: OpId }`.
- `rga.rs`: the core structure — an ordered list of `Node { id: OpId, char: char, deleted: bool }`. `insert_local(after: Option<OpId>, char: char) -> Operation` generates a new operation with a fresh local ID. `apply_remote(op: Operation)` integrates an operation from another site: for an Insert, find the `after` node, then scan forward past any nodes with higher IDs (this is the key RGA rule that guarantees convergence — concurrent inserts at the same position order consistently by ID) and insert there; for a Delete, find the node by ID and mark it tombstoned.
- `doc.rs`: wraps an `Rga`, exposes `to_string() -> String` (filters tombstones), `insert_local(index: usize, char: char) -> Operation` (translates a visible-text index into the correct `after` reference), `delete_local(index: usize) -> Operation`.
- Each site gets a persistent random or assigned `site_id` for the session.

**Definition of done:**
- Unit tests: sequential inserts produce correct string; a delete correctly removes a character from the visible string; applying the same set of operations in different orders produces identical final documents (this is the core CRDT guarantee — test it directly).
- Property-based test (`proptest`): generate random sequences of concurrent insert/delete operations across 2-4 simulated sites, apply them in every site in a different random order, assert all sites converge to the same final string.

---

## Phase 2 — WASM Bindings

**Deliverable:** The Rust CRDT engine compiled to a WebAssembly module callable from JavaScript/TypeScript.

**Spec:**
- `lib.rs`: `#[wasm_bindgen]` wrapper struct `CrdtDoc` exposing `new(site_id: u32)`, `insert(index: usize, ch: char) -> JsValue` (returns the serialized Operation to broadcast), `delete(index: usize) -> JsValue`, `apply_remote(op: JsValue)`, `to_string() -> String`.
- Operations serialize to/from JSON (via `serde` + `serde-wasm-bindgen`) for transport over the WebSocket.
- Build with `wasm-pack build --target web`, output consumed directly by the Vite frontend.

**Definition of done:**
- A minimal HTML/JS test page (no React yet) can load the WASM module, create a doc, insert characters, and see `to_string()` update correctly.
- Two instances of the test page, manually copy-pasting serialized ops between them, converge to the same text.

---

## Phase 3 — Relay Server

**Deliverable:** A Rust/Axum WebSocket server that relays operations between clients editing the same document and persists them.

**Spec:**
- `POST /docs` creates a new document, returns a doc ID.
- `GET /ws/docs/{doc_id}` — WebSocket upgrade. On connect, the server sends the client the full current operation log (or a snapshot + recent ops) so it can reconstruct the document locally by replaying through its own CRDT engine instance (the server does *not* need its own CRDT logic — it's a dumb relay plus a durable log; the client-side WASM engines are the source of truth for merging).
- On receiving an op from a client, broadcast it to every other connected client in that doc's room, and append it to the persisted op log in Postgres.
- `rooms.rs`: in-memory `HashMap<DocId, Vec<WebSocketSender>>` (behind a mutex or using a broadcast channel per room) tracking active connections per document.
- Periodic snapshotting: every N operations, ask... actually the server can't materialize the document itself without implementing RGA server-side too — simplest approach for v1: the server just stores the full ordered op log per document (Postgres table `ops(doc_id, seq, payload jsonb)`) and always replays the full log to new joiners. Note this as a known scaling limitation in the README (see Known Gaps).
- Presence: a lightweight separate message type (`{type: "cursor", user_id, index}`) broadcast the same way as ops, not persisted.

**Definition of done:**
- Two WebSocket clients (can be simple test scripts) connected to the same doc room see each other's ops in real time.
- A client that connects after others have already made edits receives the full op log and can reconstruct the current document.
- Server restart doesn't lose data — ops are read back from Postgres.

---

## Phase 4 — Frontend

**Deliverable:** A React/TypeScript editor UI wired to the WASM engine and the relay server.

**Spec:**
- `useCrdtDoc.ts`: instantiates a `CrdtDoc` (WASM) on mount with a random `site_id`; exposes `text`, `insertAt(index, char)`, `deleteAt(index)` — each local edit calls into WASM, updates local React state from `to_string()`, and returns the operation to send over the socket.
- `useDocSocket.ts`: opens the WebSocket to `/ws/docs/{doc_id}`; on receiving a remote op, calls `apply_remote` on the CRDT doc and re-syncs React state; sends local ops as they're generated; tracks connection state (connected/disconnected/reconnecting) with automatic reconnect-with-backoff.
- `Editor.tsx`: a `contenteditable` or careful `<textarea>`-based surface that translates keystrokes into index-based insert/delete calls (this is the fiddliest UI part — mapping browser cursor/selection position to CRDT index, and re-syncing cursor position after remote updates re-render the text).
- `PresenceCursors.tsx`: renders colored cursor markers for other connected users at their last-known index, using the presence message type.
- `ConnectionStatus.tsx`: simple online/offline/reconnecting badge.
- **Offline behavior:** while disconnected, local edits still apply to the local WASM doc (so typing keeps working) and queue in memory; on reconnect, queued ops are sent and any ops missed from other clients are received and applied — because of the CRDT guarantee, this converges correctly regardless of order.

**Definition of done:**
- Two browser tabs on the same doc URL show each other's edits within ~100ms.
- Disconnecting one tab (e.g. via devtools network throttling or killing the WebSocket), typing while offline, then reconnecting, results in a correctly merged document with no lost or duplicated characters.
- Multi-cursor presence is visible and updates live.

---

## Phase 5 — Persistence & Document Management

**Deliverable:** Documents survive server restarts and can be listed/reopened.

**Spec:**
- `GET /docs` lists existing documents (id, created_at, maybe a short title/preview).
- Postgres schema: `docs(id, title, created_at)`, `ops(id, doc_id, seq, site_id, payload jsonb, created_at)`.
- On the frontend, a simple document picker/landing page before entering the editor.

**Definition of done:**
- Creating a doc, editing it, restarting the relay server, and reopening the same doc URL restores the full edited content.
- The docs list shows all previously created documents.

---

## Phase 6 — Dockerization & Polish

**Deliverable:** One-command startup, README with the offline-merge demo documented.

**Spec:**
- `docker-compose.yml`: Postgres + relay-server services. Frontend built as static assets (Vite build) and either served by a simple static server container or documented as `npm run dev` for local demo purposes.
- README: explains the CRDT approach at a high level, setup instructions, and explicitly walks through the offline-conflict demo (open two tabs, disconnect one, type in both, reconnect, observe merge) since that's the single most impressive thing to show in an interview or demo video.

**Definition of done:**
- `docker compose up` brings up Postgres + relay server with zero manual steps.
- A stranger cloning the repo can follow the README to see the offline-merge demo work end to end.

---

## Testing Requirements (applies across phases)

- Rust unit tests for the RGA core (Phase 1) — no WASM, no network, fast and deterministic.
- Property-based convergence tests (`proptest`) — this is the single highest-signal test in the whole project, since it directly proves the CRDT correctness guarantee rather than just checking a few hand-picked scenarios.
- Integration-level verification: manual or scripted multi-client test hitting the real relay server and confirming convergence over the network, including a simulated disconnect/reconnect.

---

## Suggested Build Order for Claude Code

Implement and verify each phase fully (including its tests) before moving to the next:

1. Phase 1 (CRDT core in pure Rust) → verify with unit tests + property-based convergence tests. Do not proceed until convergence tests reliably pass across many random seeds.
2. Phase 2 (WASM bindings) → verify with a minimal manual HTML test page.
3. Phase 3 (relay server) → verify with two scripted WebSocket clients exchanging ops.
4. Phase 4 (frontend) → verify visually with two real browser tabs, including the offline/reconnect scenario.
5. Phase 5 (persistence) → verify server-restart durability.
6. Phase 6 (Docker + docs) → final polish pass, record the offline-merge demo.

## Known Gaps / Explicitly Out of Scope for v1

Document these in the README rather than silently fixing them:
- The relay server stores and replays the full op log to new joiners rather than maintaining its own materialized document or periodic compaction — fine at demo scale, would need snapshotting/compaction for a long-lived document with thousands of edits.
- No authentication — anyone with a doc URL can join and edit.
- No rich text (bold/italic/formatting) — plain text only; extending the CRDT to attributed text is a natural v2 direction.
- Cursor/selection mapping through remote re-renders is approximate, not pixel-perfect, especially during rapid concurrent editing.

## Notes for the Resume / Interview Story

Once built, the concrete things to point to:
- "Implemented a sequence CRDT (RGA) from scratch in Rust, compiled to WebAssembly, verified with property-based convergence tests across randomized concurrent operation orderings"
- The offline-first guarantee: real demo of disconnect → concurrent edits → reconnect → correct merge, with no central conflict resolution
- The contrast with Quorum: Raft is consensus (nodes agree on one order before committing), CRDTs are conflict-free merging (nodes never need to agree — any order converges) — being able to articulate that distinction clearly is itself a strong interview signal
