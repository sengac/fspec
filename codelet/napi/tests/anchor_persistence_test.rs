#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: Anchor Point Persistence Through Session Resume
//!
//! These tests verify that anchor points detected during compaction are:
//! 1. Stored in the session manifest on disk
//! 2. Restored when the session is resumed via /resume
//! 3. Available via /anchors command after resume
//!
//! FIX IMPLEMENTED:
//! - Added PersistedAnchorPoint struct to persistence types
//! - Added anchor_points field to SessionManifest
//! - Added persist_anchor_point() function to persist anchors after compaction
//! - Added session_restore_anchor_points() NAPI function to load anchors on resume

use codelet_napi::persistence::{
    create_session, load_session, set_data_directory, set_compaction_state,
    add_anchor_point, get_anchor_points, PersistedAnchorPoint,
};
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::tempdir;

// Global mutex for sequential test execution (shared global state)
lazy_static::lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

/// Setup isolated test environment
fn setup_test_env() -> (std::sync::MutexGuard<'static, ()>, tempfile::TempDir) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let temp_dir = tempdir().expect("Failed to create temp directory");
    set_data_directory(temp_dir.path().to_path_buf()).expect("Failed to set data directory");
    (guard, temp_dir)
}

// =============================================================================
// Scenario: Anchor points should be persisted in SessionManifest
// =============================================================================

#[test]
fn test_session_manifest_should_have_anchor_points_field() {
    // @step Given I examine the SessionManifest struct
    // @step Then it should have an anchor_points field
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    // Create a session manifest
    let session = create_session("Test Session", &project).expect("create session");
    
    // Verify anchor_points field exists and is empty for fresh session
    assert!(session.anchor_points.is_empty(), "Fresh session should have no anchors");
    
    // Verify compaction field exists
    assert!(session.compaction.is_none(), "Fresh session should have no compaction state");
}

// =============================================================================
// Scenario: Anchor points should survive session save/load cycle
// =============================================================================

#[test]
fn test_anchor_points_should_survive_session_reload() {
    // @step Given a session with anchor points stored
    // @step When I save the session to disk
    // @step And I reload the session from disk
    // @step Then the anchor points should still be present
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    // Create a session (this auto-saves)
    let mut session = create_session("Anchor Test", &project).expect("create session");
    let session_id = session.id;
    
    // Add an anchor point
    let anchor = PersistedAnchorPoint {
        turn_index: 5,
        anchor_type: "ErrorResolution".to_string(),
        weight: 0.9,
        confidence: 0.95,
        description: "Build error fixed and tests pass".to_string(),
        timestamp_ms: 1738713600000,
    };
    add_anchor_point(&mut session, anchor).expect("add anchor");
    
    // Reload the session
    let reloaded = load_session(session_id).expect("Should load session");
    
    // Verify anchor point survived
    assert_eq!(reloaded.anchor_points.len(), 1, "Should have 1 anchor point after reload");
    assert_eq!(reloaded.anchor_points[0].turn_index, 5);
    assert_eq!(reloaded.anchor_points[0].anchor_type, "ErrorResolution");
    assert_eq!(reloaded.anchor_points[0].weight, 0.9);
}

// =============================================================================
// Scenario: Compaction should persist anchor point to manifest
// =============================================================================

#[test]
fn test_compaction_should_persist_anchor_to_manifest() {
    // @step Given a session that has undergone compaction
    // @step And compaction detected an anchor point at turn 5
    // @step When I check the session manifest
    // @step Then the anchor point should be stored in the manifest
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    // Create a session
    let mut session = create_session("Compacted Session", &project).expect("create session");
    let session_id = session.id;
    
    // Set compaction state (simulating compaction)
    set_compaction_state(
        &mut session,
        "Summary of compacted turns".to_string(),
        10,
    ).expect("set compaction state");
    
    // Add anchor point (simulating what happens during compaction)
    let anchor = PersistedAnchorPoint {
        turn_index: 5,
        anchor_type: "TaskCompletion".to_string(),
        weight: 0.8,
        confidence: 0.92,
        description: "Task completed successfully".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    add_anchor_point(&mut session, anchor).expect("add anchor");
    
    // Reload to verify persistence
    let reloaded = load_session(session_id).expect("Should load");
    
    // Verify compaction state survives
    assert!(reloaded.compaction.is_some(), "Compaction state should survive reload");
    let compaction = reloaded.compaction.unwrap();
    assert_eq!(compaction.compacted_before_index, 10);
    
    // Verify anchor point survives
    assert_eq!(reloaded.anchor_points.len(), 1, "Should have 1 anchor point");
    assert_eq!(reloaded.anchor_points[0].turn_index, 5);
    assert_eq!(reloaded.anchor_points[0].anchor_type, "TaskCompletion");
}

// =============================================================================
// Scenario: Session restore should populate BackgroundSession anchor_points
// =============================================================================

#[test]
fn test_session_restore_should_populate_background_session_anchors() {
    // NOTE: This test verifies the persistence layer works.
    // The NAPI function session_restore_anchor_points() handles the
    // BackgroundSession population - tested via TypeScript integration tests.
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    // Create a session with anchors
    let mut session = create_session("Restore Test", &project).expect("create session");
    let session_id = session.id;
    
    let anchor = PersistedAnchorPoint {
        turn_index: 10,
        anchor_type: "FeatureMilestone".to_string(),
        weight: 0.85,
        confidence: 0.9,
        description: "Feature milestone reached".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    add_anchor_point(&mut session, anchor).expect("add anchor");
    
    // Verify anchors can be retrieved via the persistence API
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].turn_index, 10);
    
    // Reload and verify
    let reloaded = load_session(session_id).expect("load");
    let reloaded_anchors = get_anchor_points(&reloaded);
    assert_eq!(reloaded_anchors.len(), 1);
    assert_eq!(reloaded_anchors[0].turn_index, 10);
}

// =============================================================================
// Scenario: Multiple anchor points should be preserved
// =============================================================================

#[test]
fn test_multiple_anchor_points_should_be_preserved() {
    // @step Given a long session with multiple compaction cycles
    // @step And each compaction detected different anchor types
    // @step When I resume the session
    // @step Then all anchor points should be available
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    let mut session = create_session("Multi-Anchor Session", &project).expect("create session");
    let session_id = session.id;
    
    // Add multiple anchors of different types
    let anchors = vec![
        PersistedAnchorPoint {
            turn_index: 5,
            anchor_type: "ErrorResolution".to_string(),
            weight: 0.9,
            confidence: 0.95,
            description: "First error fixed".to_string(),
            timestamp_ms: 1738713600000,
        },
        PersistedAnchorPoint {
            turn_index: 20,
            anchor_type: "TaskCompletion".to_string(),
            weight: 0.8,
            confidence: 0.92,
            description: "Task completed".to_string(),
            timestamp_ms: 1738713700000,
        },
        PersistedAnchorPoint {
            turn_index: 35,
            anchor_type: "UserCheckpoint".to_string(),
            weight: 0.85,
            confidence: 0.88,
            description: "User checkpoint".to_string(),
            timestamp_ms: 1738713800000,
        },
    ];
    
    for anchor in anchors {
        add_anchor_point(&mut session, anchor).expect("add anchor");
    }
    
    let reloaded = load_session(session_id).expect("Should load");
    
    // Verify all anchors survive
    assert_eq!(reloaded.anchor_points.len(), 3);
    assert_eq!(reloaded.anchor_points[0].turn_index, 5);
    assert_eq!(reloaded.anchor_points[1].turn_index, 20);
    assert_eq!(reloaded.anchor_points[2].turn_index, 35);
}

// =============================================================================
// Scenario: Anchor timestamp should be preserved accurately
// =============================================================================

#[test]
fn test_anchor_timestamp_should_be_preserved() {
    // @step Given an anchor point with a specific timestamp
    // @step When I save and reload the session
    // @step Then the timestamp should match exactly
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    let mut session = create_session("Timestamp Test", &project).expect("create session");
    let session_id = session.id;
    
    let timestamp_ms: i64 = 1738713600000; // 2025-02-05 00:00:00 UTC
    
    let anchor = PersistedAnchorPoint {
        turn_index: 10,
        anchor_type: "TaskCompletion".to_string(),
        weight: 0.8,
        confidence: 0.92,
        description: "Task completed".to_string(),
        timestamp_ms,
    };
    
    add_anchor_point(&mut session, anchor).expect("add anchor");
    
    let reloaded = load_session(session_id).expect("load");
    assert_eq!(reloaded.anchor_points[0].timestamp_ms, timestamp_ms);
}

// =============================================================================
// Scenario: Empty anchors list should not cause errors
// =============================================================================

#[test]
fn test_empty_anchors_should_not_cause_errors() {
    // @step Given a session with no anchor points (never compacted)
    // @step When I save and reload the session
    // @step Then the anchors field should be an empty list (not null)
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    let session = create_session("No Anchors", &project).expect("create session");
    let session_id = session.id;
    
    let reloaded = load_session(session_id).expect("Should load");
    
    // Session should load successfully even with no anchors
    assert_eq!(reloaded.name, "No Anchors");
    assert!(reloaded.anchor_points.is_empty(), "Should have empty anchor list");
}

// =============================================================================
// Scenario: Backward compatibility - old sessions without anchors field
// =============================================================================

#[test]
fn test_backward_compatibility_old_sessions_without_anchors() {
    // @step Given an old session manifest without anchor_points field (pre-feature)
    // @step When I load the session with new code
    // @step Then it should default to empty anchors (serde default)
    // @step And not crash or return an error
    
    // The #[serde(default)] attribute on anchor_points handles this automatically
    // This test verifies the attribute is working correctly
    
    use codelet_napi::persistence::SessionManifest;
    
    // Create a JSON string mimicking old format without anchor_points
    let old_manifest_json = r#"{
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "name": "Old Session",
        "project": "/test/project",
        "provider": "claude",
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z",
        "messages": [],
        "forked_from": null,
        "merged_from": [],
        "compaction": null,
        "token_usage": {
            "current_context_tokens": 0,
            "cumulative_billed_input": 0,
            "cumulative_billed_output": 0,
            "cache_read_tokens": 0,
            "cache_creation_tokens": 0
        }
    }"#;
    
    // This should deserialize successfully despite missing anchor_points
    let loaded: SessionManifest = serde_json::from_str(old_manifest_json)
        .expect("Should deserialize old format");
    
    assert!(loaded.anchor_points.is_empty(), "Should default to empty anchors");
    assert_eq!(loaded.name, "Old Session");
}

// =============================================================================
// Integration test: Full compaction -> persist -> resume -> verify cycle
// =============================================================================

#[test]
fn test_full_compaction_persist_resume_verify_cycle() {
    // This tests the full persistence round-trip
    
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    // 1. Create session
    let mut session = create_session("Full Cycle Test", &project).expect("create session");
    let session_id = session.id;
    
    // 2. Simulate compaction with anchor detection
    set_compaction_state(
        &mut session,
        "Summary of compacted conversation".to_string(),
        20,
    ).expect("set compaction state");
    
    let anchor = PersistedAnchorPoint {
        turn_index: 15,
        anchor_type: "ErrorResolution".to_string(),
        weight: 0.9,
        confidence: 0.95,
        description: "Build error resolved".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    add_anchor_point(&mut session, anchor).expect("add anchor");
    
    // 3. Simulate "detach" (just drop the session reference)
    drop(session);
    
    // 4. Simulate "resume" (reload from disk)
    let restored = load_session(session_id).expect("load session");
    
    // 5. Verify everything is restored
    assert_eq!(restored.name, "Full Cycle Test");
    
    // Verify compaction state
    assert!(restored.compaction.is_some());
    let compaction = restored.compaction.unwrap();
    assert_eq!(compaction.compacted_before_index, 20);
    assert!(compaction.summary.contains("Summary"));
    
    // Verify anchor points  
    assert_eq!(restored.anchor_points.len(), 1);
    assert_eq!(restored.anchor_points[0].turn_index, 15);
    assert_eq!(restored.anchor_points[0].anchor_type, "ErrorResolution");
    assert_eq!(restored.anchor_points[0].weight, 0.9);
    assert_eq!(restored.anchor_points[0].confidence, 0.95);
}
