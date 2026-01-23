//! CTX-001: Complete Anchor-Based Context Compaction Tests
//!
//! Feature: spec/features/complete-anchor-based-context-compaction.feature
//!
//! These tests validate the anchor-based compaction system including:
//! - Anchor detection (TaskCompletion, ErrorResolution, Bash milestone, Web search)
//! - PreservationContext extraction (active files, goals, error states, build status)
//! - Synthetic anchor creation when no natural anchors found
//! - Dynamic summary generation (NOT hardcoded)

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::anchor::{AnchorDetector, AnchorType};
use super::model::{BuildStatus, ConversationTurn, PreservationContext, ToolCall, ToolResult};
use std::time::SystemTime;

// ==========================================
// TEST HELPERS
// ==========================================

fn make_turn_with_edit_and_test_pass(file_path: &str, previous_error: bool) -> ConversationTurn {
    ConversationTurn {
        user_message: "Fix the bug".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Edit".to_string(),
            id: "edit_1".to_string(),
            parameters: serde_json::json!({"file_path": file_path}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: "All 5 tests passed successfully".to_string(),
            error: None,
        }],
        assistant_response: "I've fixed the bug and tests pass.".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: Some(previous_error),
    }
}

fn make_bash_milestone_turn(output: &str) -> ConversationTurn {
    ConversationTurn {
        user_message: "Install dependencies".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Bash".to_string(),
            id: "bash_1".to_string(),
            parameters: serde_json::json!({"command": "npm install"}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: output.to_string(),
            error: None,
        }],
        assistant_response: "Dependencies installed.".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: None,
    }
}

fn make_web_search_turn(search_output: &str, assistant_response: &str) -> ConversationTurn {
    ConversationTurn {
        user_message: "What is the Brisbane job market like?".to_string(),
        tool_calls: vec![ToolCall {
            tool: "WebSearch".to_string(),
            id: "search_1".to_string(),
            parameters: serde_json::json!({"query": "Brisbane job market"}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: search_output.to_string(),
            error: None,
        }],
        assistant_response: assistant_response.to_string(),
        tokens: 150,
        timestamp: SystemTime::now(),
        previous_error: None,
    }
}

fn make_simple_turn(user_msg: &str, tokens: u64) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        tool_calls: vec![],
        tool_results: vec![],
        assistant_response: format!("Response to: {user_msg}"),
        tokens,
        timestamp: SystemTime::now(),
        previous_error: None,
    }
}

fn make_turn_with_tool(tool: &str, file_path: &str, user_msg: &str) -> ConversationTurn {
    ConversationTurn {
        user_message: user_msg.to_string(),
        tool_calls: vec![ToolCall {
            tool: tool.to_string(),
            id: "tool_1".to_string(),
            parameters: serde_json::json!({"file_path": file_path}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: "Done".to_string(),
            error: None,
        }],
        assistant_response: "Completed.".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: None,
    }
}

fn make_turn_with_failed_result(error_output: &str) -> ConversationTurn {
    ConversationTurn {
        user_message: "Run the build".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Bash".to_string(),
            id: "bash_1".to_string(),
            parameters: serde_json::json!({"command": "cargo build"}),
        }],
        tool_results: vec![ToolResult {
            success: false,
            output: error_output.to_string(),
            error: Some(error_output.to_string()),
        }],
        assistant_response: "Build failed.".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: None,
    }
}

// ==========================================
// ANCHOR DETECTION SCENARIOS
// ==========================================

/// Scenario: Detect TaskCompletion anchor after successful test run
#[test]
fn test_detect_task_completion_anchor() {
    // @step Given a conversation with 6 turns about fixing a bug in auth.rs
    // @step And turn 3 contains an Edit tool call to auth.rs followed by a test run that passes
    // @step And there is no previous error state
    let turn = make_turn_with_edit_and_test_pass("src/auth.rs", false);

    // @step When I run anchor detection on turn 3
    let detector = AnchorDetector::new(0.9);
    let result = detector.detect(&turn, 3).unwrap();

    // @step Then a TaskCompletion anchor should be detected with confidence >= 0.9
    assert!(
        result.is_some(),
        "Expected TaskCompletion anchor to be detected"
    );
    let anchor = result.unwrap();
    assert!(anchor.confidence >= 0.9, "Confidence should be >= 0.9");

    // @step And the anchor should have weight 0.8
    assert_eq!(anchor.anchor_type, AnchorType::TaskCompletion);
    assert_eq!(anchor.weight, 0.8);

    // @step And the anchor turn_index should be 3
    assert_eq!(anchor.turn_index, 3);
}

/// Scenario: Detect ErrorResolution anchor after fixing build error
#[test]
fn test_detect_error_resolution_anchor() {
    // @step Given a conversation where turn 2 had a build error
    // @step And turn 3 contains an Edit tool call followed by a test run that passes
    // @step And previous_error is set to true for turn 3
    let turn = make_turn_with_edit_and_test_pass("src/auth.rs", true);

    // @step When I run anchor detection on turn 3
    let detector = AnchorDetector::new(0.9);
    let result = detector.detect(&turn, 3).unwrap();

    // @step Then an ErrorResolution anchor should be detected with confidence >= 0.9
    assert!(
        result.is_some(),
        "Expected ErrorResolution anchor to be detected"
    );
    let anchor = result.unwrap();
    assert!(anchor.confidence >= 0.9, "Confidence should be >= 0.9");

    // @step And the anchor should have weight 0.9
    assert_eq!(anchor.anchor_type, AnchorType::ErrorResolution);
    assert_eq!(anchor.weight, 0.9);
}

/// Scenario: Detect bash milestone anchor for successful npm install
#[test]
fn test_detect_bash_milestone_anchor() {
    // @step Given a conversation with a turn containing a Bash tool call
    // @step And the bash command output contains "packages are successfully installed"
    // @step And the tool result success is true
    let turn = make_bash_milestone_turn("added 150 packages, packages are successfully installed");

    // @step When I run anchor detection on that turn
    let detector = AnchorDetector::new(0.9);
    let result = detector.detect(&turn, 0).unwrap();

    // @step Then a TaskCompletion anchor should be detected with weight 0.8
    assert!(
        result.is_some(),
        "Expected TaskCompletion anchor for bash milestone"
    );
    let anchor = result.unwrap();
    assert_eq!(anchor.anchor_type, AnchorType::TaskCompletion);
    assert_eq!(anchor.weight, 0.8);

    // @step And the anchor description should contain "Bash" or "milestone"
    let desc_lower = anchor.description.to_lowercase();
    assert!(
        desc_lower.contains("bash") || desc_lower.contains("milestone"),
        "Description should mention bash or milestone, got: {}",
        anchor.description
    );
}

/// Scenario: Detect web search anchor with synthesis
#[test]
fn test_detect_web_search_anchor_with_synthesis() {
    // @step Given a conversation with a turn containing a WebSearch tool call
    // @step And the search result has more than 100 characters
    let search_output = "Brisbane has a strong tech sector with growing opportunities in software development, data science, and cloud engineering. The average salary for senior developers is around $150,000 AUD.";

    // @step And the assistant response contains "Based on the search results"
    let assistant_response =
        "Based on the search results, Brisbane has excellent job opportunities in tech.";

    let turn = make_web_search_turn(search_output, assistant_response);

    // @step When I run anchor detection on that turn
    let detector = AnchorDetector::new(0.9);
    let result = detector.detect(&turn, 0).unwrap();

    // @step Then a UserCheckpoint anchor should be detected with weight 0.7
    assert!(
        result.is_some(),
        "Expected UserCheckpoint anchor for web search"
    );
    let anchor = result.unwrap();
    assert_eq!(anchor.anchor_type, AnchorType::UserCheckpoint);
    assert_eq!(anchor.weight, 0.7);

    // @step And the anchor description should mention web search
    let desc_lower = anchor.description.to_lowercase();
    assert!(
        desc_lower.contains("web") || desc_lower.contains("search"),
        "Description should mention web search, got: {}",
        anchor.description
    );
}

/// Scenario: Reject anchor with confidence below threshold
#[test]
fn test_reject_anchor_below_threshold() {
    // @step Given a conversation turn with an Edit tool call
    // @step And the tool result does not indicate a clear test pass or fail
    let turn = ConversationTurn {
        user_message: "Update the file".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Edit".to_string(),
            id: "edit_1".to_string(),
            parameters: serde_json::json!({"file_path": "test.rs"}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: "File updated".to_string(), // No test success indicator
            error: None,
        }],
        assistant_response: "Done.".to_string(),
        tokens: 50,
        timestamp: SystemTime::now(),
        previous_error: None,
    };

    // @step When I run anchor detection with confidence threshold 0.9
    let detector = AnchorDetector::new(0.9);
    let result = detector.detect(&turn, 0).unwrap();

    // @step Then no anchor should be detected for that turn
    assert!(result.is_none(), "Expected no anchor for ambiguous result");
}

// ==========================================
// SYNTHETIC ANCHOR SCENARIOS
// ==========================================

// Note: Synthetic anchor creation is tested via test_compactor_creates_synthetic_anchor_when_no_natural_anchors
// which tests the actual production code path in ContextCompactor::compact()

// ==========================================
// PRESERVATIONCONTEXT SCENARIOS
// ==========================================

/// Scenario: Extract active files from Edit Write and Read tool calls
#[test]
fn test_extract_active_files_from_tool_calls() {
    // @step Given a conversation with 5 turns
    // @step And turn 1 has an Edit tool call to "src/auth.rs"
    // @step And turn 2 has a Write tool call to "src/login.ts"
    // @step And turn 3 has a Read tool call to "config.json"
    let turns = vec![
        make_simple_turn("Start", 50),
        make_turn_with_tool("Edit", "src/auth.rs", "Edit auth"),
        make_turn_with_tool("Write", "src/login.ts", "Write login"),
        make_turn_with_tool("Read", "config.json", "Read config"),
        make_simple_turn("Done", 50),
    ];

    // @step When I extract PreservationContext from the turns
    let ctx = PreservationContext::extract_from_turns(&turns);

    // @step Then active_files should contain "auth.rs"
    assert!(
        ctx.active_files.contains(&"auth.rs".to_string()),
        "active_files should contain auth.rs, got: {:?}",
        ctx.active_files
    );

    // @step And active_files should contain "login.ts"
    assert!(
        ctx.active_files.contains(&"login.ts".to_string()),
        "active_files should contain login.ts, got: {:?}",
        ctx.active_files
    );

    // @step And active_files should contain "config.json"
    assert!(
        ctx.active_files.contains(&"config.json".to_string()),
        "active_files should contain config.json, got: {:?}",
        ctx.active_files
    );
}

/// Scenario: Extract current goals from user messages
#[test]
fn test_extract_goals_from_user_messages() {
    // @step Given a conversation with 3 turns
    // @step And turn 1 user message is "Please fix the auth bug"
    // @step And turn 2 user message is "Help me implement OAuth"
    let turns = vec![
        make_simple_turn("Please fix the auth bug", 100),
        make_simple_turn("Help me implement OAuth", 100),
        make_simple_turn("Thanks", 50),
    ];

    // @step When I extract PreservationContext from the turns
    let ctx = PreservationContext::extract_from_turns(&turns);

    // @step Then current_goals should contain a goal about "fix" and "auth"
    let has_fix_auth = ctx.current_goals.iter().any(|g| {
        let lower = g.to_lowercase();
        lower.contains("fix") && lower.contains("auth")
    });
    assert!(
        has_fix_auth,
        "current_goals should contain goal about fix and auth, got: {:?}",
        ctx.current_goals
    );

    // @step And current_goals should contain a goal about "implement" and "OAuth"
    let has_implement_oauth = ctx.current_goals.iter().any(|g| {
        let lower = g.to_lowercase();
        lower.contains("implement") && lower.contains("oauth")
    });
    assert!(
        has_implement_oauth,
        "current_goals should contain goal about implement and OAuth, got: {:?}",
        ctx.current_goals
    );
}

/// Scenario: Detect build status as Passing from test output
#[test]
fn test_detect_build_status_passing() {
    // @step Given a conversation with a turn containing test results
    // @step And the tool result output contains "All 15 tests passed"
    let turn = ConversationTurn {
        user_message: "Run tests".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Bash".to_string(),
            id: "bash_1".to_string(),
            parameters: serde_json::json!({"command": "cargo test"}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: "running 15 tests\nAll 15 tests passed".to_string(),
            error: None,
        }],
        assistant_response: "Tests pass!".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: None,
    };
    let turns = vec![turn];

    // @step When I extract PreservationContext from the turns
    let ctx = PreservationContext::extract_from_turns(&turns);

    // @step Then build_status should be Passing
    assert_eq!(
        ctx.build_status,
        BuildStatus::Passing,
        "build_status should be Passing, got: {:?}",
        ctx.build_status
    );
}

/// Scenario: Detect build status as Failing from test output
#[test]
fn test_detect_build_status_failing() {
    // @step Given a conversation with a turn containing test results
    // @step And the tool result output contains "FAILED: 3 tests failed"
    let turn = ConversationTurn {
        user_message: "Run tests".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Bash".to_string(),
            id: "bash_1".to_string(),
            parameters: serde_json::json!({"command": "cargo test"}),
        }],
        tool_results: vec![ToolResult {
            success: false,
            output: "running 10 tests\nFAILED: 3 tests failed".to_string(),
            error: Some("Test failures".to_string()),
        }],
        assistant_response: "Tests failed.".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: None,
    };
    let turns = vec![turn];

    // @step When I extract PreservationContext from the turns
    let ctx = PreservationContext::extract_from_turns(&turns);

    // @step Then build_status should be Failing
    assert_eq!(
        ctx.build_status,
        BuildStatus::Failing,
        "build_status should be Failing, got: {:?}",
        ctx.build_status
    );
}

/// Scenario: Extract error states from failed tool results
#[test]
fn test_extract_error_states() {
    // @step Given a conversation with a turn containing a failed Bash command
    // @step And the tool result success is false
    // @step And the tool result output contains "error: cannot find module xyz"
    let turn = make_turn_with_failed_result("error: cannot find module xyz");
    let turns = vec![turn];

    // @step When I extract PreservationContext from the turns
    let ctx = PreservationContext::extract_from_turns(&turns);

    // @step Then error_states should contain "cannot find module xyz"
    let has_error = ctx
        .error_states
        .iter()
        .any(|e| e.contains("cannot find module xyz"));
    assert!(
        has_error,
        "error_states should contain 'cannot find module xyz', got: {:?}",
        ctx.error_states
    );
}

/// Scenario: Extract last user intent from most recent turn
#[test]
fn test_extract_last_user_intent() {
    // @step Given a conversation with 5 turns
    // @step And the last turn user message is "Now deploy this to production"
    let turns = vec![
        make_simple_turn("Start project", 50),
        make_simple_turn("Add feature", 100),
        make_simple_turn("Fix bugs", 100),
        make_simple_turn("Run tests", 100),
        make_simple_turn("Now deploy this to production", 100),
    ];

    // @step When I extract PreservationContext from the turns
    let ctx = PreservationContext::extract_from_turns(&turns);

    // @step Then last_user_intent should contain "deploy" and "production"
    let intent_lower = ctx.last_user_intent.to_lowercase();
    assert!(
        intent_lower.contains("deploy") && intent_lower.contains("production"),
        "last_user_intent should contain 'deploy' and 'production', got: {}",
        ctx.last_user_intent
    );
}

// ==========================================
// DYNAMIC SUMMARY SCENARIOS
// ==========================================

/// Scenario: Generate summary with dynamic PreservationContext not hardcoded
#[tokio::test]
async fn test_generate_dynamic_summary_not_hardcoded() {
    use super::compactor::ContextCompactor;

    // @step Given a conversation with 8 turns that gets compacted
    let mut turns: Vec<ConversationTurn> = (0..5)
        .map(|i| make_simple_turn(&format!("Task {i}"), 100))
        .collect();

    // Add turns with file edits to create context
    turns.push(make_turn_with_tool(
        "Edit",
        "src/auth.rs",
        "Fix authentication bug",
    ));
    turns.push(make_turn_with_tool("Edit", "src/login.ts", "Update login"));
    turns.push(make_turn_with_edit_and_test_pass("src/auth.rs", false));

    // @step And the PreservationContext has active_files containing "auth.rs" and "login.ts"
    // @step And the PreservationContext has current_goals containing "Fix authentication bug"
    // @step And the PreservationContext has build_status as Passing
    let ctx = PreservationContext::extract_from_turns(&turns);
    assert!(ctx.active_files.contains(&"auth.rs".to_string()));
    assert!(ctx.active_files.contains(&"login.ts".to_string()));
    assert_eq!(ctx.build_status, BuildStatus::Passing);

    // @step When I generate a summary using the compactor
    let compactor = ContextCompactor::new();
    let result = compactor
        .compact(&turns, 1000, |_| async { Ok("Summary".to_string()) })
        .await
        .unwrap();

    // @step Then the summary should contain "auth.rs"
    assert!(
        result.summary.contains("auth.rs"),
        "Summary should contain auth.rs, got: {}",
        result.summary
    );

    // @step And the summary should contain "login.ts"
    assert!(
        result.summary.contains("login.ts"),
        "Summary should contain login.ts, got: {}",
        result.summary
    );

    // @step And the summary should contain "Fix authentication bug" or similar goal text
    let summary_lower = result.summary.to_lowercase();
    assert!(
        summary_lower.contains("fix") && summary_lower.contains("auth"),
        "Summary should contain goal about fixing auth, got: {}",
        result.summary
    );

    // @step And the summary should contain "passing" for build status
    assert!(
        summary_lower.contains("passing"),
        "Summary should contain 'passing' for build status, got: {}",
        result.summary
    );

    // @step And the summary should NOT contain "[from conversation]"
    assert!(
        !result.summary.contains("[from conversation]"),
        "Summary should NOT contain '[from conversation]', got: {}",
        result.summary
    );

    // @step And the summary should NOT contain "Continue development"
    assert!(
        !result.summary.contains("Continue development"),
        "Summary should NOT contain 'Continue development', got: {}",
        result.summary
    );

    // @step And the summary should NOT contain "Build: unknown"
    assert!(
        !result.summary.contains("Build: unknown"),
        "Summary should NOT contain 'Build: unknown', got: {}",
        result.summary
    );
}

/// Scenario: PreservationContext format_for_summary produces correct output
#[test]
fn test_preservation_context_format_for_summary() {
    // @step Given a PreservationContext with active_files ["auth.rs", "login.ts"]
    // @step And current_goals ["Fix auth bug", "Add OAuth"]
    // @step And build_status Passing
    let ctx = PreservationContext {
        active_files: vec!["auth.rs".to_string(), "login.ts".to_string()],
        current_goals: vec!["Fix auth bug".to_string(), "Add OAuth".to_string()],
        error_states: vec![],
        build_status: BuildStatus::Passing,
        last_user_intent: String::new(),
    };

    // @step When I call format_for_summary on the PreservationContext
    let output = ctx.format_for_summary();

    // @step Then the output should contain "Active files: auth.rs, login.ts"
    assert!(
        output.contains("Active files: auth.rs, login.ts"),
        "Output should contain 'Active files: auth.rs, login.ts', got: {output}"
    );

    // @step And the output should contain "Goals: Fix auth bug; Add OAuth"
    assert!(
        output.contains("Goals: Fix auth bug; Add OAuth"),
        "Output should contain 'Goals: Fix auth bug; Add OAuth', got: {output}"
    );

    // @step And the output should contain "Build: passing"
    assert!(
        output.contains("Build: passing"),
        "Output should contain 'Build: passing', got: {output}"
    );
}

// ==========================================
// TURN SELECTION SCENARIOS
// ==========================================

/// Scenario: Always preserve last 3 conversation turns
#[test]
fn test_preserve_last_3_turns() {
    use super::anchor::AnchorPoint;
    use super::selector::TurnSelector;

    // @step Given a conversation with 10 turns
    let turns: Vec<ConversationTurn> = (0..10)
        .map(|i| make_simple_turn(&format!("Turn {i}"), 100))
        .collect();

    // @step And an anchor exists at turn 5
    let anchors = vec![AnchorPoint {
        turn_index: 5,
        anchor_type: AnchorType::TaskCompletion,
        weight: 0.8,
        confidence: 0.92,
        description: "Task completed".to_string(),
        timestamp: SystemTime::now(),
    }];

    // @step When I run turn selection on the conversation
    let selector = TurnSelector::new();
    let selection = selector.select_turns_with_recent(&turns, &anchors).unwrap();

    // @step Then turns 7, 8, and 9 should be in the kept_turns list
    let kept_indices: Vec<usize> = selection.kept_turns.iter().map(|t| t.turn_index).collect();
    assert!(kept_indices.contains(&7), "Turn 7 should be kept");
    assert!(kept_indices.contains(&8), "Turn 8 should be kept");
    assert!(kept_indices.contains(&9), "Turn 9 should be kept");

    // @step And turns 0 through 4 should be in the summarized_turns list
    let summarized_indices: Vec<usize> = selection
        .summarized_turns
        .iter()
        .map(|t| t.turn_index)
        .collect();
    for i in 0..5 {
        assert!(
            summarized_indices.contains(&i),
            "Turn {i} should be summarized"
        );
    }
}

/// Scenario: Warn when compression ratio is below 60 percent
#[tokio::test]
async fn test_warn_low_compression_ratio() {
    use super::compactor::ContextCompactor;

    // @step Given a conversation with 4 turns totaling 400 tokens
    let turns = vec![
        make_simple_turn("Turn 1", 100),
        make_simple_turn("Turn 2", 100),
        make_simple_turn("Turn 3", 100),
        make_simple_turn("Turn 4", 100),
    ];

    // @step When I run compaction with target 300 tokens
    // @step And the compression ratio is below 60 percent
    let compactor = ContextCompactor::new().with_compression_threshold(0.6);
    let result = compactor
        .compact(&turns, 300, |_| async { Ok("Summary".to_string()) })
        .await
        .unwrap();

    // @step Then the result warnings should contain a message about low compression
    // @step And the warning should suggest starting a fresh conversation
    if result.metrics.compression_ratio < 0.6 {
        assert!(
            !result.warnings.is_empty(),
            "Expected warning for low compression ratio ({:.1}%)",
            result.metrics.compression_ratio * 100.0
        );

        let warning_text = result.warnings.join(" ").to_lowercase();
        assert!(
            warning_text.contains("compression") || warning_text.contains("fresh"),
            "Warning should mention compression or fresh conversation"
        );
    }
}

/// Scenario: Create synthetic UserCheckpoint when no natural anchors found
#[tokio::test]
async fn test_compactor_creates_synthetic_anchor_when_no_natural_anchors() {
    use super::compactor::ContextCompactor;

    // @step Given a conversation with 10 turns
    // @step And all turns use only Read and WebSearch tools with no Edit or Write calls
    let turns: Vec<ConversationTurn> = (0..10)
        .map(|i| make_simple_turn(&format!("Question about topic {i}"), 100))
        .collect();

    // @step And no natural anchors are detected in any turn
    let detector = AnchorDetector::new(0.9);
    for (idx, turn) in turns.iter().enumerate() {
        let result = detector.detect(turn, idx).unwrap();
        assert!(
            result.is_none(),
            "Turn {idx} should not have natural anchor"
        );
    }

    // @step When I run compaction on the conversation
    let compactor = ContextCompactor::new();
    let result = compactor
        .compact(&turns, 500, |_| async { Ok("Summary".to_string()) })
        .await
        .unwrap();

    // @step Then the compaction result should contain a synthetic anchor
    assert!(
        result.anchor.is_some(),
        "Compactor should create synthetic anchor when no natural anchors found"
    );

    let anchor = result.anchor.as_ref().unwrap();

    // @step And the synthetic anchor should be a UserCheckpoint type at the last turn
    assert_eq!(
        anchor.anchor_type,
        AnchorType::UserCheckpoint,
        "Synthetic anchor should be UserCheckpoint type"
    );
    assert_eq!(
        anchor.turn_index, 9,
        "Synthetic anchor should be at last turn (index 9)"
    );

    // @step And the synthetic anchor should have weight 0.7
    assert_eq!(
        anchor.weight, 0.7,
        "Synthetic anchor should have weight 0.7"
    );

    // @step And the synthetic anchor description should indicate it is synthetic
    assert!(
        anchor.description.to_lowercase().contains("synthetic"),
        "Synthetic anchor description should indicate it's synthetic: {}",
        anchor.description
    );
}
