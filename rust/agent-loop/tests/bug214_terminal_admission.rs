//! Feature: spec/features/terminal-compaction-admission-cancellation.feature

#[path = "../../cli/tests/bug214/model.rs"]
mod model;
#[path = "support/terminal_recovery.rs"]
pub mod support;

use codelet_agent_loop::terminal_overflow_recovery::{
    try_terminal_overflow_recovery, TerminalRecoveryError,
};
use codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler;
use codelet_core::persistence::load_session;
use codelet_providers::{
    request_admission::AdmissionLimits, selected_request::SelectedRequestPreparer,
};
use rig::{client::CompletionClient, completion::CompletionRequestBuilder};
use std::sync::Arc;

#[tokio::test]
#[serial_test::serial]
async fn scenario_terminal_recovery_vetoes_cancellation_during_replacement_admission(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given terminal recovery has a valid builder candidate and unchanged durable history
    let (_directory, session, _inputs) = support::fixture();
    let original = serde_json::to_value(load_session(session.id)?)?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .build()?;
    let model = model::InterruptingModel {
        inner: client.completion_model("claude-sonnet-4-6"),
        interrupted: session.is_interrupted.clone(),
        during_admission: true,
    };
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    session
        .inner
        .lock()
        .await
        .install_selected_preparer(SelectedRequestPreparer::new(
            model,
            template,
            AdmissionLimits {
                context_capacity: Some(200_000),
                output_reservation: Some(4096),
            },
        ));
    set_compactor_sub_agent_handler(
        session.id,
        Some(Arc::new(|_, _| {
            Box::pin(async {
                Ok(
                    "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Original task.</dag-node>"
                        .into(),
                )
            })
        })),
    );
    let mut chunks = session.subscribe_to_stream();
    // @step When the user interrupts while the replacement request is serialized
    let result = try_terminal_overflow_recovery(session.clone(), &support::overflow_error()).await;
    set_compactor_sub_agent_handler(session.id, None);
    // @step Then terminal recovery returns cancellation without a promoted generation
    assert!(
        matches!(result, Err(TerminalRecoveryError::Cancelled)),
        "{result:?}"
    );
    assert_eq!(serde_json::to_value(load_session(session.id)?)?, original);
    // @step And original context remains available without compaction completion
    assert!(session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    assert!(!session
        .compaction_in_progress
        .load(std::sync::atomic::Ordering::Acquire));
    while let Ok(chunk) = chunks.try_recv() {
        assert!(!matches!(
            chunk,
            codelet_rpc_types::StreamChunk::CompactionComplete { .. }
        ));
    }
    Ok(())
}
