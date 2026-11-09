# BUG-399 AST research

- Definition: `rust/sessions/src/handle_impl/handle_impl_1.rs:18` (impl for `SessionManager`).
- Trait: `SessionManagerHandle::create_session` / `create_session_checked` (find with
  `grep -rn "fn create_session(&self, role" rust`), with other implementers (stubs / mocks) to update.
- Callers: `grep -rn "\.create_session(None)\|create_session(Some" rust --include=*.rs`.
