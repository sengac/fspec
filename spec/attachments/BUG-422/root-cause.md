# BUG-422 root cause

tui002 session-inheritance tests fail whichever runs second: persistence stores are never reset for the new data dir

In rust/sessions/tests/tui002_default_thinking_level.rs, a_new_session_inherits_a_persisted_high_default and a_new_session_inherits_a_persisted_off_default each create a tempdir, call codelet_common::set_data_directory and create a session, but never call codelet_core::persistence::reset_stores_for_tests. The SessionStore keeps the sessions_dir of the first test's tempdir, which is deleted when that test ends, so the second test's create_session_checked fails with 'Failed to persist session manifest: manifest I/O failed: No such file or directory'. Reproduced at 62fe1f807 (HEAD) three runs out of three: whichever of the two runs second fails.

Found while running the BUG-399 regression targets (2026-11-03); filed to the backlog untouched per the user's instruction.
