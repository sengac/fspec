# AST Research: WorkUnitInfo lift + tarpc adoption

**Work Unit:** RPC-005
**Date:** 2026-05-08

## Existing definitions to migrate / re-export

### WorkUnitInfo (target of the lift)

- `codelet/napi/src/types.rs:182` — `pub struct WorkUnitInfo` with `#[napi(object)]` derive
  - Fields: `id: String`, `title: String`, `work_type: String` (js_name = "workType"), `status: String`, `description: Option<String>`, `estimate: Option<i32>`, `epic: Option<String>`
  - This file is the SINGLE definition site in the workspace (verified by ast-grep across `codelet/napi/src/`).

### get_all_work_units (TS-facing function the smoke test will hit)

- `codelet/napi/src/work_units_watcher.rs:222` — `pub fn get_all_work_units() -> Result<Vec<WorkUnitInfo>>`
  - This is the NAPI entry point exposed to TypeScript via `index.d.ts`.
  - The Vitest smoke test (Scenario 9) imports this and asserts the JS shape (`workType` camelCase) is unchanged after the lift.

## tarpc adoption

- `cargo grep tarpc codelet/`: **no existing tarpc usage** in the workspace. RPC-005 introduces it for the first time.
- `tarpc` and `bincode` will be added to `[workspace.dependencies]` in `codelet/Cargo.toml`.
- `tokio-tungstenite = "0.26"` is already a workspace dep (used by `codelet/tools` for bridge relay) — the rpc-server crate reuses it.

## Crates to be added (workspace.members)

Currently: `cli`, `common`, `core`, `git`, `napi`, `providers`, `tools`, `tui`.

To add (RPC-005):
- `rpc-types`   — pure-serde shared types (single source of truth)
- `rpc`         — `#[tarpc::service] trait FspecService { ... }`
- `rpc-embedded`— in-process `tarpc::transport::channel` transport
- `rpc-server`  — minimal WS daemon binary (`tokio-tungstenite` + bincode framed `Envelope`)

## Expansion planning (deferred to later cards)

- `codelet/napi/src/types.rs` defines ~76 NAPI types — RPC-005 lifts ONLY `WorkUnitInfo`.
- `codelet/napi/src/lib.rs` exposes ~191 NAPI functions — RPC-005 lifts ONLY `list_work_units`.

## Test fixtures location

- Reuse pattern from `codelet/tools/src/bridge_test_fixtures.rs` (already wraps `tokio-tungstenite::accept_async` for test servers binding to ephemeral ports).
- Source-shape assertions for "WorkUnitInfo defined once" / "EmbeddedTransport requires Handle" use `ast-grep` invocations from `cargo test` integration tests (no runtime spawn needed for those scenarios).
