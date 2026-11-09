# BUG-380 AST research

- `grep -rn 'Failed to persist session manifest'` -> `sessions/src/session_manager/manager_impl_3.rs:157`
  (`save_session(&manifest).map_err(..)?`).
- `grep -rn SessionOperationFailed rust/fspec-tui/src/app` -> emitters `dispatch_daemons.rs:64`,
  `dispatch_model_thinking_dialogs.rs:104`, `session_creation_tasks.rs:75`, `session_storage.rs:24`,
  `dispatch_reconnect.rs` (BUG-361); reducer `session_storage.rs:50` pushes `SessionStorageDialog`.
- Fixture: `tests/common/session_storage_fixture.rs` (`creation_actions` at :158, data dir via
  `codelet_common::set_data_directory` at :39).
