//! Feature: spec/features/dag-promotion-revalidation.feature

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_core::persistence::compaction_commit::*;
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;

#[path = "support/compaction_boundary.rs"]
mod support;

#[test]
#[serial_test::serial]
fn scenario_revalidate_whole_candidates_inside_durable_promotion(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("validator promotion", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable original")?;
    let durable = "<dag-node depth=\"D2\" turns=\"0-0\" label=\"Decision\">exact durable bytes</dag-node>\n<dag-files>\n- src/lib.rs (Modified)\n</dag-files>";
    let mut initial = support::candidate(&manifest)?;
    initial.dag = validate_dag_candidate(
        durable,
        &DagValidationContext {
            persisted_turn_count: 1,
            required_coverage_end: Some(0),
            ..Default::default()
        },
    )?;
    let committed = commit_compaction(initial, "model-a", |_, _| Ok(()))?;
    let identity = committed
        .compaction_commit
        .as_ref()
        .ok_or("missing commit")?
        .identity
        .clone();
    let ready = reconcile_compaction(&identity)?;
    let unchanged = serde_json::to_value(&ready)?;
    // @step Given a committed DAG and candidates changed after earlier validation
    for content in [
        format!("{durable}<dag-node depth=\"D0\" turns=\"0-0\" label=\"unclosed\">"),
        durable.replace("exact durable bytes", "rewritten payload"),
        durable.replace("D2", "D1"),
        durable.replace("0-0", "0-5"),
        durable.replace("- src/lib.rs (Modified)", ""),
    ] {
        let mut candidate = support::candidate(&ready)?;
        candidate.dag = validate_dag_candidate(
            durable,
            &DagValidationContext {
                persisted_turn_count: 1,
                required_coverage_end: Some(0),
                existing_dag: Some(durable.into()),
                ..Default::default()
            },
        )?;
        candidate.dag.content = content;
        // @step When promotion checks the authoritative persisted source
        let result = commit_compaction(candidate, "model-a", |_, _| {
            Err("malformed memory must not reach request admission".into())
        });
        // @step Then malformed incomplete and rewritten durable candidates are rejected without changing the generation
        assert!(
            matches!(result, Err(CompactionCommitError::Rejected(reason))
            if !reason.contains("must not reach request admission"))
        );
        reset_stores_for_tests();
        assert_eq!(serde_json::to_value(load_session(ready.id)?)?, unchanged);
    }
    let mut candidate = support::candidate(&ready)?;
    candidate.dag = validate_dag_candidate(
        durable,
        &DagValidationContext {
            persisted_turn_count: 1,
            required_coverage_end: Some(0),
            existing_dag: Some(durable.into()),
            ..Default::default()
        },
    )?;
    let mut appended = ready;
    let suffix = append_message(&mut appended, "user", "uncovered suffix stays intact")?;
    let committed = commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    let boundary = committed.compaction_commit.ok_or("missing successor")?;
    assert_eq!(boundary.identity.generation, 2);
    assert_eq!(boundary.verified_suffix, vec![suffix]);
    assert_eq!(committed.compaction.ok_or("missing DAG")?.summary, durable);
    Ok(())
}
