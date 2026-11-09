# REFAC-042 design

TOOL-027 adds exec stdin control (SIGINT and EOF) to the unified_exec `ProcessStore`. `rust/tools/src/unified_exec/process_store.rs` is 402 lines, over the 300-line limit, so it is split first. This is a pure move with no behaviour change:
- `process_store.rs` (243 lines) keeps `ChildHandle`, the output clock (`now_micros`), `ProcessEntry`, the `ProcessStore` struct, its lifecycle methods (new, close_owned_session, stash/recover exit, insert, touch, contains, is_recently_used, remove, len, is_empty, list_sessions), `Default`, `SessionInfo` and the global singleton. The `entries` field becomes `pub(super)` so the sibling modules can reach it.
- `process_store_handles.rs` (94 lines) holds the per-session handle accessors: get_stdin_tx, get_output_handles, try_wait, quiet_secs, kill_handle, close_session and session_ids.
- `process_store_eviction.rs` (86 lines) holds `evict_lru_if_full` and the pure `session_id_to_evict` policy. `mod.rs` re-exports `session_id_to_evict` from there, so the public path `codelet_tools::unified_exec::session_id_to_evict` is unchanged.
- Method bodies are byte-for-byte unchanged. Swallowed errors seen during the move (`let _ = child.kill().await`, `try_wait` mapping `Err` to "still running", `unwrap_or(false)` in eviction) are not fixed here. They are filed as their own bug card.

Verification: `cargo test --profile ci-test -p codelet-tools --test unified_exec_tool_test` plus `cargo clippy -p codelet-tools`.
