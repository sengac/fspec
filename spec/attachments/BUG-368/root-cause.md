# BUG-368 root cause

`rust/tools/src/unified_exec/tool.rs` reports guessed values:
- An exit status without a code (killed by a signal) becomes `-1` (`status.code().unwrap_or(-1)`, `.or(Some(-1))`). A reaped session with no stashed status also becomes `-1` (`recover_exit(..).map(..).unwrap_or(-1)`).
- A missing quiet time becomes `0` (`quiet_secs(..).await.unwrap_or(0)`).
- PTY sessions: the store's `Child` is a liveness anchor (`sleep`), so the reported status is the anchor's SIGKILL status, never the command's. `spawning.rs` drops the real result with `let _ = tokio::task::spawn_blocking(move || pty_child.wait()).await;`. TOOL-027's PTY scenarios can therefore only assert that the session exited, not how.

Fix direction: stash the PTY child's real exit status for poll/recover_exit to report, and represent "terminated by signal N" and "status unknown" explicitly in UnifiedExecResult instead of -1 or 0.
