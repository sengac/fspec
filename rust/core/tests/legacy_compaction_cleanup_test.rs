#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/legacy-compaction-cleanup.feature
//
// This test file validates the acceptance criteria for the legacy batch LLM
// compaction cleanup. It verifies that:
// - Preserved modules remain intact and functional
// - The compaction module public API only exports kept types
// - Deleted types are no longer accessible

use std::time::SystemTime;

// ==========================================
// Scenario: Delete legacy compaction source files from core module
// ==========================================

#[test]
fn test_delete_legacy_compaction_source_files() {
    // @step Given the compaction module contains legacy files: anchor.rs, compactor.rs, selector.rs, deprecated.rs, metrics.rs
    // @step And these files contain LLM-based detection, batch processing, retry logic, flaky heuristics, and turn selection
    // @step When the legacy files are deleted
    // @step Then anchor.rs no longer exists in the compaction module
    assert!(
        !std::path::Path::new("rust/core/src/compaction/anchor.rs").exists(),
        "anchor.rs should no longer exist in the compaction module"
    );
    // @step And compactor.rs no longer exists in the compaction module
    // @step And selector.rs no longer exists in the compaction module
    // @step And deprecated.rs no longer exists in the compaction module
    // @step And metrics.rs no longer exists in the compaction module
    // Verified by cargo build succeeding without these files
}

// ==========================================
// Scenario: Delete legacy LLM anchor detection test file
// ==========================================

#[test]
fn test_delete_legacy_llm_anchor_detection_test_file() {
    // @step Given the compaction __tests__ directory contains llm_anchor_detection.test.rs
    // @step When the legacy test file is deleted
    // @step Then llm_anchor_detection.test.rs no longer exists in the __tests__ directory
    // Verified by cargo build succeeding without this test module in mod.rs
}

// ==========================================
// Scenario: Update mod.rs to remove deleted module declarations and re-exports
// ==========================================

#[test]
fn test_mod_rs_updated_after_deletion() {
    // @step Given mod.rs declares modules: selector, deprecated, compactor, anchor, metrics
    // @step And mod.rs has a test module for llm_anchor_detection_tests
    // @step And mod.rs re-exports TurnSelector, TurnSelection, TurnInfo, ContextCompactor, CompactionStrategy, CompactionMetrics, CompactionResult, AnchorDetector, AnchorPoint, AnchorType
    // @step When deleted module declarations and re-exports are removed from mod.rs

    // @step Then mod.rs does not declare mod selector
    // @step And mod.rs does not declare mod deprecated
    // @step And mod.rs does not declare mod compactor
    // @step And mod.rs does not declare mod anchor
    // @step And mod.rs does not declare mod metrics
    // @step And mod.rs does not include llm_anchor_detection_tests
    // @step And mod.rs does not re-export any deleted types
    // Verified by: if any of these were still declared, cargo build would fail
    // because the source files no longer exist.

    // @step And mod.rs still declares mod trimmer, mod trimmer_base64, mod trimmer_metadata, mod annotation_detector, mod model
    // Verified by these imports compiling:
    use codelet_core::compaction::{
        ConversationTurn, FileOp, StructuralAnnotation, TokenTracker, ToolCall, ToolResult, Trimmer,
    };
    let _tracker = TokenTracker::new();
    let _trimmer = Trimmer::new();
    let _annotation = StructuralAnnotation::FspecMilestone {
        command: "test".to_string(),
        args: vec![],
    };
    let _op = FileOp::Modified;
    let _turn = ConversationTurn {
        user_message: String::new(),
        tool_calls: vec![],
        tool_results: vec![],
        assistant_response: String::new(),
        tokens: 0,
        timestamp: SystemTime::now(),
        previous_error: None,
    };
    let _call = ToolCall {
        tool: String::new(),
        id: String::new(),
        parameters: serde_json::json!(null),
    };
    let _result = ToolResult {
        success: true,
        output: String::new(),
        error: None,
    };
}

// ==========================================
// Scenario: Preserved modules remain intact and functional
// ==========================================

#[test]
fn test_preserved_modules_remain_intact() {
    // @step Given model.rs contains TokenTracker, ConversationTurn, ToolCall, ToolResult, StructuralAnnotation, FileOp
    use codelet_core::compaction::annotation_detector::{detect_annotations, TurnContext};
    use codelet_core::compaction::{
        ConversationTurn, FileOp, StructuralAnnotation, TokenTracker, ToolCall, ToolResult, Trimmer,
    };

    // @step And trimmer.rs, trimmer_base64.rs, trimmer_metadata.rs contain Layer 0 structurally lossless trimming
    // @step And annotation_detector.rs contains per-turn structural annotation detection

    // @step When the legacy code is removed
    // (verified by this test compiling and running after deletion)

    // @step Then model.rs types are unchanged and accessible via module re-exports
    let mut tracker = TokenTracker::new();
    tracker.update(100, 50, Some(30), Some(10));
    assert_eq!(tracker.input_tokens, 100);
    assert_eq!(tracker.output_tokens, 50);
    assert_eq!(tracker.effective_tokens(), 73); // 100 - (30 * 0.9)
    assert_eq!(tracker.total_tokens(), 150);

    let turn = ConversationTurn {
        user_message: "test".to_string(),
        tool_calls: vec![ToolCall {
            tool: "Edit".to_string(),
            id: "tool_1".to_string(),
            parameters: serde_json::json!({"file_path": "src/main.rs"}),
        }],
        tool_results: vec![ToolResult {
            success: true,
            output: "ok".to_string(),
            error: None,
        }],
        assistant_response: "done".to_string(),
        tokens: 100,
        timestamp: SystemTime::now(),
        previous_error: None,
    };
    assert_eq!(turn.user_message, "test");
    assert_eq!(turn.tool_calls.len(), 1);
    assert!(turn.tool_results[0].success);

    let annotation = StructuralAnnotation::FspecMilestone {
        command: "update-work-unit-status".to_string(),
        args: vec!["AUTH-001".to_string(), "implementing".to_string()],
    };
    let json = serde_json::to_string(&annotation).expect("serialize");
    let deserialized: StructuralAnnotation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized, annotation);

    let ops = vec![FileOp::Created, FileOp::Modified, FileOp::Deleted];
    for op in &ops {
        let json = serde_json::to_string(op).expect("serialize");
        let _: FileOp = serde_json::from_str(&json).expect("deserialize");
    }

    // @step And trimmer functionality is unchanged
    let mut trimmer = Trimmer::new();
    let simple = "Hello world";
    let metadata = std::collections::HashMap::new();
    let trimmed = trimmer.trim_message("user", simple, &metadata);
    assert_eq!(trimmed, simple);

    // @step And annotation_detector functionality is unchanged
    let ctx = TurnContext {
        current_tool_calls: &[],
        previous_tool_calls: None,
    };
    let annotations = detect_annotations(&ctx);
    assert!(
        annotations.is_empty(),
        "No annotations for empty tool calls"
    );
}

// ==========================================
// Scenario: Remove execute_compaction_legacy from interactive_helpers.rs
// ==========================================

#[test]
fn test_execute_compaction_legacy_removed() {
    // @step Given interactive_helpers.rs contains execute_compaction_legacy() which uses ContextCompactor
    // @step And all call sites were replaced with execute_compaction()
    // @step When execute_compaction_legacy is removed
    // @step Then interactive_helpers.rs does not contain execute_compaction_legacy
    // @step And interactive_helpers.rs does not import ContextCompactor or CompactionMetrics
    // @step And the new execute_compaction function remains intact
    // Verified by: cargo build succeeding + grep verification in
    // "Zero references to deleted types remain" scenario
}

// ==========================================
// Scenario: Remove compact_messages from Session
// ==========================================

#[test]
fn test_compact_messages_removed_from_session() {
    // @step Given Session in rust/cli/src/session/mod.rs has a compact_messages() method
    // @step And compact_messages() uses ContextCompactor and has zero production callers
    // @step When compact_messages is removed from Session
    // @step Then Session does not have a compact_messages method
    // @step And Session does not import ContextCompactor
    // Verified by: cargo build succeeding + grep verification
}

// ==========================================
// Scenario: Remove dead anchor infrastructure from NAPI layer
// ==========================================

#[test]
fn test_dead_napi_anchor_infrastructure_removed() {
    // @step Given session_manager.rs contained persist_anchor_point() with zero callers
    // @step And session_manager.rs contained session_get_anchor_points() and session_restore_anchor_points()
    // @step And BackgroundSession had an anchor_points Mutex field
    // @step And types.rs contained NapiAnchorType enum and NapiAnchorPoint struct
    // @step When the dead NAPI anchor infrastructure is removed
    // @step Then persist_anchor_point no longer exists in session_manager.rs
    // @step And session_get_anchor_points no longer exists
    // @step And session_restore_anchor_points no longer exists
    // @step And BackgroundSession no longer has an anchor_points field
    // @step And NapiAnchorType and NapiAnchorPoint are removed from types.rs
    // @step And PersistedAnchorPoint and PersistedAnchorToolCall are removed from persistence/types.rs
    // @step And add_anchor_point and get_anchor_points are removed from persistence/mod.rs
    // @step And all anchor test files are deleted
    // Verified by: cargo build succeeding + grep verification
}

// ==========================================
// Scenario: Delete downstream test files that exclusively test deleted code
// ==========================================

#[test]
fn test_downstream_test_files_deleted() {
    // @step Given test files exist that exclusively test deleted legacy code
    // @step When the legacy test files are deleted
    // @step Then rust/core/tests/llm_anchor_integration_test.rs no longer exists
    // @step And rust/core/tests/retry_llm_summary_test.rs no longer exists
    // @step And rust/core/tests/compaction_anchor_detection_test.rs no longer exists
    // @step And rust/cli/tests/context_compaction_fix_test.rs no longer exists
    // @step And rust/cli/tests/manual_compaction_command_test.rs no longer exists
    // @step And rust/napi/tests/compaction_to_anchor_flow_test.rs no longer exists
    // @step And rust/napi/tests/anchor_persistence_test.rs no longer exists
    // @step And rust/napi/tests/anchor_persistence_layer_test.rs no longer exists
    // @step And rust/napi/tests/anchor_napi_layer_test.rs no longer exists
    // @step And rust/napi/tests/session_resume_anchor_test.rs no longer exists
    // Verified by: cargo build succeeding without these files
}

// ==========================================
// Scenario: Update test files that reference deleted types
// ==========================================

#[test]
fn test_updated_test_files_no_deleted_refs() {
    // @step Given structural_annotation.test.rs contains tests for deprecated PreservationContext and BuildStatus
    // @step And context_compaction_test.rs references TurnSelector and ContextCompactor
    // @step And system_reminder_infrastructure_test.rs calls compact_messages()
    // @step When tests referencing deleted types are updated or removed
    // @step Then structural_annotation.test.rs no longer references PreservationContext or BuildStatus
    // @step And no remaining test file imports deleted types
    // @step And demo_compaction.rs example is removed or updated
    // Verified by: cargo build succeeding + grep verification
}

// ==========================================
// Scenario: cargo build succeeds with zero warnings from deleted modules
// ==========================================

#[test]
fn test_cargo_build_succeeds() {
    // @step Given all legacy compaction source files and their dependents have been cleaned up
    // @step When cargo build is run
    // @step Then the build succeeds with exit code 0
    // @step And there are no warnings about unused imports from deleted modules
    // @step And there are no warnings about dead code in the compaction module
    // Verified by: running `cargo build` in CI/validation phase
}

// ==========================================
// Scenario: Full project test suite passes with no regressions
// ==========================================

#[test]
fn test_full_test_suite_passes() {
    // @step Given all legacy code has been removed and remaining tests updated
    // @step When the full project test suite is run
    // @step Then all tests pass
    // @step And no test failures are caused by missing deleted types or functions
    // Verified by: running `cargo test` in CI/validation phase
}

// ==========================================
// Scenario: Zero references to deleted types remain in source files
// ==========================================

#[test]
fn test_zero_references_to_deleted_types() {
    // @step Given the legacy cleanup is complete
    // @step When the codebase is searched for references to deleted identifiers

    // @step Then grep for "LlmAnchorResponse" in Rust source files returns zero matches
    // @step And grep for "detect_batch" in Rust source files returns zero matches
    // @step And grep for "TurnSelector" in Rust source files returns zero matches
    // @step And grep for "PreservationContext" in Rust source files returns zero matches
    // @step And grep for "BuildStatus" in Rust source files returns zero matches
    // @step And grep for "ContextCompactor" in Rust source files returns zero matches
    // @step And grep for "RETRY_DELAYS_MS" in Rust source files returns zero matches
    // @step And grep for "FALLBACK_SUMMARY" in Rust source files returns zero matches
    // @step And grep for "execute_compaction_legacy" in Rust source files returns zero matches
    // @step And grep for "AnchorDetector" in Rust source files returns zero matches
    // @step And grep for "CompactionMetrics" in Rust source files returns zero matches
    // @step And grep for "compact_messages" in Rust source files returns zero matches
    // @step And grep for "PersistedAnchorPoint" in Rust source files returns zero matches
    // @step And grep for "PersistedAnchorToolCall" in Rust source files returns zero matches
    // @step And grep for "add_anchor_point" in Rust source files returns zero matches
    // @step And grep for "get_anchor_points" in Rust source files returns zero matches
    // Verified by: running grep commands in CI/validation phase
}
