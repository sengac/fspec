# BUG-422 AST research

- rust/sessions/tests/tui002_default_thinking_level.rs: seed_data_dir (40-48), a_new_session_inherits_a_persisted_high_default (~122-141), a_new_session_inherits_a_persisted_off_default (~144-163).
- Compare rpc422_session_persistence_support::set_temp_data_dir, which calls codelet_core::persistence::reset_stores_for_tests after set_data_directory.
