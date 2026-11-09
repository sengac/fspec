# BUG-409 AST research

- Test: rust/sessions/tests/tui099_session_list_ordering.rs (`sessions_ordered_by_most_recently_updated_first`).
- Creation path: rust/sessions/src/handle_creation.rs `create_session_checked` -> `create_session_in_project` ->
  `SessionManager::create_session` (rust/sessions/src/session_manager/manager_impl_1.rs:194) ->
  `create_session_with_id` (manager_impl_3.rs:36; model parse, credential resolution, manifest persistence).
- Seeding precedent: rust/sessions/tests/rpc422_session_persistence.rs `MODELS_FIXTURE` + `make_temp_data_dir`.
