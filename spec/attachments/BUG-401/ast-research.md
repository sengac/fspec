# BUG-401 AST research

- `grep -rn "session_index()" rust --include=*.rs` → only the two rpc024 test files above (BUG-391 removed the
  `pane_render.rs` caller).
- Definition: `rust/fspec-tui/src/store/agent_view.rs` `pub fn session_index(&self) -> (usize, usize)`.
- Feature steps: `grep -n "session_index()" spec/features/rpc024-multi-session-store.feature`.
