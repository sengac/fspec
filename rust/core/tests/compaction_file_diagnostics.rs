use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};

#[test]
fn invalid_file_entries_report_the_entry_and_expected_format(
) -> Result<(), Box<dyn std::error::Error>> {
    let context = DagValidationContext {
        persisted_turn_count: 1,
        required_coverage_end: Some(0),
        ..Default::default()
    };
    for entry in [
        "- No tracked repository files were Created, Modified, or Deleted.",
        "- /tmp/repro (Created; compiled repro executable)",
        "- /tmp/repro (Created)\n- /tmp/repro (Created)",
    ] {
        let content = format!(
            "<dag-node depth=\"D1\" turns=\"0-0\" label=\"review\">No repository edits.</dag-node>\n<dag-files>\n{entry}\n</dag-files>"
        );
        let error = validate_dag_candidate(&content, &context)
            .err()
            .ok_or("prose, decorated operations, and duplicate paths must reject the whole DAG")?
            .to_string();
        assert!(error.contains("dag-files line"), "{error}");
        assert!(
            error.contains("- path (Created|Modified|Deleted)"),
            "{error}"
        );
    }
    Ok(())
}

#[test]
fn an_empty_file_section_is_valid_for_a_read_only_session() -> Result<(), Box<dyn std::error::Error>>
{
    let content = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"review\">No repository edits.</dag-node>\n<dag-files></dag-files>";
    let candidate = validate_dag_candidate(
        content,
        &DagValidationContext {
            persisted_turn_count: 1,
            required_coverage_end: Some(0),
            ..Default::default()
        },
    )?;
    assert_eq!(candidate.content, content);
    Ok(())
}
