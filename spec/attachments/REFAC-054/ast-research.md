# REFAC-054 AST research

- `grep -n 'DATA_DIR_GUARD' rust/sessions/tests/prov123_active_selection_updates_default.rs`: one definition
  (`static DATA_DIR_GUARD: Mutex<()>`) and six `.lock()` uses inside the scenarios.
- `rust/sessions/tests/prov123_support/mod.rs` (29 lines) is used only by this test file (`mod prov123_support;`).
