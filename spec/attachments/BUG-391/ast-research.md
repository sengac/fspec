# BUG-391 AST research

- `grep -rn "session_index_for" rust/fspec-tui/src` → the definition in `store/agent_view/session_indexing.rs` and its
  callers in the mux pane header render path.
- The unit tests in the same file pin `(0, 0)` for an unknown session and for an empty store.
