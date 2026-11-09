# BUG-370 AST research

## fspec astgrep --pattern 'const APPROVED_APPENDED_TAILS: $T = $V;' --lang rust --path rust/fspec-tui/tests/refac019
rust/fspec-tui/tests/refac019/appended.rs:9:1 (26 names; `frozen_prefix` pops trailing lines whose name is listed)

## fspec astgrep --pattern 'insta::assert_snapshot!($$$A)' --lang rust --path rust/fspec-tui/tests/refac019_contracts.rs
rust/fspec-tui/tests/refac019_contracts.rs:139:9 (one snapshot per surface: actions, backend, embedded, websocket, rpc_declaration, rpc_implementation)

## fspec astgrep --pattern 'contracts::frozen($A)' --lang rust --path rust/fspec-tui/tests/board024_additive_api_contract.rs
rust/fspec-tui/tests/board024_additive_api_contract.rs:53:24 (board024 parses the refac019 snapshots as its frozen prefix, so one re-baseline fixes both)

## Evidence (red run after TOOL-027)
- board024: `rpc_implementation: frozen ordered member changed: expected get_session_model, found get_session_model`, and the same for backend.
- refac019_backend.snap.new against .snap: get_session_model e8a1b4df... -> bc581d42..., plus appended get_work_unit_details .. submit_input, configure_session_mcp_servers, control_exec_stdin. The approved tails are stripped once the list reaches configure/control, because stripping stops at the first unapproved name from the end.
- `git log` on session_history.rs / session_control.rs: e799d469b (PROV-166) is the last change to get_session_model.
