// Feature: spec/features/structural-annotation-data-model.feature
//
// This test file validates the acceptance criteria for the StructuralAnnotation
// and FileOp data model types.

use serde_json;

// Import the types under test
use crate::compaction::model::{FileOp, StructuralAnnotation};

// ==========================================
// Scenario: FspecMilestone annotation serializes and deserializes
// ==========================================

#[test]
fn test_fspec_milestone_serde_round_trip() {
    // @step Given a StructuralAnnotation::FspecMilestone with command "update-work-unit-status" and args ["AUTH-001", "implementing"]
    let annotation = StructuralAnnotation::FspecMilestone {
        command: "update-work-unit-status".to_string(),
        args: vec!["AUTH-001".to_string(), "implementing".to_string()],
    };

    // @step When I serialize it to JSON and deserialize back
    let json = serde_json::to_string(&annotation).expect("serialize");
    let deserialized: StructuralAnnotation =
        serde_json::from_str(&json).expect("deserialize");

    // @step Then the deserialized value should be identical to the original
    assert_eq!(deserialized, annotation);
    let json_again = serde_json::to_string(&deserialized).expect("re-serialize");
    assert_eq!(json, json_again);
}

// ==========================================
// Scenario: ErrorResolution annotation round-trips through serde
// ==========================================

#[test]
fn test_error_resolution_serde_round_trip() {
    // @step Given a StructuralAnnotation::ErrorResolution with failed_tool "Bash" and resolved_file "src/main.rs"
    let annotation = StructuralAnnotation::ErrorResolution {
        failed_tool: "Bash".to_string(),
        resolved_file: "src/main.rs".to_string(),
    };

    // @step When I serialize it to JSON and deserialize back
    let json = serde_json::to_string(&annotation).expect("serialize");
    let deserialized: StructuralAnnotation =
        serde_json::from_str(&json).expect("deserialize");

    // @step Then the deserialized value should be identical to the original
    assert_eq!(deserialized, annotation);
    let json_again = serde_json::to_string(&deserialized).expect("re-serialize");
    assert_eq!(json, json_again);
}

// ==========================================
// Scenario: FileModification annotation round-trips through serde
// ==========================================

#[test]
fn test_file_modification_serde_round_trip() {
    // @step Given a StructuralAnnotation::FileModification with path "src/auth.rs" and operation FileOp::Created
    let annotation = StructuralAnnotation::FileModification {
        path: "src/auth.rs".to_string(),
        operation: FileOp::Created,
    };

    // @step When I serialize it to JSON and deserialize back
    let json = serde_json::to_string(&annotation).expect("serialize");
    let deserialized: StructuralAnnotation =
        serde_json::from_str(&json).expect("deserialize");

    // @step Then the deserialized value should be identical to the original
    assert_eq!(deserialized, annotation);
    let json_again = serde_json::to_string(&deserialized).expect("re-serialize");
    assert_eq!(json, json_again);
}

// ==========================================
// Scenario: All FileOp variants round-trip through serde
// ==========================================

#[test]
fn test_file_op_all_variants_serde_round_trip() {
    // @step Given FileOp variants Created, Modified, and Deleted
    let variants = vec![FileOp::Created, FileOp::Modified, FileOp::Deleted];

    for variant in &variants {
        // @step When I serialize each variant to JSON and deserialize back
        let json = serde_json::to_string(variant).expect("serialize");
        let deserialized: FileOp = serde_json::from_str(&json).expect("deserialize");

        // @step Then each deserialized variant should be identical to its original
        assert_eq!(&deserialized, variant);
        let json_again = serde_json::to_string(&deserialized).expect("re-serialize");
        assert_eq!(json, json_again);
    }
}

// ==========================================
// Scenario: TokenTracker remains unchanged
// ==========================================

#[test]
fn test_token_tracker_remains_unchanged() {
    // @step Given the existing TokenTracker struct
    use crate::compaction::model::TokenTracker;

    // @step When the structural annotation changes are applied
    let mut tracker = TokenTracker::new();

    // @step Then TokenTracker::new() should work identically
    assert_eq!(tracker.input_tokens, 0);
    assert_eq!(tracker.output_tokens, 0);

    // @step And all TokenTracker methods should produce the same results
    assert_eq!(tracker.effective_tokens(), 0);
    assert_eq!(tracker.total_tokens(), 0);

    tracker.update(100, 50, Some(30), Some(10));
    assert_eq!(tracker.input_tokens, 100);
    assert_eq!(tracker.output_tokens, 50);
    assert_eq!(tracker.effective_tokens(), 73); // 100 - (30 * 0.9) = 73
    assert_eq!(tracker.total_tokens(), 150); // 100 + 50 + 0 reasoning
}

// ==========================================
// Scenario: StructuralAnnotation and FileOp are accessible via module re-exports
// ==========================================

#[test]
fn test_types_accessible_via_module_re_exports() {
    // @step Given the codelet_core::compaction module
    // (Using crate::compaction which is the module root re-export path)

    // @step When I import StructuralAnnotation and FileOp from the module
    use crate::compaction::{FileOp as ReExportedFileOp, StructuralAnnotation as ReExportedAnnotation};

    // @step Then both types should be accessible without specifying internal module paths
    let _annotation = ReExportedAnnotation::FspecMilestone {
        command: "test".to_string(),
        args: vec![],
    };
    let _op = ReExportedFileOp::Modified;
}
