# Root cause: the reconnect refetch throws away errors

`rust/fspec-tui/src/app/dispatch_reconnect.rs`:
- `:152`: `if let Ok(units) = backend.list_work_units().await { ... }`. A failed refetch after reconnect is dropped with no log and no user feedback, so the board keeps showing stale work units.
- `:153`: `let _ = action_tx.send(Action::WorkUnitsLoaded(units));` ignores a closed action channel.
- `:94`: `let _ = action_tx.send(Action::ClearReconnectNotice { .. });` does the same.

## Fix
- On refetch failure, dispatch an explicit error action that the UI shows, the way other RPC failures are surfaced (follow the existing pattern for failed `list_work_units`), and log it with `tracing::error!`.
- A send on a closed action channel means the app is shutting down. Handle it explicitly, with a `debug!`/`warn!` explaining the shutdown and the task exiting, instead of `let _`.
- Test that a failing `list_work_units` after `Reconnected` produces the visible error.

Found by the agent that worked BUG-355 through BUG-348 (2026-11-02).
