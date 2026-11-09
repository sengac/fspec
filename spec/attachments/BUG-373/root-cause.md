# BUG-373 root cause

Seen while running the TUI-116 regression batch on 2026-11-02: `cargo test -p codelet-fspec-tui --test
cmpct049_log_level_hygiene` fails 2 of 2 at HEAD (92b00d0d4, before any TUI-116 change touched these files).

```
CMPCT-049: marker '[compaction-status] turn START — status → Running (thinking mode)' not found in source
CMPCT-049: marker '[compaction-status] set_status transition' not found in source
```

## Why
1. **Module splits.** The test reads single files: `agent-loop/src/agent_loop.rs` and
   `sessions/src/background_session.rs`. Those modules were split into submodule directories, and the markers now live
   in `agent_loop/prompt.rs`, `agent_loop/reconciliation.rs`, `background_session/output.rs` and
   `background_session/progress.rs`. The markers are still logged at the right levels there.
2. **Removed audit lines.** Commit 2699693b7 ("harden provider sessions, compaction, and tool execution") rewrote
   `agent-loop/src/generate_compaction_handler.rs` around a `#[tracing::instrument]` span and dropped every
   `[generate-compaction]` INFO event. CMPCT-049 rule 6 requires the compaction sub-agent run to keep an INFO audit
   trail (ENTER and sub-agent run COMPLETE at least), so the trail is now missing in production logs.
3. **Removed feature.** The same rewrite removed the turn-END pending-DAG pin step, so the
   `turn END — checking for pending DAG to pin` marker no longer exists. The scenario still lists it.
4. `update_compaction_progress` no longer logs its own line; it delegates to `set_compaction_progress`, whose DEBUG
   line covers both.

## Fix
- The test reads a module's file plus every `.rs` file in its submodule directory, and returns errors instead of
  panicking (drops its `#![allow(clippy::...)]`).
- Restore `[generate-compaction] ENTER` and `[generate-compaction] sub-agent run COMPLETE` as `tracing::info!` in the
  handler's `run`.
- Update the feature: drop the pending-DAG pin marker from rule 5, note that `update_compaction_progress` logs through
  `set_compaction_progress`, and add a step for the handler's INFO audit lines.
