# BUG-391 root cause

`AgentViewStore::session_index_for` (rust/fspec-tui/src/store/agent_view/session_indexing.rs) returns `(0, 0)` when
the session is not open. `(0, 0)` is a sentinel: the caller cannot tell "not open" from a real slot without knowing the
convention, and the mux pane header paints it as a slot number. Found during BUG-377, which made the same store's
focus path report a missing session explicitly.

Fix direction: return `Option<(usize, usize)>` and let the header paint the missing-session case explicitly.
