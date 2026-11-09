# BUG-399 root cause

`rust/sessions/src/handle_impl/handle_impl_1.rs`:

```rust
fn create_session(&self, role: Option<String>) -> SessionId {
    crate::handle_creation::create_session_checked(self, role).unwrap_or_else(|error| {
        tracing::error!(%error, "legacy create_session failed; use create_session_checked for the cause");
        SessionId::new(String::new())
    })
}
```

A failed creation returns an empty `SessionId`, a sentinel every caller would have to know about. BUG-397 shows a test
treating it as a live session. Fix: remove the infallible `create_session` from `SessionManagerHandle` (callers use
`create_session_checked`) or change it to return `Result<SessionId, String>`, updating every implementer and caller.
