//! CodeletSession - Main class exposed to JavaScript
//!
//! Wraps codelet's Session and provides async streaming prompts via ThreadsafeFunction.
//!
//! Uses the same agent infrastructure as codelet-cli:
//! - ProviderManager for consistent provider access
//! - System-reminders for context (CLAUDE.md, environment)
//! - RigAgent with all 9 tools for full agent capabilities

use crate::types::{Message, StreamChunk, TokenTracker, ToolCallInfo, ToolResultInfo};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::sync::{Arc, Mutex};

/// CodeletSession - Main class for AI agent interactions
///
/// Exposes codelet's Rust AI agent functionality to Node.js.
#[napi]
pub struct CodeletSession {
    /// Inner session from codelet-cli
    inner: Arc<Mutex<codelet_cli::session::Session>>,
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
        })
    }

    /// Get the current provider name
    #[napi(getter)]
    pub fn current_provider_name(&self) -> Result<String> {
        let session = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;
        Ok(session.current_provider_name().to_string())
    }

    /// Get list of available providers
    #[napi(getter)]
    pub fn available_providers(&self) -> Result<Vec<String>> {
        let session = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;

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
        let session = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;

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
        })
    }

    /// Get conversation messages (simplified representation)
    #[napi(getter)]
    pub fn messages(&self) -> Result<Vec<Message>> {
        let session = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;

        let messages: Vec<Message> = session
            .messages
            .iter()
            .map(|msg| {
                let (role, content) = match msg {
                    rig::message::Message::User { content, .. } => {
                        // Use iter() to iterate over OneOrMany
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
                        // Use iter() to iterate over OneOrMany
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
        let mut session = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;

        session
            .switch_provider(&provider_name)
            .map_err(|e| Error::from_reason(format!("Failed to switch provider: {e}")))?;

        Ok(())
    }

    /// Clear conversation history
    #[napi]
    pub fn clear_history(&self) -> Result<()> {
        let mut session = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;

        session.messages.clear();
        session.turns.clear();
        session.token_tracker = codelet_core::compaction::TokenTracker {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_input_tokens: Some(0),
            cache_creation_input_tokens: Some(0),
        };

        Ok(())
    }

    /// Send a prompt and stream the response
    ///
    /// The callback receives StreamChunk objects with type: 'Text', 'ToolCall', 'ToolResult', 'Done', or 'Error'
    ///
    /// Uses the same agent infrastructure as codelet-cli:
    /// - ProviderManager for provider access (consistent with CLI)
    /// - System-reminders in messages provide context (CLAUDE.md, environment)
    /// - RigAgent with all 9 tools (Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, WebSearch)
    #[napi]
    pub async fn prompt(
        &self,
        input: String,
        #[napi(ts_arg_type = "(chunk: StreamChunk) => void")] callback: ThreadsafeFunction<
            StreamChunk,
            ErrorStrategy::Fatal,
        >,
    ) -> Result<()> {
        use codelet_core::RigAgent;
        use rig::message::{Message as RigMessage, UserContent};
        use rig::OneOrMany;

        // Clone Arc for use in async block
        let session_arc = Arc::clone(&self.inner);

        // Get current provider name from session
        // Note: CLAUDE.md content is in messages via system-reminders (same as CLI)
        let current_provider = {
            let session = session_arc
                .lock()
                .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;
            session.current_provider_name().to_string()
        };

        // Add user message to history
        {
            let mut session = session_arc
                .lock()
                .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;
            session.messages.push(RigMessage::User {
                content: OneOrMany::one(UserContent::text(&input)),
            });
        }

        // Get messages for streaming
        let mut messages = {
            let session = session_arc
                .lock()
                .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;
            session.messages.clone()
        };

        // Macro to eliminate code duplication across provider branches
        // Uses ProviderManager to get providers (consistent with CLI agent_runner.rs)
        // Note: Preamble is None - CLAUDE.md context comes via system-reminders in messages
        macro_rules! run_with_provider {
            ($session_arc:expr, $get_method:ident, $input:expr, $messages:expr, $callback:expr) => {{
                let provider = {
                    let mut session = $session_arc
                        .lock()
                        .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;
                    session
                        .provider_manager_mut()
                        .$get_method()
                        .map_err(|e| Error::from_reason(format!("Failed to get provider: {e}")))?
                };
                // Pass None for preamble - same as CLI (CLAUDE.md is in messages as system-reminder)
                let rig_agent = provider.create_rig_agent(None);
                let agent = RigAgent::with_default_depth(rig_agent);
                stream_with_agent(agent, $input, $messages, $callback).await
            }};
        }

        // Build agent and stream based on provider
        // Uses ProviderManager for consistent provider access (same as CLI)
        let result = match current_provider.as_str() {
            "claude" => {
                run_with_provider!(session_arc, get_claude, &input, &mut messages, callback)
            }
            "openai" => {
                run_with_provider!(session_arc, get_openai, &input, &mut messages, callback)
            }
            "gemini" => {
                run_with_provider!(session_arc, get_gemini, &input, &mut messages, callback)
            }
            "codex" => {
                run_with_provider!(session_arc, get_codex, &input, &mut messages, callback)
            }
            _ => {
                return Err(Error::from_reason(format!(
                    "Unsupported provider: {current_provider}"
                )));
            }
        };

        // Update session messages after streaming
        {
            let mut session = session_arc
                .lock()
                .map_err(|e| Error::from_reason(format!("Lock poisoned: {e}")))?;
            session.messages = messages;
        }

        result
    }
}

/// Stream with a specific agent type
async fn stream_with_agent<M>(
    agent: codelet_core::RigAgent<M>,
    input: &str,
    messages: &mut Vec<rig::message::Message>,
    callback: ThreadsafeFunction<StreamChunk, ErrorStrategy::Fatal>,
) -> Result<()>
where
    M: rig::completion::CompletionModel,
    M::StreamingResponse: rig::wasm_compat::WasmCompatSend + rig::completion::GetTokenUsage,
{
    use futures::StreamExt;
    use rig::agent::MultiTurnStreamItem;
    use rig::streaming::{StreamedAssistantContent, StreamedUserContent};

    let mut stream = agent.prompt_streaming_with_history(input, messages).await;

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text))) => {
                callback.call(
                    StreamChunk::text(text.text.clone()),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(
                tool_call,
            ))) => {
                let info = ToolCallInfo {
                    id: tool_call.id.clone(),
                    name: tool_call.function.name.clone(),
                    input: serde_json::to_string(&tool_call.function.arguments).unwrap_or_default(),
                };
                callback.call(
                    StreamChunk::tool_call(info),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(
                tool_result,
            ))) => {
                // Use iter() to iterate over OneOrMany content
                let content = tool_result
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        rig::message::ToolResultContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let content = if content.is_empty() {
                    "[non-text result]".to_string()
                } else {
                    content
                };

                let info = ToolResultInfo {
                    tool_call_id: tool_result.id.clone(),
                    content,
                    is_error: false, // ToolResult doesn't have is_error field
                };
                callback.call(
                    StreamChunk::tool_result(info),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                // Stream complete
                callback.call(StreamChunk::done(), ThreadsafeFunctionCallMode::NonBlocking);
                break;
            }
            Err(e) => {
                callback.call(
                    StreamChunk::error(e.to_string()),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
                return Err(Error::from_reason(format!("Streaming error: {e}")));
            }
            _ => {}
        }
    }

    Ok(())
}
