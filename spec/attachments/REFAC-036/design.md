# REFAC-036 design

TUI-115 must change the compacting-paste scenarios in two test files that were over the 300-line limit:
`rust/fspec-tui/tests/agent_input_paste_routing_rpc403.rs` (343) and `rust/fspec-tui/tests/mux_agent_input_paste_bug179.rs` (388).
Pure move, no behavior change:
- rpc403: `RecordingModal`, `render_once` and `rpc411_hitl_request` move to `tests/rpc403_paste_support/mod.rs`; the supplementary HITL comment is condensed. 295 lines.
- bug179: `sid`, `fresh_app`, `drain_pending`, `submit`, `enter_mux_board_agent_agent`, `click_pane` and `render_once` move to `tests/bug179_paste_support/mod.rs`; the 3-line scenario banners become 1-2 comment lines and unused imports go. 293 lines.
Every test and `@step` comment is unchanged. Coverage for both features is re-linked to the new line ranges, keeping the impl mappings.
