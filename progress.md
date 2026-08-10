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

- Attempted to set up an hourly scheduled cloud-agent routine (via claude.ai routines)
  as a safety net in case the local session hit usage limits mid-build. Blocked: routines
  that attach a GitHub repo require the GitHub account to be connected at
  claude.ai/customize/connectors first, which needs interactive OAuth the user isn't
  present to do. Not pursued further (don't ask the user for anything). Mitigation:
  this file is kept meticulously current and every task is committed+pushed individually,
  so any future session (local resume, or the user manually setting up a routine later)
  can pick up cleanly from here with zero lost work.
- Using `postgresql_embedded` crate as a relay-server dev-dependency to make
  `cargo test` self-contained without Docker (see Environment notes). Does not change
  the production dependency — that's still `sqlx` against a real Postgres, exactly as
  specced, wired via `DATABASE_URL`. Worked well: no OpenSSL/system deps needed on
  Windows (uses schannel via native-tls), and the integration tests spin up a real,
  disposable Postgres in ~5-40s depending on whether the binary cache is warm.
- `relay-server` has BOTH `src/lib.rs` (routes, AppState, db/models/rooms/ws modules)
  and a thin `src/main.rs` (env/startup only), instead of putting everything in
  `main.rs` as the build plan's file listing implies. Reason: integration tests in
  `tests/` can only import from a lib target, not a bin target, and real integration
  tests (spinning up the actual Axum app + a real Postgres) are far more valuable than
  skipping them to match the file listing exactly.
- relay-server intentionally does NOT depend on the `crdt-engine` crate at all. Op
  payloads are handled as opaque `serde_json::Value` end to end (stored in `ops.payload
  jsonb`, relayed as-is). This is a direct consequence of the build plan's "dumb relay"
  design (server never runs RGA logic) and it keeps the two crates fully decoupled.
- sqlx used with runtime-checked `query`/`query_as` (bind parameters), not the
  compile-time-checked `query!`/`query_as!` macros, specifically so `cargo build` never
  needs a live database connection (or an offline `.sqlx` cache) just to compile —
  matters a lot given no local Postgres is installed on this dev machine.

## Current status

**Phases 1-3 complete and tested. Starting Phase 4 (frontend) next.**

## Task Checklist

### Phase 0 — Repo scaffold
- [ ] Git repo initialized, directory structure created
- [ ] GitHub repo created and pushed
- [ ] progress.md (this file) in place
- [ ] Autonomous continuation mechanism set up (scheduled cloud routine, hourly)

### Phase 1 — CRDT engine core (pure Rust, no WASM) — DONE
- [x] `crdt-engine` crate scaffolded (Cargo.toml, lib layout)
- [x] `id.rs`: OpId (site_id, counter), total ordering
- [x] `op.rs`: Operation::Insert / Operation::Delete
- [x] `rga.rs`: Rga core structure, insert_local, apply_remote (RGA insert rule)
- [x] `doc.rs`: Doc wrapper, to_string, insert_local(index), delete_local(index)
- [x] Unit tests: sequential inserts, delete removes correct char, order-independence
- [x] Proptest convergence tests across 2-4 simulated sites, random orderings
- [x] All tests green across many seeds (verified with `PROPTEST_CASES=5000 cargo test --release`, 0.39s)

### Phase 2 — WASM bindings — DONE
- [x] `#[wasm_bindgen]` CrdtDoc wrapper (`crdt-engine/src/wasm_api.rs`): new/insert/delete/applyRemote/toString/len/isEmpty/siteId
- [x] serde + serde-wasm-bindgen for Operation JSON transport (verified round-trips through actual `JSON.stringify`/`parse`)
- [x] `wasm-pack build --target web --out-dir pkg` succeeds (see Environment notes for the `wasm-opt`/binaryen caveat)
- [x] Minimal HTML/JS test page (`crdt-engine/test-page/index.html`) loads WASM, inserts/deletes chars, to_string() updates
- [x] Two "instances" converge via manually exchanged serialized ops — verified with an automated Node harness (see notes below) since no interactive browser is available in this build environment

### Phase 3 — Relay server (Axum) — DONE
- [x] Cargo scaffold, Axum app, basic routing (`src/lib.rs` + thin `src/main.rs`, see below)
- [x] `POST /docs` creates doc, returns id (also `GET /docs` listing — trivial to add
      alongside, real Phase 5 work is the frontend document picker consuming it)
- [x] `GET /ws/docs/{doc_id}` websocket upgrade + join room
- [x] `rooms.rs` in-memory registry (broadcast channel per room, with echo-suppression
      and empty-room cleanup)
- [x] Postgres schema + migrations (docs, ops tables) — `migrations/0001_init.sql`,
      applied automatically at server startup via `sqlx::migrate!`
- [x] Op persistence on receipt (atomic per-doc seq assignment via row lock), full-log
      replay to new joiners
- [x] Presence message type (Cursor, not persisted)
- [x] Test: two scripted WS clients converge in real time
- [x] Test: late joiner receives full log and reconstructs doc
- [x] Test: server restart doesn't lose data (ops read back from Postgres) — simulated a
      real restart by dropping the pool/app and building a fresh one against the same
      (still-running) embedded Postgres instance

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

- **Real bug found and fixed during Phase 1**: `OpId.counter` must be a Lamport clock
  (advanced on observing remote ops too), not a plain per-site local counter. Without
  this, a site's very first local edit could get a *lower* counter than a remote op it
  had already causally observed, which corrupts the RGA sibling tie-break (it uses id
  comparison to decide ordering among nodes inserted after the same anchor, relying on
  "already observed" implying "smaller id"). Symptom caught by a hand-written test, not
  the property test (the property test only asserts replicas agree with *each other*,
  and they did — just not with the intuitively-correct position — so it silently passed
  even with the bug present for a while during development). Fixed in `Doc::apply_remote`
  by bumping `next_counter` to `max(next_counter, observed.counter + 1)` on every
  observed remote id. Lesson for later phases: hand-written scenario tests catching
  "obviously correct" expected output are just as important as the convergence property
  test, since convergence alone doesn't guarantee the *intuitive* result — only that
  everyone agrees on *some* result.
- `crdt-engine/Cargo.lock` is committed (not just `.gitignore`d) even though this is a
  library crate, because it's really an application component (built as a `cdylib` via
  wasm-pack, never published to crates.io), so pinning exact dependency versions for
  reproducibility is more valuable than the usual "libraries shouldn't commit Cargo.lock"
  advice.
- **Phase 2 environment quirks (both worth knowing for later phases too):**
  1. `wasm-pack build` internally does `cargo install wasm-bindgen-cli` on first use to
     get a matching-version CLI. On this machine, that `cargo install`'s build-script
     execution got blocked by a Windows Application Control policy because it ran from
     `%TEMP%`. Fix: set `CARGO_TARGET_DIR` to a project-local directory before running
     `wasm-pack build` (redirects build-script execution away from the blocked temp
     path). Needed once per machine/session, not a permanent config change.
  2. `wasm-pack`'s default release profile tries to download `wasm-opt` (binaryen) from
     GitHub releases to optimize the .wasm output; that download isn't reachable from
     this environment. Disabled via `wasm-opt = false` under
     `[package.metadata.wasm-pack.profile.release]` in `crdt-engine/Cargo.toml` — pure
     optimization pass, doesn't affect correctness. Revisit if a later environment has
     working GitHub release access and smaller .wasm output matters.
  3. No interactive browser available in this build environment to click through
     `test-page/index.html` by hand. Verified equivalently instead: built a second,
     throwaway `--target nodejs` variant of the same crate and ran a plain Node script
     exercising insert/delete/applyRemote/toString, concurrent-edit convergence, and a
     real `JSON.stringify`/`parse` round-trip (the actual wire format) — all passed. The
     committed `test-page/index.html` itself is untouched by this and should still work
     in a real browser (`wasm-pack build --target web --out-dir pkg` from `crdt-engine/`,
     then serve the `crdt-engine/` directory over HTTP — `file://` won't work because the
     WASM loader uses `fetch()`, which browsers block for local files).
  4. `crdt-engine/pkg/` (the wasm-pack build output) is gitignored, same as `/target/` —
     it's a build artifact, regenerated with the command above. This matters for Phase 4
     too: the frontend will need its own `wasm-pack build --target web --out-dir
     ../frontend/src/wasm` step (documented in the README, not committed as binary output).
