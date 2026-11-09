# BUG-383 root cause

## Symptom
fspec-tui integration tests built on `tests/common/harness` can pass while a spawned app task hangs or panics.

## Cause
`rust/fspec-tui/tests/common/harness/driving.rs`:
- `drain_pending` runs `let _ = timeout(Duration::from_secs(2), async { ... }).await;` - an elapsed timeout is
  dropped, so the test continues on half-drained state.
- `drain_pending`, `wait_for_mock` and `wait_for_self` run `let _ = handle.await;` - a panicked or cancelled task's
  `JoinError` is dropped.
Also `tests/common/mod.rs` composes `MockBackend` with `include!("mock/mock_backend_N.rs")` fragments, which the
project rules forbid for test splits (BUG-355) and which rustfmt does not reach.

## Fix direction
Make `drain_pending` return `Result` (timeout and join errors) and propagate it with `?` at call sites (or fail the
test with a message naming the stuck/panicked task); make the wait helpers report join errors. Replace the
`include!` fragments with real modules. BUG-380 already did this for `tests/common/session_storage_fixture.rs`.
