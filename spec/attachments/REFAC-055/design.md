# REFAC-055 design

BUG-399 must edit rust/sessions/tests/tui099_session_list_ordering.rs (336 lines). Its non-test items
(`DATA_DIR_GUARD`, `make_temp_data_dir`, `set_temp_data_dir`, `make_session`, `sort_sessions`) move unchanged, made
`pub`, into `tests/tui099_session_list_ordering_support/mod.rs`; the test file declares the module and imports them.
The three scenarios are unchanged, so resume-session-ordering.feature keeps one test file (293 lines). Test mappings
in resume-session-ordering.feature.coverage are remapped to the new test ranges.
