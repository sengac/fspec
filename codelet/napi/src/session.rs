//! CodeletSession - Main class exposed to JavaScript
//!
//! Wraps codelet's Session and provides async streaming prompts via ThreadsafeFunction.
//!
//! Uses the same agent infrastructure as codelet-cli:
//! - ProviderManager for consistent provider access
//! - System-reminders for context (CLAUDE.md, environment)
//! - RigAgent with all 9 tools for full agent capabilities
//! - run_agent_stream for shared streaming logic
//!
//! Key difference from CLI: JavaScript calls interrupt() to set is_interrupted flag

use crate::output::{NapiOutput, StreamCallback};
use crate::types::{CompactionResult, ContextFillInfo, DebugCommandResult, Message, TokenTracker};
use codelet_cli::compaction_threshold::calculate_usable_context;
use codelet_cli::interactive::run_agent_stream;
use codelet_cli::interactive_helpers::execute_compaction;
use codelet_common::debug_capture::{
    get_debug_capture_manager, handle_debug_command_with_dir, SessionMetadata,
};
use codelet_core::RigAgent;
use napi::bindgen_prelude::*;
use std::sync::atomic::AtomicBool;
// Use Release ordering for stores to synchronize with Acquire loads in stream_loop
use std::sync::atomic::Ordering::Release;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// CodeletSession - Main class for AI agent interactions
///
/// Exposes codelet's Rust AI agent functionality to Node.js.
#[napi]
pub struct CodeletSession {
    /// Inner session from codelet-cli (using tokio Mutex for async safety)
    inner: Arc<Mutex<codelet_cli::session::Session>>,
    /// Interrupt flag - JavaScript calls interrupt() to set this
    is_interrupted: Arc<AtomicBool>,
    /// Notify for immediate interrupt wake-up (NAPI-004)
    /// When interrupt() is called, this wakes the tokio::select! in stream_loop
    interrupt_notify: Arc<Notify>,
}

#[napi]
impl CodeletSession {
    /// Create a new CodeletSession
    ///
    /// If provider_name is not specified, auto-detects the highest priority available provider.
    /// Priority order: Claude > Gemini > Codex > OpenAI
    #[napi(constructor)]
    pub fn new(provider_name: Option<String>) -> Result<Self> {
        // Load environment variables from .env file (if present)
        // This is required for API keys to be available when running from Node.js
        let _ = dotenvy::dotenv();

        let session = codelet_cli::session::Session::new(provider_name.as_deref())
            .map_err(|e| Error::from_reason(format!("Failed to create session: {e}")))?;

        // Inject context reminders (CLAUDE.md discovery, environment info)
        let mut session = session;
        session.inject_context_reminders();

        Ok(Self {
            inner: Arc::new(Mutex::new(session)),
            is_interrupted: Arc::new(AtomicBool::new(false)),
            interrupt_notify: Arc::new(Notify::new()),
        })
    }

    /// Interrupt the current agent execution
    ///
    /// Call this when the user presses Esc in the TUI.
    /// The agent will stop immediately via tokio::sync::Notify (NAPI-004).
    /// The notify_one() call wakes the tokio::select! in stream_loop,
    /// allowing immediate response to ESC even during blocking operations.
    ///
    /// IMPORTANT: Uses notify_one() instead of notify_waiters() because:
    /// - notify_waiters() only wakes CURRENTLY waiting tasks (notification lost if none waiting)
    /// - notify_one() stores a permit if no one waiting, so next notified() returns immediately
    /// This eliminates the race condition between flag check and entering tokio::select!
    #[napi]
    pub fn interrupt(&self) {
        self.is_interrupted.store(true, Release);
        // Wake any waiting stream loop immediately (NAPI-004)
        // Uses notify_one() to store permit if not currently waiting in select
        self.interrupt_notify.notify_one();
    }

    /// Reset the interrupt flag
    ///
    /// Called automatically at the start of each prompt, but can be called
    /// manually if needed.
    #[napi]
    pub fn reset_interrupt(&self) {
        self.is_interrupted.store(false, Release);
    }

    /// Toggle debug capture mode (AGENT-021)
    ///
    /// Mirrors CLI repl_loop.rs:36-67 logic.
    /// When enabling, sets session metadata (provider, model, context_window).
    /// When disabling, stops capture and returns path to saved session file.
    ///
    /// If debug_dir is provided, debug files will be written to `{debug_dir}/debug/`
    /// instead of the default directory. For fspec, pass `~/.fspec` to write to
    /// `~/.fspec/debug/`.
    #[napi]
    pub fn toggle_debug(&self, debug_dir: Option<String>) -> Result<DebugCommandResult> {
        let result = handle_debug_command_with_dir(debug_dir.as_deref());

        // If debug was just enabled, set session metadata
        if result.enabled {
            let session = self.inner.blocking_lock();
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    manager.set_session_metadata(SessionMetadata {
                        provider: Some(session.current_provider_name().to_string()),
                        model: Some(session.current_provider_name().to_string()),
                        context_window: Some(session.provider_manager().context_window()),
                        max_output_tokens: None,
                    });
                }
            }
        }

        Ok(DebugCommandResult {
            enabled: result.enabled,
            session_file: result.session_file,
            message: result.message,
        })
    }

    /// Manually trigger context compaction (NAPI-005)
    ///
    /// Mirrors CLI repl_loop.rs /compact command logic.
    /// Calls execute_compaction from interactive_helpers to compress context.
    ///
    /// Returns CompactionResult with metrics about the compaction operation.
    /// Returns error if session is empty (nothing to compact).
    #[napi]
    pub async fn compact(&self) -> Result<CompactionResult> {
        let mut session = self.inner.lock().await;

        // Check if there's anything to compact
        if session.messages.is_empty() {
            return Err(Error::from_reason("Nothing to compact - no messages yet"));
        }

        // Get current token count for reporting
        let original_tokens = session.token_tracker.input_tokens;

        // Capture compaction.manual.start event
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    manager.capture(
                        "compaction.manual.start",
                        serde_json::json!({
                            "command": "/compact",
                            "originalTokens": original_tokens,
                            "messageCount": session.messages.len(),
                        }),
                        None,
                    );
                }
            }
        }

        // Execute compaction
        let metrics = execute_compaction(&mut session).await.map_err(|e| {
            // Capture compaction.manual.failed event
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "compaction.manual.failed",
                            serde_json::json!({
                                "command": "/compact",
                                "error": e.to_string(),
                            }),
                            None,
                        );
                    }
                }
            }
            Error::from_reason(format!("Compaction failed: {e}"))
        })?;

        // Capture compaction.manual.complete event
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    manager.capture(
                        "compaction.manual.complete",
                        serde_json::json!({
                            "command": "/compact",
                            "originalTokens": metrics.original_tokens,
                            "compactedTokens": metrics.compacted_tokens,
                            "compressionRatio": metrics.compression_ratio,
                            "turnsSummarized": metrics.turns_summarized,
                            "turnsKept": metrics.turns_kept,
                        }),
                        None,
                    );
                }
            }
        }

        Ok(CompactionResult {
            original_tokens: metrics.original_tokens as u32,
            compacted_tokens: metrics.compacted_tokens as u32,
            compression_ratio: metrics.compression_ratio * 100.0, // Convert to percentage
            turns_summarized: metrics.turns_summarized as u32,
            turns_kept: metrics.turns_kept as u32,
        })
    }

    /// Get the current provider name
    #[napi(getter)]
    pub fn current_provider_name(&self) -> Result<String> {
        // Use blocking lock for sync getter
        let session = self.inner.blocking_lock();
        Ok(session.current_provider_name().to_string())
    }

    /// Get list of available providers
    #[napi(getter)]
    pub fn available_providers(&self) -> Result<Vec<String>> {
        let session = self.inner.blocking_lock();

        // Get raw provider names without formatting
        let providers = session.provider_manager().list_available_providers();

        // Strip formatting like "Claude (/claude)" -> "claude"
        let clean_providers: Vec<String> = providers
            .into_iter()
            .map(|p| {
                // Extract provider name from format like "Claude (/claude)"
                if let Some(start) = p.find("(/") {
                    if let Some(end) = p.find(')') {
                        return p[start + 2..end].to_string();
                    }
                }
                p.to_lowercase()
            })
            .collect();

        Ok(clean_providers)
    }

    /// Get the token usage tracker
    #[napi(getter)]
    pub fn token_tracker(&self) -> Result<TokenTracker> {
        let session = self.inner.blocking_lock();

        Ok(TokenTracker {
            input_tokens: session.token_tracker.input_tokens as u32,
            output_tokens: session.token_tracker.output_tokens as u32,
            cache_read_input_tokens: session
                .token_tracker
                .cache_read_input_tokens
                .map(|t| t as u32),
            cache_creation_input_tokens: session
                .token_tracker
                .cache_creation_input_tokens
                .map(|t| t as u32),
            tokens_per_second: None, // Only set during streaming
            cumulative_billed_input: Some(session.token_tracker.cumulative_billed_input as u32),
            cumulative_billed_output: Some(session.token_tracker.cumulative_billed_output as u32),
        })
    }

    /// Get conversation messages (simplified representation)
    #[napi(getter)]
    pub fn messages(&self) -> Result<Vec<Message>> {
        let session = self.inner.blocking_lock();

        let messages: Vec<Message> = session
            .messages
            .iter()
            .map(|msg| {
                let (role, content) = match msg {
                    rig::message::Message::User { content, .. } => {
                        let text = content
                            .iter()
                            .filter_map(|c| match c {
                                rig::message::UserContent::Text(t) => Some(t.text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        let text = if text.is_empty() {
                            "[non-text content]".to_string()
                        } else {
                            text
                        };
                        ("user".to_string(), text)
                    }
                    rig::message::Message::Assistant { content, .. } => {
                        let text = content
                            .iter()
                            .filter_map(|c| match c {
                                rig::message::AssistantContent::Text(t) => Some(t.text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        let text = if text.is_empty() {
                            "[non-text content]".to_string()
                        } else {
                            text
                        };
                        ("assistant".to_string(), text)
                    }
                };
                Message { role, content }
            })
            .collect();

        Ok(messages)
    }

    /// Switch to a different provider
    #[napi]
    pub async fn switch_provider(&self, provider_name: String) -> Result<()> {
        let mut session = self.inner.lock().await;

        session
            .switch_provider(&provider_name)
            .map_err(|e| Error::from_reason(format!("Failed to switch provider: {e}")))?;

        Ok(())
    }

    /// Clear conversation history and reinject context reminders
    ///
    /// Clears messages, turns, and token tracker, then reinjects context reminders
    /// (CLAUDE.md discovery, environment info) to maintain project context.
    ///
    /// CRITICAL (AGENT-003): Must call inject_context_reminders() after clearing
    /// to restore project context (CLAUDE.md, environment info). Without this,
    /// the AI loses CLAUDE.md context on the next prompt after /clear.
    #[napi]
    pub fn clear_history(&self) -> Result<()> {
        let mut session = self.inner.blocking_lock();

        session.messages.clear();
        session.turns.clear();
        session.token_tracker = codelet_core::compaction::TokenTracker::default();

        // Reinject context reminders to restore CLAUDE.md and environment info
        // This ensures the AI retains project context after clearing history
        session.inject_context_reminders();

        Ok(())
    }

    /// Restore messages from a persisted session (NAPI-003)
    ///
    /// Restores conversation history from persistence into the CodeletSession's
    /// internal message array, enabling the LLM to have context of the restored
    /// conversation.
    ///
    /// CRITICAL: This method must be called when resuming a session to ensure
    /// the AI has context of the previous conversation. Without this, the AI
    /// would start fresh despite the UI showing historical messages.
    ///
    /// # Arguments
    /// * `messages` - Array of messages to restore (from persistenceGetSessionMessages)
    ///
    /// # Process
    /// 1. Clears existing messages, turns, and token tracker
    /// 2. Converts each persistence message to rig::message::Message format
    /// 3. Injects context reminders (CLAUDE.md, environment info)
    /// 4. Messages are ready for use in next prompt
    #[napi]
    pub fn restore_messages(&self, messages: Vec<Message>) -> Result<()> {
        use rig::message::{AssistantContent, Message as RigMessage, UserContent};
        use rig::OneOrMany;

        let mut session = self.inner.blocking_lock();

        // Clear existing state before restoring
        session.messages.clear();
        session.turns.clear();
        session.token_tracker = codelet_core::compaction::TokenTracker::default();

        // Convert persistence messages to rig messages
        for msg in messages {
            let rig_msg = match msg.role.as_str() {
                "user" => RigMessage::User {
                    content: OneOrMany::one(UserContent::text(&msg.content)),
                },
                "assistant" => RigMessage::Assistant {
                    id: None,
                    content: OneOrMany::one(AssistantContent::text(&msg.content)),
                },
                // Skip unknown roles (e.g., "tool" messages from persistence)
                _ => continue,
            };
            session.messages.push(rig_msg);
        }

        // Re-inject context reminders to ensure CLAUDE.md and environment info
        // are present after restoration
        session.inject_context_reminders();

        Ok(())
    }

    /// Restore messages from full envelope JSON strings (NAPI-008)
    ///
    /// This is the preferred method for restoring sessions as it preserves:
    /// - Structured tool_use and tool_result blocks (not just text summaries)
    /// - Multi-part message content (text + tool calls)
    /// - Turn boundaries for compaction (rebuilt via convert_messages_to_turns)
    ///
    /// Call restoreTokenState() after this to restore token counts.
    ///
    /// # Arguments
    /// * `envelopes` - Array of envelope JSON strings from persistenceGetSessionMessageEnvelopes
    #[napi]
    pub fn restore_messages_from_envelopes(&self, envelopes: Vec<String>) -> Result<()> {
        use crate::persistence::{
            AssistantContent as EnvAssistantContent, MessageEnvelope, MessagePayload,
            UserContent as EnvUserContent,
        };
        use codelet_cli::interactive_helpers::convert_messages_to_turns;
        use rig::message::{
            AssistantContent, Message as RigMessage, Text, ToolCall, ToolFunction, ToolResult,
            ToolResultContent, UserContent,
        };
        use rig::OneOrMany;

        let mut session = self.inner.blocking_lock();

        // Clear existing state before restoring
        session.messages.clear();
        session.turns.clear();
        session.token_tracker = codelet_core::compaction::TokenTracker::default();

        tracing::info!(
            "restore_messages_from_envelopes: processing {} envelopes",
            envelopes.len()
        );

        for (idx, envelope_json) in envelopes.iter().enumerate() {
            tracing::debug!(
                "Processing envelope {}: {}",
                idx,
                &envelope_json[..std::cmp::min(200, envelope_json.len())]
            );

            let envelope: MessageEnvelope = match serde_json::from_str(envelope_json) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse envelope {} during restore: {} - json: {}",
                        idx,
                        e,
                        &envelope_json[..std::cmp::min(500, envelope_json.len())]
                    );
                    continue;
                }
            };

            match envelope.message {
                MessagePayload::User(user_msg) => {
                    // Convert user content blocks
                    let mut rig_contents: Vec<UserContent> = Vec::new();
                    let mut has_text = false;
                    let mut has_tool_results = false;

                    for content in user_msg.content {
                        match content {
                            EnvUserContent::Text { text } => {
                                rig_contents.push(UserContent::Text(Text { text }));
                                has_text = true;
                            }
                            EnvUserContent::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } => {
                                // Create proper ToolResult for rig
                                // Note: rig's ToolResult uses 'id' for the call reference
                                rig_contents.push(UserContent::ToolResult(ToolResult {
                                    id: tool_use_id,
                                    call_id: None,
                                    content: OneOrMany::one(ToolResultContent::Text(Text {
                                        text: content,
                                    })),
                                }));
                                has_tool_results = true;
                            }
                            EnvUserContent::Image { .. } | EnvUserContent::Document { .. } => {
                                // Skip images/documents for now - they don't affect LLM context directly
                                // The blob references are preserved in persistence
                            }
                        }
                    }

                    if !rig_contents.is_empty() {
                        // NAPI-008 FIX: Merge consecutive tool_result-only User messages
                        // Claude API requires ALL tool_results to be in a SINGLE User message
                        // immediately after the Assistant message with tool_use blocks.
                        // When multiple tool calls occur, each tool_result is stored as a separate
                        // envelope, but they must be merged during restoration.
                        let should_merge = !has_text
                            && has_tool_results
                            && session.messages.last().map_or(false, |last| {
                                matches!(last, RigMessage::User { content } if {
                                    // Check if last message is User with only tool_results
                                    content.iter().all(|c| matches!(c, UserContent::ToolResult(_)))
                                })
                            });

                        if should_merge {
                            // Merge with the previous User message
                            if let Some(RigMessage::User { content: last_content }) =
                                session.messages.last_mut()
                            {
                                // Append new tool_results to existing message
                                let mut merged: Vec<UserContent> =
                                    last_content.iter().cloned().collect();
                                merged.extend(rig_contents);
                                *last_content = OneOrMany::many(merged)
                                    .unwrap_or_else(|_| OneOrMany::one(UserContent::text("")));
                                tracing::debug!(
                                    "Merged tool_result into previous User message (now has {} content blocks)",
                                    last_content.iter().count()
                                );
                            }
                        } else {
                            session.messages.push(RigMessage::User {
                                content: OneOrMany::many(rig_contents)
                                    .unwrap_or_else(|_| OneOrMany::one(UserContent::text(""))),
                            });
                        }
                    }
                }
                MessagePayload::Assistant(assistant_msg) => {
                    // Convert assistant content blocks
                    let mut rig_contents: Vec<AssistantContent> = Vec::new();
                    for content in assistant_msg.content {
                        match content {
                            EnvAssistantContent::Text { text } => {
                                rig_contents.push(AssistantContent::Text(Text { text }));
                            }
                            EnvAssistantContent::ToolUse { id, name, input } => {
                                // Create proper ToolCall for rig
                                rig_contents.push(AssistantContent::ToolCall(ToolCall {
                                    id,
                                    call_id: None,
                                    function: ToolFunction {
                                        name,
                                        arguments: input, // Already serde_json::Value
                                    },
                                }));
                            }
                            EnvAssistantContent::Thinking { .. } => {
                                // Skip thinking blocks - they don't need to be in message history
                            }
                        }
                    }

                    if !rig_contents.is_empty() {
                        session.messages.push(RigMessage::Assistant {
                            id: assistant_msg.id,
                            content: OneOrMany::many(rig_contents)
                                .unwrap_or_else(|_| OneOrMany::one(AssistantContent::text(""))),
                        });
                    }
                }
            }
        }

        // Rebuild turns from restored messages for compaction support
        session.turns = convert_messages_to_turns(&session.messages);

        tracing::info!(
            "restore_messages_from_envelopes: restored {} messages, {} turns",
            session.messages.len(),
            session.turns.len()
        );

        // Re-inject context reminders to ensure CLAUDE.md and environment info
        // are present after restoration
        session.inject_context_reminders();

        tracing::info!(
            "restore_messages_from_envelopes: after context injection, {} total messages",
            session.messages.len()
        );

        Ok(())
    }

    /// Restore token state from persisted session (TUI-033, NAPI-008)
    ///
    /// Call this after restoreMessages() or restoreMessagesFromEnvelopes() to restore
    /// the token counts from the persisted session manifest. This ensures getContextFillInfo()
    /// returns accurate context fill percentage after session restoration.
    ///
    /// # Arguments
    /// * `input_tokens` - Total input tokens from session manifest
    /// * `output_tokens` - Total output tokens from session manifest
    /// * `cache_read_tokens` - Cache read tokens from session manifest
    /// * `cache_creation_tokens` - Cache creation tokens from session manifest
    #[napi]
    pub fn restore_token_state(
        &self,
        input_tokens: u32,
        output_tokens: u32,
        cache_read_tokens: u32,
        cache_creation_tokens: u32,
        cumulative_billed_input: u32,
        cumulative_billed_output: u32,
    ) {
        let mut session = self.inner.blocking_lock();
        session.token_tracker.input_tokens = input_tokens as u64;
        session.token_tracker.output_tokens = output_tokens as u64;
        session.token_tracker.cache_read_input_tokens = Some(cache_read_tokens as u64);
        session.token_tracker.cache_creation_input_tokens = Some(cache_creation_tokens as u64);
        // CTX-003: Restore cumulative billing fields for accurate billing analytics after resume
        session.token_tracker.cumulative_billed_input = cumulative_billed_input as u64;
        session.token_tracker.cumulative_billed_output = cumulative_billed_output as u64;
    }

    /// Get current context fill info (TUI-033)
    ///
    /// Returns the current context fill percentage and related metrics.
    /// Call this after restoreMessages() to get initial context fill state,
    /// since restoring messages doesn't trigger streaming events.
    ///
    /// # Returns
    /// * `ContextFillInfo` with fill_percentage, effective_tokens, threshold, context_window
    #[napi]
    pub fn get_context_fill_info(&self) -> Result<ContextFillInfo> {
        let session = self.inner.blocking_lock();
        let context_window = session.provider_manager().context_window() as u64;
        let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
        // CTX-002: Use usable_context (context_window - output_reservation)
        let threshold = calculate_usable_context(context_window, max_output_tokens);
        let input_tokens = session.token_tracker.input_tokens;
        let cache_read_tokens = session.token_tracker.cache_read_input_tokens.unwrap_or(0);
        let output_tokens = session.token_tracker.output_tokens;

        // CTX-002: Simple sum of all token types
        let total_tokens = input_tokens + cache_read_tokens + output_tokens;
        let fill_percentage = if threshold > 0 {
            ((total_tokens as f64 / threshold as f64) * 100.0) as u32
        } else {
            0
        };

        Ok(ContextFillInfo {
            fill_percentage,
            effective_tokens: total_tokens as f64,
            threshold: threshold as f64,
            context_window: context_window as f64,
        })
    }

    /// Send a prompt and stream the response
    ///
    /// The callback receives StreamChunk objects with type: 'Text', 'ToolCall', 'ToolResult', 'Done', or 'Error'
    ///
    /// Uses the same streaming infrastructure as codelet-cli:
    /// - run_agent_stream for shared streaming logic
    /// - StreamOutput trait for polymorphic output
    /// - is_interrupted flag for Esc key handling (set via interrupt() method)
    #[napi]
    pub async fn prompt(
        &self,
        input: String,
        #[napi(ts_arg_type = "(chunk: StreamChunk) => void")] callback: StreamCallback,
    ) -> Result<()> {
        // Reset interrupt flag at start of each prompt
        self.is_interrupted.store(false, Release);

        // Clone Arcs for use in async block
        let session_arc = Arc::clone(&self.inner);
        let is_interrupted = Arc::clone(&self.is_interrupted);
        let interrupt_notify = Arc::clone(&self.interrupt_notify);

        // Create NAPI output handler
        let output = NapiOutput::new(&callback);

        // Lock session and run the stream
        let mut session = session_arc.lock().await;

        // Get provider and create agent
        let current_provider = session.current_provider_name().to_string();

        let result = match current_provider.as_str() {
            "claude" => {
                let provider = session
                    .provider_manager_mut()
                    .get_claude()
                    .map_err(|e| Error::from_reason(format!("Failed to get provider: {e}")))?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = RigAgent::with_default_depth(rig_agent);
                run_agent_stream(
                    agent,
                    &input,
                    &mut session,
                    is_interrupted,
                    Arc::clone(&interrupt_notify),
                    &output,
                )
                .await
            }
            "openai" => {
                let provider = session
                    .provider_manager_mut()
                    .get_openai()
                    .map_err(|e| Error::from_reason(format!("Failed to get provider: {e}")))?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = RigAgent::with_default_depth(rig_agent);
                run_agent_stream(
                    agent,
                    &input,
                    &mut session,
                    is_interrupted,
                    Arc::clone(&interrupt_notify),
                    &output,
                )
                .await
            }
            "gemini" => {
                let provider = session
                    .provider_manager_mut()
                    .get_gemini()
                    .map_err(|e| Error::from_reason(format!("Failed to get provider: {e}")))?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = RigAgent::with_default_depth(rig_agent);
                run_agent_stream(
                    agent,
                    &input,
                    &mut session,
                    is_interrupted,
                    Arc::clone(&interrupt_notify),
                    &output,
                )
                .await
            }
            "codex" => {
                let provider = session
                    .provider_manager_mut()
                    .get_codex()
                    .map_err(|e| Error::from_reason(format!("Failed to get provider: {e}")))?;
                let rig_agent = provider.create_rig_agent(None);
                let agent = RigAgent::with_default_depth(rig_agent);
                run_agent_stream(
                    agent,
                    &input,
                    &mut session,
                    is_interrupted,
                    interrupt_notify,
                    &output,
                )
                .await
            }
            _ => {
                return Err(Error::from_reason(format!(
                    "Unsupported provider: {current_provider}"
                )));
            }
        };

        result.map_err(|e| Error::from_reason(format!("Stream error: {e}")))
    }
}
