//! Feature: spec/features/missing-limits-before-compaction.feature

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::request_admission::RequestAdmissionError;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::message::Message;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use wiremock::MockServer;

#[derive(Default)]
struct Output(Mutex<Vec<StreamEvent>>);

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn unavailable_or_unusable_model_limits_fail_before_streaming_changes_history(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let server = MockServer::start().await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .base_url(server.uri())
            .build()?;
    for (context, output, expected) in [
        (None, None, RequestAdmissionError::MissingContextCapacity),
        (
            Some(200_000),
            None,
            RequestAdmissionError::MissingOutputReservation,
        ),
        (
            Some(4096),
            Some(4096),
            RequestAdmissionError::InvalidLimits {
                capacity: 4096,
                reservation: 4096,
            },
        ),
    ] {
        for existing in [false, true] {
            let mut session = Session::from_provider_manager(ProviderManager::for_testing(
                ProviderType::Codex,
                context,
                output,
            ));
            if existing {
                session.messages = vec![
                    Message::user("original question"),
                    Message::assistant("original evidence"),
                ];
                session.token_tracker.input_tokens = 3000;
                session.token_tracker.cumulative_billed_input = 5000;
            }
            let messages = serde_json::to_value(&session.messages)?;
            let tokens = serde_json::to_value(&session.token_tracker)?;
            let busy = Arc::new(AtomicBool::new(false));
            let events = Output::default();
            let error = run_agent_stream(
                RigAgent::new(client.agent("gpt-6.1-sol").build(), 1),
                "review the last commit",
                &mut session,
                Arc::new(AtomicBool::new(false)),
                busy.clone(),
                Arc::new(tokio::sync::Notify::new()),
                &events,
                uuid::Uuid::new_v4(),
            )
            .await
            .expect_err("unavailable limits must reject the prompt");
            assert_eq!(
                error.downcast_ref::<RequestAdmissionError>(),
                Some(&expected),
                "existing={existing}: {error:#}"
            );
            assert_eq!(serde_json::to_value(&session.messages)?, messages);
            assert_eq!(serde_json::to_value(&session.token_tracker)?, tokens);
            assert!(!busy.load(std::sync::atomic::Ordering::Acquire));
            assert!(!session.has_compaction_working_view());
            assert!(session.ensure_compaction_inference_ready().is_ok());
            let events = events.0.lock().map_err(|error| error.to_string())?;
            assert!(!events.iter().any(|event| matches!(
                event,
                StreamEvent::CompactionStarted
                    | StreamEvent::CompactionProgress(_)
                    | StreamEvent::CompactionComplete(_)
                    | StreamEvent::CompactionFailed { .. }
                    | StreamEvent::CompactionContinuing
            )));
        }
    }
    assert!(server
        .received_requests()
        .await
        .ok_or("requests unavailable")?
        .is_empty());
    Ok(())
}
