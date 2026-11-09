# BUG-397 AST research

- `grep -n set_data_directory rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs` → lines 146, 186, 221, 262, 328,
  all `let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());`.
- `fresh_session` (line 41) calls `SessionManagerHandle::create_session(None)`
  (`rust/sessions/src/handle_impl/handle_impl_1.rs:18`), which returns `SessionId::new(String::new())` on error.
- `rust/common/src/data_dir.rs:27` `set_data_directory` overwrites a `Mutex<Option<PathBuf>>`.
