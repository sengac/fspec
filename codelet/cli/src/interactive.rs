//! Interactive TUI mode entry point
//!
//! Main REPL loop coordinating terminal events, agent streaming, and user input.
//! Based on OpenAI codex architecture with tokio::select! pattern.

use crate::interactive_helpers::execute_compaction;
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::{
    get_debug_capture_manager, handle_debug_command, SessionMetadata,
};
use codelet_core::RigAgent;
use codelet_tui::{create_event_stream, InputQueue, StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::CompletionModel;
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{debug, info};

/// Approximate bytes per token for estimation (matches codelet's APPROX_BYTES_PER_TOKEN)
const APPROX_BYTES_PER_TOKEN: usize = 4;

/// Estimate token count from text length
///
/// Used as fallback when API doesn't provide usage metrics.
/// Based on codelet's estimateTokens function.
fn estimate_tokens(text: &str) -> u64 {
    (text.len() / APPROX_BYTES_PER_TOKEN) as u64
}

/// Run interactive TUI mode
pub async fn run_interactive_mode(provider_name: Option<&str>) -> Result<()> {
    // Initialize session with persistent context (CLI-008)
    let mut session = Session::new(provider_name)?;

    // CLI-016: Inject context reminders (CLAUDE.md discovery + environment info)
    session.inject_context_reminders();

    // Display startup card
    display_startup_card(&session)?;

    // Main REPL loop (raw mode is enabled/disabled per-request, not globally)
    let result = repl_loop(&mut session).await;

    result
}

/// Display startup card showing available providers
fn display_startup_card(session: &Session) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    println!("\nCodelet v{version}");

    let manager = session.provider_manager();
    if !manager.has_any_provider() {
        println!("Available models: No providers configured");
        println!("Please set ANTHROPIC_API_KEY, OPENAI_API_KEY, or other credentials\n");
    } else {
        let providers = manager.list_available_providers();
        println!("Available models: {}\n", providers.join(", "));
    }

    Ok(())
}

/// Main REPL loop
async fn repl_loop(session: &mut Session) -> Result<()> {
    let mut input_queue = InputQueue::new();
    let is_interrupted = Arc::new(AtomicBool::new(false));

    println!("Enter your prompt (or 'exit' to quit):");

    loop {
        // Read user input with provider-prefixed prompt
        print!("{}", session.provider_manager().get_prompt_prefix());
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Check for exit
        if matches!(input, "exit" | "/quit" | "quit") {
            println!("Goodbye!");
            break;
        }

        // Handle /debug command - CLI-022
        if input == "/debug" {
            let result = handle_debug_command();
            // Set session metadata when enabling debug capture
            if result.enabled {
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
            // CLI-022: Capture command.executed event
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "command.executed",
                            serde_json::json!({
                                "command": "/debug",
                                "result": if result.enabled { "enabled" } else { "disabled" },
                            }),
                            None,
                        );
                    }
                }
            }
            println!("{}\n", result.message);
            continue;
        }

        // Check for provider switch - CLEARS CONTEXT (CLI-008)
        if input.starts_with('/') {
            let provider = input.trim_start_matches('/');
            // Capture provider.switch event - CLI-022
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "provider.switch",
                            serde_json::json!({
                                "from": session.current_provider_name(),
                                "to": provider,
                            }),
                            None,
                        );
                    }
                }
            }
            match session.switch_provider(provider) {
                Ok(()) => {
                    info!("Provider switched to: {}", provider);
                    println!("Switched to {provider} provider\n");
                    continue;
                }
                Err(e) => {
                    debug!("Provider switch failed: {}", e);
                    eprintln!("Error switching provider: {e}\n");
                    continue;
                }
            }
        }

        if input.is_empty() {
            continue;
        }

        // Capture user.input event - CLI-022
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    manager.capture(
                        "user.input",
                        serde_json::json!({
                            "input": input,
                            "inputLength": input.len(),
                        }),
                        None,
                    );
                    // Increment turn for each user input
                    manager.increment_turn();
                }
            }
        }

        // Run agent with interruption support and persistent context (CLI-008)
        // Enable raw mode only during agent execution for ESC key detection
        is_interrupted.store(false, Ordering::Relaxed);
        enable_raw_mode()?;
        let mut event_stream = create_event_stream();

        let agent_result = run_agent_with_interruption(
            session,
            input,
            &mut event_stream,
            &mut input_queue,
            is_interrupted.clone(),
        )
        .await;

        // Always disable raw mode after agent completes
        disable_raw_mode()?;

        match agent_result {
            Ok(()) => println!("\n"),
            Err(e) => eprintln!("Error: {e}\n"),
        }
    }

    Ok(())
}

/// Run agent with interruption support and persistent context (CLI-008)
async fn run_agent_with_interruption(
    session: &mut Session,
    prompt: &str,
    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),
    input_queue: &mut InputQueue,
    is_interrupted: Arc<AtomicBool>,
) -> Result<()> {
    // Get provider name before mutable borrow (to satisfy borrow checker)
    let provider_name = session.current_provider_name().to_string();
    let manager = session.provider_manager_mut();

    // Macro to eliminate code duplication across provider branches (DRY principle)
    macro_rules! run_with_provider {
        ($get_provider:ident) => {{
            let provider = manager.$get_provider()?;
            let rig_agent = provider.create_rig_agent();
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream_with_interruption(
                agent,
                prompt,
                session, // Pass entire session for token tracking and compaction (CLI-010)
                event_stream,
                input_queue,
                is_interrupted,
            )
            .await
        }};
    }

    // Dispatch to provider-specific agent
    match provider_name.as_str() {
        "claude" => run_with_provider!(get_claude),
        "openai" => run_with_provider!(get_openai),
        "codex" => run_with_provider!(get_codex),
        "gemini" => run_with_provider!(get_gemini),
        _ => Err(anyhow::anyhow!("Unknown provider")),
    }
}

/// Add assistant text message to message history (CLI-008)
/// Extracted helper to eliminate code duplication
fn add_assistant_text_message(messages: &mut Vec<rig::message::Message>, text: String) {
    use rig::message::{AssistantContent, Message, Text};
    use rig::OneOrMany;

    messages.push(Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::Text(Text { text })),
    });
}

/// Add assistant tool calls message to message history (CLI-008)
/// Extracted helper with proper error handling for OneOrMany::many()
fn add_assistant_tool_calls_message(
    messages: &mut Vec<rig::message::Message>,
    tool_calls: Vec<rig::message::AssistantContent>,
) -> Result<()> {
    use rig::message::Message;
    use rig::OneOrMany;
    use tracing::error;

    match OneOrMany::many(tool_calls) {
        Ok(content) => {
            messages.push(Message::Assistant { id: None, content });
            Ok(())
        }
        Err(e) => {
            error!("Failed to convert tool calls to message: {:?}", e);
            Err(anyhow::anyhow!("Failed to convert tool calls: {e:?}"))
        }
    }
}

/// Handle text chunk from agent stream
fn handle_text_chunk(
    text: &str,
    assistant_text: &mut String,
    request_id: Option<&str>,
) -> Result<()> {
    // CLI-022: Capture api.response.chunk event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.response.chunk",
                    serde_json::json!({
                        "chunkLength": text.len(),
                    }),
                    request_id.map(|id| codelet_common::debug_capture::CaptureOptions {
                        request_id: Some(id.to_string()),
                    }),
                );
            }
        }
    }

    // CRITICAL: Accumulate for message history (CLI-008)
    assistant_text.push_str(text);

    // Print text immediately for real-time streaming
    // Replace \n with \r\n for proper terminal display in raw mode
    let display_text = text.replace('\n', "\r\n");
    print!("{display_text}");
    std::io::stdout().flush()?;

    Ok(())
}

/// Handle tool call from agent stream
fn handle_tool_call(
    tool_call: &rig::message::ToolCall,
    messages: &mut Vec<rig::message::Message>,
    assistant_text: &mut String,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &mut Option<String>,
) -> Result<()> {
    use rig::message::AssistantContent;

    // CRITICAL: Add assistant text to messages if we have any (CLI-008)
    if !assistant_text.is_empty() {
        add_assistant_text_message(messages, assistant_text.clone());
        assistant_text.clear();
    }

    // Display tool name and arguments before execution
    let tool_name = &tool_call.function.name;
    let args = &tool_call.function.arguments;

    // CLI-022: Capture tool.call event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "tool.call",
                    serde_json::json!({
                        "toolName": tool_name,
                        "toolId": tool_call.id,
                        "arguments": args,
                    }),
                    None,
                );
            }
        }
    }

    // Store tool name for debug logging
    *last_tool_name = Some(tool_name.clone());

    // CRITICAL: Track tool call for message history (CLI-008)
    tool_calls_buffer.push(AssistantContent::ToolCall(tool_call.clone()));

    // Log full details
    debug!("Tool call: {} with args: {:?}", tool_name, args);

    // Display tool name
    print!("\r\n[Planning to use tool: {tool_name}]");

    // Display arguments
    if let Some(obj) = args.as_object() {
        if !obj.is_empty() {
            for (key, value) in obj.iter() {
                // Format value based on type
                let formatted_value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Array(_) => format!("{value}"),
                    serde_json::Value::Object(_) => format!("{value}"),
                    serde_json::Value::Null => "null".to_string(),
                };
                print!("\r\n  {key}: {formatted_value}");
            }
        }
    }
    println!("\r\n");
    std::io::stdout().flush()?;

    Ok(())
}

/// Handle tool result from agent stream
fn handle_tool_result(
    tool_result: &rig::message::ToolResult,
    messages: &mut Vec<rig::message::Message>,
    tool_calls_buffer: &mut Vec<rig::message::AssistantContent>,
    last_tool_name: &Option<String>,
) -> Result<()> {
    use rig::message::{Message, ToolResultContent, UserContent};
    use rig::OneOrMany;

    // CRITICAL: Add buffered tool calls as assistant message (CLI-008)
    // This happens ONCE before the first tool result
    if !tool_calls_buffer.is_empty() {
        add_assistant_tool_calls_message(messages, tool_calls_buffer.clone())?;
        tool_calls_buffer.clear();
    }

    // CRITICAL: Add tool result to message history (CLI-008)
    let tool_result_clone = tool_result.clone();
    messages.push(Message::User {
        content: OneOrMany::one(UserContent::ToolResult(tool_result_clone)),
    });

    // Display tool result with preview (similar to codelet pattern)
    debug!("Tool result received: {:?}", tool_result);

    // CLI-022: Capture tool.result event
    // Determine success from result content (check for error indicators)
    let is_error = tool_result.content.clone().into_iter().any(|c| match c {
        ToolResultContent::Text(t) => t.text.contains("Error:") || t.text.contains("error:"),
        _ => false,
    });

    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                let event_type = if is_error {
                    "tool.error"
                } else {
                    "tool.result"
                };
                manager.capture(
                    event_type,
                    serde_json::json!({
                        "toolName": last_tool_name.as_deref().unwrap_or("unknown"),
                        "toolId": tool_result.id,
                        "success": !is_error,
                    }),
                    None,
                );
            }
        }
    }

    // Extract text content from tool result by iterating over OneOrMany
    let result_parts: Vec<String> = tool_result
        .content
        .clone()
        .into_iter()
        .map(|content| match content {
            ToolResultContent::Text(text) => text.text,
            ToolResultContent::Image(_) => "[Image]".to_string(),
        })
        .collect();

    let mut result_text = result_parts.join("\n");

    // Strip surrounding quotes if present (JSON-escaped string)
    if result_text.starts_with('"') && result_text.ends_with('"') {
        result_text = result_text[1..result_text.len() - 1].to_string();
    }

    // Unescape common JSON escape sequences
    result_text = result_text
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\r", "\r")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\");

    // Truncate result if too long (like codelet does at 500 chars)
    const MAX_PREVIEW_LENGTH: usize = 500;
    let preview = if result_text.len() > MAX_PREVIEW_LENGTH {
        format!("{}...", &result_text[..MAX_PREVIEW_LENGTH])
    } else {
        result_text
    };

    // Display with proper formatting for raw mode
    // Indent each line by 2 spaces and replace \n with \r\n
    let indented_lines: Vec<String> = preview.lines().map(|line| format!("  {line}")).collect();
    let formatted_preview = indented_lines.join("\r\n");

    print!("\r\n[Tool result preview]\r\n-------\r\n{formatted_preview}\r\n-------\r\n");
    std::io::stdout().flush()?;

    Ok(())
}

/// Handle final response from agent stream
fn handle_final_response(
    assistant_text: &str,
    messages: &mut Vec<rig::message::Message>,
) -> Result<()> {
    // CRITICAL: Add final assistant text to message history (CLI-008)
    if !assistant_text.is_empty() {
        add_assistant_text_message(messages, assistant_text.to_owned());
    }

    Ok(())
}

/// Run agent stream with interruption coordination using tokio::select!
/// CRITICAL: Now uses .with_history() for persistent context (CLI-008)
/// CRITICAL: Now tracks tokens and triggers compaction (CLI-010)
async fn run_agent_stream_with_interruption<M>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session, // CLI-010: Need full session for token tracking and compaction
    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),
    input_queue: &mut InputQueue,
    is_interrupted: Arc<AtomicBool>,
) -> Result<()>
where
    M: CompletionModel,
{
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;
    use std::time::Instant;
    use uuid::Uuid;

    // CLI-022: Generate request ID for correlation
    let request_id = Uuid::new_v4().to_string();
    let api_start_time = Instant::now();

    // CLI-022: Capture api.request event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.request",
                    serde_json::json!({
                        "provider": session.current_provider_name(),
                        "model": session.current_provider_name(),
                        "prompt": prompt,
                        "promptLength": prompt.len(),
                        "messageCount": session.messages.len(),
                    }),
                    Some(codelet_common::debug_capture::CaptureOptions {
                        request_id: Some(request_id.clone()),
                    }),
                );
            }
        }
    }

    // CRITICAL: Add user prompt to message history for persistence (CLI-008)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(prompt)),
    });

    // CRITICAL FIX: Pass conversation history to rig for context persistence (CLI-008)
    let mut stream = agent
        .prompt_streaming_with_history(prompt, &mut session.messages)
        .await;

    // CLI-022: Capture api.response.start event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.response.start",
                    serde_json::json!({
                        "provider": session.current_provider_name(),
                    }),
                    Some(codelet_common::debug_capture::CaptureOptions {
                        request_id: Some(request_id.clone()),
                    }),
                );
            }
        }
    }

    let status = StatusDisplay::new();
    let mut status_interval = interval(Duration::from_secs(1));

    // Track assistant response content for adding to messages (CLI-008)
    let mut assistant_text = String::new();
    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();

    // Track last tool call for debug logging
    let mut last_tool_name: Option<String> = None;

    // CLI-010: Track token usage for compaction
    let mut turn_input_tokens: u64 = 0;
    let mut turn_output_tokens: u64 = 0;
    let mut turn_cache_read_tokens: Option<u64> = None;
    let mut turn_cache_creation_tokens: Option<u64> = None;

    loop {
        tokio::select! {
            // Agent streaming
            chunk = stream.next() => {
                if is_interrupted.load(Ordering::Relaxed) {
                    // Interrupted - show queued inputs
                    println!("\n⚠️ Agent interrupted");
                    let queued = input_queue.dequeue_all();
                    if queued.is_empty() {
                        println!("Queued inputs: (none)");
                    } else {
                        println!("Queued inputs:\n{}", queued.join("\n\n"));
                    }
                    break;
                }

                match chunk {
                    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text)))) => {
                        handle_text_chunk(
                            &text.text,
                            &mut assistant_text,
                            Some(&request_id),
                        )?;
                    }
                    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(tool_call)))) => {
                        handle_tool_call(
                            &tool_call,
                            &mut session.messages,
                            &mut assistant_text,
                            &mut tool_calls_buffer,
                            &mut last_tool_name,
                        )?;
                    }
                    Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(tool_result)))) => {
                        handle_tool_result(
                            &tool_result,
                            &mut session.messages,
                            &mut tool_calls_buffer,
                            &last_tool_name,
                        )?;
                    }
                    Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                        // CLI-010: Extract token usage from FinalResponse
                        let usage = final_resp.usage();

                        // Use API values if available, otherwise estimate (matches codelet fallback pattern)
                        turn_input_tokens = if usage.input_tokens > 0 {
                            usage.input_tokens
                        } else {
                            estimate_tokens(prompt)
                        };

                        turn_output_tokens = if usage.output_tokens > 0 {
                            usage.output_tokens
                        } else {
                            estimate_tokens(&assistant_text)
                        };

                        // CLI-014: Cache token extraction from Anthropic API
                        // LIMITATION: rig 0.25+ streaming abstracts away Anthropic-specific cache fields.
                        // The MessageStart SSE event has full Usage with cache_read_input_tokens and
                        // cache_creation_input_tokens, but rig only extracts input_tokens (line 236 in
                        // rig-core/src/providers/anthropic/streaming.rs). The cache fields are lost when
                        // converting to PartialUsage and StreamingCompletionResponse.
                        //
                        // Infrastructure ready: TokenTracker, effective_tokens(), accumulation logic.
                        // When rig exposes cache tokens, update here to extract from FinalResponse.
                        // For non-Anthropic providers, cache tokens correctly remain None.
                        turn_cache_read_tokens = None;
                        turn_cache_creation_tokens = None;

                        // CLI-022: Capture api.response.end event with token usage
                        if let Ok(manager_arc) = get_debug_capture_manager() {
                            if let Ok(mut manager) = manager_arc.lock() {
                                if manager.is_enabled() {
                                    let duration_ms = api_start_time.elapsed().as_millis() as u64;
                                    manager.capture(
                                        "api.response.end",
                                        serde_json::json!({
                                            "duration": duration_ms,
                                            "usage": {
                                                "inputTokens": turn_input_tokens,
                                                "outputTokens": turn_output_tokens,
                                            },
                                            "responseLength": assistant_text.len(),
                                        }),
                                        Some(codelet_common::debug_capture::CaptureOptions {
                                            request_id: Some(request_id.clone()),
                                        }),
                                    );

                                    // CLI-022: Capture token.update event
                                    // Note: totalInputTokens = inputTokens since we REPLACE (not accumulate)
                                    manager.capture(
                                        "token.update",
                                        serde_json::json!({
                                            "inputTokens": turn_input_tokens,
                                            "outputTokens": turn_output_tokens,
                                            "totalInputTokens": turn_input_tokens,
                                            "totalOutputTokens": turn_output_tokens,
                                        }),
                                        None,
                                    );
                                }
                            }
                        }

                        handle_final_response(
                            &assistant_text,
                            &mut session.messages,
                        )?;
                        // Stream complete
                        break;
                    }
                    Some(Err(e)) => {
                        // CLI-022: Capture api.error event
                        if let Ok(manager_arc) = get_debug_capture_manager() {
                            if let Ok(mut manager) = manager_arc.lock() {
                                if manager.is_enabled() {
                                    manager.capture(
                                        "api.error",
                                        serde_json::json!({
                                            "error": e.to_string(),
                                            "duration": api_start_time.elapsed().as_millis() as u64,
                                        }),
                                        Some(codelet_common::debug_capture::CaptureOptions {
                                            request_id: Some(request_id.clone()),
                                        }),
                                    );
                                }
                            }
                        }
                        return Err(anyhow::anyhow!("Agent error: {e}"));
                    }
                    _ => {
                        // Other stream items
                    }
                }
            }

            // Terminal events
            event = event_stream.next() => {
                match event {
                    Some(TuiEvent::Key(key)) if key.code == KeyCode::Esc => {
                        is_interrupted.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }

            // Status display updates
            _ = status_interval.tick() => {
                // Update status (in real implementation, would render to UI)
                // For now, just track elapsed time
                let _ = status.format_status();
            }
        }
    }

    // CLI-010: After stream completes, update tokens and check compaction
    if !is_interrupted.load(Ordering::Relaxed) {
        // Replace token tracker with API usage values (matches TypeScript runner.ts:1143)
        // TypeScript: inputTokens: usage.inputTokens ?? tokenTracker.inputTokens
        // The API reports total input tokens for the current context, not incremental
        session.token_tracker.input_tokens = turn_input_tokens;
        session.token_tracker.output_tokens = turn_output_tokens;
        if let Some(cache_read) = turn_cache_read_tokens {
            let current = session.token_tracker.cache_read_input_tokens.unwrap_or(0);
            session.token_tracker.cache_read_input_tokens = Some(current + cache_read);
        }
        if let Some(cache_create) = turn_cache_creation_tokens {
            let current = session
                .token_tracker
                .cache_creation_input_tokens
                .unwrap_or(0);
            session.token_tracker.cache_creation_input_tokens = Some(current + cache_create);
        }

        // CTX-002: Removed eager turn creation - now done lazily during compaction
        // Turns are created from session.messages inside execute_compaction() following TypeScript implementation

        // Check if compaction should trigger
        // CLI-015: Use model-specific context window instead of hardcoded value
        // CLI-020: Apply autocompact buffer to leave headroom after compaction
        use crate::compaction_threshold::calculate_compaction_threshold;
        let context_window = session.provider_manager().context_window() as u64;
        let threshold = calculate_compaction_threshold(context_window);
        let effective = session.token_tracker.effective_tokens();

        if effective > threshold {
            // CLI-022: Capture compaction.triggered event
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "compaction.triggered",
                            serde_json::json!({
                                "effectiveTokens": effective,
                                "threshold": threshold,
                                "contextWindow": context_window,
                            }),
                            None,
                        );
                    }
                }
            }

            println!("\n[Generating summary...]");
            std::io::stdout().flush()?;

            // Execute compaction
            match execute_compaction(session).await {
                Ok(metrics) => {
                    // CLI-022: Capture context.update event after compaction
                    if let Ok(manager_arc) = get_debug_capture_manager() {
                        if let Ok(mut manager) = manager_arc.lock() {
                            if manager.is_enabled() {
                                manager.capture(
                                    "context.update",
                                    serde_json::json!({
                                        "type": "compaction",
                                        "originalTokens": metrics.original_tokens,
                                        "compactedTokens": metrics.compacted_tokens,
                                        "compressionRatio": metrics.compression_ratio,
                                    }),
                                    None,
                                );
                            }
                        }
                    }

                    println!(
                        "[Context compacted: {}→{} tokens, {:.0}% compression]\n",
                        metrics.original_tokens,
                        metrics.compacted_tokens,
                        metrics.compression_ratio * 100.0
                    );
                }
                Err(e) => {
                    eprintln!("Warning: Compaction failed: {e}");
                }
            }
        }
    }

    Ok(())
}
