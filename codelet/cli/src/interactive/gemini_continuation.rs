//! Gemini continuation handling.
//!
//! Extracts the Gemini continuation sub-loop from stream_loop.rs.
//! When Gemini returns an empty response after tool calls, a continuation
//! prompt is sent to nudge the model to respond with results.

use super::error_classifiers::{classify_compaction_branch, CompactionBranch};
use super::output::StreamOutput;
use super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};

use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_core::{
    ensure_thought_signatures, ApiTokenUsage, CompactionHook, ContinuationStrategy,
    GeminiTurnCompletionFacade, RigAgent, StreamingTokenDisplay, TokenState, TurnCompletionFacade,
};
use codelet_tools::set_tool_progress_callback;
use codelet_tui::InputQueue;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use rig::wasm_compat::WasmCompatSend;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Acquire;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// Type-erased stream for continuation loops.
type BoxedStream<'a, R> = Pin<
    Box<
        dyn futures::Stream<Item = std::result::Result<MultiTurnStreamItem<R>, anyhow::Error>>
            + Send
            + 'a,
    >,
>;

/// Result of a Gemini continuation attempt.
pub(super) enum GeminiContinuationResult {
    /// No continuation was needed — caller should proceed normally.
    NoContinuation,
    /// Continuation completed successfully — caller should return Ok(()).
    Completed,
    /// Compaction was triggered during continuation — caller should break to
    /// compaction. The payload carries the policy selected by
    /// `begin_compaction_recovery` so the primary loop can pick the correct
    /// retry prompt in its in-loop compaction restart (CMPCT-028).
    CompactionNeeded(super::recovery_compaction::CompactionRecoveryPolicy),
}

/// Check if Gemini needs a continuation and run the continuation loop if so.
///
/// Returns `GeminiContinuationResult` indicating what the caller should do.
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
    final_stop_reason: &mut Option<String>,
) -> Result<GeminiContinuationResult>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    let provider_name = session.current_provider_name();
    let model_id = session.current_model_id().unwrap_or_default();

    if provider_name != "gemini" {
        return Ok(GeminiContinuationResult::NoContinuation);
    }

    let turn_completion = GeminiTurnCompletionFacade;
    if !turn_completion.requires_turn_completion_check(&model_id) {
        return Ok(GeminiContinuationResult::NoContinuation);
    }

    let strategy = turn_completion.continuation_strategy(assistant_text, &session.messages);

    let continuation_prompt = match strategy {
        ContinuationStrategy::FullLoop { prompt } => prompt,
        _ => return Ok(GeminiContinuationResult::NoContinuation),
    };

    info!(
        "GEMINI-TURN: Empty response after tool call detected for model {}, using FullLoop strategy",
        model_id
    );

    capture_continuation_event(&model_id, continuation_prompt, "empty_response_after_tool");

    // Handle final response (add empty assistant text to history)
    handle_final_response(assistant_text, &mut session.messages)?;

    // Add the continuation as a new user message
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text(continuation_prompt)),
    });

    if let Some(model_id) = session.current_model_id() {
        ensure_thought_signatures(&mut session.messages, &model_id);
    }

    // Update display values before recursion
    let current_display = streaming_display.current();
    let turn_usage = ApiTokenUsage::new(
        current_display.input_tokens,
        current_display.cache_read_tokens,
        current_display.cache_creation_tokens,
        0,
    );
    session
        .token_tracker
        .update_display_only(&turn_usage, current_display.output_tokens);

    // CMPCT-042 invariant: `session.token_tracker.input_tokens` is
    // cache-INCLUSIVE after `update_display_only` above (PROV-001), so this
    // TRACKER-BASIS seed must route through the audited
    // `from_cache_inclusive_total` constructor which zeroes the cache fields
    // ('Don't double count', mirroring the turn-start seed in
    // stream_loop.rs). Feeding the display's non-zero cache fields alongside
    // the tracker total would inflate `TokenState::total()` and let the
    // CompactionHook trigger compaction early; pinned by
    // cmpct042_gemini_continuation_token_state_test.rs.
    let continuation_token_state = Arc::new(Mutex::new(TokenState::from_cache_inclusive_total(
        session.token_tracker.input_tokens,
        current_display.output_tokens,
    )));
    let continuation_hook = CompactionHook::new(Arc::clone(&continuation_token_state), threshold);

    debug!(
        "API REQUEST (Gemini continuation) - Provider: {}, Model: {}",
        session.current_provider_name(),
        session.current_model_id().as_deref().unwrap_or("NONE")
    );
    let raw_stream = agent
        .prompt_streaming_with_history_and_hook(
            continuation_prompt,
            &mut session.messages,
            continuation_hook,
        )
        .await;
    let mut boxed_stream: BoxedStream<'_, M::StreamingResponse> = Box::pin(raw_stream);

    let mut continuation_text = String::new();
    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
    let mut last_tool_name: Option<String> = None;
    // CMPCT-041 invariant: this is a DISPLAY-BASIS re-seed — `current_display`
    // is a TokenDisplayUpdate snapshot carrying raw input + separate cache
    // fields, so the raw `::new` constructor is correct here. It MUST NOT be
    // fed tracker cache-INCLUSIVE totals (those go through
    // `from_cache_inclusive_total`); pinned by the wiring guard in
    // cmpct041_seed_cache_double_count_test.rs.
    let mut display = StreamingTokenDisplay::new(
        current_display.input_tokens,
        current_display.output_tokens,
        current_display.cache_read_tokens,
        current_display.cache_creation_tokens,
    );

    let result = run_continuation_loop(
        agent,
        session,
        output,
        is_interrupted,
        input_queue,
        token_state,
        threshold,
        &turn_completion,
        &model_id,
        &mut boxed_stream,
        &mut continuation_text,
        &mut tool_calls_buffer,
        &mut last_tool_name,
        &mut display,
        final_stop_reason,
    )
    .await?;

    Ok(result)
}

/// Inner continuation loop that processes stream events.
#[allow(clippy::too_many_arguments)]
async fn run_continuation_loop<'a, M, O>(
    agent: &'a RigAgent<M>,
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    parent_token_state: &Arc<Mutex<TokenState>>,
    threshold: u64,
    turn_completion: &GeminiTurnCompletionFacade,
    model_id: &str,
    stream: &mut BoxedStream<'a, M::StreamingResponse>,
    text: &mut String,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &mut Option<String>,
    display: &mut StreamingTokenDisplay,
    final_stop_reason: &mut Option<String>,
) -> Result<GeminiContinuationResult>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    loop {
        if is_interrupted.load(Acquire) {
            let queued = if let Some(ref mut iq) = input_queue {
                iq.dequeue_all()
            } else {
                vec![]
            };
            output.emit_interrupted(&queued);
            if !text.is_empty() {
                handle_final_response(text, &mut session.messages)?;
            }
            update_token_tracker(session, display);
            set_tool_progress_callback(uuid::Uuid::nil(), None);
            output.emit_done_with_stop_reason(final_stop_reason.take());
            return Ok(GeminiContinuationResult::Completed);
        }

        match stream.next().await {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                chunk,
            )))) => {
                handle_text_chunk(&chunk.text, text, None, output)?;
                if let Some(update) = display.record_chunk(&chunk.text) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall(tool_call),
            ))) => {
                handle_tool_call(
                    &tool_call,
                    &mut session.messages,
                    text,
                    tool_calls_buffer,
                    last_tool_name,
                    output,
                )?;
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ReasoningDelta { reasoning, .. },
            ))) => {
                output.emit_thinking(&reasoning);
                if let Some(update) = display.record_chunk(&reasoning) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(
                tool_result,
            )))) => {
                handle_tool_result(
                    &tool_result,
                    &mut session.messages,
                    tool_calls_buffer,
                    last_tool_name,
                    output,
                )?;
            }
            Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                if usage.output_tokens == 0 {
                    display.start_new_segment(&usage);
                } else if let Some(update) = display.update_from_usage(&usage) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                let usage = final_resp.usage();
                let cont_final = if !display.has_authoritative_output() && usage.input_tokens > 0 {
                    display.update_from_final_response(&usage)
                } else {
                    display.current()
                };

                // Check if we need ANOTHER continuation (nested)
                let nested_strategy =
                    turn_completion.continuation_strategy(text, &session.messages);

                if let ContinuationStrategy::FullLoop {
                    prompt: nested_prompt,
                } = nested_strategy
                {
                    info!("GEMINI-TURN: Nested empty response detected, continuing again");
                    capture_continuation_event(
                        model_id,
                        nested_prompt,
                        "nested_empty_response_after_tool",
                    );

                    handle_final_response(text, &mut session.messages)?;
                    text.clear();

                    session.messages.push(rig::message::Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(
                            nested_prompt,
                        )),
                    });
                    if let Some(mid) = session.current_model_id() {
                        ensure_thought_signatures(&mut session.messages, &mid);
                    }

                    let nested_token_state = Arc::new(Mutex::new(TokenState {
                        input_tokens: cont_final.input_tokens,
                        cache_read_input_tokens: cont_final.cache_read_tokens,
                        cache_creation_input_tokens: cont_final.cache_creation_tokens,
                        output_tokens: cont_final.output_tokens,
                        compaction_needed: false,
                    }));
                    let nested_hook =
                        CompactionHook::new(Arc::clone(&nested_token_state), threshold);

                    debug!(
                        "API REQUEST (Gemini nested continuation) - Provider: {}, Model: {}",
                        session.current_provider_name(),
                        session.current_model_id().as_deref().unwrap_or("NONE")
                    );
                    let raw_nested = agent
                        .prompt_streaming_with_history_and_hook(
                            nested_prompt,
                            &mut session.messages,
                            nested_hook,
                        )
                        .await;
                    // Replace the boxed stream — type erased so this works
                    *stream = Box::pin(raw_nested);

                    tool_calls_buffer.clear();
                    *last_tool_name = None;
                    // CMPCT-041 invariant: DISPLAY-BASIS re-seed from the
                    // `cont_final` TokenDisplayUpdate snapshot (raw input +
                    // separate cache fields) — the raw `::new` is correct
                    // here and MUST NOT be fed tracker cache-INCLUSIVE
                    // totals; pinned by the wiring guard in
                    // cmpct041_seed_cache_double_count_test.rs.
                    *display = StreamingTokenDisplay::new(
                        cont_final.input_tokens,
                        cont_final.output_tokens,
                        cont_final.cache_read_tokens,
                        cont_final.cache_creation_tokens,
                    );
                    continue;
                }

                // Normal completion
                handle_final_response(text, &mut session.messages)?;
                // TOKEN-001: compute per-turn delta via the single TokenTracker
                // helper BEFORE update_from_usage mutates self.output_tokens.
                let per_turn_output_delta = session
                    .token_tracker
                    .compute_output_delta(cont_final.output_tokens);
                let cont_usage = ApiTokenUsage::new(
                    cont_final.input_tokens,
                    cont_final.cache_read_tokens,
                    cont_final.cache_creation_tokens,
                    per_turn_output_delta,
                );
                session
                    .token_tracker
                    .update_from_usage(&cont_usage, cont_final.output_tokens);

                set_tool_progress_callback(uuid::Uuid::nil(), None);
                output.emit_done_with_stop_reason(final_stop_reason.take());
                return Ok(GeminiContinuationResult::Completed);
            }
            Some(Err(e)) => {
                // CMPCT-026: Single-source-of-truth compaction classification.
                // `classify_compaction_branch` is the same helper used by
                // stream_loop.rs — it treats the error as authoritative
                // (structural downcast to PromptError::PromptCancelled) and
                // treats `parent_token_state.compaction_needed` as
                // defence-in-depth. The helper emits a structured
                // `tracing::warn!` when the two signals disagree, so this
                // site needs no extra warning of its own.
                let branch = classify_compaction_branch(&e, parent_token_state);

                if matches!(branch, CompactionBranch::Recover { .. }) {
                    info!("Compaction triggered during Gemini continuation - handling gracefully");

                    // CMPCT-023: unified compaction-recovery entry (Path D —
                    // Gemini continuation cancel). pop_user_prompt=false
                    // because the continuation User prompt is mid-flight, not
                    // at the tail of a completed turn.
                    //
                    // classify_compaction_branch has already defensively set
                    // compaction_needed=true if it was previously false; the
                    // helper does so uniformly (warning on disagreement), so
                    // no separate signal_compaction_needed call is required.
                    //
                    // CMPCT-028: capture the selected CompactionRecoveryPolicy
                    // and forward it to the primary stream loop so the in-loop
                    // restart picks the correct retry prompt (either
                    // `"Continue"` for EmbedInInstruction or the resume
                    // prompt for ResumeFromPartial).
                    let policy = super::recovery_compaction::begin_compaction_recovery(
                        session,
                        parent_token_state,
                        display,
                        text,
                        output,
                        false,
                    )?;

                    return Ok(GeminiContinuationResult::CompactionNeeded(policy));
                }

                set_tool_progress_callback(uuid::Uuid::nil(), None);
                output.emit_error(&e.to_string());
                return Err(anyhow::anyhow!("Gemini continuation error: {e}"));
            }
            None => {
                // CMPCT-028: capture whether we appended partial Assistant
                // text before returning CompactionNeeded(policy). This site
                // does not call `begin_compaction_recovery` (it's the
                // stream-ended-cleanly path with a deferred compaction flag),
                // so we compute the policy inline using the same rule the
                // helper applies: non-empty text → ResumeFromPartial,
                // empty text → EmbedInInstruction.
                let partial_text_saved = !text.is_empty();
                if partial_text_saved {
                    handle_final_response(text, &mut session.messages)?;
                }
                update_token_tracker(session, display);

                let compaction_needed = parent_token_state
                    .lock()
                    .map(|state| state.compaction_needed)
                    .unwrap_or(false);

                if compaction_needed {
                    let policy = if partial_text_saved {
                        super::recovery_compaction::CompactionRecoveryPolicy::ResumeFromPartial
                    } else {
                        super::recovery_compaction::CompactionRecoveryPolicy::EmbedInInstruction
                    };
                    debug!(
                        policy = ?policy,
                        partial_text_saved = partial_text_saved,
                        "[gemini_continuation] CompactionRecoveryPolicy selected on stream-end-with-compaction-flag path (CMPCT-028)"
                    );
                    return Ok(GeminiContinuationResult::CompactionNeeded(policy));
                }

                set_tool_progress_callback(uuid::Uuid::nil(), None);
                output.emit_done_with_stop_reason(final_stop_reason.take());
                return Ok(GeminiContinuationResult::Completed);
            }
            _ => {}
        }
        output.flush();
    }
}

/// Update token tracker from display values.
fn update_token_tracker(session: &mut Session, display: &StreamingTokenDisplay) {
    let current = display.current();
    // TOKEN-001: compute per-turn delta via the single TokenTracker helper
    // BEFORE update_from_usage mutates self.output_tokens.
    let per_turn_output_delta = session
        .token_tracker
        .compute_output_delta(current.output_tokens);
    let usage = ApiTokenUsage::new(
        current.input_tokens,
        current.cache_read_tokens,
        current.cache_creation_tokens,
        per_turn_output_delta,
    );
    session
        .token_tracker
        .update_from_usage(&usage, current.output_tokens);
}

/// Capture a continuation debug event.
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
