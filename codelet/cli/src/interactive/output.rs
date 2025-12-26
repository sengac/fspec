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
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    /// Tokens per second (smoothed with EMA for stable display)
    pub tokens_per_second: Option<f64>,
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
        }
    }
}
