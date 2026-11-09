# BUG-420 AST research

- rust/sessions/src/session_manager.rs:200 `self_weak: OnceLock<Weak<SessionManager>>`
- rust/sessions/src/session_manager/manager_impl_1.rs:48 `self_weak: OnceLock::new()` in `new()`; 58 `new_arc` calls `init_self_weak`; 64-66 `init_self_weak` (`let _ = self.self_weak.set(...)`).
- `unwrap_or_default()` stamps: manager_impl_3.rs:168, manager_impl_4.rs:121, manager_impl_5.rs:123.
- Callers of `SessionManager::new()` that never call `init_self_weak` include the sessions integration tests (rpc422_*, sess002_*).
