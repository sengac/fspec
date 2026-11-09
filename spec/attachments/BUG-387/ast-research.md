# BUG-387 AST research

- `grep -n "unwrap_or(0)" rust/tools/src/unified_exec/exec_stdin.rs` -> `now_ms`.
- `grep -rn "emit_exec_stdin_request" rust/` -> defined and re-exported (`unified_exec/mod.rs:55`); callers are the
  unit tests only, so a signature change is local.
- `grep -n "allow(" rust/tools/src/unified_exec/exec_stdin/tests.rs` -> the inner clippy allow (moved verbatim by
  REFAC-049).
