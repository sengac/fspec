# BUG-376 AST research

`fspec astgrep --pattern 'if let Ok($X) = LAST_FIRE_MS.write() { $$$ }' --lang rust --path rust/tools/src`
- `unified_exec/exec_stdin.rs:130` record_fire, `:144` reset_cooldown.

`fspec astgrep --pattern '$E.unwrap_or_else(|| Arc::new(|_| {}))' --lang rust --path rust/tools/src`
- `unified_exec/exec_stdin.rs:179` (detector emit closure).

`LAST_FIRE_MS.read().ok().is_none_or(..)` at `:122` (cooldown_elapsed).
