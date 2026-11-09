# BUG-379 root cause

Found while fixing BUG-361 (`rust/fspec-tui/src/app/dispatch_reconnect.rs`). The same silent-drop pattern exists in
two sibling refetch paths:

- `app/bootstrap.rs:153`: on `RecvError::Lagged` the work_units subscriber resyncs with
  `if let Ok(snapshot) = backend.list_work_units().await { let _ = tx.send(..) }`. A failed resync is invisible and
  the board stays behind the daemon.
- `app/dispatch_daemons.rs:97-101`: after a daemon switch, `list_work_units` and `checkpoint_counts` use the same
  `if let Ok` + `let _ = send`, so the newly chosen daemon's board can silently show the old daemon's work units.
- `app/dispatch_daemons.rs:64`: `let _ = tx.send(Action::SessionOperationFailed(..))` drops a closed-channel send.

Fix direction (as BUG-361): send `Action::SessionOperationFailed` with the reason and `tracing::error!` on a failed
fetch, and log closed-channel sends. Test each path with `MockBackend::fail_list_work_units` (added by BUG-361).

## Fix (2026-11-03)
New `rust/fspec-tui/src/app/work_units_refetch.rs` holds `refetch_work_units`, `refetch_checkpoint_counts` and
`send_or_log`. Reconnect (BUG-361), the lagged work-unit resync ("after missed updates") and the daemon switch
("after switching to daemon <name>", work units and checkpoint counts) all use it: a failed fetch is logged with
`tracing::error!` and sent as `SessionOperationFailed`; a closed app loop is logged. The daemon connect result sends
use `send_or_log` too. `MockBackend` gained `fail_checkpoint_counts`. Red run: all three scenarios failed (no dialog /
resync timeout); green with the reconnect, daemon switcher, bootstrap and checkpoint regressions.
Coverage line numbers of the edited files were remapped in the 13 other features that map them.
