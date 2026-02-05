//! Feature: LLM Anchor Detection Integration Tests
//!
//! These tests verify that the complete anchor detection flow works correctly,
//! including JSON extraction from LLM responses and proper logging of errors.
//!
//! The key scenarios tested:
//! 1. JSON extraction from markdown code blocks
//! 2. Error logging when LLM call fails
//! 3. Synthetic anchor creation on failure/timeout
//! 4. Full compaction flow with anchor detection
//!
//! Run with: cargo test --test llm_anchor_integration_test -- --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_core::compaction::{
    AnchorDetector, AnchorType, ContextCompactor, ConversationTurn,
    ToolCall as CompactionToolCall, ToolResult as CompactionToolResult,
};
use std::time::SystemTime;

// =============================================================================
// FIXTURE: Create conversation turns
// =============================================================================

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

fn create_turn_with_tool(
    user_msg: &str,
    tool_name: &str,
    result_success: bool,
    previous_error: bool,
    assistant_resp: &str,
) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        assistant_response: assistant_resp.to_string(),
        tool_calls: vec![CompactionToolCall {
            tool: tool_name.to_string(),
            id: format!("tool_{}", uuid::Uuid::new_v4()),
            parameters: serde_json::json!({"file_path": "/test/file.rs"}),
        }],
        tool_results: vec![CompactionToolResult {
            success: result_success,
            output: if result_success {
                "All tests passed successfully".to_string()
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

// =============================================================================
// SCENARIO: Complete compaction flow with LLM anchor detection
// =============================================================================

#[tokio::test]
async fn test_complete_compaction_flow_with_llm_anchors() {
    // @step Given a conversation with 5 turns including an error resolution
    let turns = vec![
        create_turn("Start the task", "I'll begin working on this", 500),
        create_turn("Can you read the file?", "Reading the file...", 500),
        create_turn_with_tool(
            "Fix the bug",
            "Edit",
            true,
            true, // Had previous error
            "I've fixed the bug and all tests pass",
        ),
        create_turn("What's the status?", "Everything is working now", 500),
        create_turn("Great, thank you!", "You're welcome!", 300),
    ];

    // Mock LLM that returns proper JSON with anchors
    let mock_llm = |_prompt: String| async move {
        Ok(r#"```json
[
  {"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "Routine start"},
  {"turn_index": 1, "anchor_type": null, "confidence": 0.0, "description": "File read operation"},
  {"turn_index": 2, "anchor_type": "ErrorResolution", "confidence": 0.96, "description": "Fixed bug and tests pass"},
  {"turn_index": 3, "anchor_type": null, "confidence": 0.0, "description": "Status check"},
  {"turn_index": 4, "anchor_type": null, "confidence": 0.0, "description": "Polite closure"}
]
```"#.to_string())
    };

    // @step When compaction is run
    let compactor = ContextCompactor::new();
    let result = compactor.compact(&turns, 10000, mock_llm).await;

    // @step Then compaction should succeed
    assert!(result.is_ok());
    let compaction_result = result.unwrap();

    // @step And an ErrorResolution anchor should be detected at turn 2
    assert!(compaction_result.anchor.is_some());
    let anchor = compaction_result.anchor.unwrap();
    assert_eq!(anchor.anchor_type, AnchorType::ErrorResolution);
    assert_eq!(anchor.turn_index, 2);
    assert!((anchor.confidence - 0.96).abs() < 0.01);
    assert!(anchor.description.contains("Fixed bug"));
}

#[tokio::test]
async fn test_compaction_with_llm_returning_markdown_json() {
    // @step Given a simple conversation
    let turns = vec![
        create_turn("Hello", "Hi there!", 200),
        create_turn_with_tool(
            "Create the file",
            "Write",
            true,
            false, // No previous error
            "Created the file successfully",
        ),
    ];

    // Mock LLM that wraps JSON in markdown
    let mock_llm = |_prompt: String| async move {
        Ok(r#"Based on my analysis:

```json
[
  {"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "Greeting"},
  {"turn_index": 1, "anchor_type": "TaskCompletion", "confidence": 0.93, "description": "File created successfully"}
]
```

These are the detected anchors."#.to_string())
    };

    // @step When compaction is run
    let compactor = ContextCompactor::new();
    let result = compactor.compact(&turns, 10000, mock_llm).await;

    // @step Then TaskCompletion anchor should be detected
    assert!(result.is_ok());
    let compaction_result = result.unwrap();
    assert!(compaction_result.anchor.is_some());
    let anchor = compaction_result.anchor.unwrap();
    assert_eq!(anchor.anchor_type, AnchorType::TaskCompletion);
    assert_eq!(anchor.turn_index, 1);
}

#[tokio::test]
async fn test_compaction_with_llm_error_creates_synthetic_anchor() {
    // @step Given a conversation
    let turns = vec![
        create_turn("Hello", "Hi!", 200),
        create_turn("Goodbye", "Bye!", 200),
    ];

    // Mock LLM that fails
    let mock_llm = |_prompt: String| async move {
        Err(anyhow::anyhow!("Model is required. Please select a model before creating a session."))
    };

    // @step When compaction is run and LLM fails
    let compactor = ContextCompactor::new();
    let result = compactor.compact(&turns, 10000, mock_llm).await;

    // @step Then compaction should still succeed with synthetic anchor
    assert!(result.is_ok());
    let compaction_result = result.unwrap();
    
    // @step And a synthetic anchor should be created
    assert!(compaction_result.anchor.is_some());
    let anchor = compaction_result.anchor.unwrap();
    assert_eq!(anchor.anchor_type, AnchorType::UserCheckpoint);
    assert!(anchor.description.contains("LLM analysis failed"));
    assert!(anchor.description.contains("Model is required"));
    assert_eq!(anchor.confidence, 1.0);
    assert_eq!(anchor.weight, 1.0);
}

#[tokio::test]
async fn test_compaction_with_no_anchors_creates_synthetic() {
    // @step Given a routine conversation with no significant milestones
    let turns = vec![
        create_turn("What's the weather?", "I don't have weather data", 200),
        create_turn("Oh okay", "Let me know if you need anything else", 200),
    ];

    // Mock LLM that returns no anchors (all null anchor_type)
    let mock_llm = |_prompt: String| async move {
        Ok(r#"[
  {"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "Query"},
  {"turn_index": 1, "anchor_type": null, "confidence": 0.0, "description": "Acknowledgment"}
]"#.to_string())
    };

    // @step When compaction is run
    let compactor = ContextCompactor::new();
    let result = compactor.compact(&turns, 10000, mock_llm).await;

    // @step Then a synthetic anchor should be created at the last turn
    assert!(result.is_ok());
    let compaction_result = result.unwrap();
    assert!(compaction_result.anchor.is_some());
    let anchor = compaction_result.anchor.unwrap();
    assert_eq!(anchor.anchor_type, AnchorType::UserCheckpoint);
    assert!(anchor.description.contains("no natural anchors detected"));
}

// =============================================================================
// SCENARIO: Batch anchor detection with various JSON formats
// =============================================================================

#[tokio::test]
async fn test_batch_detect_extracts_json_from_text() {
    // @step Given turns with LLM that embeds JSON in explanation text
    let turns = vec![
        create_turn("Start", "Starting...", 100),
        create_turn_with_tool("Finish", "Edit", true, false, "Done!"),
    ];

    let detector = AnchorDetector::new(0.9);

    let mock_llm = |_prompt: String| async move {
        Ok(r#"Here is my analysis of the conversation:

[{"turn_index": 1, "anchor_type": "TaskCompletion", "confidence": 0.94, "description": "Task completed"}]

The task was completed successfully at turn 1."#.to_string())
    };

    // @step When batch detection is run
    let anchors = detector.detect_batch(&turns, &mock_llm).await.unwrap();

    // @step Then anchor should be extracted correctly
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].turn_index, 1);
    assert_eq!(anchors[0].anchor_type, AnchorType::TaskCompletion);
}

#[tokio::test]
async fn test_batch_detect_with_generic_code_block() {
    // @step Given LLM that uses generic code block (not json-specific)
    let turns = vec![create_turn("Test", "Testing", 100)];

    let detector = AnchorDetector::new(0.9);

    let mock_llm = |_prompt: String| async move {
        Ok(r#"```
[{"turn_index": 0, "anchor_type": "FeatureMilestone", "confidence": 0.92, "description": "Milestone reached"}]
```"#.to_string())
    };

    // @step When batch detection is run
    let anchors = detector.detect_batch(&turns, &mock_llm).await.unwrap();

    // @step Then anchor should be extracted
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].anchor_type, AnchorType::FeatureMilestone);
}

// =============================================================================
// SCENARIO: Verify anchor weights are correctly applied
// =============================================================================

#[tokio::test]
async fn test_anchor_weights_from_llm_response() {
    let detector = AnchorDetector::new(0.9);

    // Test each anchor type
    let test_cases = vec![
        ("ErrorResolution", AnchorType::ErrorResolution, 0.9),
        ("TaskCompletion", AnchorType::TaskCompletion, 0.8),
        ("FeatureMilestone", AnchorType::FeatureMilestone, 0.75),
        ("UserCheckpoint", AnchorType::UserCheckpoint, 0.7),
    ];

    for (type_str, expected_type, expected_weight) in test_cases {
        let turns = vec![create_turn("Test", "Response", 100)];

        let mock_llm = move |_prompt: String| {
            let type_str = type_str.to_string();
            async move {
                Ok(format!(
                    r#"[{{"turn_index": 0, "anchor_type": "{}", "confidence": 0.95, "description": "Test"}}]"#,
                    type_str
                ))
            }
        };

        let anchors = detector.detect_batch(&turns, &mock_llm).await.unwrap();
        
        assert_eq!(anchors.len(), 1, "Should have 1 anchor for {}", type_str);
        assert_eq!(anchors[0].anchor_type, expected_type);
        assert!(
            (anchors[0].weight - expected_weight).abs() < 0.01,
            "Weight for {} should be {}, got {}",
            type_str,
            expected_weight,
            anchors[0].weight
        );
    }
}

// =============================================================================
// SCENARIO: Multiple anchors in single conversation
// =============================================================================

#[tokio::test]
async fn test_multiple_anchors_detected() {
    let turns = vec![
        create_turn_with_tool("Fix first bug", "Edit", true, true, "Fixed first bug"),
        create_turn("Okay", "Continuing...", 100),
        create_turn_with_tool("Fix second bug", "Edit", true, true, "Fixed second bug"),
        create_turn("Finish", "All done", 100),
    ];

    let detector = AnchorDetector::new(0.9);

    let mock_llm = |_prompt: String| async move {
        Ok(r#"[
  {"turn_index": 0, "anchor_type": "ErrorResolution", "confidence": 0.95, "description": "First bug fixed"},
  {"turn_index": 1, "anchor_type": null, "confidence": 0.0, "description": "Continuation"},
  {"turn_index": 2, "anchor_type": "ErrorResolution", "confidence": 0.94, "description": "Second bug fixed"},
  {"turn_index": 3, "anchor_type": null, "confidence": 0.0, "description": "Closure"}
]"#.to_string())
    };

    let anchors = detector.detect_batch(&turns, &mock_llm).await.unwrap();
    
    // @step Then multiple anchors should be detected
    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].turn_index, 0);
    assert_eq!(anchors[1].turn_index, 2);
    assert_eq!(anchors[0].anchor_type, AnchorType::ErrorResolution);
    assert_eq!(anchors[1].anchor_type, AnchorType::ErrorResolution);
}
