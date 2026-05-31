# RPC-055 — `/debug` debug-capture wiring

**Parent:** RPC-030 · **Phase:** 7.2 · **Estimate:** 3 pts · **Depends on:** RPC-054

## Goal

Wire `SlashCommandAction::Debug` to `backend.toggle_debug(session_id, debug_dir)`. Add a global-pre-session `set_debug_directory(path)` RPC method. Render a debug badge in `SessionFooter` when capture is active. `DebugStateChange` chunk handler from RPC-045 already updates store state.

## Backend (already in place)

`DebugCaptureManager` lives in `codelet-common::debug_capture` (NAPI-free). `BackgroundSession::toggle_debug` was at `session_manager.rs` line 7727 (now in `codelet-sessions`). `set_debug_enabled`/`get_debug_enabled` already on `SessionManagerHandle` (RPC-037).

## New RPC method

```rust
// SessionManagerHandle + FspecService + FspecBackend
fn set_debug_directory(&self, path: PathBuf) -> Result<(), String>;
```

Implementation delegates to `codelet_common::debug_capture::set_default_dir(...)` or the equivalent global setter.

## Slash command wiring

```rust
SlashCommandAction::Debug => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("/debug: no active session");
        return;
    };
    let debug_dir = std::env::var("FSPEC_DEBUG_DIR")
        .unwrap_or_else(|_| ".fspec/debug".to_string());
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        match backend.toggle_debug(session_id.clone(), debug_dir).await {
            Ok(file_path) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[debug] capture toggled → {file_path}"),
                });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[error] /debug failed: {e}"),
                });
            }
        }
    });
}
```

## SessionFooter badge

When `agent_view_store.debug_state_for(&session_id) == true`, render a small `[DBG]` badge in `SessionFooter`. Updated via RPC-045's `DebugStateChange` chunk handler.

## Acceptance criteria

1. `set_debug_directory` exists on all three layers.
2. `/debug` toggles capture for the active session and emits a notice with the file path.
3. SessionFooter shows `[DBG]` badge when capture is active.
4. Toggling off removes the badge.
5. Integration test in `codelet/fspec-tui/tests/slash_debug.rs` covers both states.

## TS reference

`AgentView.tsx` line 2643: `sessionToggleDebug(currentSessionId, debugDir)` if session, else `toggleDebug(debugDir)` (pre-session global). Status pushed into conversation.

## Out of scope

- Reading captured debug files (separate concern; usually done outside the TUI).
