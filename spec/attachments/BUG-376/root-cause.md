# BUG-376 root cause

File: `rust/tools/src/unified_exec/exec_stdin.rs` (found while reading the detector for TUI-117).

1. `cooldown_elapsed` (line ~122): `LAST_FIRE_MS.read().ok().is_none_or(..)` turns a poisoned lock into "cooldown
   elapsed", so after any panic while holding the lock every quiet tick fires a new request with no record of why.
2. `record_fire` (~130) and `reset_cooldown` (~144): `if let Ok(mut last) = LAST_FIRE_MS.write()` drops the poison
   error, so fires stop being recorded (and the 30s cooldown silently stops working) without a log line.
3. `spawn_exec_stdin_detector` (~179): a missing `EXEC_STDIN_REQUEST_CALLBACKS` entry for the agent session is
   replaced by `Arc::new(|_| {})`, so every request and clear for that session is thrown away silently. A
   detector spawned for a session with no registered callback is a wiring fault that should be reported.

All three are fallbacks forbidden by the project rules (no silent defaults, no `if let Ok` that drops the error).
Fix direction: recover the poisoned guard explicitly (`PoisonError::into_inner`) with a `tracing::error!`, or make
the cooldown map a non-poisoning structure; return/log an error when no callback is registered instead of a no-op.

## Fix (2026-11-03)
`exec_stdin.rs` gets `cooldown_read`/`cooldown_write`: a poisoned `LAST_FIRE_MS` guard is recovered with
`PoisonError::into_inner`, the poison is cleared and `tracing::error!` records the recovery, so `cooldown_elapsed`,
`record_fire` and `reset_cooldown` keep working. The detector's emit closure retries the registry lookup and, when no
callback is registered, logs `tracing::error!` with the agent session, the exec session and whether it was a fire or
a clear, instead of calling a no-op closure. REFAC-049 moved the unit tests first (file was 389 lines). Red: the
poisoned cooldown read as elapsed and no error was logged; green with the exec-stdin p1/p4 and BUG-362 regressions.
`now_ms`'s `unwrap_or(0)`, the silent `emit_exec_stdin_request` and the tests' lint allow are filed as BUG-387.
