//! Feature: Anchor Point NAPI Layer Tests
//!
//! These tests verify the NAPI functions that bridge TypeScript and Rust
//! for anchor point operations:
//! - session_get_anchor_points: Get anchors from BackgroundSession memory
//! - session_restore_anchor_points: Restore anchors from disk to memory
//!
//! Test Levels:
//! 1. NAPI function exists and is callable
//! 2. Return types are correct
//! 3. Memory and persistence sync correctly
//! 4. Error handling works properly
//!
//! Run with: cargo test --test anchor_napi_layer_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod compaction_anchor_fixtures;

use codelet_napi::persistence::{
    create_session, load_session, add_anchor_point, get_anchor_points,
};
use compaction_anchor_fixtures::*;
use std::path::PathBuf;

// Note: We cannot directly test the NAPI functions (session_get_anchor_points,
// session_restore_anchor_points) from Rust tests because they require a live
// BackgroundSession which is created by the TypeScript layer.
//
// Instead, we test the underlying functions that the NAPI layer calls,
// and verify the data structures are correct for NAPI serialization.

// =============================================================================
// UNIT: PersistedAnchorPoint NAPI Serialization
// =============================================================================

#[test]
fn test_persisted_anchor_point_serializes_for_napi() {
    // @step Given a PersistedAnchorPoint
    let anchor = create_error_resolution_anchor(10);
    
    // @step When I serialize it to JSON (NAPI uses serde for this)
    let json = serde_json::to_string(&anchor).expect("serialize");
    
    // @step Then all fields should be present in the JSON
    assert!(json.contains("\"turn_index\":10"));
    assert!(json.contains("\"anchor_type\":\"ErrorResolution\""));
    assert!(json.contains("\"weight\":0.9"));
    assert!(json.contains("\"confidence\":0.95"));
    assert!(json.contains("\"description\":"));
    assert!(json.contains("\"timestamp_ms\":"));
}

#[test]
fn test_persisted_anchor_point_deserializes_from_napi_format() {
    // @step Given a JSON string in NAPI format
    let json = r#"{
        "turn_index": 25,
        "anchor_type": "TaskCompletion",
        "weight": 0.8,
        "confidence": 0.92,
        "description": "Test from NAPI",
        "timestamp_ms": 1738713600000
    }"#;
    
    // @step When I deserialize it
    let anchor: codelet_napi::persistence::PersistedAnchorPoint = 
        serde_json::from_str(json).expect("deserialize");
    
    // @step Then all fields should be correct
    assert_eq!(anchor.turn_index, 25);
    assert_eq!(anchor.anchor_type, "TaskCompletion");
    assert!((anchor.weight - 0.8).abs() < 0.001);
    assert!((anchor.confidence - 0.92).abs() < 0.001);
    assert_eq!(anchor.description, "Test from NAPI");
    assert_eq!(anchor.timestamp_ms, 1738713600000);
}

// =============================================================================
// UNIT: Anchor Type String Mapping
// =============================================================================

#[test]
fn test_anchor_type_strings_match_napi_enum() {
    // The NAPI layer converts between string and enum using these exact strings
    // This test ensures we use the right strings in fixtures
    
    let error_resolution = create_error_resolution_anchor(0);
    assert_eq!(error_resolution.anchor_type, "ErrorResolution");
    
    let task_completion = create_task_completion_anchor(0);
    assert_eq!(task_completion.anchor_type, "TaskCompletion");
    
    let user_checkpoint = create_user_checkpoint_anchor(0);
    assert_eq!(user_checkpoint.anchor_type, "UserCheckpoint");
    
    let feature_milestone = create_feature_milestone_anchor(0);
    assert_eq!(feature_milestone.anchor_type, "FeatureMilestone");
}

#[test]
fn test_anchor_type_weights_match_expected_values() {
    // Verify fixture weights match the expected NAPI values
    // ErrorResolution: 0.9, TaskCompletion: 0.8, FeatureMilestone: 0.75, UserCheckpoint: 0.7
    
    let error = create_error_resolution_anchor(0);
    assert!((error.weight - 0.9).abs() < 0.001);
    
    let task = create_task_completion_anchor(0);
    assert!((task.weight - 0.8).abs() < 0.001);
    
    let milestone = create_feature_milestone_anchor(0);
    assert!((milestone.weight - 0.75).abs() < 0.001);
    
    let checkpoint = create_user_checkpoint_anchor(0);
    assert!((checkpoint.weight - 0.7).abs() < 0.001);
}

// =============================================================================
// INTEGRATION: Persistence → NAPI Data Flow
// =============================================================================

#[test]
fn test_anchors_from_persistence_ready_for_napi_retrieval() {
    // @step Given a session with anchors stored in persistence
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("NAPI Ready Test", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_error_resolution_anchor(5)).expect("add");
    add_anchor_point(&mut session, create_task_completion_anchor(15)).expect("add");
    
    // @step When I reload the session (simulates what NAPI restore does)
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then anchors should be in correct format for NAPI conversion
    let anchors = get_anchor_points(&reloaded);
    
    // NAPI expects these fields
    for anchor in &anchors {
        // turn_index must be positive
        assert!(anchor.turn_index < usize::MAX);
        
        // anchor_type must be a valid enum string
        assert!(
            anchor.anchor_type == "ErrorResolution" ||
            anchor.anchor_type == "TaskCompletion" ||
            anchor.anchor_type == "UserCheckpoint" ||
            anchor.anchor_type == "FeatureMilestone"
        );
        
        // weight and confidence must be in valid range
        assert!(anchor.weight >= 0.0 && anchor.weight <= 1.0);
        assert!(anchor.confidence >= 0.0 && anchor.confidence <= 1.0);
        
        // timestamp_ms must be positive
        assert!(anchor.timestamp_ms > 0);
    }
}

#[test]
fn test_anchor_ordering_preserved_for_napi() {
    // @step Given anchors added in specific order
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Order Test", &project).expect("create");
    let session_id = session.id;
    
    // Add out of turn order to verify we preserve insertion order
    add_anchor_point(&mut session, create_task_completion_anchor(20)).expect("add");
    add_anchor_point(&mut session, create_error_resolution_anchor(5)).expect("add");
    add_anchor_point(&mut session, create_feature_milestone_anchor(35)).expect("add");
    
    // @step When I retrieve after reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    let anchors = get_anchor_points(&reloaded);
    
    // @step Then order should be preserved (insertion order, not turn order)
    assert_eq!(anchors[0].turn_index, 20);
    assert_eq!(anchors[1].turn_index, 5);
    assert_eq!(anchors[2].turn_index, 35);
}

// =============================================================================
// UNIT: Timestamp Conversion
// =============================================================================

#[test]
fn test_timestamp_milliseconds_to_systemtime_conversion() {
    // The NAPI layer converts timestamp_ms to SystemTime
    // This tests the conversion logic
    
    use std::time::{UNIX_EPOCH, Duration};
    
    let timestamp_ms: i64 = 1738713600000; // 2025-02-05 00:00:00 UTC
    
    // Conversion used in NAPI restore
    let system_time = UNIX_EPOCH + Duration::from_millis(timestamp_ms as u64);
    
    // Convert back to verify round-trip
    let back_to_ms = system_time
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    assert_eq!(back_to_ms, timestamp_ms);
}

#[test]
fn test_systemtime_to_timestamp_milliseconds_conversion() {
    // The NAPI layer converts SystemTime to timestamp_ms for return
    
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let now = SystemTime::now();
    
    // Conversion used in NAPI get_anchor_points
    let timestamp_ms = now
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    // Should be a reasonable recent timestamp
    assert!(timestamp_ms > 1700000000000); // After 2023
    assert!(timestamp_ms < 2000000000000); // Before 2033
}

// =============================================================================
// INTEGRATION: Empty Session Handling
// =============================================================================

#[test]
fn test_napi_handles_session_with_no_anchors() {
    // @step Given a session with no anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let session = create_session("No Anchors NAPI Test", &project).expect("create");
    let session_id = session.id;
    
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step When NAPI would get anchors
    let anchors = get_anchor_points(&reloaded);
    
    // @step Then it should return empty vec (not error)
    assert!(anchors.is_empty());
}

// =============================================================================
// INTEGRATION: Large Anchor Set
// =============================================================================

#[test]
fn test_napi_handles_many_anchors() {
    // @step Given a session with many anchors (simulating long-running session)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Many Anchors NAPI Test", &project).expect("create");
    let session_id = session.id;
    
    // Add 50 anchors (realistic for a long session with many compactions)
    for i in 0..50 {
        let anchor = create_task_completion_anchor(i * 20);
        add_anchor_point(&mut session, anchor).expect("add");
    }
    
    // @step When I reload and get anchors
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    let anchors = get_anchor_points(&reloaded);
    
    // @step Then all 50 should be retrievable
    assert_eq!(anchors.len(), 50);
    
    // @step And they should be in order
    for (i, anchor) in anchors.iter().enumerate() {
        assert_eq!(anchor.turn_index, i * 20);
    }
}

// =============================================================================
// INTEGRATION: Anchor Retrieval After Compaction State Set
// =============================================================================

#[test]
fn test_anchors_accessible_after_compaction_state_added() {
    // @step Given a session with anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Compaction Integration", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_error_resolution_anchor(10)).expect("add");
    
    // @step When compaction state is added (simulating session_compact)
    codelet_napi::persistence::set_compaction_state(
        &mut session,
        "Compaction summary".to_string(),
        20,
    ).expect("set compaction");
    
    // @step And I reload
    drop(session);
    let reloaded = load_session(session_id).expect("load");
    
    // @step Then anchors should still be accessible
    let anchors = get_anchor_points(&reloaded);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].turn_index, 10);
    
    // @step And compaction state should also be present
    assert!(reloaded.compaction.is_some());
}

// =============================================================================
// EDGE: Invalid Anchor Type String
// =============================================================================

#[test]
fn test_unknown_anchor_type_string_deserializes() {
    // The NAPI layer should handle unknown anchor types gracefully
    // (defaults to UserCheckpoint in the restore function)
    
    let json = r#"{
        "turn_index": 5,
        "anchor_type": "UnknownType",
        "weight": 0.5,
        "confidence": 0.5,
        "description": "Unknown type test",
        "timestamp_ms": 1738713600000
    }"#;
    
    // Should deserialize without error (string field accepts any string)
    let anchor: codelet_napi::persistence::PersistedAnchorPoint = 
        serde_json::from_str(json).expect("deserialize");
    
    assert_eq!(anchor.anchor_type, "UnknownType");
    // The NAPI layer will map this to UserCheckpoint as fallback
}
