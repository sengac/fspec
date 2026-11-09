# BUG-373 AST research

`fspec astgrep --lang rust`:

- `tracing::info!($$$)` in `rust/agent-loop/src/generate_compaction_handler.rs`: **no matches**. The handler has no
  INFO events at all; only `#[tracing::instrument(skip_all, fields(target_session_id, caller_session_id))]` on
  `Builder::run` (line 42). Rule 6's audit trail is gone.
- `repo_source($P)` in `rust/fspec-tui/tests/cmpct049_log_level_hygiene.rs`: ten reads. Two of them name modules that
  are now split into directories: `agent-loop/src/agent_loop.rs` (lines 64) and `sessions/src/background_session.rs`
  (lines 66, 157). Their markers live in `agent_loop/{prompt,reconciliation}.rs` and
  `background_session/{output,progress}.rs`.

Text search for the remaining markers: `turn END — checking for pending DAG to pin` and the
`[compaction-status] update_compaction_progress` line exist nowhere in `rust/` (removed by 2699693b7);
`update_compaction_progress` now calls `set_compaction_progress` (`background_session/progress.rs:27`).
