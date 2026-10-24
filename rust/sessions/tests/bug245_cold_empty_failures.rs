#![doc = "Feature: spec/features/cold-empty-failure-display.feature"]

#[path = "bug245/support.rs"]
mod support;

use codelet_agent_loop::{persist::persist_user_message, BackgroundOutput};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::{get_session_message_envelopes, load_session};
use codelet_rpc_types::StreamChunk;
use support::{replay::*, *};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn cold_display_preserves_consecutive_empty_provider_failures_in_request_order() -> TestResult
{
    let scenario = "cold_display_preserves_consecutive_empty_provider_failures_in_request_order";
    let Some(id) = cold_process_session()? else {
        // @step Given two requests fail without assistant output before a third request succeeds
        let (directory, manager, id, _) = manager().await?;
        let session = manager.get_session(&id.value)?;
        for input in ["first empty request", "second empty request"] {
            persist_user_message(&session.id, input)?;
            let output = BackgroundOutput::with_provider(session.clone(), "anthropic".into(), None);
            output.emit(StreamEvent::Error(
                "private provider details sk-secret".into(),
            ));
        }
        persist_user_message(&session.id, "third successful request")?;
        let output = BackgroundOutput::with_provider(session.clone(), "anthropic".into(), None);
        output.emit(StreamEvent::Text("successful original response".into()));
        output.emit(StreamEvent::Done(Some("end_turn".into())));
        drop(output);
        drop(session);
        drop(manager);
        // @step When fresh session managers repeatedly restore the archived conversation
        run_cold_process(directory.path(), &id, scenario).await?;
        return run_cold_process(directory.path(), &id, scenario).await;
    };
    let uuid = uuid::Uuid::parse_str(&id.value)?;
    let before = serde_json::to_value(load_session(uuid)?)?;
    let manager = cold_resume(&id)?;
    let session = manager.get_session(&id.value)?;
    let chunks = transcript(&manager, &id)?;
    assert_eq!(
        serde_json::to_value(&chunks)?,
        serde_json::to_value(transcript(&manager, &id)?)?
    );
    let order = chunks
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::UserInput { text } | StreamChunk::Text { text, .. } => Some(text.as_str()),
            StreamChunk::Error { error } => Some(error.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    // @step Then each safe failure appears once after its input and before the next request
    assert_eq!(
        order,
        [
            "first empty request",
            "Provider request failed.",
            "second empty request",
            "Provider request failed.",
            "third successful request",
            "successful original response",
        ]
    );
    // @step And the successful response appears once without private anchors or failure prose in model history
    let model = get_session_message_envelopes(uuid)?.join("");
    assert_eq!(model.matches("successful original response").count(), 1);
    for private in [
        "_transcript",
        "Provider request failed",
        "sk-secret",
        "RequestStarted",
    ] {
        assert!(!model.contains(private));
        assert!(!serde_json::to_string(&session.inner.lock().await.messages)?.contains(private));
    }
    assert_eq!(serde_json::to_value(load_session(uuid)?)?, before);
    Ok(())
}
