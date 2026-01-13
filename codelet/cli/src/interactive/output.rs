//! Stream output abstraction for CLI vs NAPI rendering
//!
//! This module provides an enum-based event abstraction that allows the stream loop
//! to work with both CLI output (stdout) and NAPI output (JavaScript callbacks).
//!
//! Uses a single `emit(StreamEvent)` method instead of multiple trait methods,
//! reducing virtual dispatch overhead and simplifying the API.
//!
//! The trait separates I/O concerns from message history management, enabling
//! code reuse between codelet-cli and codelet-napi.

use std::io::Write;

/// Token usage information for streaming updates
///
/// PROV-001: input_tokens should be the TOTAL input (raw + cache_read + cache_creation)
/// when displayed to users. Use `from_usage()` to create from ApiTokenUsage.
#[derive(Debug, Clone)]
pub struct TokenInfo {
    /// Total input tokens for display (includes cache)
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    /// Tokens per second (smoothed with EMA for stable display)
    pub tokens_per_second: Option<f64>,
}

impl TokenInfo {
    /// Create TokenInfo from ApiTokenUsage with tokens per second
    ///
    /// PROV-001: This automatically calculates total_input for display.
    pub fn from_usage(usage: codelet_core::ApiTokenUsage, tokens_per_second: Option<f64>) -> Self {
        Self {
            input_tokens: usage.total_input(), // Display total, not raw
            output_tokens: usage.output_tokens,
            cache_read_input_tokens: Some(usage.cache_read_input_tokens),
            cache_creation_input_tokens: Some(usage.cache_creation_input_tokens),
            tokens_per_second,
        }
    }
}

/// Tool call information
#[derive(Debug, Clone)]
pub struct ToolCallEvent {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

/// Tool result information
#[derive(Debug, Clone)]
pub struct ToolResultEvent {
    pub id: String,
    pub content: String,
    pub is_error: bool,
}

/// Context window fill percentage information (TUI-033)
#[derive(Debug, Clone)]
pub struct ContextFillInfo {
    /// Fill percentage (0-100+, can exceed 100 near compaction)
    pub fill_percentage: u32,
    /// Effective tokens (after cache discount)
    pub effective_tokens: u64,
    /// Compaction threshold (usable context after output reservation)
    pub threshold: u64,
    /// Provider's context window size
    pub context_window: u64,
}

/// Tool progress information for streaming bash output (TOOL-011)
#[derive(Debug, Clone)]
pub struct ToolProgressEvent {
    /// Tool call ID this progress is for
    pub tool_call_id: String,
    /// Tool name (e.g., "bash", "run_shell_command")
    pub tool_name: String,
    /// Output chunk (new text since last progress event)
    pub output_chunk: String,
}

/// Stream event enum - all possible events in a single type
///
/// Using an enum instead of multiple trait methods:
/// - Reduces virtual dispatch overhead in hot paths
/// - Enables batching of events before emission
/// - Simplifies the StreamOutput trait to a single method
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Streaming text chunk
    Text(String),
    /// Tool call notification
    ToolCall(ToolCallEvent),
    /// Tool result
    ToolResult(ToolResultEvent),
    /// Stream completion
    Done,
    /// Error occurred
    Error(String),
    /// Agent was interrupted
    Interrupted(Vec<String>),
    /// Status message (e.g., compaction notifications)
    Status(String),
    /// Token usage update
    Tokens(TokenInfo),
    /// Context window fill percentage (TUI-033)
    ContextFill(ContextFillInfo),
    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    ToolProgress(ToolProgressEvent),
    /// Thinking/reasoning content from extended thinking (TOOL-010)
    Thinking(String),
}

/// Stream output handler trait
///
/// Implementations handle rendering stream events to their target output:
/// - CliOutput: Prints to stdout with terminal formatting
/// - NapiOutput: Batches and sends via ThreadsafeFunction callback
///
/// Default methods provide a convenient API that wraps events in StreamEvent.
pub trait StreamOutput: Send + Sync {
    /// Emit a stream event (core method that implementations must provide)
    fn emit(&self, event: StreamEvent);

    /// Flush any buffered events (called periodically and at end of stream)
    /// Default implementation does nothing (for unbuffered outputs like CLI)
    fn flush(&self) {}

    /// Get a clonable emitter for use in tool progress callbacks (TOOL-011)
    ///
    /// This returns an `Arc<dyn StreamOutput>` that can be captured by `'static`
    /// closures (like the global tool progress callback). This is necessary because
    /// tool execution happens inside `stream.next()` and tokio::select! cannot
    /// interleave with it - progress must be emitted directly, not through a channel.
    ///
    /// Default returns None (progress streaming not supported).
    fn progress_emitter(&self) -> Option<std::sync::Arc<dyn StreamOutput>> {
        None
    }

    // Convenience methods with default implementations

    /// Emit streaming text chunk
    #[inline]
    fn emit_text(&self, text: &str) {
        self.emit(StreamEvent::Text(text.to_string()));
    }

    /// Emit tool call notification
    #[inline]
    fn emit_tool_call(&self, id: &str, name: &str, args: &serde_json::Value) {
        self.emit(StreamEvent::ToolCall(ToolCallEvent {
            id: id.to_string(),
            name: name.to_string(),
            args: args.clone(),
        }));
    }

    /// Emit tool result
    #[inline]
    fn emit_tool_result(&self, id: &str, content: &str, is_error: bool) {
        self.emit(StreamEvent::ToolResult(ToolResultEvent {
            id: id.to_string(),
            content: content.to_string(),
            is_error,
        }));
    }

    /// Emit stream completion
    #[inline]
    fn emit_done(&self) {
        self.emit(StreamEvent::Done);
    }

    /// Emit error
    #[inline]
    fn emit_error(&self, error: &str) {
        self.emit(StreamEvent::Error(error.to_string()));
    }

    /// Emit interruption notification
    #[inline]
    fn emit_interrupted(&self, queued_inputs: &[String]) {
        self.emit(StreamEvent::Interrupted(queued_inputs.to_vec()));
    }

    /// Emit status message
    #[inline]
    fn emit_status(&self, message: &str) {
        self.emit(StreamEvent::Status(message.to_string()));
    }

    /// Emit token usage update
    #[inline]
    fn emit_tokens(&self, tokens: &TokenInfo) {
        self.emit(StreamEvent::Tokens(tokens.clone()));
    }

    /// Emit context fill percentage (TUI-033)
    #[inline]
    fn emit_context_fill(&self, info: &ContextFillInfo) {
        self.emit(StreamEvent::ContextFill(info.clone()));
    }

    /// Emit tool execution progress - streaming output from bash/shell tools (TOOL-011)
    #[inline]
    fn emit_tool_progress(&self, tool_call_id: &str, tool_name: &str, output_chunk: &str) {
        self.emit(StreamEvent::ToolProgress(ToolProgressEvent {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            output_chunk: output_chunk.to_string(),
        }));
    }

    /// Emit thinking/reasoning content from extended thinking (TOOL-010)
    #[inline]
    fn emit_thinking(&self, thinking: &str) {
        self.emit(StreamEvent::Thinking(thinking.to_string()));
    }
}

/// CLI output implementation - prints to stdout
///
/// Handles events immediately without buffering since terminal output
/// is already efficient for single-character writes.
pub struct CliOutput;

impl StreamOutput for CliOutput {
    fn emit(&self, event: StreamEvent) {
        match event {
            StreamEvent::Text(text) => {
                // Replace \n with \r\n for proper terminal display in raw mode
                let display_text = text.replace('\n', "\r\n");
                print!("{display_text}");
                std::io::stdout().flush().ok();
            }
            StreamEvent::ToolCall(tool_call) => {
                // Display tool name
                print!("\r\n[Planning to use tool: {}]", tool_call.name);

                // Display arguments
                if let Some(obj) = tool_call.args.as_object() {
                    if !obj.is_empty() {
                        for (key, value) in obj.iter() {
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
                std::io::stdout().flush().ok();
            }
            StreamEvent::ToolResult(tool_result) => {
                // Truncate if too long
                const MAX_PREVIEW_LENGTH: usize = 500;
                let preview = if tool_result.content.len() > MAX_PREVIEW_LENGTH {
                    format!("{}...", &tool_result.content[..MAX_PREVIEW_LENGTH])
                } else {
                    tool_result.content
                };

                // Indent each line and format for raw mode
                let indented_lines: Vec<String> =
                    preview.lines().map(|line| format!("  {line}")).collect();
                let formatted_preview = indented_lines.join("\r\n");

                print!(
                    "\r\n[Tool result preview]\r\n-------\r\n{formatted_preview}\r\n-------\r\n"
                );
                std::io::stdout().flush().ok();
            }
            StreamEvent::Done => {
                // CLI doesn't need explicit done signal
            }
            StreamEvent::Error(error) => {
                eprintln!("\r\nError: {error}");
            }
            StreamEvent::Interrupted(queued_inputs) => {
                // Use \r\n for raw mode compatibility
                print!("\r\n⚠️ Agent interrupted\r\n");
                if queued_inputs.is_empty() {
                    print!("Queued inputs: (none)\r\n");
                } else {
                    let joined = queued_inputs.join("\r\n\r\n");
                    print!("Queued inputs:\r\n{joined}\r\n");
                }
                std::io::stdout().flush().ok();
            }
            StreamEvent::Status(message) => {
                // Use \r\n for raw mode compatibility
                let formatted = message.replace('\n', "\r\n");
                print!("{formatted}\r\n");
                std::io::stdout().flush().ok();
            }
            StreamEvent::Tokens(_) => {
                // CLI doesn't display real-time token updates (shown in status line instead)
            }
            StreamEvent::ContextFill(_) => {
                // CLI doesn't display context fill percentage (TUI-only feature)
            }
            StreamEvent::ToolProgress(progress) => {
                // TOOL-011: Stream bash output to terminal in real-time
                // Replace \n with \r\n for proper terminal display in raw mode
                let display_text = progress.output_chunk.replace('\n', "\r\n");
                print!("{display_text}");
                std::io::stdout().flush().ok();
            }
            StreamEvent::Thinking(thinking) => {
                // TOOL-010: Display thinking/reasoning content
                // Format similar to Gemini CLI's LoadingIndicator
                let display_text = thinking.replace('\n', "\r\n");
                print!("\r\n💭 {display_text}\r\n");
                std::io::stdout().flush().ok();
            }
        }
    }

    /// TOOL-011: Return a clonable emitter for tool progress callbacks
    ///
    /// CliOutput is a stateless unit struct, so we can simply create a new
    /// instance wrapped in Arc. This enables the global tool progress callback
    /// to emit directly to stdout without going through a channel.
    fn progress_emitter(&self) -> Option<std::sync::Arc<dyn StreamOutput>> {
        Some(std::sync::Arc::new(CliOutput))
    }
}

// TOOL-011: Blanket implementation for Arc<O> to enable shared ownership
// This allows the tool progress callback to emit directly via StreamOutput
// without going through a channel that would block during tool execution.
impl<O: StreamOutput> StreamOutput for std::sync::Arc<O> {
    fn emit(&self, event: StreamEvent) {
        (**self).emit(event);
    }

    fn flush(&self) {
        (**self).flush();
    }
}
