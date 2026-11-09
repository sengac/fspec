# BUG-396 AST research

- `grep -n "compaction_threshold\|set_model_limits" rust/sessions/src/session_creation_helper.rs` → line 240
  `session.set_model_limits(resolved);`, inline test lines 358-361 assert `set_model_limits(resolved)`.
- `rust/sessions/tests/rpc425_session_creation_refactor.rs` lines 167-185: reads the helper via `read(&helper_path())`
  and asserts `resolved.compaction_threshold`.
