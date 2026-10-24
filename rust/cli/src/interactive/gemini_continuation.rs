//! Gemini empty-after-tool continuation handling.

mod continuation_loop;
mod state;

use super::output::StreamOutput;
use super::stream_handlers::handle_final_response;
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_core::{
    ensure_thought_signatures, ApiTokenUsage, CompactionHook, ContinuationStrategy,
    GeminiTurnCompletionFacade, RigAgent, StreamingTokenDisplay, TokenState, TurnCompletionFacade,
};
use codelet_tui::InputQueue;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage, Usage};
use rig::wasm_compat::WasmCompatSend;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing::info;
use uuid::Uuid;

type BoxedStream<'a, Response> = Pin<
    Box<
        dyn futures::Stream<
                Item = std::result::Result<MultiTurnStreamItem<Response>, anyhow::Error>,
            > + Send
            + 'a,
    >,
>;

pub(super) enum GeminiContinuationResult {
    NoContinuation,
    Completed {
        text: String,
        display: StreamingTokenDisplay,
        usage: Usage,
        tool_calls: usize,
        stop_reason: Option<String>,
    },
    Finished {
        display: StreamingTokenDisplay,
    },
    CompactionNeeded(super::recovery_compaction::CompactionRecoveryPolicy),
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_gemini_continuation<M, O>(
    agent: &RigAgent<M>,
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    token_state: &Arc<Mutex<TokenState>>,
    threshold: u64,
    assistant_text: &str,
    streaming_display: &StreamingTokenDisplay,
    session_id: Uuid,
) -> Result<GeminiContinuationResult>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    let model_id = session.current_model_id().unwrap_or_default();
    let turn_completion = GeminiTurnCompletionFacade;
    if session.current_provider_name() != "gemini"
        || !turn_completion.requires_turn_completion_check(&model_id)
    {
        return Ok(GeminiContinuationResult::NoContinuation);
    }
    let ContinuationStrategy::FullLoop {
        prompt: continuation_prompt,
    } = turn_completion.continuation_strategy(assistant_text, &session.messages)
    else {
        return Ok(GeminiContinuationResult::NoContinuation);
    };

    info!("GEMINI-TURN: Empty response after tool call detected for model {model_id}");
    capture_continuation_event(&model_id, continuation_prompt, "empty_response_after_tool");
    handle_final_response(assistant_text, &mut session.messages)?;
    append_continuation_prompt(session, continuation_prompt);

    let current_display = streaming_display.current();
    let turn_usage = ApiTokenUsage::new(
        current_display.input_tokens,
        current_display.cache_read_tokens,
        current_display.cache_creation_tokens,
        0,
    )
    .with_reasoning_tokens(current_display.reasoning_tokens);
    session
        .token_tracker
        .update_display_only(&turn_usage, current_display.output_tokens);

    let continuation_token_state = Arc::new(Mutex::new(TokenState::from_cache_inclusive_total(
        session.token_tracker.input_tokens,
        current_display.output_tokens,
    )));
    let continuation_hook = CompactionHook::new(Arc::clone(&continuation_token_state), threshold);
    let raw_stream = agent
        .prompt_streaming_with_history_and_hook(
            continuation_prompt,
            &mut session.messages,
            continuation_hook,
        )
        .await;
    let mut boxed_stream: BoxedStream<'_, M::StreamingResponse> = Box::pin(raw_stream);
    let display = StreamingTokenDisplay::new(
        current_display.input_tokens,
        current_display.output_tokens,
        current_display.cache_read_tokens,
        current_display.cache_creation_tokens,
    )
    .with_prev_reasoning(current_display.reasoning_tokens);
    let state = state::ContinuationState::new(display, continuation_token_state);

    continuation_loop::run_continuation_loop(
        agent,
        session,
        output,
        is_interrupted,
        input_queue,
        token_state,
        threshold,
        &model_id,
        &mut boxed_stream,
        state,
        session_id,
    )
    .await
}

fn append_continuation_prompt(session: &mut Session, prompt: &str) {
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text(prompt)),
    });
    if let Some(model_id) = session.current_model_id() {
        ensure_thought_signatures(&mut session.messages, &model_id);
    }
}

fn capture_continuation_event(model_id: &str, prompt: &str, reason: &str) {
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "gemini.continuation",
                    serde_json::json!({
                        "reason": reason,
                        "strategy": "FullLoop",
                        "model": model_id,
                        "prompt": prompt,
                    }),
                    None,
                );
            }
        }
    }
}
