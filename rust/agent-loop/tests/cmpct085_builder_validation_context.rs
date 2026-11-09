//! Feature: spec/features/durable-dag-preservation.feature

use codelet_agent_loop::compaction_evidence::CompactionEvidence;
use codelet_core::compaction::{validate_dag_candidate, DagValidationError};
use codelet_core::persistence::compaction_boundary::capture_compaction_source;
use codelet_core::persistence::{
    append_message_with_metadata, create_session_with_provider, reset_stores_for_tests,
};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn the_builder_validates_candidates_against_the_same_file_identities_as_commit() -> TestResult {
    // @step Given a captured compaction source whose persisted originals record an authoritative file modification
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_test_helpers::isolated_user_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("cmpct085", directory.path(), "openai")?;
    for (turn, annotations) in [
        ("edit the library", json!([])),
        (
            "done",
            json!([{"FileModification": {"path": "src/lib.rs", "operation": "Modified"}}]),
        ),
    ] {
        append_message_with_metadata(
            &mut manifest,
            "user",
            turn,
            [
                ("uuid".into(), json!(uuid::Uuid::new_v4())),
                ("message".into(), json!({"role": "user", "content": turn})),
                ("annotations".into(), annotations),
            ]
            .into(),
        )?;
    }
    let source = capture_compaction_source(&manifest, "model-a", "full-current-dag")?;

    // @step When the compaction evidence builds its validation context
    let context = CompactionEvidence::capture(source)?.validation_context();

    // @step Then the context requires that authoritative file identity
    assert_eq!(
        context.required_files.get("src/lib.rs").map(String::as_str),
        Some("Modified")
    );
    assert_eq!(context.persisted_turn_count, 2);

    // @step And a candidate whose file ledger omits it is rejected with "required file identity changed or missing"
    let candidate =
        "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Edited the library</dag-node>\n\
                     <dag-files>\n- src/main.rs (Created)\n</dag-files>";
    let result = validate_dag_candidate(candidate, &context);
    assert!(
        matches!(&result, Err(DagValidationError::InvalidCandidate(message))
            if message.contains("required file identity changed or missing")),
        "{result:?}"
    );
    Ok(())
}
