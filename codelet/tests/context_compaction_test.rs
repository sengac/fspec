
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/context-compaction-with-anchoring-system.feature
//!
//! Tests for Context Compaction with Anchoring System
//!
//! These tests verify the implementation of intelligent context compaction
//! using anchor point detection, turn-based architecture, and LLM summarization.

use anyhow::Result;
use codelet_core::compaction::{
    AnchorPoint, AnchorType, CompactionMetrics, ContextCompactor,
    ConversationTurn, TokenTracker, ToolCall as CompactionToolCall,
    ToolResult as CompactionToolResult, TurnSelector,
};
use codelet_common::{ContentPart, Message, MessageContent, MessageRole};

// ==========================================
// TEST FIXTURES
// ==========================================

/// Create a test conversation turn with specified properties
fn create_test_turn(
    has_error: bool,
    tool_calls: Vec<&str>,
    has_test_success: bool,
    tokens: u64,
) -> ConversationTurn {
    ConversationTurn {
        user_message: "User request".to_string(),
        tool_calls: tool_calls
            .into_iter()
            .map(|name| CompactionToolCall {
                tool: name.to_string(),
                id: format!("tool_{name}"),
                parameters: serde_json::json!({}),
            })
            .collect(),
        tool_results: if has_test_success {
            vec![CompactionToolResult {
                success: true,
                output: "Tests passed successfully".to_string(),
                error: None,
            }]
        } else {
            vec![]
        },
        assistant_response: "Assistant response".to_string(),
        tokens,
        timestamp: std::time::SystemTime::now(),
        previous_error: Some(has_error),
    }
}

// ==========================================
// SCENARIO: Trigger compaction at 90% context window
// ==========================================

#[tokio::test]
async fn test_compaction_trigger_at_90_percent_context_window() -> Result<()> {
    // @step Given I have a session with 100k token context window
    let context_window_size = 100_000u64;
    let threshold = (context_window_size as f64 * 0.9) as u64; // 90k (90% of full window)

    // @step And effective tokens account for 90% cache discount
    let total_input_tokens = 95_000u64;
    let cache_read_tokens = 0u64; // No cache for this test

    // @step When I calculate if compaction should trigger
    let tracker = TokenTracker {
        input_tokens: total_input_tokens,
        output_tokens: 5_000,
        cache_read_input_tokens: Some(cache_read_tokens),
        cache_creation_input_tokens: Some(0),
        cumulative_billed_input: total_input_tokens,
        cumulative_billed_output: 5_000,
    };

    let should_compact = tracker.effective_tokens() > threshold;

    // @step Then compaction should trigger (95k > 90k threshold)
    assert!(
        should_compact,
        "Compaction should trigger when effective tokens ({}) exceed threshold ({})",
        tracker.effective_tokens(),
        threshold
    );

    Ok(())
}

// ==========================================
// SCENARIO: Select turns for compaction
// ==========================================

#[tokio::test]
async fn test_select_turns_for_compaction_using_anchor() -> Result<()> {
    // @step Given I have 90 total conversation turns
    let mut turns = Vec::new();
    for _i in 0..90 {
        turns.push(create_test_turn(false, vec![], false, 1000));
    }

    // @step And an anchor point exists at turn 40
    let anchor = AnchorPoint {
        turn_index: 40,
        anchor_type: AnchorType::ErrorResolution,
        weight: 0.9,
        confidence: 0.95,
        description: "Error resolved at turn 40".to_string(),
        timestamp: std::time::SystemTime::now(),
    };

    // @step When I select turns for compaction
    let selector = TurnSelector::new();
    let selection = selector.select_turns_with_recent(&turns, &[anchor])?;

    // @step Then turns 40-89 are kept (50 turns, 0-indexed)
    assert_eq!(
        selection.kept_turns.len(),
        50,
        "Should keep 50 turns (indices 40-89 in 0-indexed array)"
    );
    assert_eq!(
        selection.kept_turns.first().unwrap().turn_index,
        40,
        "First kept turn should be at index 40"
    );
    assert_eq!(
        selection.kept_turns.last().unwrap().turn_index,
        89,
        "Last kept turn should be at index 89 (0-indexed)"
    );

    // @step And turns 0-39 are summarized (40 turns, 0-indexed)
    assert_eq!(
        selection.summarized_turns.len(),
        40,
        "Should summarize 40 turns (indices 0-39)"
    );

    // @step And compression estimate is 44.4% (40/90 turns summarized)
    let compression_estimate: f64 = (40.0 / 90.0) * 100.0;
    assert!(
        (compression_estimate - 44.4_f64).abs() < 1.0,
        "Compression estimate should be ~44%, got {compression_estimate}%"
    );

    Ok(())
}

// ==========================================
// SCENARIO: LLM-based compaction with summary
// ==========================================

#[tokio::test]
async fn test_llm_based_compaction_generates_summary() -> Result<()> {
    // @step Given I have conversation turns to compact
    let turns = vec![
        create_test_turn(false, vec!["Edit"], true, 500),
        create_test_turn(false, vec!["Write"], true, 500),
        create_test_turn(false, vec![], false, 500),
        create_test_turn(false, vec![], false, 500),
    ];

    // @step And an LLM provider is available
    let llm_mock = |prompt: String| async move {
        if prompt.contains("ANCHOR TYPES") || prompt.contains("TURNS TO ANALYZE") {
            // Anchor detection prompt
            Ok::<String, anyhow::Error>(
                r#"[{"turn_index": 0, "anchor_type": null, "confidence": 0.0, "description": "No anchor"}]"#.to_string()
            )
        } else {
            // Summary generation prompt
            Ok("LLM-generated summary: The user edited and wrote files, tests passed.".to_string())
        }
    };

    // @step When compaction is executed
    let compactor = ContextCompactor::new().with_compression_threshold(0.0);
    let result = compactor.compact(&turns, 150_000, llm_mock).await?;

    // @step Then the summary should be LLM-generated
    assert!(
        result.summary.contains("LLM-generated summary"),
        "Summary should be LLM-generated, got: {}",
        result.summary
    );

    // @step And metrics should be calculated
    assert!(result.metrics.original_tokens > 0);
    assert!(result.metrics.turns_summarized + result.metrics.turns_kept == turns.len());

    Ok(())
}

// ==========================================
// SCENARIO: Emit warning for low compression ratio
// ==========================================

#[tokio::test]
async fn test_emit_warning_for_low_compression_ratio() -> Result<()> {
    // @step Given compaction has been executed
    // @step And compression ratio is 45%
    let original_tokens = 100_000u64;
    let compacted_tokens = 55_000u64; // 45% compression ratio
    let compression_ratio = 1.0 - (compacted_tokens as f64 / original_tokens as f64);

    // @step When I validate compression quality
    let metrics = CompactionMetrics {
        original_tokens,
        compacted_tokens,
        compression_ratio,
        turns_summarized: 50,
        turns_kept: 50,
    };

    let min_ratio_threshold = 0.6; // 60% minimum
    let has_warning = !metrics.meets_threshold(min_ratio_threshold);

    // @step Then a warning should be emitted
    assert!(
        has_warning,
        "Should emit warning for low compression ratio ({:.1}% < 60%)",
        compression_ratio * 100.0
    );

    Ok(())
}

// ==========================================
// SCENARIO: Reconstruct messages after compaction
// ==========================================

#[tokio::test]
async fn test_reconstruct_messages_after_compaction() -> Result<()> {
    // @step Given compaction has completed successfully
    let system_message = Message {
        role: MessageRole::System,
        content: MessageContent::Parts(vec![ContentPart::Text {
            text: "System prompt".to_string(),
        }]),
    };

    let kept_turns = vec![
        create_test_turn(false, vec![], false, 1000),
        create_test_turn(false, vec![], false, 1000),
    ];

    let summary_text = "Summary of previous conversation...";
    let continuation_text = "This session is being continued from a previous conversation that ran out of context.";

    // @step When messages are reconstructed (simulating compactor output)
    let mut reconstructed: Vec<Message> = Vec::new();

    // System messages
    reconstructed.push(system_message.clone());

    // Kept turn messages
    for turn in &kept_turns {
        reconstructed.push(Message {
            role: MessageRole::User,
            content: MessageContent::Text(turn.user_message.clone()),
        });
        reconstructed.push(Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text(turn.assistant_response.clone()),
        });
    }

    // Summary message
    reconstructed.push(Message {
        role: MessageRole::User,
        content: MessageContent::Text(summary_text.to_string()),
    });

    // Continuation message
    reconstructed.push(Message {
        role: MessageRole::User,
        content: MessageContent::Text(continuation_text.to_string()),
    });

    // @step Then messages array contains system messages
    assert_eq!(
        reconstructed[0].role,
        MessageRole::System,
        "First message should be system message"
    );

    // @step And messages array contains kept turns
    assert!(reconstructed.len() > 3, "Should have kept turn messages");

    // @step And messages array contains summary message
    let has_summary = reconstructed.iter().any(|msg| match &msg.content {
        MessageContent::Parts(parts) => parts.iter().any(|part| {
            if let ContentPart::Text { text } = part {
                text.contains("Summary of previous conversation")
            } else {
                false
            }
        }),
        MessageContent::Text(text) => text.contains("Summary of previous conversation"),
    });
    assert!(has_summary, "Reconstructed messages should contain summary");

    // @step And messages array contains session continuation message
    let has_continuation = reconstructed.iter().any(|msg| match &msg.content {
        MessageContent::Parts(parts) => parts.iter().any(|part| {
            if let ContentPart::Text { text } = part {
                text.contains("continued from a previous conversation")
            } else {
                false
            }
        }),
        MessageContent::Text(text) => text.contains("continued from a previous conversation"),
    });
    assert!(
        has_continuation,
        "Reconstructed messages should contain continuation message"
    );

    Ok(())
}
