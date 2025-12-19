//! NAPI output implementation for stream events
//!
//! Implements StreamOutput trait to send stream events to JavaScript
//! via ThreadsafeFunction callbacks.
//!
//! Uses CalleeHandled=false so JS callback receives (value) not (err, value).

use crate::types::{StreamChunk, ToolCallInfo, ToolResultInfo};
use codelet_cli::interactive::StreamOutput;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode, UnknownReturnValue};
use napi::Status;

/// Type alias for our ThreadsafeFunction with CalleeHandled=false
/// Generic params: <T, Return, CallJsBackArgs, ErrorStatus, CalleeHandled>
pub type StreamCallback = ThreadsafeFunction<StreamChunk, UnknownReturnValue, StreamChunk, Status, false>;

/// NAPI output handler - sends callbacks to JavaScript
pub struct NapiOutput<'a> {
    callback: &'a StreamCallback,
}

impl<'a> NapiOutput<'a> {
    pub fn new(callback: &'a StreamCallback) -> Self {
        Self { callback }
    }
}

impl StreamOutput for NapiOutput<'_> {
    fn emit_text(&self, text: &str) {
        // With CalleeHandled=false, call takes T directly (not Result<T>)
        let _ = self.callback.call(
            StreamChunk::text(text.to_string()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_tool_call(&self, id: &str, name: &str, args: &serde_json::Value) {
        let info = ToolCallInfo {
            id: id.to_string(),
            name: name.to_string(),
            input: serde_json::to_string(args).unwrap_or_default(),
        };
        let _ = self.callback.call(
            StreamChunk::tool_call(info),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_tool_result(&self, id: &str, content: &str, is_error: bool) {
        let info = ToolResultInfo {
            tool_call_id: id.to_string(),
            content: content.to_string(),
            is_error,
        };
        let _ = self.callback.call(
            StreamChunk::tool_result(info),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_done(&self) {
        let _ = self.callback.call(StreamChunk::done(), ThreadsafeFunctionCallMode::NonBlocking);
    }

    fn emit_error(&self, error: &str) {
        let _ = self.callback.call(
            StreamChunk::error(error.to_string()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_interrupted(&self, queued_inputs: &[String]) {
        let _ = self.callback.call(
            StreamChunk::interrupted(queued_inputs.to_vec()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_status(&self, message: &str) {
        let _ = self.callback.call(
            StreamChunk::status(message.to_string()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }
}
