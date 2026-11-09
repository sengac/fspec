# BUG-401 root cause

`AgentViewStore::session_index` (`rust/fspec-tui/src/store/agent_view.rs`, ~line 291) returns `(0, 0)` when no
session is open and `(current_session_index + 1, len)` otherwise. `(0, 0)` is a sentinel the caller must know about.
BUG-391 made the id-keyed `session_index_for` return `Option` and moved `pane_render` off `session_index()`, so the
only remaining users are tests: `tests/store_agent_view_multisession_rpc024.rs` (lines 40, 65, 90, 110, 248-254) and
`tests/app_dispatch_rpc024.rs` (lines 56, 81, 101), plus the steps "session_index() returns (0, 0)" in
`spec/features/rpc024-multi-session-store.feature` (lines 41 and 127).

Fix: return `Option<(usize, usize)>` (or delete the method if nothing but tests needs it) and update those steps and
tests. Note `agent_view.rs` is 299 lines and a source-shape test pins `pub fn session_index` to it
(`agent_view_store_no_longer_exposes_set_session_index`).
