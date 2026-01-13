//! Gemini History Preparation Hook
//!
//! Combines compaction checking with Gemini-specific history preparation.
//! For Gemini 2.5/3 preview models, this ensures thought signatures are
//! added to function calls before each API call.
//!
//! # The Problem
//!
//! Gemini 2.5/3 preview models with thinking enabled require a `thoughtSignature`
//! on the first `functionCall` part in each model turn within the "active loop".
//! Without this, the API returns 400 errors or the model stops responding after
//! tool calls.
//!
//! # The Solution
//!
//! This hook wraps the CompactionHook and adds Gemini-specific history preparation.
//! Before each API call, it:
//! 1. Checks for compaction (via CompactionHook)
//! 2. Adds synthetic thought signatures to function calls in the active loop
//!
//! # Reference
//!
//! Based on Gemini CLI's `ensureActiveLoopHasThoughtSignatures()` function:
//! https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/core/geminiChat.ts

use crate::compaction_hook::{CompactionHook, TokenState};
use crate::message_estimator::estimate_messages_tokens;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::message::{AssistantContent, Message, UserContent};
use std::sync::{Arc, Mutex};

/// Synthetic thought signature used when no real signature is present.
/// This value is recognized by Gemini API to bypass signature validation.
pub const SYNTHETIC_THOUGHT_SIGNATURE: &str = "skip_thought_signature_validator";

/// Gemini History Preparation Hook
///
/// Wraps CompactionHook to add Gemini-specific history preparation.
/// For preview models, ensures thought signatures are present on function calls.
#[derive(Clone)]
pub struct GeminiHistoryHook {
    /// Inner compaction hook for token tracking
    inner: CompactionHook,
    /// Model name to check if preview features are needed
    model: String,
}

impl GeminiHistoryHook {
    /// Create a new GeminiHistoryHook
    ///
    /// # Arguments
    /// * `state` - Shared state for token tracking
    /// * `threshold` - Token threshold that triggers compaction
    /// * `model` - Model name to check for preview model behavior
    pub fn new(state: Arc<Mutex<TokenState>>, threshold: u64, model: String) -> Self {
        Self {
            inner: CompactionHook::new(state, threshold),
            model,
        }
    }

    /// Check if a model is a Gemini preview model that requires thought signatures
    pub fn is_preview_model(model: &str) -> bool {
        model.contains("gemini-2.5") || model.contains("preview") || model.contains("exp")
    }

    /// Get reference to the inner compaction hook's state
    pub fn state(&self) -> &Arc<Mutex<TokenState>> {
        &self.inner.state
    }

    /// Find the index where the "active loop" starts.
    ///
    /// The active loop starts at the last user message that contains text
    /// (not just function responses/tool results).
    fn find_active_loop_start(messages: &[Message]) -> Option<usize> {
        for (i, msg) in messages.iter().enumerate().rev() {
            if let Message::User { content } = msg {
                // Check if this user message has text (not just tool results)
                let has_text = content.iter().any(|c| matches!(c, UserContent::Text(_)));
                if has_text {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Prepare history by adding thought signatures to function calls.
    ///
    /// This modifies the history in-place, adding synthetic thought signatures
    /// to the first function call in each model turn within the active loop.
    fn prepare_history(history: &mut [Message], start_idx: usize) {
        for msg in history[start_idx..].iter_mut() {
            if let Message::Assistant { content, .. } = msg {
                // Find first tool call and ensure it has a signature
                for c in content.iter_mut() {
                    if let AssistantContent::ToolCall(tool_call) = c {
                        // First tool call in this turn - ensure it has signature
                        if tool_call.signature.is_none() {
                            tracing::debug!(
                                "Adding synthetic thought signature to tool call: {}",
                                tool_call.function.name
                            );
                            tool_call.signature =
                                Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string());
                        }
                        // Only process first tool call per model turn
                        break;
                    }
                }
            }
        }
    }
}

impl<M> StreamingPromptHook<M> for GeminiHistoryHook
where
    M: CompletionModel,
    <M as CompletionModel>::StreamingResponse: GetTokenUsage,
{
    /// Called BEFORE each API call
    ///
    /// For Gemini preview models:
    /// 1. Adds synthetic thought signatures to function calls in active loop
    /// 2. Checks for compaction (via inner hook logic)
    async fn on_completion_call(
        &self,
        prompt: &Message,
        history: &[Message],
        cancel_sig: CancelSignal,
    ) {
        // NOTE: Unfortunately rig passes history as immutable slice, so we can't
        // modify it in place here. The signature addition needs to happen BEFORE
        // calling rig's stream_prompt, which means we need to modify the messages
        // in stream_loop.rs BEFORE creating the hook.
        //
        // This hook still performs the compaction check, and we log if signatures
        // would have been needed (for debugging).

        if Self::is_preview_model(&self.model) {
            if let Some(start_idx) = Self::find_active_loop_start(history) {
                // Check if any function calls in active loop are missing signatures
                for msg in history[start_idx..].iter() {
                    if let Message::Assistant { content, .. } = msg {
                        for c in content.iter() {
                            if let AssistantContent::ToolCall(tc) = c {
                                if tc.signature.is_none() {
                                    tracing::warn!(
                                        "Gemini preview model {} has tool call '{}' without signature in history. \
                                         History should be prepared with ensure_thought_signatures() before API call.",
                                        self.model,
                                        tc.function.name
                                    );
                                }
                                break; // Only check first tool call per turn
                            }
                        }
                    }
                }
            }
        }

        // Perform compaction check (same as CompactionHook)
        let Ok(mut state) = self.inner.state.lock() else {
            tracing::warn!("GeminiHistoryHook state mutex poisoned, skipping compaction check");
            return;
        };

        let mut all_messages = history.to_vec();
        all_messages.push(prompt.clone());
        let estimated_payload = estimate_messages_tokens(&all_messages) as u64;
        let last_known_total = state.total();
        let effective_total = last_known_total.max(estimated_payload);

        tracing::debug!(
            "Compaction check: last_known={}, estimated={}, effective={}, threshold={}",
            last_known_total,
            estimated_payload,
            effective_total,
            self.inner.threshold()
        );

        if effective_total > self.inner.threshold() {
            state.compaction_needed = true;
            tracing::info!(
                "Compaction triggered: {} tokens > {} threshold",
                effective_total,
                self.inner.threshold()
            );
            cancel_sig.cancel();
        }
    }

    /// Called AFTER each API call - capture per-request usage
    async fn on_stream_completion_response_finish(
        &self,
        prompt: &Message,
        response: &<M as CompletionModel>::StreamingResponse,
        cancel_sig: CancelSignal,
    ) {
        <CompactionHook as StreamingPromptHook<M>>::on_stream_completion_response_finish(
            &self.inner,
            prompt,
            response,
            cancel_sig,
        )
        .await;
    }

    async fn on_text_delta(&self, delta: &str, total_text: &str, cancel_signal: CancelSignal) {
        <CompactionHook as StreamingPromptHook<M>>::on_text_delta(
            &self.inner,
            delta,
            total_text,
            cancel_signal,
        )
        .await;
    }

    async fn on_tool_call(
        &self,
        name: &str,
        call_id: Option<String>,
        args: &str,
        cancel_signal: CancelSignal,
    ) {
        <CompactionHook as StreamingPromptHook<M>>::on_tool_call(
            &self.inner,
            name,
            call_id,
            args,
            cancel_signal,
        )
        .await;
    }

    async fn on_tool_call_delta(
        &self,
        id: &str,
        name: Option<&str>,
        delta: &str,
        cancel_signal: CancelSignal,
    ) {
        <CompactionHook as StreamingPromptHook<M>>::on_tool_call_delta(
            &self.inner,
            id,
            name,
            delta,
            cancel_signal,
        )
        .await;
    }

    async fn on_tool_result(
        &self,
        name: &str,
        call_id: Option<String>,
        args: &str,
        result: &str,
        cancel_signal: CancelSignal,
    ) {
        <CompactionHook as StreamingPromptHook<M>>::on_tool_result(
            &self.inner,
            name,
            call_id,
            args,
            result,
            cancel_signal,
        )
        .await;
    }
}

/// Utility function to prepare message history for Gemini preview models.
///
/// Call this BEFORE creating the stream to ensure thought signatures are present.
/// This modifies the messages vector in-place.
///
/// # Arguments
/// * `messages` - Message history to prepare
/// * `model` - Model name to check if preparation is needed
///
/// # Example
/// ```ignore
/// // Before streaming
/// ensure_thought_signatures(&mut session.messages, "gemini-2.5-pro-preview-06-05");
///
/// // Now create stream
/// let stream = agent.prompt_streaming_with_history(&prompt, &session.messages).await;
/// ```
pub fn ensure_thought_signatures(messages: &mut Vec<Message>, model: &str) {
    if !GeminiHistoryHook::is_preview_model(model) {
        return;
    }

    if let Some(start_idx) = GeminiHistoryHook::find_active_loop_start(messages) {
        tracing::debug!(
            "Preparing Gemini history: active loop starts at index {}",
            start_idx
        );
        GeminiHistoryHook::prepare_history(messages, start_idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::message::{Text, ToolCall, ToolFunction};
    use rig::one_or_many::OneOrMany;
    use serde_json::json;

    fn make_user_text_message(text: &str) -> Message {
        Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: text.to_string(),
            })),
        }
    }

    fn make_tool_result_message(id: &str, result: &str) -> Message {
        Message::User {
            content: OneOrMany::one(UserContent::ToolResult(rig::message::ToolResult {
                id: id.to_string(),
                call_id: Some(id.to_string()),
                content: OneOrMany::one(rig::message::ToolResultContent::Text(Text {
                    text: result.to_string(),
                })),
            })),
        }
    }

    fn make_tool_call_message(name: &str, with_signature: bool) -> Message {
        let mut tool_call = ToolCall::new(
            name.to_string(),
            ToolFunction::new(name.to_string(), json!({"arg": "value"})),
        );
        if with_signature {
            tool_call.signature = Some("real_signature".to_string());
        }
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(tool_call)),
        }
    }

    #[test]
    fn test_is_preview_model() {
        assert!(GeminiHistoryHook::is_preview_model("gemini-2.5-pro"));
        assert!(GeminiHistoryHook::is_preview_model(
            "gemini-2.5-flash-preview-04-17"
        ));
        assert!(GeminiHistoryHook::is_preview_model("gemini-2.5-pro-exp-03-25"));
        assert!(!GeminiHistoryHook::is_preview_model("gemini-2.0-flash"));
    }

    #[test]
    fn test_ensure_thought_signatures_adds_synthetic() {
        let mut messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", false),
        ];

        ensure_thought_signatures(&mut messages, "gemini-2.5-pro-preview-06-05");

        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string()));
            } else {
                panic!("Expected ToolCall");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_ensure_thought_signatures_preserves_existing() {
        let mut messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", true), // Has real signature
        ];

        ensure_thought_signatures(&mut messages, "gemini-2.5-pro-preview-06-05");

        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, Some("real_signature".to_string()));
            }
        }
    }

    #[test]
    fn test_ensure_thought_signatures_no_op_for_non_preview() {
        let mut messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", false),
        ];

        ensure_thought_signatures(&mut messages, "gemini-2.0-flash");

        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, None);
            }
        }
    }

    #[test]
    fn test_ensure_thought_signatures_multiple_turns() {
        let mut messages = vec![
            make_user_text_message("Do task 1"),
            make_tool_call_message("tool1", false),
            make_tool_result_message("call_1", "result 1"),
            make_tool_call_message("tool2", false),
            make_tool_result_message("call_2", "result 2"),
        ];

        ensure_thought_signatures(&mut messages, "gemini-2.5-pro-preview-06-05");

        // Check first tool call
        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string()));
            }
        }

        // Check second tool call
        if let Message::Assistant { content, .. } = &messages[3] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string()));
            }
        }
    }
}
