# BUG-398 AST research

- `rust/sessions/tests/wt004_session_tool_callbacks/work_unit_stage_lookup.rs` lines 72-105 (the failing scenario),
  `#[serial_test::serial]` plus an `ENV_GUARD` mutex.
- `codelet_sessions::session_tool_callbacks::{unregister_manager, get_isolation_context}` — the registry and the
  singleton fallback under test.
- Sibling modules in `tests/wt004_session_tool_callbacks/` (e.g. `isolated_session_under_the.rs`) — check which are not
  `#[serial]` and touch the registry or the data directory.
