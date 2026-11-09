# REFAC-061 AST research

Tests at HEAD: full_round_trip 65, resume_restores_token_state 145, resume_preserves_message_ordering 195, resume_fails_for_non_existent_session 258, multiple_sessions_persist_independently 284. Helpers 29-59.

## Result

- New features session-persistence-round-trip.feature (full round trip, token state, ordering) and session-resume-isolation.feature (unknown session, independent sessions), both tagged @RPC-002 first; scenario steps are the tests' existing @step comments.
- rpc002_session_persistence.rs 216 lines, rpc002_session_resume_isolation.rs 126 lines; DATA_DIR_GUARD, setup_data_dir and OFFLINE_DEFAULT_MODEL in rpc002_session_persistence_support/mod.rs.
- Impl mappings: persist_user_message / persist_assistant_message_internal (agent-loop persist.rs 16-18, 58-65), persist_token_state (persist_state.rs 3-18), resume_session (sessions handle_impl_1.rs 39-133; 39-51 for the load failure).
