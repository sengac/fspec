# BUG-383 AST research

- `grep -n "let _ =" rust/fspec-tui/tests/common/harness/driving.rs` -> `drain_pending` timeout and join results,
  `wait_for_mock` / `wait_for_self` join results.
- `grep -rn "drain_pending()" rust/fspec-tui/tests | wc -l` -> many call sites; changing the signature touches all of
  them (batch the change).
- `grep -n "include!" rust/fspec-tui/tests/common/mod.rs` -> `mock/mock_backend_1.rs` .. `mock_backend_7.rs`.
