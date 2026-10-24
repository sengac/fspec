#![doc = "Feature: spec/features/background-response-attempt-rollback.feature"]

use std::sync::{atomic::Ordering, Arc};

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::{
    interactive::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent},
    session::Session,
};
use codelet_core::persistence::{
    create_session_with_provider, get_session_message_envelopes, reset_stores_for_tests,
};
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use tokio::sync::broadcast;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn session(
    id: uuid::Uuid,
    chunks: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>,
) -> Arc<BackgroundSession> {
    Arc::new(BackgroundSession::new(
        id,
        "rollback".into(),
        "/test".into(),
        Some("codex".into()),
        Some("shared-model".into()),
        Session::from_provider_manager(ProviderManager::for_testing(
            ProviderType::Codex,
            Some(272_000),
            Some(4096),
        )),
        tokio::sync::mpsc::channel(1).0,
        None,
        None,
        None,
        chunks,
        broadcast::channel(32).0,
        broadcast::channel(32).0,
    ))
}

/// @step Given earlier completed assistant output and a tool result
/// @step When a later response streams text and reasoning and is aborted
/// @step And a replacement response completes
/// @step Then persistence contains earlier output and tools and only replacement output
/// @step And subscribers receive attempt boundaries alongside live deltas
#[tokio::test]
#[serial_test::serial]
#[serial_test::serial]
async fn background_persistence_excludes_abandoned_text_and_reasoning() -> TestResult {
    let directory = setup_temp_directory()?;
    reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let manifest = create_session_with_provider("rollback", directory.path(), "codex")?;
    let (sender, mut receiver) = broadcast::channel(64);
    let background = session(manifest.id, sender);
    let output = BackgroundOutput::with_provider(background, "codex".into(), None);
    output.emit(StreamEvent::ResponseAttemptStarted);
    output.emit(StreamEvent::Text("earlier answer".into()));
    output.emit(StreamEvent::Thinking("earlier reasoning".into()));
    output.emit(StreamEvent::ToolCall(ToolCallEvent {
        id: "tool".into(),
        call_id: Some("call_tool".into()),
        name: "Read".into(),
        args: json!({}),
    }));
    output.emit(StreamEvent::ToolResult(ToolResultEvent {
        id: "tool".into(),
        call_id: Some("call_tool".into()),
        content: "completed tool result".into(),
        is_error: false,
    }));
    output.emit(StreamEvent::ResponseAttemptStarted);
    output.emit(StreamEvent::Text("completed continuation".into()));
    output.emit(StreamEvent::Thinking(
        "completed continuation reasoning".into(),
    ));
    output.emit(StreamEvent::ResponseAttemptStarted);
    output.emit(StreamEvent::Text("abandoned text".into()));
    output.emit(StreamEvent::Thinking("abandoned reasoning".into()));
    let mut live = Vec::new();
    while let Ok((_, chunk)) = receiver.try_recv() {
        live.push(chunk);
    }
    assert!(live
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Text { text, .. } if text == "abandoned text")));
    assert_eq!(
        live.iter()
            .filter(|chunk| matches!(chunk, StreamChunk::ResponseAttemptStarted))
            .count(),
        3
    );
    output.emit(StreamEvent::ResponseAttemptAborted);
    assert!(matches!(
        receiver.try_recv()?.1,
        StreamChunk::ResponseAttemptAborted
    ));
    output.emit(StreamEvent::ResponseAttemptStarted);
    output.emit(StreamEvent::Text("replacement answer".into()));
    output.emit(StreamEvent::Thinking("replacement reasoning".into()));
    output.emit(StreamEvent::Done(Some("end_turn".into())));
    let messages: Vec<Value> = get_session_message_envelopes(manifest.id)?
        .into_iter()
        .map(|message| serde_json::from_str(&message))
        .collect::<Result<_, _>>()?;
    let encoded = serde_json::to_string(&messages)?;
    for expected in [
        "earlier answer",
        "earlier reasoning",
        "completed tool result",
        "call_tool",
        "completed continuation",
        "completed continuation reasoning",
        "replacement answer",
        "replacement reasoning",
    ] {
        assert!(encoded.contains(expected), "missing {expected}: {encoded}");
    }
    assert!(!encoded.contains("abandoned"), "{encoded}");
    assert_eq!(messages.len(), 3);
    reset_stores_for_tests();
    Ok(())
}

/// @step Given a response attempt has nearly triggered repetition detection
/// @step When the attempt is aborted and replacement output repeats its words
/// @step Then the replacement is not interrupted by abandoned detector state
#[tokio::test]
async fn aborted_output_does_not_poison_loop_detection() -> TestResult {
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let (sender, _) = broadcast::channel(64);
    let background = session(uuid::Uuid::new_v4(), sender);
    let output = BackgroundOutput::with_provider(background.clone(), "codex".into(), None);
    output.emit(StreamEvent::ResponseAttemptStarted);
    output.emit(StreamEvent::Text("one two three ".repeat(9)));
    output.emit(StreamEvent::Thinking("four five six ".repeat(9)));
    assert!(!background.is_interrupted.load(Ordering::Acquire));
    output.emit(StreamEvent::ResponseAttemptAborted);
    output.emit(StreamEvent::ResponseAttemptStarted);
    for _ in 0..2 {
        output.emit(StreamEvent::Text("one two three ".into()));
        output.emit(StreamEvent::Thinking("four five six ".into()));
    }
    assert!(!background.is_interrupted.load(Ordering::Acquire));
    Ok(())
}

/// @step Given response attempt start and abort chunks
/// @step When the chunks are serialized and deserialized and converted for a bridge
/// @step Then both unit boundaries retain their identity
#[test]
fn attempt_boundaries_survive_rpc_serialization_and_bridge_conversion() -> TestResult {
    for (chunk, kind) in [
        (
            StreamChunk::ResponseAttemptStarted,
            "responseAttemptStarted",
        ),
        (
            StreamChunk::ResponseAttemptAborted,
            "responseAttemptAborted",
        ),
    ] {
        let decoded: StreamChunk = serde_json::from_str(&serde_json::to_string(&chunk)?)?;
        let bridge = codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(&decoded);
        assert_eq!(bridge, json!({"type": kind}));
    }
    Ok(())
}
