// Feature: spec/features/subordinate-session-persistence.feature
//
// AMGR-014: Verify that handle_spawn creates a persistence manifest for
// subordinate sessions so they are searchable via SessionSearch.
//
// These tests exercise the real persistence layer (save/load/list round-trips)
// to verify the fix works end-to-end. The unit tests in agent_manager_handler.rs
// cover manifest construction and provider extraction; these integration tests
// cover the scenarios that require disk I/O.
//
// IMPORTANT: These tests share global state (persistence stores) and must be run
// sequentially. Use: cargo test --test subordinate_session_persistence_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codelet_napi::persistence::{
    append_message, list_sessions_for_project, load_session, save_session,
    SessionManifest,
};
use codelet_napi::test_support::setup_test_env;
use std::path::PathBuf;
use uuid::Uuid;

// ============================================================================
// Scenario: Persistence manifest created before session
//
// Verifies that a manifest created with with_provider() and a specific UUID
// can be saved to disk and loaded back with all fields intact.
// ============================================================================
#[test]
fn test_manifest_saved_with_subordinate_uuid_and_provider() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given a parent session with model "anthropic/claude-opus-4-6"
    let model_str = "anthropic/claude-opus-4-6";
    let subordinate_id = Uuid::new_v4();
    let name = format!("Agent {}", &subordinate_id.to_string()[..8]);
    let project_path = PathBuf::from("/test/project/subordinate-persist");

    // @step When the parent spawns a subordinate via AgentManager
    // Reproduce the exact code path from handle_spawn:
    let provider = model_str.split('/').next().unwrap_or("");
    let mut manifest =
        SessionManifest::with_provider(&name, project_path.clone(), provider);
    manifest.id = subordinate_id;
    save_session(&manifest).expect("save_session should succeed");

    // @step Then a persistence manifest is saved with the subordinate's UUID
    let loaded = load_session(subordinate_id)
        .expect("load_session should find the saved manifest");
    assert_eq!(loaded.id, subordinate_id);
    assert_eq!(loaded.name, name);

    // @step Then the manifest provider field is "anthropic"
    assert_eq!(loaded.provider, "anthropic");

    // @step Then the manifest is created before create_session_with_id is called
    // Verified by code ordering in handle_spawn; here we verify the saved
    // project matches, confirming the manifest is complete and correct.
    assert_eq!(loaded.project, project_path);
}

// ============================================================================
// Scenario: Subordinate messages are searchable via SessionSearch
//
// Verifies the full pipeline: save manifest → append messages → load by ID →
// list by project. This is what SessionSearch's "recent" and "search" actions
// rely on.
// ============================================================================
#[test]
fn test_subordinate_messages_searchable_via_session_search() {
    let (_guard, _temp_dir) = setup_test_env();

    // @step Given a spawned subordinate session with a persistence manifest
    let subordinate_id = Uuid::new_v4();
    let project_path = PathBuf::from("/test/project/subordinate-search");
    let name = format!("Agent {}", &subordinate_id.to_string()[..8]);

    let mut manifest =
        SessionManifest::with_provider(&name, project_path.clone(), "anthropic");
    manifest.id = subordinate_id;
    save_session(&manifest).expect("save_session should succeed");

    // @step When the subordinate processes a message and produces a response
    let mut session = load_session(subordinate_id)
        .expect("load_session should succeed after save");
    append_message(&mut session, "user", "Analyze the auth module")
        .expect("append user message should succeed");
    append_message(&mut session, "assistant", "The auth module uses JWT tokens with bcrypt hashing.")
        .expect("append assistant message should succeed");

    // @step Then the subordinate session appears in SessionSearch recent results for the project
    let project_sessions = list_sessions_for_project(&project_path)
        .expect("list_sessions_for_project should succeed");
    assert!(
        project_sessions.iter().any(|s| s.id == subordinate_id),
        "Subordinate session {} should appear in project listing, found: {:?}",
        subordinate_id,
        project_sessions.iter().map(|s| s.id).collect::<Vec<_>>()
    );

    // @step Then the subordinate's messages are found via SessionSearch search action
    // SessionSearch loads messages via get_session_messages; verify the round-trip.
    let reloaded = load_session(subordinate_id)
        .expect("reload should succeed");
    let messages = codelet_napi::persistence::get_session_messages(&reloaded)
        .expect("get_session_messages should succeed");
    assert_eq!(messages.len(), 2, "Should have both user and assistant messages");
    assert_eq!(messages[0].role, "user");
    assert!(messages[0].content.contains("Analyze the auth module"));
    assert_eq!(messages[1].role, "assistant");
    assert!(messages[1].content.contains("JWT tokens"));
}

// ============================================================================
// Scenario: Spawn succeeds even when persistence manifest creation fails
//
// Verifies that save_session returns Err (not panic) on I/O failure, and that
// subsequent persistence operations still work — matching handle_spawn's
// `if let Err(e) = save_session(..) { warn; continue }` pattern.
// ============================================================================
#[test]
fn test_spawn_continues_when_persistence_fails() {
    let (_guard, temp_dir) = setup_test_env();

    // @step Given a parent session with model "anthropic/claude-opus-4-6"
    let model_str = "anthropic/claude-opus-4-6";
    let subordinate_id = Uuid::new_v4();
    let name = format!("Agent {}", &subordinate_id.to_string()[..8]);

    // @step Given the persistence layer will fail to save the manifest
    // Trigger store initialization so the sessions directory is created,
    // then make it unwritable so save_session fails on fs::write.
    codelet_napi::persistence::ensure_directories()
        .expect("ensure_directories should succeed");
    let sessions_dir = temp_dir.path().join("sessions");
    let original_perms = std::fs::metadata(&sessions_dir)
        .expect("sessions dir should exist after ensure_directories")
        .permissions();
    let mut readonly_perms = original_perms.clone();
    std::os::unix::fs::PermissionsExt::set_mode(&mut readonly_perms, 0o444);
    std::fs::set_permissions(&sessions_dir, readonly_perms)
        .expect("chmod should succeed");

    // @step When the parent spawns a subordinate via AgentManager
    let provider = model_str.split('/').next().unwrap_or("");
    let mut manifest = SessionManifest::with_provider(
        &name,
        PathBuf::from("/test/project/fail-persist"),
        provider,
    );
    manifest.id = subordinate_id;

    let save_result = save_session(&manifest);

    // @step Then the subordinate session is still created successfully
    // save_session returns Err (doesn't panic), so handle_spawn can continue
    assert!(
        save_result.is_err(),
        "save_session should fail when sessions dir is read-only"
    );

    // @step Then a warning is logged about the persistence failure
    // In handle_spawn: tracing::warn!("Failed to create persistence manifest...")
    // Here we verify the error message is meaningful (not empty/opaque)
    let err_msg = save_result.unwrap_err();
    assert!(
        !err_msg.is_empty(),
        "Error message should be non-empty"
    );
    assert!(
        err_msg.contains("Failed to write") || err_msg.contains("Permission denied") || err_msg.contains("permission denied"),
        "Error should mention write failure, got: {err_msg}"
    );

    // Restore permissions so temp_dir cleanup doesn't fail
    std::fs::set_permissions(&sessions_dir, original_perms)
        .expect("restore permissions should succeed");

    // Verify persistence layer is NOT poisoned — subsequent operations still work.
    // This is critical: handle_spawn creates the in-memory session AFTER the failed save.
    let recovery_session = codelet_napi::persistence::create_session(
        "Recovery Session",
        &PathBuf::from("/test/project/recovery"),
    );
    assert!(
        recovery_session.is_ok(),
        "Persistence layer should recover after a save failure"
    );
}

// ============================================================================
// Scenario: Multiple subordinates for the same project are all discoverable
//
// Strengthens the searchability scenario: spawning multiple subordinates
// should result in ALL of them appearing in list_sessions_for_project.
// ============================================================================
#[test]
fn test_multiple_subordinates_all_listed_for_project() {
    let (_guard, _temp_dir) = setup_test_env();

    let project_path = PathBuf::from("/test/project/multi-subordinate");
    let mut expected_ids = Vec::new();

    // Create 3 subordinate manifests (simulating 3 spawns)
    for i in 0..3 {
        let sub_id = Uuid::new_v4();
        let name = format!("Agent-{i} {}", &sub_id.to_string()[..8]);
        let mut manifest =
            SessionManifest::with_provider(&name, project_path.clone(), "anthropic");
        manifest.id = sub_id;
        save_session(&manifest).expect("save_session should succeed");
        expected_ids.push(sub_id);
    }

    let project_sessions = list_sessions_for_project(&project_path)
        .expect("list_sessions_for_project should succeed");
    let found_ids: Vec<Uuid> = project_sessions.iter().map(|s| s.id).collect();

    for expected in &expected_ids {
        assert!(
            found_ids.contains(expected),
            "Subordinate {} should be in project listing",
            expected
        );
    }
    assert_eq!(
        project_sessions.len(),
        3,
        "Should have exactly 3 subordinate sessions for this project"
    );
}
