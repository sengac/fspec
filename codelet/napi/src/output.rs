//! NAPI output implementation for stream events
//!
//! Implements StreamOutput trait to send stream events to JavaScript
//! via ThreadsafeFunction callbacks.
//!
//! Key optimization: Text chunks are batched to reduce callback frequency.
//! Instead of calling JavaScript for every small text chunk (which can be
//! hundreds per response), we accumulate text and flush on:
//! - Non-text events (tool calls, tool results, etc.)
//! - Periodic flush calls from the stream loop
//! - Stream completion
//!
//! This dramatically reduces React state update frequency and eliminates
//! the need for setImmediate workarounds in the JavaScript callback handler.

use crate::types::{
    ContextFillInfo, StreamChunk, TokenTracker, ToolCallInfo, ToolProgressInfo, ToolResultInfo,
};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use napi::threadsafe_function::{
    ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue,
};
use napi::Status;
use std::sync::Mutex;
use tracing::info;

/// Type alias for our ThreadsafeFunction with CalleeHandled=false
/// Generic params: <T, Return, CallJsBackArgs, ErrorStatus, CalleeHandled>
pub type StreamCallback =
    ThreadsafeFunction<StreamChunk, UnknownReturnValue, StreamChunk, Status, false>;

/// Text buffer for batching text chunks
struct TextBuffer {
    text: String,
    /// Last token info to send with batched text (for real-time updates)
    last_tokens: Option<TokenTracker>,
}

impl TextBuffer {
    fn new() -> Self {
        Self {
            text: String::new(),
            last_tokens: None,
        }
    }

    fn append(&mut self, text: &str) {
        self.text.push_str(text);
    }

    fn take(&mut self) -> Option<String> {
        if self.text.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.text))
        }
    }

    fn set_tokens(&mut self, tokens: TokenTracker) {
        self.last_tokens = Some(tokens);
    }

    fn take_tokens(&mut self) -> Option<TokenTracker> {
        self.last_tokens.take()
    }
}

/// NAPI output handler - sends batched callbacks to JavaScript
///
/// Text chunks are accumulated in a buffer and flushed together to reduce
/// the number of JavaScript callbacks. This prevents React state thrashing
/// and eliminates the need for setImmediate workarounds.
///
/// TOOL-011: Uses Arc<StreamCallback> to enable sharing with progress_emitter.
/// The ThreadsafeFunction doesn't implement Clone, but we can clone Arc.
pub struct NapiOutput {
    callback: std::sync::Arc<StreamCallback>,
    buffer: Mutex<TextBuffer>,
}

impl NapiOutput {
    pub fn new(callback: std::sync::Arc<StreamCallback>) -> Self {
        Self {
            callback,
            buffer: Mutex::new(TextBuffer::new()),
        }
    }

    /// Flush the text buffer, sending accumulated text as a single callback
    ///
    /// IMPORTANT: We extract data from the buffer first, then release the lock
    /// before calling the callback. This prevents holding the lock during
    /// potentially slow JS callback execution.
    fn flush_text(&self) {
        // Extract data while holding the lock, then release it
        let (text_to_send, tokens_to_send) = {
            let mut buffer = self.buffer.lock().unwrap();
            (buffer.take(), buffer.take_tokens())
        };
        // Lock is now released - safe to call callbacks

        if let Some(text) = text_to_send {
            let _ = self.callback.call(
                StreamChunk::text(text),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
        if let Some(tokens) = tokens_to_send {
            let _ = self.callback.call(
                StreamChunk::token_update(tokens),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
    }
}

/// TOOL-011: Owned progress emitter for tool progress callbacks
///
/// This struct owns an Arc<StreamCallback>, enabling it to be wrapped
/// in Arc and captured by 'static closures (like the global tool progress callback).
/// ThreadsafeFunction doesn't implement Clone, but Arc does.
pub struct NapiProgressEmitter {
    callback: std::sync::Arc<StreamCallback>,
}

impl NapiProgressEmitter {
    pub fn new(callback: std::sync::Arc<StreamCallback>) -> Self {
        Self { callback }
    }
}

impl StreamOutput for NapiProgressEmitter {
    fn emit(&self, event: StreamEvent) {
        // Only handle ToolProgress events - this is a specialized emitter
        if let StreamEvent::ToolProgress(progress) = event {
            let info = ToolProgressInfo {
                tool_call_id: progress.tool_call_id,
                tool_name: progress.tool_name,
                output_chunk: progress.output_chunk,
            };
            let _ = self.callback.call(
                StreamChunk::tool_progress(info),
                ThreadsafeFunctionCallMode::NonBlocking,
            );
        }
        // Other events are ignored - this emitter is only for tool progress
    }

    fn flush(&self) {
        // No buffering in progress emitter
    }
}

impl StreamOutput for NapiOutput {
    fn emit(&self, event: StreamEvent) {
        match event {
            StreamEvent::Text(text) => {
                // Accumulate text in buffer instead of immediate callback
                let mut buffer = self.buffer.lock().unwrap();
                buffer.append(&text);
                // Don't send yet - wait for flush or non-text event
            }
            StreamEvent::ToolCall(tool_call) => {
                // Flush any pending text first
                self.flush_text();

                let info = ToolCallInfo {
                    id: tool_call.id,
                    name: tool_call.name,
                    input: serde_json::to_string(&tool_call.args).unwrap_or_default(),
                };
                let _ = self.callback.call(
                    StreamChunk::tool_call(info),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::ToolResult(tool_result) => {
                // Flush any pending text first
                self.flush_text();

                let info = ToolResultInfo {
                    tool_call_id: tool_result.id,
                    content: tool_result.content,
                    is_error: tool_result.is_error,
                };
                let _ = self.callback.call(
                    StreamChunk::tool_result(info),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::Done => {
                // Flush any pending text first
                self.flush_text();

                let _ = self
                    .callback
                    .call(StreamChunk::done(), ThreadsafeFunctionCallMode::NonBlocking);
            }
            StreamEvent::Error(error) => {
                // Flush any pending text first
                self.flush_text();

                let _ = self.callback.call(
                    StreamChunk::error(error),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::Interrupted(queued_inputs) => {
                // Flush any pending text first
                self.flush_text();

                let _ = self.callback.call(
                    StreamChunk::interrupted(queued_inputs),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::Status(message) => {
                // Flush any pending text first
                self.flush_text();

                let _ = self.callback.call(
                    StreamChunk::status(message),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::Tokens(tokens) => {
                // PROV-001 DEBUG: Log token emission for display debugging
                info!(
                    "[PROV-001] Token emit to UI: input_tokens={} (TOTAL for display), output_tokens={}, cache_read={:?}, cache_creation={:?}, tok/s={:?}",
                    tokens.input_tokens,
                    tokens.output_tokens,
                    tokens.cache_read_input_tokens,
                    tokens.cache_creation_input_tokens,
                    tokens.tokens_per_second
                );
                // Store token info to send with next flush
                // This batches token updates with text for efficiency
                let tracker = TokenTracker {
                    input_tokens: tokens.input_tokens as u32,
                    output_tokens: tokens.output_tokens as u32,
                    cache_read_input_tokens: tokens.cache_read_input_tokens.map(|t| t as u32),
                    cache_creation_input_tokens: tokens
                        .cache_creation_input_tokens
                        .map(|t| t as u32),
                    tokens_per_second: tokens.tokens_per_second,
                    // Cumulative values are tracked at session level, not per-streaming-event
                    cumulative_billed_input: None,
                    cumulative_billed_output: None,
                };
                let mut buffer = self.buffer.lock().unwrap();
                buffer.set_tokens(tracker);
            }
            StreamEvent::ContextFill(info) => {
                // TUI-033: Send context fill percentage to JavaScript
                // Don't need to flush text - this is independent data
                // Convert u64 to f64 for NAPI compatibility
                let fill_info = ContextFillInfo {
                    fill_percentage: info.fill_percentage,
                    effective_tokens: info.effective_tokens as f64,
                    threshold: info.threshold as f64,
                    context_window: info.context_window as f64,
                };
                let _ = self.callback.call(
                    StreamChunk::context_fill_update(fill_info),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::ToolProgress(progress) => {
                // TOOL-011: Stream tool execution progress to JavaScript
                // Don't flush text buffer - tool progress is separate from LLM text streaming
                let info = ToolProgressInfo {
                    tool_call_id: progress.tool_call_id,
                    tool_name: progress.tool_name,
                    output_chunk: progress.output_chunk,
                };
                let _ = self.callback.call(
                    StreamChunk::tool_progress(info),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
            StreamEvent::Thinking(thinking) => {
                // TOOL-010: Stream thinking/reasoning content to JavaScript
                // Flush any pending text first to maintain event ordering
                self.flush_text();

                let _ = self.callback.call(
                    StreamChunk::thinking(thinking),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
            }
        }
    }

    fn flush(&self) {
        self.flush_text();
    }

    /// TOOL-011: Return a clonable emitter for tool progress callbacks
    ///
    /// Creates a NapiProgressEmitter that owns a cloned Arc<StreamCallback>.
    /// This enables the global tool progress callback to emit directly to
    /// JavaScript without going through a channel that would block during
    /// tool execution inside stream.next().
    fn progress_emitter(&self) -> Option<std::sync::Arc<dyn StreamOutput>> {
        Some(std::sync::Arc::new(NapiProgressEmitter::new(
            std::sync::Arc::clone(&self.callback),
        )))
    }
}
