//! CMPCT-086: a compaction commit stores its covered prefix once.
//!
//! Feature: spec/features/compaction-commit-prefix-stored-once.feature

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_core::persistence::compaction_boundary::*;
use codelet_core::persistence::compaction_commit::*;
use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A committed manifest over three persisted messages.
fn committed_over_three() -> Result<(tempfile::TempDir, SessionManifest), Box<dyn std::error::Error>>
{
    let directory = setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("prefix once", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "first")?;
    append_message(&mut manifest, "assistant", "second")?;
    append_message(&mut manifest, "user", "third")?;
    let source = capture_compaction_source(&manifest, "model-a", "dag-v1")?;
    let dag = validate_dag_candidate(
        "<dag-node depth=\"D1\" turns=\"0-2\" label=\"Work\">three turns</dag-node>\n<dag-files>\n</dag-files>",
        &DagValidationContext {
            persisted_turn_count: 3,
            required_coverage_end: Some(2),
            ..Default::default()
        },
    )?;
    let candidate = CompactionCandidate {
        operation_id: Uuid::new_v4(),
        checkpoint: CompactionCheckpoint {
            provider: manifest.provider.clone(),
            model: "model-a".into(),
            generation: source.generation + 1,
            payload: serde_json::json!({"lineage":"explicit-full-DAG"}),
        },
        source,
        dag,
        input_tokens: 23,
        output_reservation: 8,
        token_provenance: CompactionTokenProvenance::Estimated,
    };
    let committed = commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    Ok((directory, committed))
}

/// The committed manifest as JSON with a legacy `covered_prefix` key set.
fn with_legacy_prefix(
    manifest: &SessionManifest,
    legacy: Vec<Uuid>,
) -> Result<SessionManifest, Box<dyn std::error::Error>> {
    let mut json = serde_json::to_value(manifest)?;
    json["compaction_commit"]["covered_prefix"] = serde_json::to_value(legacy)?;
    Ok(serde_json::from_value(json)?)
}

#[test]
#[serial_test::serial]
fn a_committed_generation_serialises_its_prefix_once() -> TestResult {
    // @step Given a committed generation over a three-message prefix
    let (_directory, committed) = committed_over_three()?;
    let commit = committed.compaction_commit.as_ref().ok_or("no commit")?;
    assert_eq!(commit.source.prefix.len(), 3);

    // @step When it is serialised into a manifest
    let json = serde_json::to_value(&committed)?;
    let text = serde_json::to_string(&json["compaction_commit"])?;

    // @step Then the prefix message ids appear once
    for id in &commit.source.prefix {
        assert_eq!(text.matches(&id.to_string()).count(), 1, "{id} in {text}");
    }

    // @step And no covered_prefix key is written
    assert!(json["compaction_commit"].get("covered_prefix").is_none());
    Ok(())
}

#[test]
#[serial_test::serial]
fn a_legacy_manifest_with_a_matching_covered_prefix_still_loads_and_validates() -> TestResult {
    // @step Given a manifest written with a covered_prefix equal to source.prefix
    let (_directory, committed) = committed_over_three()?;
    let prefix = committed
        .compaction_commit
        .as_ref()
        .ok_or("no commit")?
        .source
        .prefix
        .clone();
    let legacy = with_legacy_prefix(&committed, prefix.clone())?;

    // @step When it is loaded and its committed boundary is validated
    let validation = validate_committed_boundary(&legacy);

    // @step Then validation succeeds
    assert_eq!(validation, Ok(()));

    // @step And the covered prefix is the source prefix
    let commit = legacy.compaction_commit.as_ref().ok_or("no commit")?;
    assert_eq!(commit.covered_prefix(), prefix.as_slice());
    Ok(())
}

#[test]
#[serial_test::serial]
fn a_legacy_manifest_whose_covered_prefix_differs_is_still_rejected() -> TestResult {
    // @step Given a manifest written with a covered_prefix that differs from source.prefix
    let (_directory, committed) = committed_over_three()?;
    let mut prefix = committed
        .compaction_commit
        .as_ref()
        .ok_or("no commit")?
        .source
        .prefix
        .clone();
    prefix.pop();
    let legacy = with_legacy_prefix(&committed, prefix)?;

    // @step When its committed boundary is validated
    let validation = validate_committed_boundary(&legacy);

    // @step Then validation fails as an incompatible committed generation boundary
    assert_eq!(
        validation,
        Err("incompatible committed generation boundary".to_string())
    );
    Ok(())
}
