# BUG-364 root cause

`rust/fspec-tui/src/app/dispatch_exec_stdin.rs` drops errors in two ways:
- `probe_exec_stdin_for` handles a failed `backend.get_exec_stdin_request` with `tracing::debug!(.., "get_exec_stdin_request probe failed (silently dropped)")`. The slot keeps whatever it held, so a stale notice can stay up, or a real one never shows, with nothing telling the user the probe failed.
- `let _ = action_tx.send(..)` discards the result when the probe or the write task reports back. A closed action channel (the app shutting down or the reducer gone) is never noticed.

Fix direction: report probe failures at error level with the session id and give the store a visible error state, and handle `SendError` explicitly.
