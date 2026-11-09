# BUG-407 AST research

- `grep -n "fn scenario_shared_callbacks_are_plain_function_pointers" rust/sessions/tests/wt004_session_tool_callbacks/work_unit_stage_lookup.rs` → line 111 (after BUG-398).
- The WT-004 `.feature.coverage` has no test mapping covering that range.
- Business rule 2 of the WT-004 feature describes the plain-function-pointer requirement.
