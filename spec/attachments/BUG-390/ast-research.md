# BUG-390 AST research

- `grep -n "@step" rust/fspec-tui/tests/multilineinput_parity_rpc095.rs` → line 245 `"Type a message..."`; the
  assertion pins `INPUT_PLACEHOLDER_HINT` (`views/agent.rs:111`).
- `grep -n "@step Then row y=" rust/fspec-tui/tests/scrollback_wrap_rpc078.rs` → y=2 (MENU-003) and y=4 (RPC-401).
- `grep -n "@step And the bottom 1-row" rust/fspec-tui/tests/source_shape_rpc013.rs` → line 114, string on line 115.
- `grep -n "AgentEscPressed" rust/fspec-tui/tests/view_agent_multiline_input_rpc019.rs` → line 353 step comment.
