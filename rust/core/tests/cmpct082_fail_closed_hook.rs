//! Feature: spec/features/core-admission-hook-cancellation.feature

use codelet_core::{CompactionHook, TokenState};
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::message::Message;
use rig::providers::openai::responses_api::{
    streaming::StreamingCompletionResponse, ResponsesCompletionModel,
};
use std::sync::{Arc, Mutex};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn inaccessible_admission_state_cancels_before_request_and_at_finish() -> TestResult {
    // @step Given a core streaming hook whose admission state lock is inaccessible
    let state = Arc::new(Mutex::new(TokenState::default()));
    let inaccessible = state.clone();
    let stopped = std::thread::spawn(move || {
        let Ok(_guard) = inaccessible.lock() else {
            return;
        };
        std::panic::resume_unwind(Box::new("scripted admission owner termination"));
    })
    .join();
    assert!(stopped.is_err());
    let hook = CompactionHook::new(state, 9000);
    let prompt = Message::user("inspect");
    let signal = CancelSignal::new();
    // @step When request and response finish hooks check coherent and inconsistent usage
    <CompactionHook as StreamingPromptHook<ResponsesCompletionModel>>::on_completion_call(
        &hook,
        &prompt,
        &[],
        signal.clone(),
    )
    .await;
    let before_request_cancelled = signal.is_cancelled();
    let mut finish_cancelled = Vec::new();
    for usage in [
        serde_json::json!({"input_tokens":10,"output_tokens":1,"total_tokens":11,
            "output_tokens_details":{"reasoning_tokens":0}}),
        serde_json::json!({"input_tokens":10,"output_tokens":1,"total_tokens":1,
            "output_tokens_details":{"reasoning_tokens":0}}),
    ] {
        let response: StreamingCompletionResponse =
            serde_json::from_value(serde_json::json!({"usage":usage}))?;
        let signal = CancelSignal::new();
        <CompactionHook as StreamingPromptHook<ResponsesCompletionModel>>::on_stream_completion_response_finish(
            &hook, &prompt, &response, signal.clone(),
        )
        .await;
        finish_cancelled.push(signal.is_cancelled());
    }
    // @step Then every hook phase cancels instead of authorizing inference
    assert!(
        before_request_cancelled && finish_cancelled.iter().all(|cancelled| *cancelled),
        "unavailable state must cancel before send and finish with valid/invalid usage: before={before_request_cancelled}, finish={finish_cancelled:?}"
    );
    Ok(())
}
