//! NAPI output implementation for stream events
//!
//! Implements StreamOutput trait to send stream events to JavaScript
//! via ThreadsafeFunction callbacks instead of printing to stdout.

use crate::types::{StreamChunk, ToolCallInfo, ToolResultInfo};
use codelet_cli::interactive::{StreamOutput};
use napi::threadsafe_function::{ErrorStrategy, ThreadsafeFunction, ThreadsafeFunctionCallMode};

/// NAPI output handler - sends callbacks to JavaScript
pub struct NapiOutput<'a> {
    callback: &'a ThreadsafeFunction<StreamChunk, ErrorStrategy::Fatal>,
}

impl<'a> NapiOutput<'a> {
    pub fn new(callback: &'a ThreadsafeFunction<StreamChunk, ErrorStrategy::Fatal>) -> Self {
        Self { callback }
    }
}

impl StreamOutput for NapiOutput<'_> {
    fn emit_text(&self, text: &str) {
        self.callback.call(
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
        self.callback.call(
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
        self.callback.call(
            StreamChunk::tool_result(info),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_done(&self) {
        self.callback.call(
            StreamChunk::done(),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_error(&self, error: &str) {
        self.callback.call(
            StreamChunk::error(error.to_string()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_interrupted(&self, queued_inputs: &[String]) {
        self.callback.call(
            StreamChunk::interrupted(queued_inputs.to_vec()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }

    fn emit_status(&self, message: &str) {
        self.callback.call(
            StreamChunk::status(message.to_string()),
            ThreadsafeFunctionCallMode::NonBlocking,
        );
    }
}
