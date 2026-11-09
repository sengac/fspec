# BUG-402 root cause

`SessionManager::get_session(&self, id) -> Result<Arc<BackgroundSession>, String>`
(`rust/sessions/src/session_manager/manager_impl_3.rs:26`) reports why a lookup failed. The `SessionManagerHandle`
implementation in `rust/sessions/src/handle_impl/` throws that reason away at about 30 sites:

- `Err(_) => Err(format!("Session not found: {}", ..))` replaces the real error with a fixed message
  (e.g. `send_hitl_response`, `handle_impl_4.rs:23`).
- `.ok()` turns the error into `None` for methods that return `Option` (`get_pause_state` `handle_impl_4.rs:29`,
  and the getters at lines 37 and 53), so "the session lookup failed" and "nothing is pending" look the same to the
  caller.

Both break the no-fallbacks rule (no `if let Ok`/`.ok()` that drops the error). Fix: pass the `get_session` error
through unchanged for `Result` methods; for `Option` methods, either change the trait to return
`Result<Option<_>, String>` or log the error with `tracing::warn!` and document why `None` is correct. The trait
change touches the RPC surface, so `refac019_contracts` and `board024_additive_api_contract` must run.

Found during BUG-397 while correcting the RPC-410 send-path coverage mapping.
