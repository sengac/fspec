//! Feature: Anchor Point Persistence Layer Tests
//!
//! These tests verify the persistence layer correctly stores and retrieves
//! anchor points at the lowest level - direct database operations.
//!
//! Test Levels:
//! 1. SessionManifest struct has anchor_points field
//! 2. add_anchor_point() stores to manifest
//! 3. get_anchor_points() retrieves from manifest
//! 4. Anchors survive save/load cycle
//! 5. Multiple anchors with ordering
//! 6. Backward compatibility with old sessions
//!
//! Run with: cargo test --test anchor_persistence_layer_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod compaction_anchor_fixtures;

use codelet_napi::persistence::{
    create_session, load_session, add_anchor_point, get_anchor_points,
    PersistedAnchorPoint, SessionManifest,
};
use compaction_anchor_fixtures::*;
use std::path::PathBuf;

// =============================================================================
// UNIT: SessionManifest Structure
// =============================================================================

#[test]
fn test_session_manifest_has_anchor_points_field() {
    // @step Given I create a new session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    let session = create_session("Test Session", &project)
        .expect("create_session should succeed");
    
    // @step Then the session should have an anchor_points field
    // @step And it should be empty by default
    assert!(
        session.anchor_points.is_empty(),
        "New session should have empty anchor_points"
    );
}

#[test]
fn test_session_manifest_anchor_points_is_vec() {
    // @step Given I create a new session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    
    let session = create_session("Test Session", &project)
        .expect("create_session should succeed");
    
    // @step Then anchor_points should be a Vec (can push, get length)
    let anchors: &Vec<PersistedAnchorPoint> = &session.anchor_points;
    assert_eq!(anchors.len(), 0);
}

// =============================================================================
// UNIT: add_anchor_point Function
// =============================================================================

#[test]
fn test_add_anchor_point_stores_in_manifest() {
    // @step Given an empty session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Add Anchor Test", &project)
        .expect("create_session should succeed");
    
    // @step When I add an anchor point
    let anchor = create_error_resolution_anchor(5);
    add_anchor_point(&mut session, anchor).expect("add_anchor_point should succeed");
    
    // @step Then the session should have 1 anchor point
    assert_eq!(session.anchor_points.len(), 1);
    
    // @step And it should match the added anchor
    assert_eq!(session.anchor_points[0].turn_index, 5);
    assert_eq!(session.anchor_points[0].anchor_type, "ErrorResolution");
}

#[test]
fn test_add_anchor_point_preserves_all_fields() {
    // @step Given a session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Field Preservation Test", &project)
        .expect("create_session");
    
    // @step When I add an anchor with specific values
    let timestamp_ms = 1738713600000i64; // Fixed timestamp
    let anchor = PersistedAnchorPoint {
        turn_index: 42,
        anchor_type: "FeatureMilestone".to_string(),
        weight: 0.75,
        confidence: 0.88,
        description: "Test description with special chars: <>&\"'".to_string(),
        timestamp_ms,
    };
    add_anchor_point(&mut session, anchor).expect("add_anchor_point");
    
    // @step Then all fields should be preserved
    let stored = &session.anchor_points[0];
    assert_eq!(stored.turn_index, 42);
    assert_eq!(stored.anchor_type, "FeatureMilestone");
    assert!((stored.weight - 0.75).abs() < 0.001);
    assert!((stored.confidence - 0.88).abs() < 0.001);
    assert_eq!(stored.description, "Test description with special chars: <>&\"'");
    assert_eq!(stored.timestamp_ms, timestamp_ms);
}

#[test]
fn test_add_anchor_point_updates_session_timestamp() {
    // @step Given a session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Timestamp Update Test", &project)
        .expect("create_session");
    
    let original_updated_at = session.updated_at;
    
    // Small delay to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // @step When I add an anchor point
    let anchor = create_task_completion_anchor(10);
    add_anchor_point(&mut session, anchor).expect("add_anchor_point");
    
    // @step Then the session's updated_at should be newer
    assert!(
        session.updated_at > original_updated_at,
        "updated_at should be updated when adding anchor"
    );
}

// =============================================================================
// UNIT: get_anchor_points Function  
// =============================================================================

#[test]
fn test_get_anchor_points_returns_empty_for_new_session() {
    // @step Given a new session with no anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let session = create_session("Empty Test", &project).expect("create_session");
    
    // @step When I get anchor points
    let anchors = get_anchor_points(&session);
    
    // @step Then the result should be an empty vec
    assert!(anchors.is_empty());
}

#[test]
fn test_get_anchor_points_returns_added_anchors() {
    // @step Given a session with anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Get Anchors Test", &project).expect("create_session");
    
    add_anchor_point(&mut session, create_error_resolution_anchor(5)).expect("add");
    add_anchor_point(&mut session, create_task_completion_anchor(15)).expect("add");
    
    // @step When I get anchor points
    let anchors = get_anchor_points(&session);
    
    // @step Then I should get 2 anchors
    assert_eq!(anchors.len(), 2);
    
    // @step And they should be in order added
    assert_eq!(anchors[0].turn_index, 5);
    assert_eq!(anchors[1].turn_index, 15);
}

#[test]
fn test_get_anchor_points_returns_clone() {
    // @step Given a session with an anchor
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Clone Test", &project).expect("create_session");
    
    add_anchor_point(&mut session, create_task_completion_anchor(10)).expect("add");
    
    // @step When I get anchor points twice
    let anchors1 = get_anchor_points(&session);
    let anchors2 = get_anchor_points(&session);
    
    // @step Then both should have the same content
    assert_eq!(anchors1.len(), anchors2.len());
    assert_eq!(anchors1[0].turn_index, anchors2[0].turn_index);
}

// =============================================================================
// INTEGRATION: Save/Load Cycle
// =============================================================================

#[test]
fn test_single_anchor_survives_save_load_cycle() {
    // @step Given a session with one anchor
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Save Load Test", &project).expect("create_session");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_error_resolution_anchor(7)).expect("add");
    
    // @step When I drop and reload the session
    drop(session);
    let reloaded = load_session(session_id).expect("load_session");
    
    // @step Then the anchor should still be present
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].turn_index, 7);
    assert_eq!(anchors[0].anchor_type, "ErrorResolution");
}

#[test]
fn test_multiple_anchors_survive_save_load_cycle() {
    // @step Given a session with multiple anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Multi Save Load", &project).expect("create_session");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_error_resolution_anchor(5)).expect("add");
    add_anchor_point(&mut session, create_task_completion_anchor(20)).expect("add");
    add_anchor_point(&mut session, create_user_checkpoint_anchor(35)).expect("add");
    add_anchor_point(&mut session, create_feature_milestone_anchor(50)).expect("add");
    
    // @step When I drop and reload
    drop(session);
    let reloaded = load_session(session_id).expect("load_session");
    
    // @step Then all 4 anchors should be present
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors.len(), 4);
    
    // @step And they should maintain order
    assert_eq!(anchors[0].turn_index, 5);
    assert_eq!(anchors[1].turn_index, 20);
    assert_eq!(anchors[2].turn_index, 35);
    assert_eq!(anchors[3].turn_index, 50);
    
    // @step And types should be preserved
    assert_eq!(anchors[0].anchor_type, "ErrorResolution");
    assert_eq!(anchors[1].anchor_type, "TaskCompletion");
    assert_eq!(anchors[2].anchor_type, "UserCheckpoint");
    assert_eq!(anchors[3].anchor_type, "FeatureMilestone");
}

#[test]
fn test_anchor_numeric_precision_survives_save_load() {
    // @step Given an anchor with precise float values
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Precision Test", &project).expect("create_session");
    let session_id = session.id;
    
    let anchor = PersistedAnchorPoint {
        turn_index: 42,
        anchor_type: "TaskCompletion".to_string(),
        weight: 0.8765432,
        confidence: 0.9123456,
        description: "Precision test".to_string(),
        timestamp_ms: 1738713654321,
    };
    add_anchor_point(&mut session, anchor).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then numeric precision should be preserved
    let anchors = get_anchor_points(&reloaded);
    assert!((anchors[0].weight - 0.8765432).abs() < 0.0000001);
    assert!((anchors[0].confidence - 0.9123456).abs() < 0.0000001);
    assert_eq!(anchors[0].timestamp_ms, 1738713654321);
}

#[test]
fn test_anchor_special_characters_survive_save_load() {
    // @step Given an anchor with special characters in description
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Special Chars Test", &project).expect("create_session");
    let session_id = session.id;
    
    let special_desc = r#"Test with "quotes", <tags>, & ampersands, 日本語, emoji 🎉"#;
    let anchor = PersistedAnchorPoint {
        turn_index: 1,
        anchor_type: "UserCheckpoint".to_string(),
        weight: 0.7,
        confidence: 0.85,
        description: special_desc.to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    add_anchor_point(&mut session, anchor).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then special characters should be preserved
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors[0].description, special_desc);
}

// =============================================================================
// INTEGRATION: Anchors with Compaction State
// =============================================================================

#[test]
fn test_anchors_coexist_with_compaction_state() {
    // @step Given a session with both compaction state and anchors
    let (_guard, _temp_dir) = setup_test_env();
    let mut session = create_compacted_session(
        "Coexist Test",
        20,  // turns before compaction
        5,   // turns kept
        "Summary of compacted turns"
    );
    let session_id = session.id;
    
    // Add anchor (would be detected during compaction)
    add_anchor_point(&mut session, create_task_completion_anchor(15)).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then both compaction state and anchors should be present
    assert!(reloaded.compaction.is_some(), "Compaction state should exist");
    assert_eq!(get_anchor_points(&reloaded).len(), 1, "Anchor should exist");
    
    // @step And compaction state should be intact
    let compaction = reloaded.compaction.unwrap();
    assert!(compaction.summary.contains("Summary of compacted turns"));
}

#[test]
fn test_anchors_persist_across_multiple_compactions() {
    // @step Given a session that goes through multiple compaction cycles
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Multi Compact Test", &project).expect("create");
    let session_id = session.id;
    
    // First compaction adds anchor at turn 10
    add_anchor_point(&mut session, create_error_resolution_anchor(10)).expect("add");
    
    // Simulate more conversation and second compaction
    add_anchor_point(&mut session, create_task_completion_anchor(30)).expect("add");
    
    // Third compaction 
    add_anchor_point(&mut session, create_feature_milestone_anchor(50)).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then all anchors from all compactions should be present
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors.len(), 3);
    assert_eq!(anchors[0].turn_index, 10);
    assert_eq!(anchors[1].turn_index, 30);
    assert_eq!(anchors[2].turn_index, 50);
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_empty_anchors_list_serializes_correctly() {
    // @step Given a session with no anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let session = create_session("Empty Anchors", &project).expect("create");
    let session_id = session.id;
    
    // @step When I save and reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then anchor_points should be empty (not null)
    assert!(reloaded.anchor_points.is_empty());
}

#[test]
fn test_anchor_at_turn_zero() {
    // @step Given an anchor at turn index 0
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Turn Zero Test", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_task_completion_anchor(0)).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then anchor at turn 0 should be preserved
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors[0].turn_index, 0);
}

#[test]
fn test_anchor_with_very_large_turn_index() {
    // @step Given an anchor with a very large turn index
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Large Index Test", &project).expect("create");
    let session_id = session.id;
    
    let large_index = usize::MAX / 2; // Very large but not overflow
    let anchor = PersistedAnchorPoint {
        turn_index: large_index,
        anchor_type: "TaskCompletion".to_string(),
        weight: 0.8,
        confidence: 0.9,
        description: "Large index test".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    add_anchor_point(&mut session, anchor).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then the large index should be preserved
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors[0].turn_index, large_index);
}

#[test]
fn test_synthetic_anchor_persists_correctly() {
    // @step Given a synthetic anchor (created when LLM detection fails)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Synthetic Test", &project).expect("create");
    let session_id = session.id;
    
    let synthetic = create_synthetic_anchor(25, "LLM timeout during detection");
    add_anchor_point(&mut session, synthetic).expect("add");
    
    // @step When I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then the synthetic anchor should be preserved with full details
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].weight, 1.0); // Synthetic anchors have weight 1.0
    assert_eq!(anchors[0].confidence, 1.0);
    assert!(anchors[0].description.contains("Synthetic anchor"));
}

// =============================================================================
// BACKWARD COMPATIBILITY
// =============================================================================

#[test]
fn test_old_session_without_anchor_points_field_loads() {
    // @step Given a JSON string representing an old session without anchor_points
    // (This tests the serde(default) attribute)
    
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
    
    // @step When I deserialize it with current code
    let loaded: SessionManifest = serde_json::from_str(old_manifest_json)
        .expect("Should deserialize old format");
    
    // @step Then it should load with empty anchor_points (default)
    assert!(loaded.anchor_points.is_empty());
    assert_eq!(loaded.name, "Old Session");
}

// =============================================================================
// CONCURRENT ACCESS (Sequential due to global state)
// =============================================================================

#[test]
fn test_add_anchors_to_same_session_sequentially() {
    // @step Given a session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Sequential Add Test", &project).expect("create");
    
    // @step When I add anchors in rapid succession
    for i in 0..10 {
        let anchor = create_task_completion_anchor(i * 5);
        add_anchor_point(&mut session, anchor).expect("add");
    }
    
    // @step Then all 10 anchors should be present in order
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 10);
    
    for (i, anchor) in anchors.iter().enumerate() {
        assert_eq!(anchor.turn_index, i * 5);
    }
}
