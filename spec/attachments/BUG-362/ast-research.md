# BUG-362 AST research

## fspec astgrep --pattern 'let _ = $A.write_all($$$B)$$$C' --lang rust --path rust/tools/src/unified_exec
rust/tools/src/unified_exec/spawning.rs:533:21:let _ = w.write_all(&data);


The pipe forwarder (spawning.rs:292-293) uses an awaited form that this pattern does not match. grep finds it: `let _ = stdin.write_all(&data).await;` and `let _ = stdin.flush().await;`.
