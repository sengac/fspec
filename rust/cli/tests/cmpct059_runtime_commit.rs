//! Feature: spec/features/durable-versioned-dag-boundary.feature
use codelet_cli::session::Session;
use codelet_core::persistence::compaction_commit::CompactionCommitError;
use codelet_core::persistence::*;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::message::Message;
use uuid::Uuid;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>\n<dag-files>\n</dag-files>";

fn runtime() -> Result<(Session, SelectedRequestPreparer), Box<dyn std::error::Error>> {
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
    manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    let session = Session::from_provider_manager(manager);
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline-no-send")
            .build()?;
    let model = client.completion_model("o3");
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .preamble("mandatory instructions".into())
        .max_tokens(4096)
        .build();
    Ok((
        session,
        SelectedRequestPreparer::new(
            model,
            template,
            AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            },
        ),
    ))
}

#[test]
#[serial_test::serial]
fn runtime_installation_reconciles_only_the_durable_promoted_generation() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("runtime", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let (mut session, preparer) = runtime()?;
    session.messages.push(Message::user("covered original"));
    // @step Given a captured durable source and a real selected request serializer
    let source = session.capture_durable_compaction(manifest.id)?;
    // @step And additional structured tool work arrives after capture
    append_message_with_metadata(
        &mut manifest,
        "assistant",
        "",
        std::collections::HashMap::from([(
            "message".into(),
            serde_json::json!({"role":"assistant","content":[{"type":"tool_use","id":"tool-1","name":"read","input":{"path":"exact"}}]}),
        )]),
    )?;
    // @step When the runtime commits and publishes the candidate
    let committed = session.commit_durable_compaction(
        source,
        Uuid::new_v4(),
        DAG,
        |messages, generation| preparer.prepare(messages, generation),
        |_| Ok(()),
    )?;
    // @step Then its installed DAG suffix provider state and token count match the durable generation
    let boundary = committed.compaction_commit.as_ref().ok_or("boundary")?;
    assert_eq!(boundary.identity.generation, 1);
    assert!(!boundary.publication_pending);
    assert_eq!(boundary.verified_suffix.len(), 1);
    assert_eq!(session.token_tracker.input_tokens, boundary.input_tokens);
    let installed = serde_json::to_string(&session.messages)?;
    assert!(installed.contains("exact DAG"));
    assert!(installed.contains("tool-1"));
    assert!(!installed.contains("covered original"));
    // @step And ordinary inference opens only after exact identity reconciliation
    session.ensure_compaction_inference_ready()?;
    reset_stores_for_tests();
    let durable = load_session(manifest.id)?;
    assert_eq!(durable.compaction_commit, committed.compaction_commit);
    Ok(())
}

#[test]
#[serial_test::serial]
fn runtime_rejects_a_candidate_that_drops_authoritative_file_identity() -> TestResult {
    // @step Given a captured durable source with an authoritative file modification
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("file obligations", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let (mut session, preparer) = runtime()?;
    session.annotations.insert(
        0,
        vec![
            codelet_core::compaction::StructuralAnnotation::FileModification {
                path: "src/critical.rs".into(),
                operation: codelet_core::compaction::FileOp::Modified,
            },
        ],
    );
    let source = session.capture_durable_compaction(manifest.id)?;
    let mut published = false;
    // @step When a candidate explicitly supplies an empty file ledger
    let result = session.commit_durable_compaction(
        source,
        Uuid::new_v4(),
        DAG,
        |messages, generation| preparer.prepare(messages, generation),
        |_| {
            published = true;
            Ok(())
        },
    );
    // @step Then the runtime rejects it before promotion without completion
    let failure = result
        .err()
        .ok_or("an explicit empty ledger cannot discard a known file")?;
    assert!(failure.to_string().contains("file identity"), "{failure}");
    assert!(!published);
    assert!(load_session(manifest.id)?.compaction_commit.is_none());
    assert!(session.ensure_compaction_inference_ready().is_err());
    Ok(())
}

#[test]
#[serial_test::serial]
fn runtime_assembles_missing_file_identity_and_retains_mandatory_reminders() -> TestResult {
    // @step Given a captured durable source with an authoritative file modification and mandatory reminder
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest =
        create_session_with_provider("retained obligations", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let (mut session, preparer) = runtime()?;
    session.annotations.insert(
        0,
        vec![
            codelet_core::compaction::StructuralAnnotation::FileModification {
                path: "src/critical.rs".into(),
                operation: codelet_core::compaction::FileOp::Modified,
            },
        ],
    );
    session.add_system_reminder(
        codelet_cli::session::SystemReminderType::SystemPrompt,
        "mandatory exact policy",
    );
    let reminder = serde_json::to_value(&session.messages[0])?;
    let source = session.capture_durable_compaction(manifest.id)?;
    let candidate = DAG.split("\n<dag-files>").next().ok_or("DAG node")?;
    // @step When a candidate omits its file ledger
    let committed = session.commit_durable_compaction(
        source,
        Uuid::new_v4(),
        candidate,
        |messages, generation| preparer.prepare(messages, generation),
        |_| Ok(()),
    )?;
    // @step Then the admitted generation contains the authoritative file identity
    assert!(committed
        .compaction
        .ok_or("committed DAG")?
        .summary
        .contains("- src/critical.rs (Modified)"));
    // @step And the exact mandatory reminder remains installed
    assert_eq!(serde_json::to_value(&session.messages[0])?, reminder);
    Ok(())
}

#[test]
#[serial_test::serial]
fn runtime_publication_failure_cannot_authorize_ordinary_inference() -> TestResult {
    // @step Given a captured durable source and a real selected request serializer
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("runtime failure", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let (mut session, preparer) = runtime()?;
    let source = session.capture_durable_compaction(manifest.id)?;
    // @step When runtime publication fails after durable promotion
    let result = session.commit_durable_compaction(
        source,
        Uuid::new_v4(),
        DAG,
        |messages, generation| preparer.prepare(messages, generation),
        |_| Err("disconnected".into()),
    );
    let CompactionCommitError::Publication { identity, .. } =
        result.err().ok_or("publication fails")?
    else {
        return Err("expected promoted identity".into());
    };
    // @step Then the new identity remains authoritative and ordinary inference stays blocked
    assert!(session.ensure_compaction_inference_ready().is_err());
    // @step And an incorrect reconciliation identity cannot clear that block
    let mut wrong = identity.clone();
    wrong.operation_id = Uuid::new_v4();
    assert!(session.reconcile_durable_compaction(&wrong).is_err());
    assert!(session.ensure_compaction_inference_ready().is_err());
    session.reconcile_durable_compaction(&identity)?;
    session.ensure_compaction_inference_ready()?;
    assert_eq!(
        load_session(manifest.id)?
            .compaction_commit
            .ok_or("boundary")?
            .identity,
        identity
    );
    Ok(())
}
