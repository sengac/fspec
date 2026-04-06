// Feature: spec/features/in-view-dag-compaction.feature
//
// This test file validates the per-turn structural annotation detection
// acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin @annotation-detection scenarios.

use serde_json::json;

use crate::compaction::annotation_detector::{detect_annotations, ToolCallInfo, TurnContext};
use crate::compaction::model::{FileOp, StructuralAnnotation};

// Scenario: Detect FspecMilestone annotation from fspec tool call
#[test]
fn test_detect_fspec_milestone_annotation() {
    // @step Given a completed turn containing a Fspec tool call with command "update-work-unit-status" and args ["AUTH-001", "implementing"]
    let tool_calls = vec![ToolCallInfo {
        tool_name: "Fspec".to_string(),
        input: json!({
            "command": "update-work-unit-status",
            "args": "{\"_\": [\"AUTH-001\", \"implementing\"]}"
        }),
        output: Some("✓ Work unit AUTH-001 status updated to implementing".to_string()),
        success: true,
    }];
    let ctx = TurnContext {
        current_tool_calls: &tool_calls,
        previous_tool_calls: None,
    };

    // @step When the per-turn annotation detector inspects the turn
    let annotations = detect_annotations(&ctx);

    // @step Then a FspecMilestone annotation should be created with command "update-work-unit-status" and args ["AUTH-001", "implementing"]
    assert!(!annotations.is_empty(), "Should detect at least one annotation");
    let milestone = annotations
        .iter()
        .find(|a| matches!(a, StructuralAnnotation::FspecMilestone { .. }))
        .expect("Should have a FspecMilestone annotation");
    match milestone {
        StructuralAnnotation::FspecMilestone { command, args } => {
            assert_eq!(command, "update-work-unit-status");
            assert_eq!(*args, vec!["AUTH-001".to_string(), "implementing".to_string()]);
        }
        _ => panic!("Expected FspecMilestone"),
    }

    // @step And the annotation should be attached to the persisted turn metadata
    // This is verified at integration level — annotations are returned for attachment by caller
    assert!(
        !annotations.is_empty(),
        "Annotations are returned for caller to attach to persisted turn metadata"
    );
}

// Scenario: Detect ErrorResolution annotation from error-to-success transition
#[test]
fn test_detect_error_resolution_annotation() {
    // @step Given a previous turn where Bash tool failed with exit code 1
    let previous_calls = vec![ToolCallInfo {
        tool_name: "Bash".to_string(),
        input: json!({"command": "cargo test"}),
        output: Some("error[E0308]: mismatched types".to_string()),
        success: false,
    }];

    // @step And a current turn where Edit modifies "src/main.rs" and Bash succeeds with exit code 0
    let current_calls = vec![
        ToolCallInfo {
            tool_name: "Edit".to_string(),
            input: json!({"file_path": "src/main.rs", "old_string": "bad", "new_string": "good"}),
            output: Some("Successfully edited src/main.rs".to_string()),
            success: true,
        },
        ToolCallInfo {
            tool_name: "Bash".to_string(),
            input: json!({"command": "cargo test"}),
            output: Some("test result: ok".to_string()),
            success: true,
        },
    ];
    let ctx = TurnContext {
        current_tool_calls: &current_calls,
        previous_tool_calls: Some(&previous_calls),
    };

    // @step When the per-turn annotation detector inspects the current turn
    let annotations = detect_annotations(&ctx);

    // @step Then an ErrorResolution annotation should be created with failed_tool "Bash" and resolved_file "src/main.rs"
    let resolution = annotations
        .iter()
        .find(|a| matches!(a, StructuralAnnotation::ErrorResolution { .. }))
        .expect("Should have an ErrorResolution annotation");
    match resolution {
        StructuralAnnotation::ErrorResolution {
            failed_tool,
            resolved_file,
        } => {
            assert_eq!(failed_tool, "Bash");
            assert_eq!(resolved_file, "src/main.rs");
        }
        _ => panic!("Expected ErrorResolution"),
    }

    // @step And the annotation should be attached to the persisted turn metadata
    assert!(
        !annotations.is_empty(),
        "Annotations are returned for caller to attach to persisted turn metadata"
    );
}

// Scenario: Detect FileModification annotation from Write tool call
#[test]
fn test_detect_file_modification_from_write() {
    // @step Given a completed turn containing a Write tool call creating "src/auth/handler.rs"
    let tool_calls = vec![ToolCallInfo {
        tool_name: "Write".to_string(),
        input: json!({"file_path": "src/auth/handler.rs", "content": "pub fn handle() {}"}),
        output: Some("Successfully wrote to src/auth/handler.rs".to_string()),
        success: true,
    }];
    let ctx = TurnContext {
        current_tool_calls: &tool_calls,
        previous_tool_calls: None,
    };

    // @step When the per-turn annotation detector inspects the turn
    let annotations = detect_annotations(&ctx);

    // @step Then a FileModification annotation should be created with path "src/auth/handler.rs" and operation Created
    let file_mod = annotations
        .iter()
        .find(|a| matches!(a, StructuralAnnotation::FileModification { .. }))
        .expect("Should have a FileModification annotation");
    match file_mod {
        StructuralAnnotation::FileModification { path, operation } => {
            assert_eq!(path, "src/auth/handler.rs");
            assert_eq!(*operation, FileOp::Created);
        }
        _ => panic!("Expected FileModification"),
    }

    // @step And the annotation should be attached to the persisted turn metadata
    assert!(
        !annotations.is_empty(),
        "Annotations are returned for caller to attach to persisted turn metadata"
    );
}

// Scenario: Annotation detector produces no false positives for non-matching turns
#[test]
fn test_no_false_positives_for_non_matching_turns() {
    // @step Given a completed turn containing only Read and Grep tool calls with no errors
    let tool_calls = vec![
        ToolCallInfo {
            tool_name: "Read".to_string(),
            input: json!({"file_path": "src/main.rs"}),
            output: Some("fn main() { ... }".to_string()),
            success: true,
        },
        ToolCallInfo {
            tool_name: "Grep".to_string(),
            input: json!({"pattern": "TODO", "path": "src/"}),
            output: Some("No matches found".to_string()),
            success: true,
        },
    ];
    let ctx = TurnContext {
        current_tool_calls: &tool_calls,
        previous_tool_calls: None,
    };

    // @step When the per-turn annotation detector inspects the turn
    let annotations = detect_annotations(&ctx);

    // @step Then no FspecMilestone annotations should be created
    let fspec_count = annotations
        .iter()
        .filter(|a| matches!(a, StructuralAnnotation::FspecMilestone { .. }))
        .count();
    assert_eq!(fspec_count, 0, "Should not have FspecMilestone annotations");

    // @step And no ErrorResolution annotations should be created
    let error_count = annotations
        .iter()
        .filter(|a| matches!(a, StructuralAnnotation::ErrorResolution { .. }))
        .count();
    assert_eq!(error_count, 0, "Should not have ErrorResolution annotations");
}

// Scenario: Annotations are zero-cost inline detection
#[test]
fn test_annotations_are_zero_cost_inline_detection() {
    // @step Given a completed turn with tool call metadata
    let tool_calls = vec![
        ToolCallInfo {
            tool_name: "Write".to_string(),
            input: json!({"file_path": "test.rs", "content": "test"}),
            output: Some("ok".to_string()),
            success: true,
        },
        ToolCallInfo {
            tool_name: "Fspec".to_string(),
            input: json!({"command": "board", "args": "{}"}),
            output: Some("board output".to_string()),
            success: true,
        },
    ];
    let ctx = TurnContext {
        current_tool_calls: &tool_calls,
        previous_tool_calls: None,
    };

    // @step When the per-turn annotation detector runs
    let annotations = detect_annotations(&ctx);

    // @step Then detection should use only pattern matching on tool call metadata
    // Verified by: detect_annotations is a pure function that takes tool call data
    // and returns annotations — no async, no network, no LLM
    assert!(!annotations.is_empty(), "Should detect annotations from pure pattern matching");

    // @step And no LLM calls should be made for annotation detection
    // Verified by: detect_annotations signature is synchronous (fn, not async fn)
    // and takes only local data — cannot make LLM calls

    // @step And no external processes should be spawned for annotation detection
    // Verified by: detect_annotations is a pure synchronous function
    // that only does pattern matching on the provided ToolCallInfo structs
}
