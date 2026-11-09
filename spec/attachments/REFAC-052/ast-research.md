# REFAC-052 AST research

- `fspec astgrep --lang rust --pattern 'fresh_session($A)' --path rust/sessions/tests`: 5 calls, all in
  `rpc410_hitl_wire_protocol_parity.rs` (the five session scenarios); no other test uses the rpc410 helpers.
- `fspec astgrep --lang rust --pattern 'codelet_common::set_data_directory($A)'` on the parity file: 5 per-test calls
  (lines 52, 92, 127, 168, 234 after the split), the BUG-397 cause; untouched here.
- The send-path and round-trip tests use only `codelet_rpc_types` wire types, `serde_json` and
  `codelet_test_helpers::source_composition::read_tree_or_file`, so they move without the helpers.
- Coverage: only `spec/features/hitl-wire-protocol-parity.feature.coverage` maps this test file.
