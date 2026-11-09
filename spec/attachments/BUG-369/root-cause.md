# BUG-369 root cause

Three silent paths around exec-stdin and session ids:
- `rust/sessions/src/background_session/exec_stdin.rs`: `set_exec_stdin_request` does `if let Ok(mut guard) = self.exec_stdin_request.write()`, and `get_exec_stdin_request` does `.read().ok()?`. A poisoned lock silently drops the update or reports "no request".
- `rust/core/src/session_manager_handle/handle_trait_3.rs`: the default `write_exec_stdin` returns `Ok(())` without writing anything, so a handle that does not override it claims success. TOOL-027's new `control_exec_stdin` default refuses instead.
- `rust/sessions/src/handle_impl.rs` `uuid_from` maps an unparsable session id to `Uuid::nil()`, so the caller looks up the nil session instead of rejecting the id.

Fix direction: return errors (or log and surface them) for the poisoned lock, make the default write refuse, and make `uuid_from` fallible.
