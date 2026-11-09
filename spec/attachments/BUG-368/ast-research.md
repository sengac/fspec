# BUG-368 AST research

## fspec astgrep --pattern 'status.code().unwrap_or(-1)' --lang rust --path rust/tools/src/unified_exec
rust/tools/src/unified_exec/tool.rs:242:37:status.code().unwrap_or(-1)
rust/tools/src/unified_exec/tool.rs:504:31:status.code().unwrap_or(-1)


grep also finds `exit_code: status.code().or(Some(-1))` (tool.rs ~468), `quiet_secs(session_id).await.unwrap_or(0)` (tool.rs ~483) and spawning.rs `let _ = tokio::task::spawn_blocking(move || pty_child.wait()).await;`.
