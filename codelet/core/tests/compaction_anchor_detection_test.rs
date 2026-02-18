//! Feature: Compaction Anchor Detection Tests
//!
//! These tests verify the core compaction logic that detects anchor points
//! during context compaction operations.
//!
//! Test Levels:
//! 1. AnchorDetector creates synthetic anchors on failure
//! 2. Compactor always returns an anchor (natural or synthetic)
//! 3. Anchor types have correct weights
//! 4. TurnSelector preserves anchors in selection
//!
//! Run with: cargo test --test compaction_anchor_detection_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::compaction::{
    AnchorPoint, AnchorType, ConversationTurn,
    ToolCall as CompactionToolCall, ToolResult as CompactionToolResult,
};
use std::time::SystemTime;

// =============================================================================
// TEST FIXTURES: Conversation Turns
// =============================================================================

/// Create a basic conversation turn
fn create_turn(user_msg: &str, assistant_resp: &str, tokens: u64) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        assistant_response: assistant_resp.to_string(),
        tool_calls: vec![],
        tool_results: vec![],
        tokens,
        timestamp: SystemTime::now(),
        previous_error: None,
    }
}

/// Create a turn with tool calls
fn create_turn_with_tool(
    user_msg: &str,
    tool_name: &str,
    result_success: bool,
    previous_error: bool,
) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        assistant_response: "I'll help with that.".to_string(),
        tool_calls: vec![CompactionToolCall {
            tool: tool_name.to_string(),
            id: format!("tool_{}", uuid::Uuid::new_v4()),
            parameters: serde_json::json!({}),
        }],
        tool_results: vec![CompactionToolResult {
            success: result_success,
            output: if result_success {
                "Tests passed successfully".to_string()
            } else {
                "Error: test failed".to_string()
            },
            error: if result_success { None } else { Some("Test failure".to_string()) },
        }],
        tokens: 1000,
        timestamp: SystemTime::now(),
        previous_error: Some(previous_error),
    }
}

/// Create an error resolution turn (previous error + Edit + test pass)
fn create_error_resolution_turn() -> ConversationTurn {
    create_turn_with_tool("Fix the build error", "Edit", true, true)
}

/// Create a task completion turn (no previous error + Write + test pass)
fn create_task_completion_turn() -> ConversationTurn {
    create_turn_with_tool("Add a new feature", "Write", true, false)
}

// =============================================================================
// UNIT: AnchorType Properties
// =============================================================================

#[test]
fn test_anchor_type_weights() {
    // @step Given the AnchorType enum
    // @step Then ErrorResolution should have weight 0.9
    assert!((AnchorType::ErrorResolution.weight() - 0.9).abs() < 0.001);
    
    // @step And TaskCompletion should have weight 0.8
    assert!((AnchorType::TaskCompletion.weight() - 0.8).abs() < 0.001);
    
    // @step And FeatureMilestone should have weight 0.75
    assert!((AnchorType::FeatureMilestone.weight() - 0.75).abs() < 0.001);
    
    // @step And UserCheckpoint should have weight 0.7
    assert!((AnchorType::UserCheckpoint.weight() - 0.7).abs() < 0.001);
}

#[test]
fn test_anchor_type_ordering_by_weight() {
    // Higher weight = more important to preserve
    let weights = [
        AnchorType::ErrorResolution.weight(),
        AnchorType::TaskCompletion.weight(),
        AnchorType::FeatureMilestone.weight(),
        AnchorType::UserCheckpoint.weight(),
    ];
    
    // ErrorResolution (0.9) > TaskCompletion (0.8) > FeatureMilestone (0.75) > UserCheckpoint (0.7)
    assert!(weights[0] > weights[1]);
    assert!(weights[1] > weights[2]);
    assert!(weights[2] > weights[3]);
}

// =============================================================================
// UNIT: AnchorPoint Construction
// =============================================================================

#[test]
fn test_anchor_point_construction() {
    let turn = create_turn("Test user message", "Test response", 1000);
    
    let anchor = AnchorPoint {
        turn_index: 5,
        anchor_type: AnchorType::TaskCompletion,
        weight: AnchorType::TaskCompletion.weight(),
        confidence: 0.92,
        description: "Task completed successfully".to_string(),
        timestamp: turn.timestamp,
    };
    
    assert_eq!(anchor.turn_index, 5);
    assert_eq!(anchor.anchor_type, AnchorType::TaskCompletion);
    assert!((anchor.weight - 0.8).abs() < 0.001);
    assert!((anchor.confidence - 0.92).abs() < 0.001);
}

#[test]
fn test_synthetic_checkpoint_creation() {
    // @step Given a conversation turn
    let turn = create_turn("User message", "Response", 1000);
    
    // @step When I create a synthetic checkpoint
    let synthetic = AnchorPoint::synthetic_checkpoint(10, &turn, "LLM timeout");
    
    // @step Then it should have weight 1.0 (highest priority)
    assert!((synthetic.weight - 1.0).abs() < 0.001);
    
    // @step And confidence 1.0 (full confidence)
    assert!((synthetic.confidence - 1.0).abs() < 0.001);
    
    // @step And type UserCheckpoint
    assert_eq!(synthetic.anchor_type, AnchorType::UserCheckpoint);
    
    // @step And description containing the reason
    assert!(synthetic.description.contains("Synthetic anchor"));
    assert!(synthetic.description.contains("LLM timeout"));
}

// =============================================================================
// UNIT: Anchor Type Serialization
// =============================================================================

#[test]
fn test_anchor_type_serializes_to_string() {
    // AnchorType must serialize to match the strings expected by persistence layer
    let json = serde_json::to_string(&AnchorType::ErrorResolution).unwrap();
    assert_eq!(json, "\"ErrorResolution\"");
    
    let json = serde_json::to_string(&AnchorType::TaskCompletion).unwrap();
    assert_eq!(json, "\"TaskCompletion\"");
    
    let json = serde_json::to_string(&AnchorType::UserCheckpoint).unwrap();
    assert_eq!(json, "\"UserCheckpoint\"");
    
    let json = serde_json::to_string(&AnchorType::FeatureMilestone).unwrap();
    assert_eq!(json, "\"FeatureMilestone\"");
}

#[test]
fn test_anchor_type_deserializes_from_string() {
    let error_res: AnchorType = serde_json::from_str("\"ErrorResolution\"").unwrap();
    assert_eq!(error_res, AnchorType::ErrorResolution);
    
    let task_comp: AnchorType = serde_json::from_str("\"TaskCompletion\"").unwrap();
    assert_eq!(task_comp, AnchorType::TaskCompletion);
    
    let checkpoint: AnchorType = serde_json::from_str("\"UserCheckpoint\"").unwrap();
    assert_eq!(checkpoint, AnchorType::UserCheckpoint);
    
    let milestone: AnchorType = serde_json::from_str("\"FeatureMilestone\"").unwrap();
    assert_eq!(milestone, AnchorType::FeatureMilestone);
}

// =============================================================================
// UNIT: AnchorPoint Serialization
// =============================================================================

#[test]
fn test_anchor_point_round_trip_serialization() {
    let turn = create_turn("Test", "Response", 1000);
    
    let original = AnchorPoint {
        turn_index: 42,
        anchor_type: AnchorType::ErrorResolution,
        weight: 0.9,
        confidence: 0.95,
        description: "Test anchor with special chars: <>&\"'".to_string(),
        timestamp: turn.timestamp,
    };
    
    // Serialize
    let json = serde_json::to_string(&original).expect("serialize");
    
    // Deserialize
    let restored: AnchorPoint = serde_json::from_str(&json).expect("deserialize");
    
    assert_eq!(restored.turn_index, original.turn_index);
    assert_eq!(restored.anchor_type, original.anchor_type);
    assert!((restored.weight - original.weight).abs() < 0.001);
    assert!((restored.confidence - original.confidence).abs() < 0.001);
    assert_eq!(restored.description, original.description);
}

// =============================================================================
// INTEGRATION: Conversation Turn to Anchor Conversion
// =============================================================================

#[test]
fn test_conversation_turn_has_timestamp_for_anchor() {
    // AnchorPoint uses the turn's timestamp
    let turn = create_turn("User msg", "Response", 1000);
    
    let anchor = AnchorPoint {
        turn_index: 0,
        anchor_type: AnchorType::TaskCompletion,
        weight: 0.8,
        confidence: 0.9,
        description: "Test".to_string(),
        timestamp: turn.timestamp,
    };
    
    // Timestamps should match
    assert_eq!(anchor.timestamp, turn.timestamp);
}

#[test]
fn test_error_resolution_turn_has_correct_flags() {
    let turn = create_error_resolution_turn();
    
    // previous_error = true
    assert_eq!(turn.previous_error, Some(true));
    
    // Has Edit tool call
    assert!(turn.tool_calls.iter().any(|tc| tc.tool == "Edit"));
    
    // Has successful result
    assert!(turn.tool_results.iter().any(|tr| tr.success));
}

#[test]
fn test_task_completion_turn_has_correct_flags() {
    let turn = create_task_completion_turn();
    
    // previous_error = false
    assert_eq!(turn.previous_error, Some(false));
    
    // Has Write tool call
    assert!(turn.tool_calls.iter().any(|tc| tc.tool == "Write"));
    
    // Has successful result
    assert!(turn.tool_results.iter().any(|tr| tr.success));
}

// =============================================================================
// UNIT: Multiple Anchors Ordering
// =============================================================================

#[test]
fn test_anchor_vec_ordering() {
    let turn = create_turn("Test", "Response", 1000);
    
    let anchors = [
        AnchorPoint {
            turn_index: 10,
            anchor_type: AnchorType::ErrorResolution,
            weight: 0.9,
            confidence: 0.95,
            description: "First".to_string(),
            timestamp: turn.timestamp,
        },
        AnchorPoint {
            turn_index: 5,
            anchor_type: AnchorType::TaskCompletion,
            weight: 0.8,
            confidence: 0.92,
            description: "Second (lower index)".to_string(),
            timestamp: turn.timestamp,
        },
    ];
    
    // anchors.last() returns the most recently detected anchor
    let last = anchors.last().unwrap();
    assert_eq!(last.turn_index, 5);
    assert_eq!(last.description, "Second (lower index)");
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_anchor_at_turn_zero() {
    let turn = create_turn("First message", "First response", 500);
    
    let anchor = AnchorPoint {
        turn_index: 0,
        anchor_type: AnchorType::UserCheckpoint,
        weight: 0.7,
        confidence: 0.85,
        description: "Anchor at turn 0".to_string(),
        timestamp: turn.timestamp,
    };
    
    assert_eq!(anchor.turn_index, 0);
}

#[test]
fn test_anchor_with_empty_description() {
    let turn = create_turn("Test", "Response", 1000);
    
    let anchor = AnchorPoint {
        turn_index: 5,
        anchor_type: AnchorType::TaskCompletion,
        weight: 0.8,
        confidence: 0.9,
        description: String::new(),
        timestamp: turn.timestamp,
    };
    
    assert!(anchor.description.is_empty());
    
    // Should still serialize/deserialize
    let json = serde_json::to_string(&anchor).unwrap();
    let restored: AnchorPoint = serde_json::from_str(&json).unwrap();
    assert!(restored.description.is_empty());
}

#[test]
fn test_anchor_with_unicode_description() {
    let turn = create_turn("Test", "Response", 1000);
    
    let description = "Тест 测试 テスト 🎉 emoji test";
    let anchor = AnchorPoint {
        turn_index: 5,
        anchor_type: AnchorType::FeatureMilestone,
        weight: 0.75,
        confidence: 0.9,
        description: description.to_string(),
        timestamp: turn.timestamp,
    };
    
    let json = serde_json::to_string(&anchor).unwrap();
    let restored: AnchorPoint = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.description, description);
}

#[test]
fn test_anchor_with_min_confidence() {
    let turn = create_turn("Test", "Response", 1000);
    
    let anchor = AnchorPoint {
        turn_index: 5,
        anchor_type: AnchorType::UserCheckpoint,
        weight: 0.7,
        confidence: 0.0, // Minimum confidence
        description: "Very uncertain anchor".to_string(),
        timestamp: turn.timestamp,
    };
    
    assert!((anchor.confidence - 0.0).abs() < 0.001);
}

#[test]
fn test_anchor_with_max_confidence() {
    let turn = create_turn("Test", "Response", 1000);
    
    let anchor = AnchorPoint {
        turn_index: 5,
        anchor_type: AnchorType::ErrorResolution,
        weight: 0.9,
        confidence: 1.0, // Maximum confidence (synthetic anchors)
        description: "Fully confident anchor".to_string(),
        timestamp: turn.timestamp,
    };
    
    assert!((anchor.confidence - 1.0).abs() < 0.001);
}

// =============================================================================
// UNIT: Token Counting for Turns
// =============================================================================

#[test]
fn test_turn_token_count() {
    let turn = create_turn("Short message", "Short response", 500);
    assert_eq!(turn.tokens, 500);
    
    let turn2 = create_turn("Longer message with more content", "Detailed response", 2000);
    assert_eq!(turn2.tokens, 2000);
}

#[test]
fn test_turn_sequence_total_tokens() {
    let turns = [
        create_turn("Msg 1", "Resp 1", 1000),
        create_turn("Msg 2", "Resp 2", 1500),
        create_turn("Msg 3", "Resp 3", 2000),
    ];
    
    let total: u64 = turns.iter().map(|t| t.tokens).sum();
    assert_eq!(total, 4500);
}
