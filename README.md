# Cadence

**A real-time collaborative text editor built around a sequence CRDT (RGA) written from
scratch in Rust, compiled to WebAssembly, and run directly in the browser.**

[![CI](https://github.com/jamesmartin6/cadence/actions/workflows/ci.yml/badge.svg)](https://github.com/jamesmartin6/cadence/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Multiple people can edit the same document at once — including while completely offline —
and every edit merges back together automatically and deterministically. There's no
central "last write wins," no locking, and no lost keystrokes. Disconnect, keep typing,
reconnect: the document converges correctly every time, because the merge logic is
mathematically guaranteed to, not just tested to usually work.

```
                 ┌─────────────────────┐         ┌─────────────────────┐
                 │   Browser tab A      │         │   Browser tab B      │
                 │  ┌────────────────┐  │         │  ┌────────────────┐  │
                 │  │ React editor UI │  │         │  │ React editor UI │  │
                 │  ├────────────────┤  │         │  ├────────────────┤  │
                 │  │  RGA CRDT (WASM)│  │         │  │  RGA CRDT (WASM)│  │
                 │  └───────┬────────┘  │         │  └───────┬────────┘  │
                 └──────────┼───────────┘         └──────────┼───────────┘
                             │   WebSocket (JSON ops)          │
                             ▼                                 ▼
                        ┌─────────────────────────────────────────┐
                        │     relay-server (Axum, Rust)            │
                        │  dumb relay: broadcast + durable op log  │
                        │        (never runs CRDT logic itself)    │
                        └───────────────────┬───────────────────────┘
                                             ▼
                                     ┌───────────────┐
                                     │   Postgres     │
                                     │  full op log   │
                                     └───────────────┘
```

## The single most impressive thing to try

1. `docker compose up` (see [Quickstart](#quickstart)).
2. Open `http://localhost:8080`, create a document, open the same URL in a second tab.
3. Type in both tabs — edits show up in the other tab in real time, with a live colored
   cursor showing where the other person is typing.
4. In tab B, open dev tools → Network → set throttling to **Offline** (or just disconnect
   your WiFi). Tab B's connection badge turns red. Keep typing in tab B — it keeps working
   locally, just not syncing.
5. Meanwhile, type something different in tab A.
6. Turn tab B's network back on. Watch both tabs converge to the *same* merged document,
   automatically, with both edits present and nothing lost or duplicated — no matter how
   the edits interleaved.

That's the whole point of the project: step 6 works with **no conflict resolution UI, no
manual merge, and no central authority deciding whose edit "wins."** Both replicas ran the
exact same deterministic merge rule and landed on the same answer independently.

---

## Why a CRDT, and why build it from scratch

Real-time collaborative editors (Google Docs, Notion, Figma) face a hard problem: two
people type in the same spot at the same time, on two different machines, with no shared
clock and no guarantee either machine even has network access. There are two broad
families of solution:

- **Consensus** (e.g. Raft, Paxos): every node agrees on a single total order of
  operations *before* committing them. Strong guarantees, but requires nodes to
  coordinate — an offline node can't safely commit anything until it reconnects and
  re-synchronizes with the group.
- **CRDTs** (Conflict-free Replicated Data Types): every node applies operations
  *independently*, using a data structure and merge rule specifically designed so that
  applying the same set of operations in *any* order produces the *same* final result.
  No coordination required, ever — which is exactly what "keep typing while offline"
  needs.

This project implements **RGA (Replicated Growable Array)**, a sequence CRDT: every
inserted character gets a globally unique, totally-ordered id (`{site_id, counter}`,
with `counter` as a Lamport clock — see [`doc.rs`](crdt-engine/src/doc.rs) for why that
distinction actually matters). Each character also remembers the id of the character it
was inserted immediately after. When two sites insert concurrently at the same position,
both operations carry enough information that *any* replica — regardless of what order it
receives the operations in — can deterministically compute the same final ordering.
Deletes are tombstones (marked deleted, never physically removed) so a concurrent
operation that still refers to a since-deleted character resolves correctly instead of
crashing or silently corrupting the document.

## Architecture

| Component | What it is | Where |
|---|---|---|
| `crdt-engine` | The RGA CRDT, pure Rust, zero network/browser concerns. Compiled to WebAssembly (`wasm-bindgen`) for the frontend. | [`crdt-engine/`](crdt-engine/) |
| `relay-server` | Axum + WebSocket relay. Broadcasts ops between clients on the same document and persists the full op log to Postgres. **Deliberately dumb** — it never runs RGA logic itself; the browser's WASM engine is the only source of truth for merging. | [`relay-server/`](relay-server/) |
| `frontend` | React + TypeScript + Vite. Calls into the WASM module directly for every keystroke; talks to the relay server over a WebSocket. | [`frontend/`](frontend/) |

The client is where all the interesting logic lives — the server's job is narrow on
purpose: relay live operations to connected peers, and durably log every operation so a
newly-joined (or reconnecting) client can replay the full history and reconstruct the
document with its own CRDT engine. This keeps the server simple and horizontally
uninteresting, and keeps the "no coordination needed" CRDT property intact end to end.

## Quickstart

Requires Docker (and nothing else — no Rust or Node toolchain needed to just run it):

```sh
git clone https://github.com/jamesmartin6/cadence.git
cd cadence
docker compose up
```

Then open **http://localhost:8080**. That's the whole setup: Postgres, the relay server
(with migrations applied automatically on startup), and the frontend, all in one command.

## Development setup

Running each piece natively (with hot reload) instead of through Docker:

```sh
# 1. Postgres (only Postgres needs Docker here; everything else runs natively)
docker compose up postgres

# 2. Relay server
cd relay-server
cp .env.example .env   # defaults already point at the compose Postgres above
cargo run

# 3. Frontend, in another terminal
cd frontend
npm install
npm run dev             # http://localhost:5173
```

If you change `crdt-engine`, rebuild the WASM bindings the frontend uses (the built
output is committed to the repo so a plain `npm install` works without a Rust toolchain,
but it needs regenerating after any CRDT change):

```sh
cd crdt-engine
wasm-pack build --target web --out-dir ../frontend/src/wasm
```

### Running the tests

```sh
# CRDT engine: unit tests + the property-based convergence test (the highest-signal
# test in the whole project — see below)
cd crdt-engine && cargo test

# Relay server: spins up its own disposable Postgres automatically
# (via the `postgresql_embedded` crate) -- no Docker or local Postgres install needed
cd relay-server && cargo test

# Frontend end-to-end tests: real headless Chromium against the real stack.
# Needs relay-server (+ Postgres) already running (see Development setup above).
cd frontend && npm run test:e2e
```

The convergence test is worth calling out specifically: it generates random sequences of
concurrent insert/delete operations across 2-4 simulated sites, builds several
independently-randomized (but causally valid) delivery orders of the exact same
operations, replays each into a fresh document, and asserts every replica ends up
byte-for-byte identical. That's not a smoke test — it's a direct, repeated proof of the
CRDT convergence guarantee this whole project is built around. It runs by default across
256 random cases per `cargo test`; CI runs it with 2048.

## Known gaps / explicitly out of scope for v1

These are intentional scope cuts for a v1, not oversights — documented here instead of
silently working around them:

- **No compaction.** The relay server stores and replays the *entire* op log to every new
  or reconnecting client, rather than maintaining its own materialized snapshot. Fine at
  demo scale; a long-lived document with thousands of edits would want periodic
  snapshotting/compaction.
- **No authentication.** Anyone with a document URL can view and edit it.
- **Plain text only.** No bold/italic/rich formatting. Extending the CRDT to attributed
  text (a "run of formatted spans" model layered on top of the same RGA ordering) is the
  natural v2 direction.
- **Cursor/selection mapping is approximate, not pixel-perfect**, especially during rapid
  concurrent editing in wrapped text — see the mirror-div technique in
  [`PresenceCursors.tsx`](frontend/src/components/PresenceCursors.tsx).

## The Raft comparison

If you're comparing this to a Raft-based system: Raft is *consensus* — nodes agree on one
canonical order for every operation before any of them commit it, which means an
unreachable node can't safely proceed until it rejoins the cluster. A CRDT is
*conflict-free replication* — nodes never need to agree on anything in advance; any
delivery order converges to the same result by construction. Different tools for
different jobs: consensus when you need one authoritative order (leader election, a
replicated log, distributed locks), CRDTs when replicas need to make progress
independently and merge later without coordination (this editor, offline-first mobile
apps, multi-region caches).

## Repo structure

```
cadence/
├── docker-compose.yml
├── crdt-engine/        # Rust, compiles to WASM — the CRDT itself
│   └── src/{id,op,rga,doc,wasm_api}.rs
├── relay-server/        # Rust, Axum — WebSocket relay + Postgres op log
│   └── src/{main,lib,ws,rooms,db,models}.rs
└── frontend/             # React + TypeScript + Vite
    └── src/{hooks,components,lib}/
```
