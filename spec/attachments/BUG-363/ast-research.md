# BUG-363 AST research

## fspec astgrep --pattern 'let _ = $A.child.kill().await;' --lang rust --path rust/tools/src/unified_exec
rust/tools/src/unified_exec/process_store_handles.rs:83:17:let _ = entry.child.kill().await;
rust/tools/src/unified_exec/process_store.rs:118:13:let _ = entry.child.kill().await;
rust/tools/src/unified_exec/process_store_eviction.rs:36:17:let _ = e.child.kill().await;


`try_wait` is in process_store_handles.rs (`Err(_) => Some(None)`). The eviction `unwrap_or(false)` is in process_store_eviction.rs.
