# REFAC-036 AST research

## fspec astgrep --pattern 'render_once($A)' --lang rust --path rust/fspec-tui/tests (filtered to the two files)
rust/fspec-tui/tests/mux_agent_input_paste_bug179.rs:210:5:render_once(&mut app)
rust/fspec-tui/tests/agent_input_paste_routing_rpc403.rs:192:5:render_once(&mut h)

The helpers have no callers outside their own test file, so moving them into a per-file support module (the `bug210_progress/support.rs` pattern) changes no other target.
