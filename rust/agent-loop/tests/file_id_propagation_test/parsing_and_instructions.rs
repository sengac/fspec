//! Feature: spec/features/file-id-propagation-through-dag.feature
use super::*;

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
