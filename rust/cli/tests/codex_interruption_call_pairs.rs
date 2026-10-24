#![doc = "Feature: spec/features/codex-interruption-outcome-recording.feature"]

#[path = "fixtures/committed_interruption.rs"]
mod committed_interruption;
#[path = "fixtures/interruption_tool.rs"]
mod interruption_tool;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::{create_session_with_provider, reset_stores_for_tests};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_test_helpers::setup_temp_directory;
use rig::message::{AssistantContent, Message, ToolResultContent, UserContent};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::{Mutex, PoisonError};
use transport::TestResult;

#[derive(Default)]
struct RecordingOutput(Mutex<Vec<StreamEvent>>);

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

/// @step Given a committed batch contains distinct function calls with a namespaced alias and a namespaced custom patch call
/// @step And zero or one tools have completed before the next tool starts running
/// @step When I interrupt the streaming consumer
/// @step Then each committed call is published exactly once before its matching outcome
/// @step And unpublished calls retain their item and invocation identities and normalized names and inputs
/// @step And completed outcomes remain unchanged
/// @step And conversational history contains every committed call and outcome
/// @step And the next request succeeds with the original invocation identities
#[tokio::test]
#[serial_test::serial]
async fn interrupted_batches_publish_missing_committed_calls_before_their_outcomes() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    for completed in [0, 1] {
        let manifest = create_session_with_provider("batch pairs", directory.path(), "codex")?;
        let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
        let mut session = Session::from_provider_manager(manager);
        let batch = committed_interruption::Batch::new(&session, completed).await?;
        let output = RecordingOutput::default();
        batch.run(&mut session, manifest.id, &output).await?;
        assert_eq!(batch.execution.completed.load(Ordering::SeqCst), completed);
        assert_eq!(
            batch.execution.started.load(Ordering::SeqCst),
            completed + 1
        );
        committed_interruption::assert_pairs(
            &output.0.lock().map_err(|_| "events poisoned")?,
            completed,
        );
        let calls: Vec<_> = session
            .messages
            .iter()
            .filter_map(|message| match message {
                Message::Assistant { content, .. } => Some(content.iter()),
                _ => None,
            })
            .flatten()
            .filter_map(|content| match content {
                AssistantContent::ToolCall(call) => Some(call),
                _ => None,
            })
            .collect();
        let results: Vec<_> = session
            .messages
            .iter()
            .filter_map(|message| match message {
                Message::User { content } => Some(content.iter()),
                _ => None,
            })
            .flatten()
            .filter_map(|content| match content {
                UserContent::ToolResult(result) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 3);
        assert_eq!(results.len(), 3);
        for (index, (call, result)) in calls.iter().zip(results).enumerate() {
            assert_eq!(
                (call.function.name.as_str(), call.function.arguments.clone()),
                committed_interruption::expected(index)
            );
            assert_eq!(call.id, format!("item-{index}"));
            assert_eq!(
                call.call_id.as_deref(),
                Some(format!("call-{index}").as_str())
            );
            assert_eq!(result.id, call.id);
            assert_eq!(result.call_id, call.call_id);
            if index < completed {
                let text: String = result
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        ToolResultContent::Text(text) => Some(text.text.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    serde_json::from_str::<Value>(&text)?,
                    json!({"completed":true})
                );
            }
        }
        batch.assert_replay(session.messages.clone()).await?;
    }
    Ok(())
}
