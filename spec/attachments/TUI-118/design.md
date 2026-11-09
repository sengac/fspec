# TUI-118 design (draft, from TUI-115)

TUI-115 keeps the composer editable while the focused session is Running or Compacting. The hardware cursor is still hidden then:
`AgentView::is_cursor_visible_for` (rust/fspec-tui/src/views/agent.rs) returns false for Running/Compacting (RPC-093 rule [8]).

Proposal: show the cursor while busy whenever the busy composer is painted (`busy_composer.rs`), since its anchor already points at the composer row (`last_hitl_input_offset = activity row + queued rows`). The RPC-093 rule [8] scenario must change with it. Needs a user decision first: RPC-093 hid the cursor on purpose while the spinner replaced the composer.

## Decision and handover (2026-11-02)

- The coordinator relayed the user's decision: no user question is needed. The composer is always visible and editable while Running or Compacting (TUI-115 interview-decisions.md), so it must show its caret. Show the hardware cursor on the composer row while busy, and change the RPC-093 rule [8] scenarios to match. Those are "Terminal cursor is suppressed while session is Running/Compacting" in spec/features/agentview-thinking-indicator-animation-parity.feature, lines 48-58.
- Code: `AgentView::is_cursor_visible_for` (rust/fspec-tui/src/views/agent.rs:217) returns false for Running/Compacting. It is called from `hitl_keys.rs:80/85` (`is_cursor_visible`). `cursor_position` already anchors on the busy composer row through `last_hitl_input_offset`, which `busy_composer.rs::paint_busy_input_area` sets. views/agent.rs is at 295 lines.
- Blocker to plan for: the RPC-093 tests live in rust/fspec-tui/tests/thinking_indicator_animation_parity_rpc093.rs, which is 535 lines. Before changing `cursor_suppressed_during_running` / `cursor_suppressed_during_compacting` (lines 108-140), split it under a REFAC card. Use the REFAC-036/037 pattern: helpers go into a #[path] support module and coverage is re-linked with impl mappings kept. Getting it under 300 likely needs the scenario bodies condensed too, because helpers alone are not enough.
- Not started: no code or test changes yet.

## Implementation (2026-11-02)

- Test file split first under REFAC-038 (535 -> 295 lines, helpers in tests/thinking_indicator_rpc093_support).
- The two RPC-093 scenarios became "Terminal cursor is visible on the composer while session is Running/Compacting". Their tests render the agent view at 80x24 through the TUI-115 App harness (`tui115_support::agent_view_with`) and check `is_cursor_visible` is true and that `cursor_position()` is the row under the activity row.
- `AgentView::is_cursor_visible_for` now returns `Running | Compacting || transition.is_cursor_painted()`. HITL Options and pause prompts still hide the cursor before this predicate runs (hitl_keys.rs), and the Hiding/Showing sweep (status Idle) still hides it.
