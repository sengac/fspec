//! Feature: Full Compaction to Anchor Retrieval Integration Tests
//!
//! These tests verify the complete flow from compaction through anchor persistence
//! to retrieval. This is the critical path that must work for /anchors to show data.
//!
//! Flow under test:
//! 1. Compaction runs → detects/creates anchor point
//! 2. Anchor is stored in BackgroundSession memory
//! 3. Anchor is persisted to disk via persist_anchor_point
//! 4. On resume, anchors are restored from disk
//! 5. /anchors retrieves from BackgroundSession memory
//!
//! Run with: cargo test --test compaction_to_anchor_flow_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::persistence::{
    add_anchor_point, append_message_with_metadata, create_session, get_anchor_points,
    load_session, set_compaction_state, PersistedAnchorPoint,
};
use codelet_napi::test_support::{
    create_assistant_text_envelope, create_error_resolution_anchor,
    create_feature_milestone_anchor, create_session_with_anchors, create_session_with_tool_calls,
    create_session_with_turns, create_synthetic_anchor, create_task_completion_anchor,
    create_user_checkpoint_anchor, create_user_envelope, setup_test_env,
};
use std::path::PathBuf;

// =============================================================================
// SCENARIO: Compaction produces an anchor that is persisted
// =============================================================================

#[test]
fn test_simulated_compaction_persists_anchor() {
    // @step Given a session with conversation history
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Compaction Flow Test", &project).expect("create");
    let session_id = session.id;
    
    // Add messages to simulate pre-compaction state
    for i in 0..10 {
        let user_meta = create_user_envelope(&format!("Message {}", i));
        append_message_with_metadata(&mut session, "user", &format!("Message {}", i), user_meta).expect("append");
        
        let asst_meta = create_assistant_text_envelope(&format!("Response {}", i));
        append_message_with_metadata(&mut session, "assistant", &format!("Response {}", i), asst_meta).expect("append");
    }
    
    // @step When compaction runs and detects an anchor
    // (Simulating what session_compact does)
    set_compaction_state(&mut session, "Summary of turns 0-7".to_string(), 16).expect("compact");
    
    // Simulate anchor detection during compaction (turn 8 was an error resolution)
    let anchor = create_error_resolution_anchor(8);
    add_anchor_point(&mut session, anchor).expect("add anchor");
    
    // @step Then the anchor should be in the session
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 1, "Should have 1 anchor after compaction");
    
    // @step And it should survive session reload (persistence works)
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    
    let restored_anchors = get_anchor_points(&reloaded);
    assert_eq!(restored_anchors.len(), 1, "Anchor should survive reload");
    assert_eq!(restored_anchors[0].turn_index, 8);
    assert_eq!(restored_anchors[0].anchor_type, "ErrorResolution");
}

#[test]
fn test_multiple_compactions_accumulate_anchors() {
    // @step Given a long-running session with multiple compaction cycles
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Multi Compaction Flow", &project).expect("create");
    let session_id = session.id;
    
    // First compaction cycle
    set_compaction_state(&mut session, "First summary".to_string(), 20).expect("compact 1");
    add_anchor_point(&mut session, create_task_completion_anchor(15)).expect("add anchor 1");
    
    // More conversation...
    for i in 0..5 {
        let user_meta = create_user_envelope(&format!("Post-compact message {}", i));
        append_message_with_metadata(&mut session, "user", &format!("Msg {}", i), user_meta).expect("append");
    }
    
    // Second compaction cycle
    set_compaction_state(&mut session, "Second summary".to_string(), 30).expect("compact 2");
    add_anchor_point(&mut session, create_error_resolution_anchor(28)).expect("add anchor 2");
    
    // Third compaction cycle
    set_compaction_state(&mut session, "Third summary".to_string(), 45).expect("compact 3");
    add_anchor_point(&mut session, create_feature_milestone_anchor(42)).expect("add anchor 3");
    
    // @step Then all 3 anchors should be present
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 3, "Should have 3 anchors");
    
    // @step And they should survive reload
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 3, "All 3 anchors should survive");
    assert_eq!(restored[0].turn_index, 15);
    assert_eq!(restored[1].turn_index, 28);
    assert_eq!(restored[2].turn_index, 42);
}

// =============================================================================
// SCENARIO: Synthetic anchors from timeout/failure
// =============================================================================

#[test]
fn test_synthetic_anchor_persists_through_flow() {
    // @step Given compaction where LLM detection failed (timeout)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Synthetic Anchor Flow", &project).expect("create");
    let session_id = session.id;
    
    // Compaction happens, but LLM detection timed out
    // System creates synthetic anchor as fallback
    set_compaction_state(&mut session, "Summary".to_string(), 20).expect("compact");
    
    let synthetic = create_synthetic_anchor(19, "LLM detection timeout");
    add_anchor_point(&mut session, synthetic).expect("add");
    
    // @step Then synthetic anchor should have full confidence
    let anchors = get_anchor_points(&session);
    assert!((anchors[0].weight - 1.0).abs() < 0.001);
    assert!((anchors[0].confidence - 1.0).abs() < 0.001);
    
    // @step And it should persist and restore correctly
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    
    let restored = get_anchor_points(&reloaded);
    assert!((restored[0].weight - 1.0).abs() < 0.001);
    assert!(restored[0].description.contains("LLM detection timeout"));
}

// =============================================================================
// SCENARIO: Anchor retrieval after session operations
// =============================================================================

#[test]
fn test_anchors_available_after_message_append() {
    // @step Given a compacted session with anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Append After Compaction", &project).expect("create");
    let session_id = session.id;
    
    set_compaction_state(&mut session, "Summary".to_string(), 10).expect("compact");
    add_anchor_point(&mut session, create_task_completion_anchor(8)).expect("add");
    
    // @step When more messages are appended after compaction
    let user_meta = create_user_envelope("New message after compaction");
    append_message_with_metadata(&mut session, "user", "New msg", user_meta).expect("append");
    
    // @step Then anchors should still be retrievable
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 1, "Anchor should still be present");
    
    // @step And survive reload
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 1);
}

#[test]
fn test_anchors_available_after_compaction_state_update() {
    // @step Given a session with an anchor from first compaction
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Compaction State Update", &project).expect("create");
    let session_id = session.id;
    
    set_compaction_state(&mut session, "First summary".to_string(), 10).expect("compact 1");
    add_anchor_point(&mut session, create_task_completion_anchor(8)).expect("add 1");
    
    // @step When compaction state is updated (new compaction)
    set_compaction_state(&mut session, "Second summary".to_string(), 20).expect("compact 2");
    add_anchor_point(&mut session, create_error_resolution_anchor(18)).expect("add 2");
    
    // @step Then both anchors should be present
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 2);
    
    // @step And compaction state should be the latest
    assert!(session.compaction.as_ref().unwrap().summary.contains("Second"));
    
    // @step And survive reload
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 2);
}

// =============================================================================
// SCENARIO: Anchor data integrity through the flow
// =============================================================================

#[test]
fn test_anchor_data_integrity_through_full_flow() {
    // @step Given an anchor with precise data
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Data Integrity Test", &project).expect("create");
    let session_id = session.id;
    
    let original = PersistedAnchorPoint {
        turn_index: 42,
        anchor_type: "FeatureMilestone".to_string(),
        weight: 0.7532,
        confidence: 0.8891,
        description: "Complex description: \"quotes\", <tags>, 日本語".to_string(),
        timestamp_ms: 1738713654321,
        user_message: Some("Test message".to_string()),
        assistant_response: Some("Test response".to_string()),
        tool_calls: vec![],
    };
    
    add_anchor_point(&mut session, original.clone()).expect("add");
    
    // @step When session goes through save/load cycle
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    
    // @step Then all data should be exactly preserved
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 1);
    
    let r = &restored[0];
    assert_eq!(r.turn_index, original.turn_index);
    assert_eq!(r.anchor_type, original.anchor_type);
    assert!((r.weight - original.weight).abs() < 0.0001);
    assert!((r.confidence - original.confidence).abs() < 0.0001);
    assert_eq!(r.description, original.description);
    assert_eq!(r.timestamp_ms, original.timestamp_ms);
}

// =============================================================================
// SCENARIO: No anchors when no compaction
// =============================================================================

#[test]
fn test_no_anchors_without_compaction() {
    // @step Given a session with messages but no compaction
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("No Compaction Session", &project).expect("create");
    let session_id = session.id;
    
    // Add messages without compaction
    for i in 0..5 {
        let user_meta = create_user_envelope(&format!("Message {}", i));
        append_message_with_metadata(&mut session, "user", &format!("Msg {}", i), user_meta).expect("append");
    }
    
    // @step Then anchors should be empty
    let anchors = get_anchor_points(&session);
    assert!(anchors.is_empty(), "No anchors without compaction");
    
    // @step And reload should also have no anchors
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    let restored = get_anchor_points(&reloaded);
    assert!(restored.is_empty());
    
    // @step And compaction state should be None
    assert!(reloaded.compaction.is_none());
}

// =============================================================================
// SCENARIO: Anchors with tool calls in conversation
// =============================================================================

#[test]
fn test_anchor_from_tool_call_conversation() {
    // @step Given a session with tool call messages
    let (_guard, _temp_dir) = setup_test_env();
    let mut session = create_session_with_tool_calls("Tool Call Session");
    let session_id = session.id;
    
    // @step When compaction detects an anchor during tool usage
    set_compaction_state(&mut session, "Summary including tool operations".to_string(), 4).expect("compact");
    add_anchor_point(&mut session, create_task_completion_anchor(3)).expect("add");
    
    // @step Then anchor should be present
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 1);
    
    // @step And survive reload with tool messages
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 1);
}

// =============================================================================
// SCENARIO: Rapid anchor additions
// =============================================================================

#[test]
fn test_rapid_anchor_additions_persist_correctly() {
    // @step Given rapid anchor additions (simulating quick compactions)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Rapid Additions", &project).expect("create");
    let session_id = session.id;
    
    // Add 20 anchors quickly
    for i in 0..20 {
        let anchor = create_task_completion_anchor(i * 5);
        add_anchor_point(&mut session, anchor).expect("add");
    }
    
    // @step Then all should be in memory
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 20);
    
    // @step And all should persist
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 20);
    
    // @step And order should be preserved
    for (i, anchor) in restored.iter().enumerate() {
        assert_eq!(anchor.turn_index, i * 5);
    }
}

// =============================================================================
// SCENARIO: Session with all anchor types
// =============================================================================

#[test]
fn test_all_anchor_types_in_single_session() {
    // @step Given a session with all 4 anchor types
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("All Types Session", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_error_resolution_anchor(10)).expect("add error");
    add_anchor_point(&mut session, create_task_completion_anchor(20)).expect("add task");
    add_anchor_point(&mut session, create_user_checkpoint_anchor(30)).expect("add checkpoint");
    add_anchor_point(&mut session, create_feature_milestone_anchor(40)).expect("add milestone");
    
    // @step Then all types should be present
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 4);
    assert_eq!(anchors[0].anchor_type, "ErrorResolution");
    assert_eq!(anchors[1].anchor_type, "TaskCompletion");
    assert_eq!(anchors[2].anchor_type, "UserCheckpoint");
    assert_eq!(anchors[3].anchor_type, "FeatureMilestone");
    
    // @step And weights should be correct
    assert!((anchors[0].weight - 0.9).abs() < 0.001);  // ErrorResolution
    assert!((anchors[1].weight - 0.8).abs() < 0.001);  // TaskCompletion
    assert!((anchors[2].weight - 0.7).abs() < 0.001);  // UserCheckpoint
    assert!((anchors[3].weight - 0.75).abs() < 0.001); // FeatureMilestone
    
    // @step And all should survive reload
    drop(session);
    let reloaded = load_session(session_id).expect("reload");
    let restored = get_anchor_points(&reloaded);
    assert_eq!(restored.len(), 4);
}

// =============================================================================
// SCENARIO: Verify fixture uses correct session factory
// =============================================================================

#[test]
fn test_fixture_session_with_tool_calls_has_correct_structure() {
    // @step Given a session created with tool calls fixture
    let (_guard, _temp_dir) = setup_test_env();
    let session = create_session_with_tool_calls("Fixture Test");
    
    // @step Then it should have the expected message structure
    // (6 messages: user, assistant+tool_use, tool_result, user, assistant+tool_use, tool_result)
    assert!(session.messages.len() >= 6, "Should have at least 6 messages");
}

#[test]
fn test_fixture_session_with_turns_creates_correct_count() {
    // @step Given session fixture with specific turn count
    let (_guard, _temp_dir) = setup_test_env();
    let session = create_session_with_turns("Turn Count Test", 5);
    
    // @step Then message count should be turns * 2
    assert_eq!(session.messages.len(), 10, "5 turns = 10 messages");
}

#[test]
fn test_fixture_session_with_anchors_creates_both() {
    // @step Given session fixture with turns and anchors
    let (_guard, _temp_dir) = setup_test_env();
    let session = create_session_with_anchors("Anchor Fixture Test", 5, &[1, 3]);
    
    // @step Then should have both turns and anchors
    assert_eq!(session.messages.len(), 10);
    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].turn_index, 1);
    assert_eq!(anchors[1].turn_index, 3);
}
