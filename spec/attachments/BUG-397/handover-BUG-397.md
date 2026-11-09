# BUG-397 handover (2026-11-03)

Status: backlog, not started (root cause confirmed, see root-cause.md).

Next steps, in order:
1. REFAC card first: `rust/sessions/tests/rpc410_hitl_wire_protocol_parity.rs` is 498 lines (the one test file of
   `spec/features/hitl-wire-protocol-parity.feature`, @RPC-410). Move its helpers (`fresh_session`,
   `option_question`, the round-trip helper near line 426, etc.) into `tests/rpc410_hitl_wire_protocol_parity_support/
   mod.rs`; if the tests alone still exceed 300 lines, split the feature's scenarios into a second feature with its
   own test file (REFAC-051 is the pattern).
2. BUG-397 (tag the feature @BUG-397 after @RPC-410): give the binary ONE data directory that outlives every test
   (e.g. a `std::sync::OnceLock<tempfile::TempDir>` in the support module, set once and its `Result` handled instead
   of `let _ =`), and create sessions with `create_session_checked` so a failure names its cause.
   Red evidence already seen: with `--test-threads=1`, 4 of 7 tests fail at `get_session(..).expect("session must
   exist")` (lines 149, 189, 265, 331); alone, `question_without_options_surfaces_empty_options_array` passes. A
   scenario such as "Every HITL parity test creates its session in a data directory that outlives the other tests",
   verified by running the binary with `--test-threads=1`, gives the red/green.
3. Remap coverage for the moved tests (test ranges directly in the JSON, BUG-375).

**Done (2026-11-03):** REFAC-052 split the file first; the fix is `use_shared_data_dir` + `create_session_checked` in
`rust/sessions/tests/rpc410_hitl_wire_protocol_parity_support/mod.rs`. Red: the new sequence test and 5 of 6 tests
failed with `--test-threads=1`; green: 6/6 in parallel and on one thread.
