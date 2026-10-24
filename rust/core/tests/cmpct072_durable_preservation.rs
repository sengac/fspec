//! Feature: spec/features/durable-dag-preservation.feature

use codelet_core::compaction::{validate_dag_candidate, DagValidationContext, DagValidationError};

#[path = "support/compaction_boundary.rs"]
mod support;

fn durable() -> &'static str {
    r#"<dag-node depth="D2" turns="0-3" label="decision">Keep exact
  payload bytes.</dag-node>"#
}

#[test]
#[serial_test::serial]
fn scenario_cached_originals_cannot_conceal_missing_durable_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_core::persistence::compaction_boundary::capture_compaction_source;
    use codelet_core::persistence::compaction_commit::commit_compaction;
    use codelet_core::persistence::*;

    // @step Given immutable originals remain cached after their durable storage disappears
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("warm evidence", directory.path(), "openai")?;
    let original = append_message(&mut manifest, "user", "immutable cached original")?;
    let candidate = support::candidate(&manifest)?;
    append_message(&mut manifest, "assistant", "immutable cached suffix")?;
    assert!(get_message(original)?.is_some());
    let path = directory
        .path()
        .join(format!("sessions/{}.json", manifest.id));
    let before = std::fs::read(&path)?;
    std::fs::remove_file(directory.path().join("messages/messages.jsonl"))?;
    // @step When source capture promotion or replay checks the persisted evidence
    let capture = capture_compaction_source(&manifest, "model-a", "dag-v1");
    let replay = get_session_message_envelopes(manifest.id);
    let mut admitted = false;
    let promotion = commit_compaction(candidate, "model-a", |_, _| {
        admitted = true;
        Ok(())
    });
    // @step Then every operation rejects the missing originals before admission or publication
    assert!(
        capture.is_err(),
        "warm cache authorized source capture: {capture:?}"
    );
    assert!(replay.is_err(), "warm cache authorized replay: {replay:?}");
    assert!(
        promotion.is_err(),
        "warm cache authorized promotion: {promotion:?}"
    );
    assert!(
        !admitted,
        "unavailable originals reached selected admission"
    );
    // @step And the committed manifest bytes remain unchanged
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}

fn context() -> DagValidationContext {
    DagValidationContext {
        persisted_turn_count: 4,
        required_coverage_end: Some(3),
        existing_dag: Some(format!(
            "{}<dag-files>\n- src/lib.rs (Modified)\n</dag-files>",
            durable()
        )),
        required_files: [("src/lib.rs".into(), "Modified".into())].into(),
        required_reminders: vec!["<!-- required:project-policy -->".into()],
    }
}

fn complete() -> String {
    format!(
        "{}<dag-files>\n- src/lib.rs (Modified)\n</dag-files>\n<!-- required:project-policy -->",
        durable()
    )
}

#[test]
fn scenario_preserve_durable_payloads_and_file_awareness_in_the_shared_validator(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given authoritative durable nodes file identities and required reminders
    let source = context();
    // @step When a complete candidate is validated against the shared preservation contract
    let candidate = complete();
    let accepted = validate_dag_candidate(&candidate, &source)?;
    // @step Then complete candidates preserve the exact durable payload and identities
    assert_eq!(accepted.content, candidate);
    Ok(())
}

#[test]
fn scenario_reject_missing_or_rewritten_durable_content() {
    // @step Given authoritative durable nodes file identities and required reminders
    let source = context();
    // @step When a candidate omits or rewrites required content
    for candidate in [
        complete().replace("Keep exact", "Rewrite"),
        complete().replace("D2", "D1"),
        complete().replace("<!-- required:project-policy -->", ""),
        complete().replace("src/lib.rs", "src/other.rs"),
    ] {
        // @step Then the whole candidate is rejected without fragment repair
        assert!(matches!(
            validate_dag_candidate(&candidate, &source),
            Err(DagValidationError::InvalidCandidate(_))
        ));
    }
}

#[test]
fn scenario_reject_malformed_or_empty_required_file_metadata() {
    // @step Given an authoritative nonempty file awareness contract
    let source = context();
    // @step When a candidate supplies malformed empty or duplicate file metadata
    for files in [
        "<dag-files></dag-files>",
        "<dag-files>\n- src/lib.rs (Modified)",
        "<dag-files>\n- src/lib.rs (Guessed)\n</dag-files>",
        "<dag-files>\n- src/lib.rs (Modified)\n- src/lib.rs (Modified)\n</dag-files>",
        "<dag-files>\n- src/lib.rs (Modified)\n</dag-files><dag-files></dag-files>",
    ] {
        let candidate = format!("{}{files}<!-- required:project-policy -->", durable());
        // @step Then validation fails without inventing file awareness
        assert!(
            validate_dag_candidate(&candidate, &source).is_err(),
            "{candidate}"
        );
    }
}

#[test]
#[serial_test::serial]
fn scenario_reject_inaccessible_or_invalid_authoritative_source(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an invalid existing marked DAG
    use codelet_core::persistence::compaction_boundary::capture_compaction_source;
    use codelet_core::persistence::*;
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("invalid source", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable source")?;
    manifest.compaction = Some(CompactionState {
        summary: "<!-- type:compaction-dag --><dag-node>unclosed".into(),
        compacted_before_index: 1,
        compacted_at: chrono::Utc::now(),
    });
    // @step When a candidate requests a fresh rebuild instead
    let captured = capture_compaction_source(&manifest, "model-a", "full-current-dag");
    let mut source = context();
    source.existing_dag = manifest
        .compaction
        .as_ref()
        .map(|state| state.summary.clone());
    let result = validate_dag_candidate(&complete(), &source);
    // @step Then validation returns a typed invalid source failure
    assert!(matches!(result, Err(DagValidationError::InvalidSource(_))));
    assert!(
        captured.is_err(),
        "invalid persisted DAG must fail before builder invocation"
    );
    Ok(())
}

#[test]
fn scenario_assemble_missing_file_metadata_deterministically_before_validation(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given authoritative file changes and a candidate with no file metadata
    let mut source = context();
    source
        .required_files
        .insert("a.rs".into(), "Created".into());
    source
        .required_files
        .insert("dag-files.rs".into(), "Modified".into());
    let candidate = format!("{}<!-- required:project-policy -->", durable());
    // @step When the selected protocol prepares the complete candidate
    let accepted = validate_dag_candidate(&candidate, &source)?;
    // @step Then sorted file metadata is assembled once and validated without changing durable payloads
    assert_eq!(
        accepted.content,
        format!(
            "{candidate}\n<dag-files>\n- a.rs (Created)\n- dag-files.rs (Modified)\n- src/lib.rs (Modified)\n</dag-files>"
        )
    );
    assert!(accepted.content.contains(durable()));
    assert_eq!(
        validate_dag_candidate(&accepted.content, &source)?,
        accepted
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_unavailable_originals_reject_source_capture_and_promotion(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_core::persistence::compaction_boundary::{
        capture_compaction_source, CompactionCheckpoint, CompactionTokenProvenance,
    };
    use codelet_core::persistence::compaction_commit::{commit_compaction, CompactionCandidate};
    use codelet_core::persistence::*;
    // @step Given a persisted source whose immutable originals become unavailable
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("unavailable source", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable source")?;
    let source = capture_compaction_source(&manifest, "model-a", "dag-v1")?;
    let candidate = CompactionCandidate {
        operation_id: uuid::Uuid::new_v4(),
        checkpoint: CompactionCheckpoint {
            provider: "openai".into(),
            model: "model-a".into(),
            generation: 1,
            payload: serde_json::json!({}),
        },
        source,
        dag: validate_dag_candidate(
            "<dag-node depth=\"D1\" turns=\"0-0\" label=\"memory\">source</dag-node>",
            &DagValidationContext {
                persisted_turn_count: 1,
                required_coverage_end: Some(0),
                ..Default::default()
            },
        )?,
        input_tokens: 20,
        output_reservation: 8,
        token_provenance: CompactionTokenProvenance::Estimated,
    };
    let path = directory
        .path()
        .join(format!("sessions/{}.json", manifest.id));
    let before = std::fs::read(&path)?;
    std::fs::remove_file(directory.path().join("messages/messages.jsonl"))?;
    reset_stores_for_tests();
    // @step When a builder captures the source or a captured candidate requests promotion
    assert!(capture_compaction_source(&manifest, "model-a", "dag-v1").is_err());
    let mut admitted = false;
    let result = commit_compaction(candidate, "model-a", |_, _| {
        admitted = true;
        Ok(())
    });
    // @step Then both operations fail explicitly before admission without changing the durable boundary
    assert!(
        result.is_err(),
        "missing original must reject promotion: {result:?}"
    );
    assert!(
        !admitted,
        "missing original must fail before selected admission"
    );
    assert_eq!(std::fs::read(path)?, before);
    Ok(())
}

#[test]
fn scenario_empty_authoritative_reminders_cannot_authorize_preservation() {
    // @step Given authoritative reminders containing an empty or whitespace-only obligation
    for reminder in ["", " ", "\n\t", "\u{2003}"] {
        let mut source = context();
        source.required_reminders.push(reminder.into());
        // @step When an otherwise complete candidate is validated
        let result = validate_dag_candidate(&complete(), &source);
        // @step Then validation returns a typed invalid source failure rather than vacuous preservation
        assert!(
            matches!(result, Err(DagValidationError::InvalidSource(_))),
            "invalid authoritative reminder {reminder:?} must fail as source: {result:?}"
        );
    }
}
