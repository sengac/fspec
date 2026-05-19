# RPC-022 — AST research for modal dialog port surface

This document captures the AST-based research that informed the
RPC-022 implementation plan. The goal is to anchor the new symbols
(types, traits, methods, enums) against the existing source so the
modal dialog port lands without ambiguity about where each new piece
plugs in.

## Methodology

Ran `AstGrep` queries against the Rust workspace under `codelet/`
for the following structural patterns:

1. `pub enum Priority { $$$VARIANTS }` in
   `codelet/fspec-tui/src/components/mod.rs` — to locate the enum we
   extend with the new `Foreground = 900` variant (rule [4]).
2. `async fn $NAME($$$ARGS) -> $RET;` in `codelet/rpc/src/lib.rs` — to
   inventory the existing tarpc `FspecService` surface so the five new
   methods (rule [0]) extend rather than collide with the current set.
3. `fn $NAME(&self, $$$ARGS) -> $RET { $$$BODY }` in
   `codelet/core/src/session_manager_handle.rs` — to inventory the
   existing `SessionManagerHandle` default-impl pattern so the four
   new methods (rule [2]) follow the same shape.

## Findings

### Priority enum (rule [4])

Located at `codelet/fspec-tui/src/components/mod.rs:27`. Current
variants are `Background = 100`, `Low = 200`, `Medium = 500`,
`High = 800`, `Critical = 1000`. The new `Foreground = 900` variant
slots between `High` and `Critical`. The `#[repr(u32)]` attribute is
already present; no Cargo manifest change is needed.

### FspecService trait (rule [0])

The trait currently exposes 17 async methods. Naming convention is
either:

  - `Result<T, String>` for fallible writes that surface diagnostics
    across the wire (e.g. `move_work_unit_up`, `persistence_*`), or
  - bare `T` for queries with safe defaults
    (e.g. `list_work_units`, `get_model_info`, `get_thinking_level`).

The five new methods naturally split:

  - `list_providers() -> Vec<ProviderInfo>` — query, safe default
    `Vec::new()`.
  - `get_session_role(session_id: SessionId) -> Option<String>` —
    query, safe default `None`.
  - `set_session_model(session_id, provider_id, model_id) -> Result<(), String>` —
    write.
  - `set_thinking_level(session_id, level: ThinkingLevel) -> Result<(), String>` —
    write.
  - `set_session_role(session_id, role: Option<String>) -> Result<(), String>` —
    write.

### SessionManagerHandle trait (rule [2])

Located at `codelet/core/src/session_manager_handle.rs`. The trait
already has two default-impl methods (`get_model_info` and
`get_thinking_level`) from RPC-018 that return safe defaults via the
unused-arg pattern `let _ = session_id;`. RPC-022 extends with:

  - `fn list_providers(&self) -> Vec<ProviderInfo>` —
    default returns `Vec::new()`.
  - `fn set_model(&self, sid: &SessionId, provider: &str, model: &str) -> Result<(), String>` —
    default returns `Ok(())`.
  - `fn set_thinking_level(&self, sid: &SessionId, level: ThinkingLevel) -> Result<(), String>` —
    default returns `Ok(())`.
  - `fn get_role(&self, sid: &SessionId) -> Option<String>` —
    default returns `None`.
  - `fn set_role(&self, sid: &SessionId, role: Option<String>) -> Result<(), String>` —
    default returns `Ok(())`.

The existing `StubSessionManagerHandle` impl picks up the defaults
without source changes.

## Implementation order derived from AST research

1. Add `Foreground = 900` to `Priority` enum (single-file change).
2. Add `ProviderInfo` + `ModelEntry` to `codelet/rpc-types/src/lib.rs`.
3. Add five default-impl methods to `SessionManagerHandle` trait.
4. Add five async methods to `FspecService` trait + impl on
   `FspecServiceImpl`.
5. Add five trait methods to `FspecBackend` plus impls on
   `EmbeddedFspecBackend` + `WebSocketFspecBackend`.
6. Add new `Action` variants in `components/mod.rs`.
7. Add `role_by_session` field + accessors to `AgentViewStore`.
8. Author `model_selector_dialog.rs`, `thinking_level_dialog.rs`,
   `role_banner.rs`.
9. Author `app/dispatch_rpc022.rs` with the new helpers + the
   `parse_slash_command` parser.
10. Wire up `App::dispatch` match arms + `handle_input_submitted`
    interception.
11. Add cross-transport parity tests in `codelet/fspec-tui/tests/`.
