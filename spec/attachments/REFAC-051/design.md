# REFAC-051 design

`rust/fspec-tui/tests/view_agent_unit_rpc029.rs` (579 lines) is the one test file of
`spec/features/rpc029-agent-structure-alignment.feature` (13 scenarios). BUG-391 has to change the `SessionHeader`
literals in it, so it is split first, with no behaviour change:

- New feature `spec/features/rpc029-agent-header-alignment.feature` (@done @RPC-029 @REFAC-051, same user story)
  takes the six header scenarios unchanged: "Header inserts work-unit prefix ...", "Header omits work-unit prefix ...",
  "Header and footer rows paint dark grey #333333 ...", "Header [DEBUG] badge ...", "Header [ISOLATED] badge ...",
  "Header prefix + work unit + model run ...". Their coverage entries (impl mappings kept) move to its `.coverage`.
- Their tests move to `tests/rpc029_agent_header_alignment.rs` (with `DARK_GREY` and `render_header_row`, used only
  there).
- Helpers used by both files (`render_buffer`, `rows_of`, `find_row`, `find_substr_xy`, `fresh_view`,
  `fresh_view_with_session`) move to `tests/rpc029_support/mod.rs`; both test files use every one of them, so no
  dead-code suppression is needed.
- Both test files end up under 300 lines; test ranges in both coverage files are recomputed.

Verification: `cargo test -p codelet-fspec-tui --test view_agent_unit_rpc029 --test rpc029_agent_header_alignment`
(13 tests in total) and clippy on both targets.
