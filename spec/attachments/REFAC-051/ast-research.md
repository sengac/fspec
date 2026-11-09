# REFAC-051 AST research

- Helper usage per test (script over `fn` ranges in `view_agent_unit_rpc029.rs`): header tests (lines 190-261,
  265-293, 445-579) use `render_buffer`, `rows_of`, `find_row`, `find_substr_xy`, `fresh_view`,
  `fresh_view_with_session`, `render_header_row`, `DARK_GREY`; the remaining tests use the same six shared helpers but
  not `render_header_row` / `DARK_GREY`.
- `SessionHeader { session_index: (1, 1), .. }` literals at lines 457 and 510 (DEBUG / ISOLATED badge tests) move to
  the header file.
- Coverage: `spec/features/rpc029-agent-structure-alignment.feature.coverage` maps all 13 scenarios to this file.
