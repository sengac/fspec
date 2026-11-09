# BUG-404 AST research

- `rust/tools/src/facade/wrapper.rs:676` `pub type GetEffectiveCwdCallback = fn(String) -> Option<IsolationContext>;`
- `rust/tools/src/facade/wrapper.rs:734` `get_isolation_context` = `GET_EFFECTIVE_CWD_CALLBACK.get().and_then(|cb| cb(id))`.
- `rust/sessions/src/session_tool_callbacks.rs` `session_by_id` (`.ok()?` on the uuid parse and on `get_session`),
  `isolation_context` (`session.worktree_path()?`).
- Callers: `fspec astgrep --lang rust --pattern 'get_isolation_context($A)' --path rust` lists the path validators
  (`validate_and_resolve_path*`) and tests.
