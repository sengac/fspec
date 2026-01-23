//! Gemini-specific history preparation and turn completion facade
//!
//! This module implements the facade pattern for:
//! 1. Preparing message history before sending to Gemini API
//! 2. Handling turn completion detection for Gemini models
//!
//! # Problem 1: Thought Signatures
//!
//! Gemini 2.5/3 preview models with thinking enabled require a `thoughtSignature`
//! on the first `functionCall` part in each model turn within the "active loop".
//! Without this, the API returns 400 errors or the model stops responding after
//! tool calls.
//!
//! # Problem 2: Empty Responses After Tool Calls
//!
//! Gemini 3 models sometimes return empty responses after tool calls, expecting
//! a continuation signal. This is unlike Claude which naturally continues after
//! receiving tool results.
//!
//! # Solutions
//!
//! ## Thought Signatures
//! Before each API call, we:
//! 1. Find the start of the "active loop" (last user message with text)
//! 2. For each model turn in the active loop, ensure the first function call
//!    has a `thoughtSignature` (add synthetic one if missing)
//!
//! ## Turn Completion
//! After receiving an empty response following a tool result, we:
//! 1. Detect the empty response condition
//! 2. Generate a continuation prompt to nudge the model to respond
//!
//! # Reference
//!
//! Based on Gemini CLI's behavior:
//! - `ensureActiveLoopHasThoughtSignatures()` in geminiChat.ts
//! - `checkNextSpeaker()` heuristic in client.ts
//!
//! https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/core/geminiChat.ts

use rig::message::{AssistantContent, Message, Text, UserContent};
use rig::one_or_many::OneOrMany;

/// Synthetic thought signature used when no real signature is present.
/// This value is recognized by Gemini API to bypass signature validation.
pub const SYNTHETIC_THOUGHT_SIGNATURE: &str = "skip_thought_signature_validator";

/// Trait for provider-specific history preparation.
///
/// Different providers may need to transform message history before API calls.
/// This trait abstracts those transformations.
pub trait HistoryPreparationFacade: Send + Sync {
    /// Returns the provider this facade is for
    fn provider(&self) -> &'static str;

    /// Prepare message history before sending to API.
    /// May modify messages in place (e.g., adding thought signatures).
    fn prepare_history(&self, messages: &mut Vec<Message>, model: &str);

    /// Check if a model requires special history preparation
    fn requires_preparation(&self, model: &str) -> bool;
}

/// Gemini history preparation facade.
///
/// Handles the `thoughtSignature` requirement for Gemini 2.5/3 preview models.
pub struct GeminiHistoryFacade;

impl GeminiHistoryFacade {
    /// Check if a model is a Gemini preview model that requires thought signatures
    pub fn is_preview_model(model: &str) -> bool {
        model.contains("gemini-2.5") || model.contains("preview") || model.contains("exp")
    }

    /// Find the index where the "active loop" starts.
    ///
    /// The active loop starts at the last user message that contains text
    /// (not just function responses). This matches Gemini CLI's logic.
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

    /// Ensure thought signatures are present on function calls in the active loop.
    ///
    /// For each model turn in the active loop, adds a synthetic thought signature
    /// to the first function call if it doesn't have one.
    fn ensure_thought_signatures(messages: &mut [Message], start_idx: usize) {
        for msg in messages[start_idx..].iter_mut() {
            if let Message::Assistant { content, .. } = msg {
                // Find first tool call and ensure it has a signature
                for c in content.iter_mut() {
                    if let AssistantContent::ToolCall(tool_call) = c {
                        // First tool call in this turn - ensure it has signature
                        if tool_call.signature.is_none() {
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

impl HistoryPreparationFacade for GeminiHistoryFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn prepare_history(&self, messages: &mut Vec<Message>, model: &str) {
        // Only apply to preview models
        if !Self::is_preview_model(model) {
            return;
        }

        // Find active loop start
        if let Some(start_idx) = Self::find_active_loop_start(messages) {
            Self::ensure_thought_signatures(messages, start_idx);
        }
    }

    fn requires_preparation(&self, model: &str) -> bool {
        Self::is_preview_model(model)
    }
}

/// No-op history preparation facade for providers that don't need it.
pub struct DefaultHistoryFacade;

impl HistoryPreparationFacade for DefaultHistoryFacade {
    fn provider(&self) -> &'static str {
        "default"
    }

    fn prepare_history(&self, _messages: &mut Vec<Message>, _model: &str) {
        // No-op - most providers don't need history preparation
    }

    fn requires_preparation(&self, _model: &str) -> bool {
        false
    }
}

// =============================================================================
// Turn Completion Detection for Gemini
// =============================================================================

/// Strategy for handling continuation after a response.
///
/// Different situations require different continuation strategies:
/// - `None`: No continuation needed, the response is complete
/// - `FullLoop`: Re-run the full agentic loop with a continuation prompt
///
/// The `FullLoop` strategy is used when the model may need to make additional
/// tool calls after receiving a continuation prompt. This is common with Gemini
/// models that return empty responses after tool results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuationStrategy {
    /// No continuation needed - the response is complete
    None,
    /// Re-run the full agentic loop with this prompt.
    /// This allows the model to make additional tool calls if needed.
    FullLoop {
        /// The prompt to send to continue the conversation
        prompt: &'static str,
    },
}

impl ContinuationStrategy {
    /// Check if this strategy requires continuation
    pub fn needs_continuation(&self) -> bool {
        !matches!(self, ContinuationStrategy::None)
    }

    /// Get the continuation prompt if this strategy requires one
    pub fn prompt(&self) -> Option<&'static str> {
        match self {
            ContinuationStrategy::None => None,
            ContinuationStrategy::FullLoop { prompt } => Some(prompt),
        }
    }
}

/// Trait for provider-specific turn completion behavior.
///
/// Gemini models may return empty responses after tool calls, requiring
/// continuation prompts. Claude naturally continues without this.
///
/// # Strategy Pattern
///
/// Instead of just detecting whether continuation is needed, this trait
/// returns a `ContinuationStrategy` that tells the caller HOW to continue.
/// This allows provider-specific behavior without coupling the facade to
/// the streaming infrastructure.
pub trait TurnCompletionFacade: Send + Sync {
    /// Returns the provider this facade is for
    fn provider(&self) -> &'static str;

    /// Check if a model requires turn completion handling
    fn requires_turn_completion_check(&self, model: &str) -> bool;

    /// Determine the continuation strategy based on response and history.
    ///
    /// Returns a `ContinuationStrategy` that tells the caller how to continue:
    /// - `None`: Response is complete, no action needed
    /// - `FullLoop`: Re-run the full agentic loop with the given prompt
    ///
    /// # Arguments
    ///
    /// * `response_text` - The text content of the model's response
    /// * `messages` - The current message history
    ///
    /// # Returns
    ///
    /// A `ContinuationStrategy` indicating what action to take
    fn continuation_strategy(&self, response_text: &str, messages: &[Message]) -> ContinuationStrategy;

    /// Check if the response indicates the model needs a continuation prompt.
    ///
    /// This is a convenience method that checks if `continuation_strategy()`
    /// returns anything other than `ContinuationStrategy::None`.
    ///
    /// Returns true if:
    /// - The response text is empty or whitespace-only
    /// - The last message in history is a tool result (not user text)
    fn needs_continuation(&self, response_text: &str, messages: &[Message]) -> bool {
        self.continuation_strategy(response_text, messages).needs_continuation()
    }

    /// Generate a continuation prompt to nudge the model to respond.
    ///
    /// This is called when `needs_continuation()` returns true.
    fn continuation_prompt(&self) -> &'static str;

    /// Create a continuation message to add to history.
    ///
    /// Returns a User message with the continuation prompt.
    fn create_continuation_message(&self) -> Message {
        Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: self.continuation_prompt().to_string(),
            })),
        }
    }
}

/// Gemini turn completion facade.
///
/// Handles the case where Gemini 3 models return empty responses after tool calls.
/// This implements a simpler version of Gemini CLI's `checkNextSpeaker()` heuristic.
pub struct GeminiTurnCompletionFacade;

impl GeminiTurnCompletionFacade {
    /// Continuation prompt that works well with Gemini models.
    /// Based on Gemini CLI's approach of sending "Please continue."
    /// but made more specific to encourage the model to use the tool results.
    const CONTINUATION_PROMPT: &'static str =
        "Based on the information retrieved, please provide your response.";

    /// Check if a model is a Gemini 3 model that may need continuation prompts.
    ///
    /// Gemini 3 models are more conversational and may stop after tool calls
    /// expecting a continuation signal. Gemini 2.x models don't have this issue.
    pub fn is_gemini_3_model(model: &str) -> bool {
        model.contains("gemini-3") || model.contains("gemini-2.5")
    }

    /// Check if the last message in history is a tool result.
    fn last_message_is_tool_result(messages: &[Message]) -> bool {
        if let Some(Message::User { content }) = messages.last() {
            return content
                .iter()
                .any(|c| matches!(c, UserContent::ToolResult(_)));
        }
        false
    }
}

impl TurnCompletionFacade for GeminiTurnCompletionFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn requires_turn_completion_check(&self, model: &str) -> bool {
        Self::is_gemini_3_model(model)
    }

    fn continuation_strategy(&self, response_text: &str, messages: &[Message]) -> ContinuationStrategy {
        // Condition 1: Response is empty or whitespace-only
        let response_empty = response_text.trim().is_empty();

        // Condition 2: Last message was a tool result (not a new user prompt)
        let after_tool_result = Self::last_message_is_tool_result(messages);

        // Both conditions must be true to trigger continuation
        if response_empty && after_tool_result {
            // Use FullLoop strategy because Gemini may need to make more tool calls
            // after receiving the continuation prompt. A simple prompt-response
            // wouldn't handle cases where Gemini decides to call another tool.
            ContinuationStrategy::FullLoop {
                prompt: Self::CONTINUATION_PROMPT,
            }
        } else {
            ContinuationStrategy::None
        }
    }

    fn continuation_prompt(&self) -> &'static str {
        Self::CONTINUATION_PROMPT
    }
}

/// No-op turn completion facade for providers that don't need it.
pub struct DefaultTurnCompletionFacade;

impl TurnCompletionFacade for DefaultTurnCompletionFacade {
    fn provider(&self) -> &'static str {
        "default"
    }

    fn requires_turn_completion_check(&self, _model: &str) -> bool {
        false
    }

    fn continuation_strategy(&self, _response_text: &str, _messages: &[Message]) -> ContinuationStrategy {
        ContinuationStrategy::None
    }

    fn continuation_prompt(&self) -> &'static str {
        "" // Never used since continuation_strategy always returns None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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

    fn make_assistant_text_message(text: &str) -> Message {
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: text.to_string(),
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

    // =========================================================================
    // Scenario: Preview model detection
    // =========================================================================

    #[test]
    fn test_is_preview_model_gemini_25() {
        assert!(GeminiHistoryFacade::is_preview_model("gemini-2.5-pro"));
        assert!(GeminiHistoryFacade::is_preview_model(
            "gemini-2.5-flash-preview-04-17"
        ));
        assert!(GeminiHistoryFacade::is_preview_model(
            "gemini-2.5-pro-preview-06-05"
        ));
    }

    #[test]
    fn test_is_preview_model_gemini_exp() {
        assert!(GeminiHistoryFacade::is_preview_model(
            "gemini-2.5-pro-exp-03-25"
        ));
    }

    #[test]
    fn test_is_not_preview_model() {
        assert!(!GeminiHistoryFacade::is_preview_model("gemini-2.0-flash"));
        assert!(!GeminiHistoryFacade::is_preview_model("gemini-1.5-pro"));
    }

    // =========================================================================
    // Scenario: Active loop detection
    // =========================================================================

    #[test]
    fn test_find_active_loop_start_with_text() {
        let messages = vec![
            make_user_text_message("Hello"),
            make_assistant_text_message("Hi there"),
            make_user_text_message("What's 2+2?"),
        ];

        let start = GeminiHistoryFacade::find_active_loop_start(&messages);
        assert_eq!(start, Some(2)); // Last user text message
    }

    #[test]
    fn test_find_active_loop_start_with_tool_results() {
        let messages = vec![
            make_user_text_message("Read file.txt"), // idx 0
            make_tool_call_message("read_file", false), // idx 1
            make_tool_result_message("call_1", "file contents"), // idx 2 - tool result, not text
        ];

        let start = GeminiHistoryFacade::find_active_loop_start(&messages);
        assert_eq!(start, Some(0)); // First message is the last with text
    }

    #[test]
    fn test_find_active_loop_start_empty() {
        let messages: Vec<Message> = vec![];
        let start = GeminiHistoryFacade::find_active_loop_start(&messages);
        assert_eq!(start, None);
    }

    // =========================================================================
    // Scenario: Thought signature injection
    // =========================================================================

    #[test]
    fn test_ensure_thought_signatures_adds_synthetic() {
        let mut messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", false), // No signature
        ];

        let facade = GeminiHistoryFacade;
        facade.prepare_history(&mut messages, "gemini-2.5-pro-preview-06-05");

        // Check that signature was added
        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(
                    tc.signature,
                    Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string())
                );
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

        let facade = GeminiHistoryFacade;
        facade.prepare_history(&mut messages, "gemini-2.5-pro-preview-06-05");

        // Check that original signature was preserved
        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, Some("real_signature".to_string()));
            } else {
                panic!("Expected ToolCall");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    #[test]
    fn test_no_preparation_for_non_preview_model() {
        let mut messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", false), // No signature
        ];

        let facade = GeminiHistoryFacade;
        facade.prepare_history(&mut messages, "gemini-2.0-flash"); // Not a preview model

        // Signature should NOT be added for non-preview models
        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, None);
            } else {
                panic!("Expected ToolCall");
            }
        } else {
            panic!("Expected Assistant message");
        }
    }

    // =========================================================================
    // Scenario: Multiple tool calls in sequence
    // =========================================================================

    #[test]
    fn test_multiple_turns_each_gets_signature() {
        let mut messages = vec![
            make_user_text_message("Do task 1"),
            make_tool_call_message("tool1", false), // Should get signature
            make_tool_result_message("call_1", "result 1"),
            make_tool_call_message("tool2", false), // Should get signature
            make_tool_result_message("call_2", "result 2"),
        ];

        let facade = GeminiHistoryFacade;
        facade.prepare_history(&mut messages, "gemini-2.5-pro-preview-06-05");

        // Check first tool call
        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(
                    tc.signature,
                    Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string()),
                    "First tool call should have synthetic signature"
                );
            }
        }

        // Check second tool call
        if let Message::Assistant { content, .. } = &messages[3] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(
                    tc.signature,
                    Some(SYNTHETIC_THOUGHT_SIGNATURE.to_string()),
                    "Second tool call should have synthetic signature"
                );
            }
        }
    }

    // =========================================================================
    // Scenario: Default facade is no-op
    // =========================================================================

    #[test]
    fn test_default_facade_is_noop() {
        let mut messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", false),
        ];

        let facade = DefaultHistoryFacade;
        facade.prepare_history(&mut messages, "any-model");

        // Signature should NOT be added
        if let Message::Assistant { content, .. } = &messages[1] {
            if let AssistantContent::ToolCall(tc) = content.first() {
                assert_eq!(tc.signature, None);
            }
        }
    }

    #[test]
    fn test_default_facade_requires_no_preparation() {
        let facade = DefaultHistoryFacade;
        assert!(!facade.requires_preparation("any-model"));
    }

    // =========================================================================
    // Scenario: Turn completion - Gemini 3 model detection
    // =========================================================================

    #[test]
    fn test_is_gemini_3_model() {
        assert!(GeminiTurnCompletionFacade::is_gemini_3_model("gemini-3-pro"));
        assert!(GeminiTurnCompletionFacade::is_gemini_3_model(
            "gemini-3-flash-preview"
        ));
        assert!(GeminiTurnCompletionFacade::is_gemini_3_model("gemini-2.5-pro"));
    }

    #[test]
    fn test_is_not_gemini_3_model() {
        assert!(!GeminiTurnCompletionFacade::is_gemini_3_model("gemini-2.0-flash"));
        assert!(!GeminiTurnCompletionFacade::is_gemini_3_model("gemini-1.5-pro"));
    }

    // =========================================================================
    // Scenario: Turn completion - continuation strategy detection
    // =========================================================================

    #[test]
    fn test_continuation_strategy_full_loop_after_tool_result() {
        let messages = vec![
            make_user_text_message("Search for something"),
            make_tool_call_message("google_web_search", false),
            make_tool_result_message("call_1", "Search results here..."),
        ];

        let facade = GeminiTurnCompletionFacade;

        // Empty response after tool result should return FullLoop strategy
        let strategy = facade.continuation_strategy("", &messages);
        assert!(matches!(strategy, ContinuationStrategy::FullLoop { .. }));
        
        // Whitespace-only should also trigger FullLoop
        let strategy = facade.continuation_strategy("   ", &messages);
        assert!(matches!(strategy, ContinuationStrategy::FullLoop { .. }));
        
        // Newlines/tabs should also trigger FullLoop
        let strategy = facade.continuation_strategy("\n\t", &messages);
        assert!(matches!(strategy, ContinuationStrategy::FullLoop { .. }));
    }

    #[test]
    fn test_continuation_strategy_none_when_response_has_text() {
        let messages = vec![
            make_user_text_message("Search for something"),
            make_tool_call_message("google_web_search", false),
            make_tool_result_message("call_1", "Search results here..."),
        ];

        let facade = GeminiTurnCompletionFacade;

        // Non-empty response should return None strategy
        let strategy = facade.continuation_strategy("Here's what I found...", &messages);
        assert!(matches!(strategy, ContinuationStrategy::None));
        
        let strategy = facade.continuation_strategy("Based on the results...", &messages);
        assert!(matches!(strategy, ContinuationStrategy::None));
    }

    #[test]
    fn test_continuation_strategy_none_when_last_message_is_user_text() {
        let messages = vec![
            make_user_text_message("Search for something"),
            make_assistant_text_message("I'll search for that."),
            make_user_text_message("Thanks, now tell me more"),
        ];

        let facade = GeminiTurnCompletionFacade;

        // Even with empty response, if last message is user text, no continuation
        let strategy = facade.continuation_strategy("", &messages);
        assert!(matches!(strategy, ContinuationStrategy::None));
    }

    #[test]
    fn test_continuation_strategy_none_when_history_is_empty() {
        let messages: Vec<Message> = vec![];

        let facade = GeminiTurnCompletionFacade;

        // Empty history should return None strategy
        let strategy = facade.continuation_strategy("", &messages);
        assert!(matches!(strategy, ContinuationStrategy::None));
    }

    // =========================================================================
    // Scenario: Turn completion - needs_continuation helper
    // =========================================================================

    #[test]
    fn test_needs_continuation_empty_response_after_tool_result() {
        let messages = vec![
            make_user_text_message("Search for something"),
            make_tool_call_message("google_web_search", false),
            make_tool_result_message("call_1", "Search results here..."),
        ];

        let facade = GeminiTurnCompletionFacade;

        // Empty response after tool result should need continuation
        assert!(facade.needs_continuation("", &messages));
        assert!(facade.needs_continuation("   ", &messages)); // Whitespace only
        assert!(facade.needs_continuation("\n\t", &messages)); // Newlines/tabs
    }

    #[test]
    fn test_no_continuation_when_response_has_text() {
        let messages = vec![
            make_user_text_message("Search for something"),
            make_tool_call_message("google_web_search", false),
            make_tool_result_message("call_1", "Search results here..."),
        ];

        let facade = GeminiTurnCompletionFacade;

        // Non-empty response should NOT need continuation
        assert!(!facade.needs_continuation("Here's what I found...", &messages));
        assert!(!facade.needs_continuation("Based on the results...", &messages));
    }

    #[test]
    fn test_no_continuation_when_last_message_is_user_text() {
        let messages = vec![
            make_user_text_message("Search for something"),
            make_assistant_text_message("I'll search for that."),
            make_user_text_message("Thanks, now tell me more"),
        ];

        let facade = GeminiTurnCompletionFacade;

        // Even with empty response, if last message is user text, no continuation
        // This handles the case where the model just hasn't responded yet
        assert!(!facade.needs_continuation("", &messages));
    }

    #[test]
    fn test_no_continuation_when_history_is_empty() {
        let messages: Vec<Message> = vec![];

        let facade = GeminiTurnCompletionFacade;

        // Empty history should not trigger continuation
        assert!(!facade.needs_continuation("", &messages));
    }

    // =========================================================================
    // Scenario: ContinuationStrategy helper methods
    // =========================================================================

    #[test]
    fn test_continuation_strategy_needs_continuation() {
        assert!(!ContinuationStrategy::None.needs_continuation());
        assert!(ContinuationStrategy::FullLoop { prompt: "test" }.needs_continuation());
    }

    #[test]
    fn test_continuation_strategy_prompt() {
        assert_eq!(ContinuationStrategy::None.prompt(), None);
        assert_eq!(
            ContinuationStrategy::FullLoop { prompt: "test prompt" }.prompt(),
            Some("test prompt")
        );
    }

    // =========================================================================
    // Scenario: Turn completion - continuation prompt
    // =========================================================================

    #[test]
    fn test_continuation_prompt_is_non_empty() {
        let facade = GeminiTurnCompletionFacade;
        let prompt = facade.continuation_prompt();

        assert!(!prompt.is_empty());
        assert!(prompt.contains("response") || prompt.contains("continue"));
    }

    #[test]
    fn test_create_continuation_message() {
        let facade = GeminiTurnCompletionFacade;
        let message = facade.create_continuation_message();

        // Should be a User message
        if let Message::User { content } = &message {
            // Should contain text
            let has_text = content.iter().any(|c| {
                if let UserContent::Text(text) = c {
                    !text.text.is_empty()
                } else {
                    false
                }
            });
            assert!(has_text, "Continuation message should have non-empty text");
        } else {
            panic!("Expected User message");
        }
    }

    // =========================================================================
    // Scenario: Default turn completion facade is no-op
    // =========================================================================

    #[test]
    fn test_default_turn_completion_no_check_required() {
        let facade = DefaultTurnCompletionFacade;
        assert!(!facade.requires_turn_completion_check("any-model"));
    }

    #[test]
    fn test_default_turn_completion_never_needs_continuation() {
        let messages = vec![
            make_user_text_message("Do something"),
            make_tool_call_message("some_tool", false),
            make_tool_result_message("call_1", "result"),
        ];

        let facade = DefaultTurnCompletionFacade;
        assert!(!facade.needs_continuation("", &messages));
    }
}
