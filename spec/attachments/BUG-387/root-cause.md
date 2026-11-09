# BUG-387 root cause

Left in `rust/tools/src/unified_exec/exec_stdin.rs` after BUG-376 fixed the cooldown lock and the detector callback:

1. `now_ms()`: `SystemTime::now().duration_since(UNIX_EPOCH)...unwrap_or(0)`. A clock before the epoch yields 0:
   requests carry `ts_ms: 0` and `cooldown_elapsed` compares against 0, so the cooldown arithmetic silently breaks.
   The cooldown could use `std::time::Instant` (monotonic, no error path); `ts_ms` needs an explicit error (log and
   skip the fire, or carry the error) instead of 0.
2. `emit_exec_stdin_request` (public): when no callback is registered for the agent session it does nothing
   (`SessionRegistry::with` no-op) and `test_emit_with_no_callback_is_noop` locks that in. Same fault class BUG-376
   fixed in the detector: report it (or return a `Result`) so callers know the request went nowhere.
3. `exec_stdin/tests.rs` starts with `#![allow(clippy::unwrap_used, clippy::expect_used)]`; the tests should return
   `Result` instead (same cleanup as BUG-366 for fspec-tui).
