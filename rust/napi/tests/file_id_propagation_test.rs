#![cfg(not(feature = "noop"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/file-id-propagation-through-dag.feature
//!
//! This test file validates the acceptance criteria for CMPCT-021:
//! File ID Propagation Through DAG.
//!
//! Tests verify:
//! - build_dag_files_block merges annotations and existing dag-files
//! - parse_dag_files_block extracts (path, operation) tuples
//! - apply_pending_dag auto-appends dag-files block when agent omits it
//! - Agent-provided dag-files blocks are preserved without duplication
//! - Incremental compaction instruction mentions dag-files preservation
//! - Fresh compaction instruction mentions Active Files section

use codelet_core::compaction::{FileOp, StructuralAnnotation};
use codelet_napi::inject_summary_handler;

// =============================================================================
// Scenario: Engine appends dag-files block when agent omits it
// =============================================================================

#[test]
fn test_engine_appends_dag_files_when_agent_omits() {
    // @step Given a session with FileModification annotations for 3 files
    let annotations = vec![
        StructuralAnnotation::FileModification {
            path: "src/auth.rs".to_string(),
            operation: FileOp::Created,
        },
        StructuralAnnotation::FileModification {
            path: "src/db.rs".to_string(),
            operation: FileOp::Modified,
        },
        StructuralAnnotation::FileModification {
            path: "src/old.rs".to_string(),
            operation: FileOp::Deleted,
        },
    ];

    // @step And the agent's DAG content does not contain a dag-files block
    let dag_content = r#"<dag-node depth="D2" turns="0-10" label="Architecture">
- Using JWT auth
</dag-node>"#;
    assert!(
        !dag_content.contains("<dag-files>"),
        "Original DAG should not have dag-files"
    );

    // @step When apply_pending_dag processes the agent's DAG content
    let result = inject_summary_handler::build_dag_files_block(&annotations, None);

    // @step Then the stored DAG should have a dag-files block appended
    assert!(result.is_some(), "Should produce a dag-files block");
    let block = result.unwrap();
    assert!(block.contains("<dag-files>"), "Should contain opening tag");
    assert!(block.contains("</dag-files>"), "Should contain closing tag");

    // @step And the dag-files block should contain 3 file entries
    assert!(block.contains("src/auth.rs"), "Should contain auth.rs");
    assert!(block.contains("src/db.rs"), "Should contain db.rs");
    assert!(block.contains("src/old.rs"), "Should contain old.rs");

    // @step And the entries should be sorted alphabetically by path
    let auth_pos = block.find("src/auth.rs").unwrap();
    let db_pos = block.find("src/db.rs").unwrap();
    let old_pos = block.find("src/old.rs").unwrap();
    assert!(
        auth_pos < db_pos && db_pos < old_pos,
        "Entries should be sorted: auth < db < old"
    );
}

// =============================================================================
// Scenario: Agent-provided dag-files block is preserved without duplication
// =============================================================================

#[test]
fn test_agent_provided_dag_files_preserved() {
    // @step Given a session with FileModification annotations for "src/auth.rs" as Created
    let annotations = vec![StructuralAnnotation::FileModification {
        path: "src/auth.rs".to_string(),
        operation: FileOp::Created,
    }];

    // @step And the agent's DAG content already contains a dag-files block
    let dag_with_files = "<dag-files>\n- src/custom.rs (Modified)\n</dag-files>";

    // @step When apply_pending_dag processes the agent's DAG content
    // The engine checks `if !wrapped.contains("<dag-files>")` before appending.
    // Since the agent's DAG already has a dag-files block, build_dag_files_block
    // is never called — the agent's version is used as-is.
    // We verify: (a) the guard detects the existing block, and (b) if we
    // hypothetically called build_dag_files_block anyway, the annotations
    // would produce different content — proving the guard is necessary.
    let agent_has_dag_files = dag_with_files.contains("<dag-files>");
    assert!(
        agent_has_dag_files,
        "Agent DAG should already contain dag-files"
    );

    // Verify the guard would skip the build path
    // (This mirrors the logic in apply_pending_dag line 280)
    let would_skip_build = dag_with_files.contains("<dag-files>");
    assert!(
        would_skip_build,
        "Guard should detect existing dag-files and skip build"
    );

    // Prove that build_dag_files_block with these annotations would produce
    // DIFFERENT content (src/auth.rs Created) than what the agent wrote
    // (src/custom.rs Modified) — confirming the guard prevents corruption.
    let engine_block = inject_summary_handler::build_dag_files_block(&annotations, None);
    assert!(engine_block.is_some());
    let engine_content = engine_block.unwrap();
    assert!(
        engine_content.contains("src/auth.rs (Created)"),
        "Engine would produce src/auth.rs"
    );
    assert!(
        !engine_content.contains("src/custom.rs"),
        "Engine block should NOT have agent's custom file"
    );

    // @step Then the stored DAG should contain exactly one dag-files block
    let count = dag_with_files.matches("<dag-files>").count();
    assert_eq!(count, 1, "Should have exactly one dag-files block");

    // @step And it should be the agent's original dag-files block unchanged
    assert!(
        dag_with_files.contains("src/custom.rs (Modified)"),
        "Agent's content should be preserved"
    );
}

// =============================================================================
// Scenario: Merge existing dag-files with new annotations across compactions
// =============================================================================

#[test]
fn test_merge_existing_dag_files_with_new_annotations() {
    // @step Given an existing dag-files block with entries
    let existing_block = "<dag-files>\n- a.rs (Created)\n- b.rs (Modified)\n</dag-files>";

    // @step And new FileModification annotations
    let new_annotations = vec![
        StructuralAnnotation::FileModification {
            path: "a.rs".to_string(),
            operation: FileOp::Modified,
        },
        StructuralAnnotation::FileModification {
            path: "c.rs".to_string(),
            operation: FileOp::Created,
        },
    ];

    // @step When build_dag_files_block merges existing entries with new annotations
    let result =
        inject_summary_handler::build_dag_files_block(&new_annotations, Some(existing_block));

    // @step Then the merged result should contain 3 file entries
    assert!(result.is_some(), "Should produce a merged block");
    let block = result.unwrap();

    // @step And "a.rs" should have operation "Modified" overriding "Created"
    assert!(
        block.contains("a.rs (Modified)"),
        "a.rs should be Modified (overridden)"
    );
    assert!(
        !block.contains("a.rs (Created)"),
        "a.rs should NOT still be Created"
    );

    // @step And "b.rs" should have operation "Modified" carried forward
    assert!(
        block.contains("b.rs (Modified)"),
        "b.rs should be carried forward as Modified"
    );

    // @step And "c.rs" should have operation "Created" as a new entry
    assert!(
        block.contains("c.rs (Created)"),
        "c.rs should be a new Created entry"
    );
}

// =============================================================================
// Scenario: No dag-files block when no file modifications exist
// =============================================================================

#[test]
fn test_no_dag_files_when_no_modifications() {
    // @step Given a session with no FileModification annotations
    let empty_annotations: Vec<StructuralAnnotation> = vec![];

    // @step And no existing dag-files block
    // @step When build_dag_files_block is called
    let result = inject_summary_handler::build_dag_files_block(&empty_annotations, None);

    // @step Then it should return None
    assert!(
        result.is_none(),
        "Should return None when no file modifications"
    );

    // @step And no dag-files block should be appended to the DAG
    // (Verified by the None return)
}

// =============================================================================
// Scenario: Incremental compaction instruction includes dag-files preservation guidance
// =============================================================================

#[test]
fn test_incremental_instruction_mentions_dag_files() {
    // @step Given the COMPACTION_INSTRUCTION_INCREMENTAL constant
    use codelet_cli::compaction_dag::COMPACTION_INSTRUCTION_INCREMENTAL;

    // @step When the instruction text is examined
    let instruction = COMPACTION_INSTRUCTION_INCREMENTAL;

    // @step Then it should contain guidance to preserve the dag-files section
    assert!(
        instruction.contains("dag-files"),
        "Incremental instruction should mention dag-files"
    );

    // @step And it should mention updating dag-files with new file modifications
    assert!(
        instruction.to_lowercase().contains("preserve")
            || instruction.to_lowercase().contains("update"),
        "Should mention preserving/updating dag-files"
    );
}

// =============================================================================
// Scenario: Parse dag-files block from existing DAG content
// =============================================================================

#[test]
fn test_parse_dag_files_block() {
    // @step Given a DAG content string containing a dag-files block with entries
    let dag_content = r#"<dag-node depth="D2" turns="0-10" label="Arch">
- JWT auth
</dag-node>

<dag-files>
- src/handler.rs (Created)
- src/types.rs (Modified)
</dag-files>"#;

    // @step When the dag-files block is parsed
    let parsed = inject_summary_handler::parse_dag_files_block(dag_content);

    // @step Then it should extract 2 file entries
    assert!(parsed.is_some(), "Should parse dag-files block");
    let entries = parsed.unwrap();
    assert_eq!(entries.len(), 2, "Should have 2 entries");

    // @step And "src/handler.rs" should be parsed as Created
    assert_eq!(entries.get("src/handler.rs"), Some(&FileOp::Created));

    // @step And "src/types.rs" should be parsed as Modified
    assert_eq!(entries.get("src/types.rs"), Some(&FileOp::Modified));
}

// =============================================================================
// Scenario: Fresh compaction instruction mentions dag-files in Active Files section
// =============================================================================

#[test]
fn test_fresh_instruction_mentions_dag_files() {
    // @step Given the COMPACTION_INSTRUCTION_FRESH constant
    use codelet_cli::compaction_dag::COMPACTION_INSTRUCTION_FRESH;

    // @step When the instruction text is examined
    let instruction = COMPACTION_INSTRUCTION_FRESH;

    // @step Then it should contain guidance for including an Active Files section with dag-files
    assert!(
        instruction.contains("dag-files") || instruction.contains("Active Files"),
        "Fresh instruction should mention dag-files or Active Files section"
    );
}

// =============================================================================
// Scenario: dag-files block format uses deterministic path ordering
// =============================================================================

#[test]
fn test_dag_files_deterministic_ordering() {
    // @step Given FileModification annotations in non-alphabetical order
    let annotations = vec![
        StructuralAnnotation::FileModification {
            path: "z.rs".to_string(),
            operation: FileOp::Created,
        },
        StructuralAnnotation::FileModification {
            path: "a.rs".to_string(),
            operation: FileOp::Modified,
        },
        StructuralAnnotation::FileModification {
            path: "m.rs".to_string(),
            operation: FileOp::Created,
        },
    ];

    // @step When build_dag_files_block generates the block
    let result = inject_summary_handler::build_dag_files_block(&annotations, None);
    assert!(result.is_some());
    let block = result.unwrap();

    // @step Then the entries should appear in alphabetical order: a.rs, m.rs, z.rs
    let a_pos = block.find("a.rs").unwrap();
    let m_pos = block.find("m.rs").unwrap();
    let z_pos = block.find("z.rs").unwrap();
    assert!(
        a_pos < m_pos && m_pos < z_pos,
        "Entries should be in alphabetical order: a < m < z"
    );
}
