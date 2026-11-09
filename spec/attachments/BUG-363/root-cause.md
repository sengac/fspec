# BUG-363 root cause

`ProcessStore` in `rust/tools/src/unified_exec/` discards process-control errors:
- `close_owned_session` (process_store.rs), `close_session` (process_store_handles.rs) and `evict_lru_if_full` (process_store_eviction.rs) call the platform kill handle and then `let _ = entry.child.kill().await;`. A failed kill of the direct child is never reported.
- `try_wait` maps `Err(_)` to `Some(None)`, so an OS error waiting on the child is reported as "still running".
- `evict_lru_if_full` does `e.child.try_wait().map(|s| s.is_some()).unwrap_or(false)`, so the same error silently counts as "not exited" for eviction.

The errors are lost and callers act on guessed state. Fix: give `try_wait` a `Result`, and propagate kill errors (or log them with the session id where the caller can't act on them) instead of dropping them.
