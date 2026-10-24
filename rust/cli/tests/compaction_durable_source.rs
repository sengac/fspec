use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_providers::{ProviderManager, ProviderType};
use rig::message::Message;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

struct Output;

impl codelet_cli::interactive::StreamOutput for Output {
    fn emit(&self, _event: codelet_cli::interactive::StreamEvent) {}
}

#[tokio::test]
#[serial_test::serial]
async fn terminal_builder_receives_the_exact_current_durable_source(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("source", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "immutable evidence")?;
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
    manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    session.messages = vec![Message::user("Explain <!-- type:compaction-dag -->")];
    let source = session.capture_durable_compaction(manifest.id)?;
    assert_eq!(source.prefix.len(), 1);

    let durable =
        r#"<dag-node depth="D2" turns="0-0" label="decision">Keep exact bytes.</dag-node>"#;
    manifest = update_session_manifest(manifest.id, |current| {
        current.compaction = Some(CompactionState {
            summary: durable.to_string(),
            compacted_before_index: 1,
            compacted_at: chrono::Utc::now(),
        });
        Ok(())
    })?;
    append_message(&mut manifest, "user", "new evidence after the previous DAG")?;
    let (source, existing) = session.capture_durable_compaction_state(manifest.id)?;
    assert_eq!(source.prefix.len(), 2);
    let expected = source.clone();
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        manifest.id,
        Some(Arc::new(move |target, captured| {
            assert_eq!(target, expected.session_id);
            assert_eq!(captured, expected);
            Box::pin(async { Err("verified exact captured source".to_string()) })
        })),
    );
    let result = codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round(
        &mut session,
        manifest.id,
        &Arc::new(Mutex::new(codelet_core::TokenState::default())),
        &Arc::new(AtomicBool::new(false)),
        &Output,
    )
    .await;
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(manifest.id, None);
    assert!(result
        .expect_err("the fixture deliberately stops after inspecting the captured source")
        .to_string()
        .contains("verified exact captured source"));
    Ok(())
}
