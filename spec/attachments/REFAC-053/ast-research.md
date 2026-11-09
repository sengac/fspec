# REFAC-053 AST research

- Helper usage per fragment (`grep -c` of each helper name): `isolated_session_under_the.rs` uses `ENV_GUARD`,
  `fresh_git_repo`, `manager_with_seeded_cache`, `register_tool_callbacks`, `create_isolated`, `create_plain`,
  `get_isolation_context`, `validate_and_resolve_path_with_isolation`, `ToolError`; `work_unit_stage_lookup.rs` uses
  `ENV_GUARD`, `fresh_git_repo`, `manager_with_seeded_cache`, `register_tool_callbacks`, `create_plain`,
  `register_chunk_sender`, `next_matching_chunk`, `rust_root`, `strip_line_comments`, `build_service_body`,
  `get_isolation_context`, `IsolationContext`. (`create_isolated` matched there only as `.create_isolated_session_with_id`.)
- `MODELS_FIXTURE` is an `include_str!` relative to the including file, so the support module uses
  `../fixtures/prov101_models.json`.
- Only the WT-004 `.feature.coverage` maps these test files.
