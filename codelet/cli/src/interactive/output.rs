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

use crate::error_display::{format_cli_error, format_tool_error};
use std::io::Write;
use tracing::{error, warn};

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
    /// Reasoning/thinking tokens (OpenAI o-series, Codex extended thinking)
    pub reasoning_tokens: Option<u64>,
}

impl TokenInfo {
    /// Create TokenInfo from ApiTokenUsage with tokens per second
    ///
    /// PROV-001: This automatically calculates total_input for display.
    pub fn from_usage(usage: codelet_core::ApiTokenUsage, tokens_per_second: Option<f64>) -> Self {
        let reasoning = if usage.reasoning_tokens > 0 {
            Some(usage.reasoning_tokens)
        } else {
            None
        };
        Self {
            input_tokens: usage.total_input(), // Display total, not raw
            output_tokens: usage.output_tokens,
            cache_read_input_tokens: Some(usage.cache_read_input_tokens),
            cache_creation_input_tokens: Some(usage.cache_creation_input_tokens),
            tokens_per_second,
            reasoning_tokens: reasoning,
        }
    }
}

impl From<codelet_core::TokenDisplayUpdate> for TokenInfo {
    fn from(update: codelet_core::TokenDisplayUpdate) -> Self {
        let reasoning = if update.reasoning_tokens > 0 {
            Some(update.reasoning_tokens)
        } else {
            None
        };
        Self {
            input_tokens: update.total_input(), // Display total, not raw (PROV-001)
            output_tokens: update.output_tokens,
            cache_read_input_tokens: Some(update.cache_read_tokens),
            cache_creation_input_tokens: Some(update.cache_creation_tokens),
            tokens_per_second: update.tokens_per_second,
            reasoning_tokens: reasoning,
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
    /// Whether this output is from stderr (should be styled as error/red)
    pub is_stderr: bool,
}

/// UX-002: Compaction progress information for structured events
#[derive(Debug, Clone)]
pub struct CompactionProgressInfo {
    /// Current phase (e.g., "Preparing compaction", "Analyzing context")
    pub phase: String,
    /// Current progress count
    pub current: u32,
    /// Total items to process
    pub total: u32,
}

/// CONT-007: which counter transition produced a [`ContinueStateEvent`].
///
/// The reason exists on the CLI-side event so `CliOutput` can render
/// the stdout nudging line for a consumed nudge (preserving CLI repl
/// behavior); the background twins DROP it when mapping to the pure-state
/// `StreamChunk::ContinueStateUpdate` — except `GoalSatisfied`, which they
/// translate to `goalCleared: true` after performing the CONT-008 chrome
/// goal write-back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueStateReason {
    /// A new real user turn started (counters just reset).
    TurnStart,
    /// The refund accounting for a nudged segment settled.
    RefundSettled,
    /// A zero-progress nudge was consumed.
    NudgeConsumed,
    /// The zero-progress budget is exhausted.
    BudgetExhausted,
    /// An accepted done() ran the shared FinishWithSummary teardown
    /// without an active goal.
    DoneAccepted,
    /// CONT-008: an accepted done() satisfied and cleared an ACTIVE goal
    /// in the shared teardown — the dedicated goal-cleared signal. The
    /// background twins write the chrome goal state back and set
    /// `goalCleared: true` on the wire so the TUI drops its 🎯 cache.
    GoalSatisfied,
}

/// CONT-007: live auto-continue / goal counter snapshot emitted at every
/// counter transition. Mirrors `codelet_rpc_types::ContinueStateInfo`
/// plus the CLI-only [`ContinueStateReason`].
#[derive(Debug, Clone)]
pub struct ContinueStateEvent {
    pub enabled: bool,
    pub budget: u32,
    pub nudges_used: u32,
    pub goal_active: bool,
    /// Display budget: `max(explicit, 15)` while a goal is active,
    /// the explicit `/continue` budget otherwise.
    pub effective_budget: u32,
    /// CONT-008: done() rejection count (`session.done_rejections`,
    /// registry-synced at the settle point). Carried to the TUI so bare
    /// `/goal` shows real rejections instead of a hard-coded 0.
    pub done_rejections: u32,
    pub reason: ContinueStateReason,
}

/// UX-002: Compaction completion result for structured events
#[derive(Debug, Clone)]
pub struct CompactionCompleteInfo {
    /// Original token count before compaction
    pub original_tokens: u32,
    /// Token count after compaction
    pub compacted_tokens: u32,
    /// Compression ratio (0.0 - 1.0)
    pub compression_ratio: f64,
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
    /// PROV-039: Carries the stop_reason from the provider (e.g., "end_turn", "max_tokens")
    Done(Option<String>),
    /// Error occurred
    Error(String),
    /// Agent was interrupted
    Interrupted(Vec<String>),
    /// Status message (for non-compaction status messages only)
    Status(String),
    /// Token usage update
    Tokens(TokenInfo),
    /// Context window fill percentage (TUI-033)
    ContextFill(ContextFillInfo),
    /// Tool execution progress - streaming output from bash/shell tools (TOOL-011)
    ToolProgress(ToolProgressEvent),
    /// Thinking/reasoning content from extended thinking (TOOL-010)
    Thinking(String),
    /// UX-002: Compaction has started
    CompactionStarted,
    /// UX-002: Compaction progress update
    CompactionProgress(CompactionProgressInfo),
    /// UX-002: Compaction completed successfully
    CompactionComplete(CompactionCompleteInfo),
    /// UX-002: Compaction failed
    CompactionFailed { reason: String },
    /// UX-002: Continuing after compaction (informational for CLI)
    CompactionContinuing,
    /// CONT-007: live continue/goal counter snapshot (state-only for the
    /// TUI bar; CLI renders the nudging stdout line on NudgeConsumed).
    ContinueState(ContinueStateEvent),
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
    /// PROV-039: Optionally carries the stop_reason from the provider
    #[inline]
    fn emit_done(&self) {
        self.emit(StreamEvent::Done(None));
    }

    /// PROV-039: Emit stream completion with stop_reason
    #[inline]
    fn emit_done_with_stop_reason(&self, stop_reason: Option<String>) {
        self.emit(StreamEvent::Done(stop_reason));
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
    fn emit_tool_progress(
        &self,
        tool_call_id: &str,
        tool_name: &str,
        output_chunk: &str,
        is_stderr: bool,
    ) {
        self.emit(StreamEvent::ToolProgress(ToolProgressEvent {
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            output_chunk: output_chunk.to_string(),
            is_stderr,
        }));
    }

    /// Emit thinking/reasoning content from extended thinking (TOOL-010)
    #[inline]
    fn emit_thinking(&self, thinking: &str) {
        self.emit(StreamEvent::Thinking(thinking.to_string()));
    }

    /// UX-002: Emit compaction started event
    #[inline]
    fn emit_compaction_started(&self) {
        self.emit(StreamEvent::CompactionStarted);
    }

    /// UX-002: Emit compaction progress update
    #[inline]
    fn emit_compaction_progress(&self, phase: &str, current: u32, total: u32) {
        self.emit(StreamEvent::CompactionProgress(CompactionProgressInfo {
            phase: phase.to_string(),
            current,
            total,
        }));
    }

    /// UX-002: Emit compaction completed event
    #[inline]
    fn emit_compaction_complete(
        &self,
        original_tokens: u32,
        compacted_tokens: u32,
        compression_ratio: f64,
    ) {
        self.emit(StreamEvent::CompactionComplete(CompactionCompleteInfo {
            original_tokens,
            compacted_tokens,
            compression_ratio,
        }));
    }

    /// UX-002: Emit compaction failed event
    #[inline]
    fn emit_compaction_failed(&self, reason: &str) {
        self.emit(StreamEvent::CompactionFailed {
            reason: reason.to_string(),
        });
    }

    /// UX-002: Emit compaction continuing event (after successful compaction)
    #[inline]
    fn emit_compaction_continuing(&self) {
        self.emit(StreamEvent::CompactionContinuing);
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
                    tool_result.content.clone()
                };

                // Format output based on error status
                if tool_result.is_error {
                    // Error result - use red coloring and clean formatting
                    let formatted_error = format_tool_error(&preview);
                    let display_error = formatted_error.replace('\n', "\r\n");
                    print!("\r\n{display_error}\r\n");
                } else {
                    // Success result - normal formatting
                    // Indent each line and format for raw mode
                    let indented_lines: Vec<String> =
                        preview.lines().map(|line| format!("  {line}")).collect();
                    let formatted_preview = indented_lines.join("\r\n");

                    print!(
                        "\r\n[Tool result preview]\r\n-------\r\n{formatted_preview}\r\n-------\r\n"
                    );
                }
                std::io::stdout().flush().ok();
            }
            StreamEvent::Done(ref stop_reason) => {
                // PROV-039: Display truncation warning if stop_reason is max_tokens
                if let Some(reason) = stop_reason {
                    if reason == "max_tokens" {
                        warn!("Response truncated: model hit max_tokens output limit");
                    }
                }
            }
            StreamEvent::Error(error) => {
                // Clean up error message and display in red
                let formatted = format_cli_error(&error);
                let display_error = formatted.replace('\n', "\r\n");
                error!("{display_error}");
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
                if progress.is_stderr {
                    // Stderr output in red
                    use crate::terminal::style::{RED, RESET};
                    print!("{RED}{display_text}{RESET}");
                } else {
                    print!("{display_text}");
                }
                std::io::stdout().flush().ok();
            }
            StreamEvent::Thinking(thinking) => {
                // TOOL-010: Display thinking/reasoning content
                // Format similar to Gemini CLI's LoadingIndicator
                let display_text = thinking.replace('\n', "\r\n");
                print!("\r\n💭 {display_text}\r\n");
                std::io::stdout().flush().ok();
            }
            StreamEvent::CompactionStarted => {
                // UX-002: Display compaction started message for CLI
                print!("\r\n[Context near limit, generating summary...]\r\n");
                std::io::stdout().flush().ok();
            }
            StreamEvent::CompactionProgress(progress) => {
                // UX-002: Display compaction progress for CLI
                print!(
                    "\r[{}... {}/{} turns]",
                    progress.phase, progress.current, progress.total
                );
                std::io::stdout().flush().ok();
            }
            StreamEvent::CompactionComplete(info) => {
                // UX-002: Display compaction completion for CLI
                print!(
                    "\r\n[Context compacted: {}→{} tokens, {:.0}% compression]\r\n",
                    info.original_tokens,
                    info.compacted_tokens,
                    info.compression_ratio * 100.0
                );
                std::io::stdout().flush().ok();
            }
            StreamEvent::CompactionFailed { reason } => {
                // UX-002: Display compaction failure for CLI
                print!("\r\n[Compaction failed: {reason}]\r\n");
                std::io::stdout().flush().ok();
            }
            StreamEvent::CompactionContinuing => {
                // UX-002: Display continuation message for CLI
                print!("[Continuing with compacted context...]\r\n");
                std::io::stdout().flush().ok();
            }
            StreamEvent::ContinueState(cs) => {
                // CONT-007: the CLI repl has no status bar — preserve the
                // stdout nudging line for a CONSUMED nudge only (moved
                // here from the stream loop's emit_status). The line now
                // prints the effective budget (max(explicit, 15) in goal
                // mode) instead of continue_budget. All other transitions
                // render nothing on stdout.
                if cs.reason == ContinueStateReason::NudgeConsumed {
                    print!(
                        "\u{23E9} auto-continue: nudging ({}/{})\r\n",
                        cs.nudges_used, cs.effective_budget
                    );
                    std::io::stdout().flush().ok();
                }
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
