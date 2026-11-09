# BUG-420 root cause

`SessionManager.self_weak` is a `OnceLock<Weak<SessionManager>>` that is only filled by
`SessionManager::new_arc()` / `init_self_weak()` (manager_impl_1.rs 54-66). `SessionManager::new()`
leaves it empty, and many tests (for example rust/sessions/tests/rpc422_session_persistence.rs) build
managers with `Arc::new(SessionManager::new())`.

The three session constructors then stamp the owning-manager back-reference with a silent default:

- manager_impl_3.rs:168 `create_session_with_id`
- manager_impl_4.rs:121 `create_session_from_manifest`
- manager_impl_5.rs:123 `create_isolated_session_with_id`

`self.self_weak.get().cloned().unwrap_or_default()` yields `Weak::new()` when the lock is empty, so the
session's AgentManager handler holds a dangling reference and nothing reports that the manager was
built without its self-reference. `init_self_weak` also discards `OnceLock::set`'s result with `let _ =`.

Fix direction: make the self-reference structural (construct only through `new_arc`, e.g.
`Arc::new_cyclic`) or return an error from the constructors when it is missing; never default.
