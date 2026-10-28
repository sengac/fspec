//! Feature: spec/features/compaction-cancellation-state-ownership.feature

use codelet_cli::interactive::{
    begin_compaction_recovery, classify_compaction_branch, retry_compaction_hook, CompactionBranch,
    CompactionDisagreement, StreamEvent, StreamOutput,
};
use codelet_core::TokenState;
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::PromptError;
use rig::streaming::StreamingPrompt;
use std::sync::{Arc, Mutex};

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
async fn scenario_retry_threshold_cancellation_agrees_with_current_turn_recovery(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a current turn and an independent session with separate token states
    let current = Arc::new(Mutex::new(TokenState::default()));
    let independent = Arc::new(Mutex::new(TokenState::default()));
    let server = wiremock::MockServer::start().await;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline-synthetic")
            .base_url(server.uri())
            .build()?;
    let agent = client.agent("o3").build();
    // @step When a refreshed retry stream crosses its compaction threshold
    let hook = retry_compaction_hook(&current, 190_847, 0, 190_000);
    assert!(Arc::ptr_eq(&hook.state, &current));
    let stream = agent.stream_prompt("continue").with_hook(hook).await;
    futures::pin_mut!(stream);
    let failure = loop {
        match stream.next().await {
            Some(Err(error)) => break anyhow::Error::from(error).context("nested stream"),
            Some(Ok(_)) => continue,
            None => break anyhow::Error::msg("missing cancellation"),
        }
    };
    // @step Then nested typed cancellation recovers without a missing-intent disagreement
    assert_eq!(
        classify_compaction_branch(&failure, &current),
        CompactionBranch::Recover { disagreement: None }
    );
    assert!(
        current
            .lock()
            .map_err(|error| error.to_string())?
            .compaction_needed
    );
    let mut session = codelet_cli::session::Session::from_provider_manager(
        codelet_providers::ProviderManager::for_testing(
            codelet_providers::ProviderType::OpenAI,
            Some(200_000),
            Some(4096),
        ),
    );
    let output = Output::default();
    begin_compaction_recovery(
        &mut session,
        &current,
        &codelet_core::StreamingTokenDisplay::new(190_847, 0, 0, 0),
        &mut String::new(),
        &output,
        true,
    )?;
    assert_eq!(
        output
            .0
            .lock()
            .map_err(|error| error.to_string())?
            .iter()
            .filter(|event| matches!(event, StreamEvent::CompactionStarted))
            .count(),
        1
    );
    assert!(server
        .received_requests()
        .await
        .ok_or("capture unavailable")?
        .is_empty());
    // @step And the independent session remains unchanged
    assert!(
        !independent
            .lock()
            .map_err(|error| error.to_string())?
            .compaction_needed
    );
    Ok(())
}

#[test]
fn scenario_refreshing_retry_usage_preserves_pending_compaction_intent(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given the current turn already requires compaction
    let current = Arc::new(Mutex::new(TokenState {
        compaction_needed: true,
        ..Default::default()
    }));
    // @step When retry or continuation usage is refreshed
    for (input, output) in [(100, 0), (200, 30), (300, 40)] {
        let hook = retry_compaction_hook(&current, input, output, 190_000);
        // @step Then the shared allocation and pending intent remain intact
        assert!(Arc::ptr_eq(&hook.state, &current));
        let state = current.lock().map_err(|error| error.to_string())?;
        assert_eq!(state.input_tokens, input);
        assert_eq!(state.output_tokens, output);
        assert_eq!(state.cache_read_input_tokens, 0);
        assert!(state.compaction_needed);
    }
    Ok(())
}

#[test]
fn scenario_missing_intent_still_receives_defensive_typed_recovery(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a typed cancellation whose current turn intent was reset
    let current = Arc::new(Mutex::new(TokenState::default()));
    let failure = anyhow::Error::from(PromptError::PromptCancelled {
        chat_history: Box::default(),
    })
    .context("nested cancellation");
    // @step When recovery classifies the cancellation
    let branch = classify_compaction_branch(&failure, &current);
    // @step Then intent is repaired and the mismatch remains observable
    assert_eq!(
        branch,
        CompactionBranch::Recover {
            disagreement: Some(CompactionDisagreement::FlagMissing)
        }
    );
    assert!(
        current
            .lock()
            .map_err(|error| error.to_string())?
            .compaction_needed
    );
    // @step And unrelated errors do not authorize compaction
    assert!(matches!(
        classify_compaction_branch(&anyhow::Error::msg("user interrupted"), &current),
        CompactionBranch::NotCompaction { .. }
    ));
    Ok(())
}
