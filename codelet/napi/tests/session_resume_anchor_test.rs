//! Feature: Session Resume Anchor Restoration Tests
//!
//! These tests verify the full session resume flow, including:
//! - Anchor restoration from persisted manifest
//! - Compaction state restoration
//! - Message restoration with anchors
//!
//! This tests the integration between persistence layer and session management.
//!
//! Run with: cargo test --test session_resume_anchor_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod compaction_anchor_fixtures;

use codelet_napi::persistence::{
    create_session, load_session, add_anchor_point, get_anchor_points,
    set_compaction_state, append_message_with_metadata,
    get_session_messages,
};
use compaction_anchor_fixtures::*;
use std::path::PathBuf;

// =============================================================================
// SCENARIO: Resume session with anchors
// =============================================================================

#[test]
fn test_resume_session_with_single_anchor() {
    // @step Given a session that was compacted and has an anchor
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Resume Single Anchor", &project).expect("create");
    let session_id = session.id;
    
    // Add some messages
    let user_meta = create_user_envelope("Test message");
    append_message_with_metadata(&mut session, "user", "Test message", user_meta).expect("append");
    let asst_meta = create_assistant_text_envelope("Response");
    append_message_with_metadata(&mut session, "assistant", "Response", asst_meta).expect("append");
    
    // Add compaction state and anchor
    set_compaction_state(&mut session, "Summary of conversation".to_string(), 0).expect("compact");
    add_anchor_point(&mut session, create_task_completion_anchor(0)).expect("add anchor");
    
    // @step When I "resume" by loading from disk
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then the session should have the anchor
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].turn_index, 0);
    assert_eq!(anchors[0].anchor_type, "TaskCompletion");
    
    // @step And the compaction state should be present
    assert!(resumed.compaction.is_some());
}

#[test]
fn test_resume_session_with_multiple_anchors_from_multiple_compactions() {
    // @step Given a session that went through multiple compaction cycles
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Multi Compaction Resume", &project).expect("create");
    let session_id = session.id;
    
    // First compaction - anchor at turn 10
    add_anchor_point(&mut session, create_error_resolution_anchor(10)).expect("add");
    
    // Second compaction - anchor at turn 30
    add_anchor_point(&mut session, create_task_completion_anchor(30)).expect("add");
    
    // Third compaction - anchor at turn 50
    add_anchor_point(&mut session, create_feature_milestone_anchor(50)).expect("add");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then all anchors from all compactions should be present
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors.len(), 3);
    
    // @step And they should be in chronological order (order added)
    assert_eq!(anchors[0].turn_index, 10);
    assert_eq!(anchors[0].anchor_type, "ErrorResolution");
    
    assert_eq!(anchors[1].turn_index, 30);
    assert_eq!(anchors[1].anchor_type, "TaskCompletion");
    
    assert_eq!(anchors[2].turn_index, 50);
    assert_eq!(anchors[2].anchor_type, "FeatureMilestone");
}

#[test]
fn test_resume_session_without_anchors_has_empty_list() {
    // @step Given a session that was never compacted (no anchors)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let session = create_session("No Anchors Resume", &project).expect("create");
    let session_id = session.id;
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then anchors should be empty (not an error)
    let anchors = get_anchor_points(&resumed);
    assert!(anchors.is_empty());
}

// =============================================================================
// SCENARIO: Resume preserves anchor metadata
// =============================================================================

#[test]
fn test_resume_preserves_anchor_timestamps() {
    // @step Given a session with anchors at specific timestamps
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Timestamp Resume", &project).expect("create");
    let session_id = session.id;
    
    let timestamp1: i64 = 1738713600000; // Fixed timestamp 1
    let timestamp2: i64 = 1738717200000; // Fixed timestamp 2 (1 hour later)
    
    add_anchor_point(&mut session, create_anchor_with_timestamp(5, "TaskCompletion", timestamp1)).expect("add");
    add_anchor_point(&mut session, create_anchor_with_timestamp(15, "ErrorResolution", timestamp2)).expect("add");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then timestamps should be exactly preserved
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors[0].timestamp_ms, timestamp1);
    assert_eq!(anchors[1].timestamp_ms, timestamp2);
}

#[test]
fn test_resume_preserves_anchor_confidence_and_weight() {
    // @step Given an anchor with specific confidence and weight
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Confidence Resume", &project).expect("create");
    let session_id = session.id;
    
    let anchor = codelet_napi::persistence::PersistedAnchorPoint {
        turn_index: 10,
        anchor_type: "FeatureMilestone".to_string(),
        weight: 0.75,
        confidence: 0.88,
        description: "Custom weight/confidence test".to_string(),
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    add_anchor_point(&mut session, anchor).expect("add");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then weight and confidence should be exactly preserved
    let anchors = get_anchor_points(&resumed);
    assert!((anchors[0].weight - 0.75).abs() < 0.0001);
    assert!((anchors[0].confidence - 0.88).abs() < 0.0001);
}

#[test]
fn test_resume_preserves_anchor_descriptions() {
    // @step Given anchors with various descriptions
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Description Resume", &project).expect("create");
    let session_id = session.id;
    
    let descriptions = vec![
        "Simple description",
        "Description with\nnewlines\nand\ttabs",
        "Special chars: <>&\"'",
        "Unicode: 日本語 🎉 emoji",
        "", // Empty description
    ];
    
    for (i, desc) in descriptions.iter().enumerate() {
        let anchor = codelet_napi::persistence::PersistedAnchorPoint {
            turn_index: i,
            anchor_type: "TaskCompletion".to_string(),
            weight: 0.8,
            confidence: 0.9,
            description: desc.to_string(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };
        add_anchor_point(&mut session, anchor).expect("add");
    }
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then all descriptions should be preserved exactly
    let anchors = get_anchor_points(&resumed);
    for (i, desc) in descriptions.iter().enumerate() {
        assert_eq!(&anchors[i].description, *desc, "Description mismatch at index {}", i);
    }
}

// =============================================================================
// SCENARIO: Resume with messages and anchors
// =============================================================================

#[test]
fn test_resume_session_with_messages_and_anchors() {
    // @step Given a session with messages and anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Messages and Anchors", &project).expect("create");
    let session_id = session.id;
    
    // Add 10 message pairs (20 messages total)
    for i in 0..10 {
        let user_meta = create_user_envelope(&format!("User message {}", i));
        append_message_with_metadata(&mut session, "user", &format!("User message {}", i), user_meta).expect("append");
        
        let asst_meta = create_assistant_text_envelope(&format!("Response {}", i));
        append_message_with_metadata(&mut session, "assistant", &format!("Response {}", i), asst_meta).expect("append");
    }
    
    // Add anchors at specific conversation points
    add_anchor_point(&mut session, create_task_completion_anchor(3)).expect("add");
    add_anchor_point(&mut session, create_error_resolution_anchor(7)).expect("add");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then both messages and anchors should be present
    let messages = get_session_messages(&resumed).expect("get messages");
    assert_eq!(messages.len(), 20);
    
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors.len(), 2);
}

#[test]
fn test_resume_compacted_session_with_anchors() {
    // @step Given a compacted session with summary, messages, and anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Compacted Session Resume", &project).expect("create");
    let session_id = session.id;
    
    // Add kept messages (post-compaction) - 5 turns = 10 messages
    for i in 0..5 {
        let user_meta = create_user_envelope(&format!("Kept message {}", i));
        append_message_with_metadata(&mut session, "user", &format!("Kept message {}", i), user_meta).expect("append");
        
        let asst_meta = create_assistant_text_envelope(&format!("Kept response {}", i));
        append_message_with_metadata(&mut session, "assistant", &format!("Kept response {}", i), asst_meta).expect("append");
    }
    
    // Set compaction state - boundary is 0 because all messages are "kept" (post-compaction)
    // In a real compaction, the boundary would indicate how many messages to skip on reload
    let summary = "This is a summary of the compacted conversation.\nIt covers turns 0-20.";
    set_compaction_state(&mut session, summary.to_string(), 0).expect("compact");
    
    // Add anchor detected during compaction
    add_anchor_point(&mut session, create_task_completion_anchor(15)).expect("add anchor");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then all state should be preserved
    // Messages - 10 kept messages + 1 synthetic summary = 11 messages
    // (get_session_messages injects compaction summary as first message)
    let messages = get_session_messages(&resumed).expect("messages");
    assert_eq!(messages.len(), 11, "Expected 11 messages (1 summary + 10 kept), got {}", messages.len());
    
    // Compaction state
    let compaction = resumed.compaction.as_ref().expect("compaction");
    assert!(compaction.summary.contains("summary of the compacted conversation"));
    assert_eq!(compaction.compacted_before_index, 0);
    
    // Anchors
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].turn_index, 15);
}

// =============================================================================
// SCENARIO: Edge cases in resume
// =============================================================================

#[test]
fn test_resume_session_multiple_times() {
    // @step Given a session with anchors
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Multi Resume", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_task_completion_anchor(5)).expect("add");
    
    // @step When I resume multiple times
    drop(session);
    
    let resumed1 = load_session(session_id).expect("first load");
    let anchors1 = get_anchor_points(&resumed1);
    drop(resumed1);
    
    let resumed2 = load_session(session_id).expect("second load");
    let anchors2 = get_anchor_points(&resumed2);
    drop(resumed2);
    
    let resumed3 = load_session(session_id).expect("third load");
    let anchors3 = get_anchor_points(&resumed3);
    
    // @step Then anchors should be consistent across all loads
    assert_eq!(anchors1.len(), 1);
    assert_eq!(anchors2.len(), 1);
    assert_eq!(anchors3.len(), 1);
    
    assert_eq!(anchors1[0].turn_index, anchors2[0].turn_index);
    assert_eq!(anchors2[0].turn_index, anchors3[0].turn_index);
}

#[test]
fn test_add_anchor_after_resume_persists() {
    // @step Given a resumed session
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Add After Resume", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_task_completion_anchor(5)).expect("add first");
    drop(session);
    
    // Resume and add another anchor
    let mut resumed = load_session(session_id).expect("load");
    add_anchor_point(&mut resumed, create_error_resolution_anchor(15)).expect("add second");
    
    // @step When I resume again
    drop(resumed);
    let final_session = load_session(session_id).expect("final load");
    
    // @step Then both anchors should be present
    let anchors = get_anchor_points(&final_session);
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].turn_index, 5);
    assert_eq!(anchors[1].turn_index, 15);
}

// =============================================================================
// SCENARIO: Anchor order preservation
// =============================================================================

#[test]
fn test_anchor_insertion_order_preserved_through_resume() {
    // @step Given anchors added in specific order (not by turn index)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Order Preservation", &project).expect("create");
    let session_id = session.id;
    
    // Add in reverse turn order
    add_anchor_point(&mut session, create_task_completion_anchor(30)).expect("add");
    add_anchor_point(&mut session, create_error_resolution_anchor(10)).expect("add");
    add_anchor_point(&mut session, create_feature_milestone_anchor(50)).expect("add");
    add_anchor_point(&mut session, create_user_checkpoint_anchor(20)).expect("add");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then insertion order should be preserved (not sorted by turn)
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors[0].turn_index, 30);
    assert_eq!(anchors[1].turn_index, 10);
    assert_eq!(anchors[2].turn_index, 50);
    assert_eq!(anchors[3].turn_index, 20);
}

// =============================================================================
// SCENARIO: Large session resume
// =============================================================================

#[test]
fn test_resume_session_with_many_anchors() {
    // @step Given a session with many anchors (simulating very long conversation)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Many Anchors Resume", &project).expect("create");
    let session_id = session.id;
    
    // Add 100 anchors
    for i in 0..100 {
        let anchor = create_task_completion_anchor(i * 10);
        add_anchor_point(&mut session, anchor).expect("add");
    }
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then all 100 anchors should be present
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors.len(), 100);
    
    // @step And they should maintain order
    for (i, anchor) in anchors.iter().enumerate() {
        assert_eq!(anchor.turn_index, i * 10);
    }
}

// =============================================================================
// SCENARIO: Synthetic anchors in resume
// =============================================================================

#[test]
fn test_resume_preserves_synthetic_anchors() {
    // @step Given a session with synthetic anchors (from LLM detection failure)
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project");
    let mut session = create_session("Synthetic Resume", &project).expect("create");
    let session_id = session.id;
    
    add_anchor_point(&mut session, create_synthetic_anchor(20, "LLM timeout")).expect("add");
    add_anchor_point(&mut session, create_synthetic_anchor(40, "no natural anchors")).expect("add");
    
    // @step When I resume
    drop(session);
    let resumed = load_session(session_id).expect("load");
    
    // @step Then synthetic anchors should be preserved with full properties
    let anchors = get_anchor_points(&resumed);
    assert_eq!(anchors.len(), 2);
    
    // Synthetic anchors have weight 1.0 and confidence 1.0
    assert!((anchors[0].weight - 1.0).abs() < 0.001);
    assert!((anchors[0].confidence - 1.0).abs() < 0.001);
    assert!(anchors[0].description.contains("Synthetic anchor"));
    assert!(anchors[0].description.contains("LLM timeout"));
    
    assert!((anchors[1].weight - 1.0).abs() < 0.001);
    assert!(anchors[1].description.contains("no natural anchors"));
}
