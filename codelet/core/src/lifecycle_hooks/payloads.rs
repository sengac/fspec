//! Agent Lifecycle Hooks — JSON Payloads
//!
//! Per-event JSON payload structs serialized to hook child process stdin.
//! Each event type has its own payload structure.

use serde::Serialize;

/// Payload for `session_start` hooks.
#[derive(Debug, Clone, Serialize)]
pub struct SessionStartPayload {
    pub hook_event_name: String,
    pub session_id: String,
    pub cwd: String,
    pub source: String,
    pub transcript_path: String,
}

/// Payload for `session_end` hooks.
#[derive(Debug, Clone, Serialize)]
pub struct SessionEndPayload {
    pub hook_event_name: String,
    pub session_id: String,
    pub cwd: String,
    pub reason: String,
    pub transcript_path: String,
}

/// Payload for `user_prompt_submit` hooks.
#[derive(Debug, Clone, Serialize)]
pub struct UserPromptSubmitPayload {
    pub hook_event_name: String,
    pub session_id: String,
    pub cwd: String,
    pub prompt: String,
    pub transcript_path: String,
}

/// Payload for `pre_tool_use` hooks.
#[derive(Debug, Clone, Serialize)]
pub struct PreToolUsePayload {
    pub hook_event_name: String,
    pub session_id: String,
    pub cwd: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub transcript_path: String,
}

/// Payload for `post_tool_use` hooks.
#[derive(Debug, Clone, Serialize)]
pub struct PostToolUsePayload {
    pub hook_event_name: String,
    pub session_id: String,
    pub cwd: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub tool_response: String,
    pub transcript_path: String,
}

/// Payload for `notification` hooks.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationPayload {
    pub hook_event_name: String,
    pub session_id: String,
    pub cwd: String,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub transcript_path: String,
}
