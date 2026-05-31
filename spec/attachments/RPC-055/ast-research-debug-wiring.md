# RPC-055 AST research — current state of `/debug` wiring

## Goal
Identify every site that needs to be touched to wire `/debug` end-to-end, plus the existing reference implementation in `codelet/napi/src/session_bindings.rs::session_toggle_debug` that the extracted-crate path will mirror.

## SessionManagerHandle (trait + stub)
`fn toggle_debug(...)` already declared with default impl + stub override at:
- `codelet/core/src/session_manager_handle.rs:390` — trait default Ok("")
- `codelet/core/src/session_manager_handle.rs:1106` — `StubSessionManagerHandle` impl (toggles + emits `StreamChunk::DebugStateChange`)
- `codelet/sessions/src/handle_impl.rs:357` — `SessionManager` override (toggles atomic only — TODO comment at line 366 says "The full DebugCaptureManager start/stop is wired in RPC-055.")

`fn set_debug_directory(...)` — **does not yet exist anywhere**. Needs to be added at:
- `codelet/core/src/session_manager_handle.rs` (trait + stub override + per-call counter on stub)
- `codelet/rpc/src/lib.rs` (FspecService tarpc + FspecServiceImpl routing)
- `codelet/fspec-tui/src/transport/mod.rs` (FspecBackend trait default)
- `codelet/fspec-tui/src/transport/embedded.rs` (forwarder)
- `codelet/fspec-tui/src/transport/websocket.rs` (forwarder)
- `codelet/sessions/src/handle_impl.rs` (real impl delegating to `codelet_common::debug_capture::set_debug_directory(...)` on the global manager)

## FspecService (tarpc) — toggle_debug already wired
- `codelet/rpc/src/lib.rs:291–294` — RPC declaration
- `codelet/rpc/src/lib.rs:1186–1196` — `FspecServiceImpl::toggle_debug` routes through `session_manager()`

## FspecBackend trait + transports — toggle_debug already wired
- `codelet/fspec-tui/src/transport/mod.rs:398–404` — default Ok(String::new())
- `codelet/fspec-tui/src/transport/embedded.rs:428–437` — forwarder
- `codelet/fspec-tui/src/transport/websocket.rs:743–752` — forwarder

## Slash-command dispatch wiring — currently a no-op notice
- `codelet/fspec-tui/src/views/agent/slash_commands.rs:59` — `Debug` action name
- `codelet/fspec-tui/src/views/agent/slash_commands.rs:129` — palette entry
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs:151–158` — currently falls through to the `other` arm that emits `[notice] /debug not yet implemented in Rust TUI`

## SessionHeader `[DEBUG]` badge — field exists but is hardcoded
- `codelet/fspec-tui/src/views/agent/header.rs:61` — field declaration
- `codelet/fspec-tui/src/views/agent/header_build.rs:22` — accepted as a parameter
- `codelet/fspec-tui/src/views/agent/header_build.rs:78` — renders `" [DEBUG]"` (red + bold) when true
- `codelet/fspec-tui/src/views/agent.rs:255` — **construction site is hardcoded to `false`**

## Store accessor already exists
- `codelet/fspec-tui/src/store/agent_view/isolation_state.rs:90` — `pub fn debug_enabled_for(&self, session: &SessionId) -> Option<bool>` is already on `AgentViewStore` (added in RPC-045)

## DebugStateChange chunk handler — already updates the store
- `codelet/fspec-tui/src/app/dispatch_rpc045.rs:90–93` — `StreamChunk::DebugStateChange { enabled }` arm calls `agent_view_store.set_debug_enabled(session_id.clone(), *enabled)`

## NAPI reference implementation — port pattern
- `codelet/napi/src/session_bindings.rs:2645–2735` (`session_toggle_debug`) — full reference implementation:
  1. Snapshot `SessionMetadata` BEFORE locking debug_capture mutex
  2. Lock debug_capture mutex
  3. Set per-session debug directory via `set_debug_directory_raw({debug_dir}/debug/{session_id}/)`
  4. If enabled → `stop_capture()` → return file path
  5. If disabled → `set_session_metadata(...)` → `start_capture()` → return file path
  6. Persist enabled flag onto BackgroundSession atomic
  7. Emit `StreamChunk::debug_state_change(enabled)` for the TUI to observe

## BackgroundSession's existing surface
- `codelet/sessions/src/background_session.rs:62` — `use codelet_common::debug_capture::{DebugCaptureManager, PoisonRecoveryMutex}`
- `codelet/sessions/src/background_session.rs:318` — `pub debug_capture: Arc<PoisonRecoveryMutex<DebugCaptureManager>>` (per-session)
- `codelet/sessions/src/background_session.rs:631` — `get_debug_enabled()`
- `codelet/sessions/src/background_session.rs:636` — `set_debug_enabled(enabled)`

The extracted-crate `toggle_debug` in `handle_impl.rs` therefore has access to `session.debug_capture` and `session.handle_output` — it can port the NAPI reference 1:1 minus the NAPI `Error::from_reason` wrapping.

## Test patterns to mirror
- `codelet/fspec-tui/tests/slash_clear_rpc046.rs` — `/clear` slash command end-to-end with MockBackend
- `codelet/fspec-tui/tests/rpc054_cross_transport_parity.rs` — cross-transport parity for `/provider`
- `codelet/fspec-tui/tests/source_shape_rpc054.rs` — source-shape source-file scanning pattern
- `codelet/fspec-tui/tests/common/mod.rs` — `MockBackend` struct already exposes `toggle_debug` would need adding (currently only has `clear_history_calls`, etc.)

## Decisions captured during research
1. **Badge stays in SessionHeader, not SessionFooter**. The TS Ink original puts `[DEBUG]` in `SessionHeader.tsx:170`. The Rust SessionHeader already has the field. The attachment's mention of "SessionFooter" appears to be a misread of the TS source.
2. **debug_dir default**: `".fspec/debug"` per attachment. `FSPEC_DEBUG_DIR` env var override matches the attachment's spec.
3. **No FspecService alias**: `set_debug_directory` accepts a `String` over the wire (napi(object) compat) and converts to `PathBuf` on the trait side.
4. **Single dispatch file `dispatch_rpc055.rs`**: keeps `dispatch_rpc020.rs::handle_slash_command` under the 300-LoC source-shape ceiling — same pattern as `dispatch_rpc046.rs` (`/clear`) and `dispatch_rpc054.rs` (`/provider`).
