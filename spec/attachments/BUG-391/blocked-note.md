# BUG-391 blocked note (2026-11-03)

The fix makes `AgentViewStore::session_index_for` return `Option<(usize, usize)>` and threads the `Option` through
`views/agent/pane_render.rs` -> `chrome_paint::paint_header_and_role` -> `SessionHeader.session_index` (header.rs)
-> `header_build::build_left_line`, and through `menu_snapshot::chip_inputs` -> `ChipInput.index` / `Chip.index` ->
`chips::index_prefix`.

`SessionHeader` is a public struct built with a literal in `rust/fspec-tui/tests/prov168_rejected_context.rs`
(`session_index: (1, 1)`). That file belongs to PROV-168, which another tool owns and this queue must not touch, so
the header type cannot change until PROV-168 is finished. Keeping the tuple in `SessionHeader` would only move the
`(0, 0)` sentinel one layer down, which the no-sentinel rule forbids.

Other constructors to update when unblocked: tests `cmpct067_header_provenance.rs`, `view_agent_unit_rpc029.rs`
(SessionHeader); `menu001_menu_bar.rs`, `menu008_menu_bar_reorg.rs`, `menu009_zone_c_buttons.rs`,
`bug199_agent_bar_zone_c_semantics.rs` (ChipInput); the unit tests in `session_indexing.rs` and `chips.rs`.
Scenario home: `agent-panes-report-missing-session-context.feature` (@BUG-377 first, add @BUG-391 after it); its test
file `tests/bug377_agent_panes_report_missing_session_context.rs` is 104 lines.

**Resolved (2026-11-03):** PROV-168 is done (1bea8f29). The coordinator allowed changing the one
`session_index: (1, 1)` literal in `tests/prov168_rejected_context.rs` to the new `Option` type, and nothing else in
that file. BUG-391 is unblocked and proceeds with the plan above.
