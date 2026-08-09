# Cadence — Build Progress

This file is the single source of truth for build status. It is updated after every
completed task. If you are picking this project back up (fresh session, fresh agent,
after a reset, etc.), **read this file first**, then `cadence-build-plan.md` for full
spec detail, then continue with the first unchecked task below.

## How to resume this project

1. Read this file top to bottom.
2. Read `cadence-build-plan.md` for the full spec of whichever phase is next.
3. Check environment: local toolchain notes are in "Environment notes" below.
4. Pick the first unchecked task, in order. Phases are sequential — do not skip ahead,
   later phases depend on earlier ones working and tested.
5. For each task: plan briefly, implement, test, commit with a real message
   (no AI co-author trailer), then check it off here and update "Current status".
6. Commit progress.md updates together with the code change they describe.
7. If you finish everything, do a final polish pass (README, demo instructions),
   mark the project COMPLETE at the top of this file, and stop.

## Ground rules

- Small, working commits. Never commit code that doesn't build.
- Every phase must be tested before moving to the next (see build plan's
  "Definition of done" per phase).
- Do not ask the user questions — make the best reasonable engineering decision
  and note it in this file under "Decisions log" if it deviates from the build plan.
- Do not add scope beyond the build plan. Known gaps are intentionally deferred —
  see `cadence-build-plan.md` "Known Gaps" section, and don't silently fix them.
- Git commits must NOT include Claude/AI co-author trailers.

## Environment notes

- OS: Windows 11, no admin rights available.
- Rust: installed via `rustup` (winget, MSVC toolchain — MSVC linker is available via
  an existing Visual Studio 2026 Community install). `~/.cargo/bin` has cargo/rustc/rustup.
- wasm32-unknown-unknown target: installed.
- wasm-pack: installed via `npm install -g wasm-pack` (v0.15.0) — chosen over `cargo install`
  because it fetches a prebuilt binary instead of a slow source compile.
- Node/npm: present system-wide (v22 / 10.9).
- **Docker: NOT available** (not installed, and Docker Desktop needs admin rights this
  environment doesn't have). This means `docker compose up` cannot be verified locally.
  Dockerfiles/compose are still written correctly per spec; a note is added to README
  flagging that a machine with Docker should be used to verify the one-command startup.
  If a later session (e.g. a cloud sandbox) has Docker available, verify it there and
  update this note.
- **Postgres**: no system install (winget's PostgreSQL package is an EDB installer that
  needs admin for the Windows service). Plan: use the `postgresql_embedded` Rust crate
  as a dev-dependency for relay-server integration tests, so `cargo test` works standalone
  without Docker or a system Postgres install. Production/docker-compose path still uses
  real Postgres via `sqlx` exactly per spec.
- GitHub: `gh` CLI authenticated as `jamesmartin6`.

## Decisions log

(Deviations from the build plan, with rationale, go here as they happen.)

- Using `postgresql_embedded` crate as a relay-server dev-dependency to make
  `cargo test` self-contained without Docker (see Environment notes). Does not change
  the production dependency — that's still `sqlx` against a real Postgres, exactly as
  specced, wired via `DATABASE_URL`.

## Current status

**Phase: 0 (scaffold) — in progress.**

## Task Checklist

### Phase 0 — Repo scaffold
- [ ] Git repo initialized, directory structure created
- [ ] GitHub repo created and pushed
- [ ] progress.md (this file) in place
- [ ] Autonomous continuation mechanism set up (scheduled cloud routine, hourly)

### Phase 1 — CRDT engine core (pure Rust, no WASM)
- [ ] `crdt-engine` crate scaffolded (Cargo.toml, lib layout)
- [ ] `id.rs`: OpId (site_id, counter), total ordering
- [ ] `op.rs`: Operation::Insert / Operation::Delete
- [ ] `rga.rs`: Rga core structure, insert_local, apply_remote (RGA insert rule)
- [ ] `doc.rs`: Doc wrapper, to_string, insert_local(index), delete_local(index)
- [ ] Unit tests: sequential inserts, delete removes correct char, order-independence
- [ ] Proptest convergence tests across 2-4 simulated sites, random orderings
- [ ] All tests green across many seeds (run proptest with a high case count)

### Phase 2 — WASM bindings
- [ ] `#[wasm_bindgen]` CrdtDoc wrapper in lib.rs (new/insert/delete/apply_remote/to_string)
- [ ] serde + serde-wasm-bindgen for Operation JSON transport
- [ ] `wasm-pack build --target web` succeeds
- [ ] Minimal HTML/JS test page loads WASM, inserts chars, to_string() updates
- [ ] Two test-page instances converge via manually copy-pasted serialized ops

### Phase 3 — Relay server (Axum)
- [ ] Cargo scaffold, Axum app, basic routing
- [ ] `POST /docs` creates doc, returns id
- [ ] `GET /ws/docs/{doc_id}` websocket upgrade + join room
- [ ] `rooms.rs` in-memory registry (broadcast channel per room)
- [ ] Postgres schema + migrations (docs, ops tables)
- [ ] Op persistence on receipt, full-log replay to new joiners
- [ ] Presence message type (not persisted)
- [ ] Test: two scripted WS clients converge in real time
- [ ] Test: late joiner receives full log and reconstructs doc
- [ ] Test: server restart doesn't lose data (ops read back from Postgres)

### Phase 4 — Frontend
- [ ] Vite + React + TS scaffold
- [ ] wasm-pack output wired into frontend build
- [ ] `useCrdtDoc.ts`
- [ ] `useDocSocket.ts` (reconnect w/ backoff, offline queueing)
- [ ] `Editor.tsx`
- [ ] `PresenceCursors.tsx`
- [ ] `ConnectionStatus.tsx`
- [ ] Manual verification: two tabs sync within ~100ms
- [ ] Manual verification: offline edit + reconnect merges cleanly
- [ ] Manual verification: multi-cursor presence visible live

### Phase 5 — Persistence & document management
- [ ] `GET /docs` listing endpoint
- [ ] Frontend document picker / landing page
- [ ] Verify: restart + reopen doc restores full content
- [ ] Verify: docs list shows all created documents

### Phase 6 — Dockerization & polish
- [ ] `docker-compose.yml` (postgres + relay-server; frontend build documented)
- [ ] Dockerfile(s) for relay-server (and frontend static serving)
- [ ] README: architecture explanation, setup, offline-merge demo walkthrough,
      Known Gaps section, resume story notes
- [ ] Final pass: verify `docker compose up` where Docker is available; otherwise
      document clearly that it needs verification on a Docker-capable machine

## Notes / gotchas discovered during build

(Append here as encountered — e.g. platform quirks, crate version issues.)
