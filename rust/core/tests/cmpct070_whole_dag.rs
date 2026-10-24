//! Feature: spec/features/whole-dag-candidate-validation.feature

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext, DagValidationError};
use codelet_test_helpers::setup_temp_directory;

fn node(depth: &str, start: usize, end: usize) -> String {
    format!(r#"<dag-node depth="{depth}" turns="{start}-{end}" label="memory">payload</dag-node>"#)
}

fn context(persisted_turn_count: usize) -> DagValidationContext {
    DagValidationContext {
        persisted_turn_count,
        required_coverage_end: persisted_turn_count.checked_sub(1),
        ..Default::default()
    }
}

#[test]
fn scenario_validate_persisted_coordinates_after_active_history_shrinks(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given persisted source turns extend beyond the active history
    let directory = setup_temp_directory()?;
    let path = directory.path().join("candidate.txt");
    let candidate = node("D1", 0, 99);
    std::fs::write(&path, &candidate)?;
    // @step When a complete candidate covers the required persisted prefix
    let stored = std::fs::read_to_string(path)?;
    let active_history_length = 4;
    let persisted_turn_count = 100;
    assert!(persisted_turn_count > active_history_length);
    let parsed = validate_dag_candidate(&stored, &context(persisted_turn_count))?;
    // @step Then its original text and exact persisted bounds are accepted unchanged
    assert_eq!(parsed.nodes.len(), 1);
    assert_eq!(parsed.nodes[0].turn_end, 99);
    assert_eq!(parsed.content, candidate);
    assert_eq!(stored, candidate);
    Ok(())
}

#[test]
fn scenario_reject_malformed_candidates_without_salvaging_valid_nodes() {
    // @step Given a valid node already covering the complete required prefix and additional malformed structure
    let valid = node("D1", 0, 3);
    let source = context(4);
    let control = validate_dag_candidate(&valid, &source);
    assert!(control.is_ok(), "valid-only control must pass: {control:?}");
    let malformed = [
        r#"<dag-node depth="D0" turns="2-3" label="open">unclosed"#,
        r#"<dag-node depth="D3" turns="2-3" label="bad">bad</dag-node>"#,
        r#"<dag-node depth="D0" turns="2-3">bad</dag-node>"#,
        r#"<dag-node depth="D0" turns="2-3" label="bad" extra="x">bad</dag-node>"#,
        r#"<dag-node depth="D0" turns="2-3" label="empty"></dag-node>"#,
        "</dag-node>",
        "<dag-files broken><dag-files></dag-files>",
        "<dag-files><dag-node depth=\"D0\" turns=\"2-3\" label=\"nested\">bad</dag-node></dag-files>",
    ];
    // @step When the whole candidate is validated
    for suffix in malformed {
        let parsed = validate_dag_candidate(&format!("{valid}\n{suffix}"), &source);
        // @step Then validation fails without returning a surviving subset
        assert!(
            matches!(parsed, Err(DagValidationError::InvalidCandidate(_))),
            "must reject entire candidate: {suffix}"
        );
    }
}

#[test]
fn scenario_reject_invalid_provenance_and_missing_coverage() {
    // @step Given candidates with inverted bounds unknown turns or coverage gaps
    let candidates = [
        format!("{}{}", node("D1", 0, 1), node("D0", 3, 2)),
        node("D1", 0, 99),
        format!("{}{}", node("D1", 0, 0), node("D0", 2, 3)),
    ];
    // @step When the whole candidate is validated against persisted source identities
    for candidate in candidates {
        let parsed = validate_dag_candidate(&candidate, &context(4));
        // @step Then each candidate is rejected without clamping or invented coverage
        assert!(parsed.is_err(), "must reject: {candidate}");
    }
}

#[test]
fn scenario_preserve_cross_depth_overlap_but_reject_same_depth_overlap(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given candidates with overlapping hierarchical turn ranges
    let first = node("D2", 0, 3);
    // @step When the whole candidate is validated
    let cross = validate_dag_candidate(&format!("{first}{}", node("D0", 2, 3)), &context(4));
    let same = validate_dag_candidate(&format!("{first}{}", node("D2", 2, 3)), &context(4));
    // @step Then cross-depth overlap is accepted and same-depth overlap is rejected
    assert_eq!(cross?.nodes.len(), 2);
    assert!(same.is_err(), "same-depth overlap must reject whole DAG");
    let mut runner = proptest::test_runner::TestRunner::default();
    runner.run(&(1usize..1000, 0usize..1000), |(end, raw_start)| {
        let start = raw_start % (end + 1);
        let first = node("D2", 0, end);
        let source = context(end + 1);
        let cross_depth =
            validate_dag_candidate(&format!("{first}{}", node("D0", start, end)), &source);
        let same_depth =
            validate_dag_candidate(&format!("{first}{}", node("D2", start, end)), &source);
        proptest::prop_assert!(cross_depth.is_ok());
        proptest::prop_assert!(same_depth.is_err());
        Ok(())
    })?;
    Ok(())
}

#[test]
fn scenario_reject_invalid_existing_marked_dag_as_invalid_source() {
    // @step Given an existing marked DAG containing malformed nodes
    let source = r#"<dag-node depth="D2" turns="0-3" label="durable">unclosed"#;
    // @step When the source preservation contract is captured
    let mut source_context = context(4);
    source_context.existing_dag = Some(source.into());
    let parsed = validate_dag_candidate(&node("D1", 0, 3), &source_context);
    // @step Then a typed invalid-source failure is returned rather than zero coverage
    assert!(
        matches!(parsed, Err(DagValidationError::InvalidSource(_))),
        "opening-only source must not appear valid"
    );
    source_context.existing_dag = Some(format!(
        "{}<dag-files broken><dag-files></dag-files>",
        node("D2", 0, 3)
    ));
    assert!(matches!(
        validate_dag_candidate(&node("D1", 0, 3), &source_context),
        Err(DagValidationError::InvalidSource(_))
    ));
}
