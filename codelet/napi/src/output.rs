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

use crate::types::{StreamChunk, TokenTracker, ToolCallInfo, ToolResultInfo};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue};
use napi::Status;
use std::sync::Mutex;

/// Type alias for our ThreadsafeFunction with CalleeHandled=false
/// Generic params: <T, Return, CallJsBackArgs, ErrorStatus, CalleeHandled>
pub type StreamCallback = ThreadsafeFunction<StreamChunk, UnknownReturnValue, StreamChunk, Status, false>;

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
pub struct NapiOutput<'a> {
    callback: &'a StreamCallback,
    buffer: Mutex<TextBuffer>,
}

impl<'a> NapiOutput<'a> {
    pub fn new(callback: &'a StreamCallback) -> Self {
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

impl StreamOutput for NapiOutput<'_> {
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

                let _ = self.callback.call(
                    StreamChunk::done(),
                    ThreadsafeFunctionCallMode::NonBlocking,
                );
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
                // Store token info to send with next flush
                // This batches token updates with text for efficiency
                let tracker = TokenTracker {
                    input_tokens: tokens.input_tokens as u32,
                    output_tokens: tokens.output_tokens as u32,
                    cache_read_input_tokens: tokens.cache_read_input_tokens.map(|t| t as u32),
                    cache_creation_input_tokens: tokens.cache_creation_input_tokens.map(|t| t as u32),
                };
                let mut buffer = self.buffer.lock().unwrap();
                buffer.set_tokens(tracker);
            }
        }
    }

    fn flush(&self) {
        self.flush_text();
    }
}
