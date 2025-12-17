use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Event types supported by the debug capture system
pub type DebugEventType = String;

/// Debug event structure written to JSONL files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugEvent {
    pub timestamp: String,
    pub sequence: u32,
    #[serde(rename = "eventType")]
    pub event_type: DebugEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub data: Value,
}

/// Options for capturing events
#[derive(Debug, Clone, Default)]
pub struct CaptureOptions {
    pub request_id: Option<String>,
}

/// Result from handle_debug_command
#[derive(Debug, Clone)]
pub struct DebugCommandResult {
    pub enabled: bool,
    pub session_file: Option<String>,
    pub message: String,
}

/// Session metadata captured at start
#[derive(Debug, Clone, Default)]
pub struct SessionMetadata {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub context_window: Option<usize>,
    pub max_output_tokens: Option<usize>,
}
