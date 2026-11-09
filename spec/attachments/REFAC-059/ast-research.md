# REFAC-059 AST research

Tests at HEAD: create_session_persists_manifest_to_disk 60, create_session_with_provider_persists_provider_field 122, destroy_session_removes_from_memory_preserves_manifest 155, persistence_delete_session_removes_manifest_from_disk 210, list_sessions_includes_persisted_sessions 242, resume_session_restores_messages 288, create_session_persists_provider_in_manifest 351, create_session_fails_gracefully_when_persistence_fails 384, resume_session_preserves_manifest 427, resume_empty_session 505, resume_session_already_in_memory 565, session_creation_survives_clean_offline_environment 622. Helpers 19-58.

## Result

- rpc422_session_persistence.rs (224 lines): the four creation scenarios of session-persistence-integration.feature; create_session_persists_provider_in_manifest now sits next to the other provider test and the provider scenario's test range covers both (88-149).
- rpc422_session_lifecycle_persistence.rs (214 lines): new session-persistence-lifecycle.feature (destroy, delete, list, resume).
- sess002_resume_session_manifest.rs (213 lines): resume-session-overwrites-manifest-destroying-message-references.feature.
- rpc422_session_persistence_support/mod.rs: DATA_DIR_GUARD, MODELS_FIXTURE, make_temp_data_dir, set_temp_data_dir.
- Every impl mapping was stale (pointed at logging, other functions or closing braces); all eleven scenarios were relinked to create_session_with_id (manager_impl_3 143-186), destroy_session (manager_impl_6 17-135), persistence delete_session (core sessions.rs 25-27, manifest_branches.rs 127-129), list_sessions (manager_impl_2 17-86), resume_session (handle_impl_1 39-133) and ModelCache::get (providers cache.rs 237-260).
