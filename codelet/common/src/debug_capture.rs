//! Debug Capture System for LLM Session Diagnostics
//!
//! Provides comprehensive capture of LLM API communication, tool executions,
//! and application logs for debugging agent issues.
//!
//! ACDD Work Unit: CLI-022
//! Port of codelet's debug-capture.ts to Rust.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during debug capture operations
#[derive(Error, Debug)]
pub enum DebugCaptureError {
    #[error("Could not determine home directory")]
    NoHomeDirectory,

    #[error("Failed to create directory: {0}")]
    DirectoryCreation(#[from] io::Error),

    #[error("Failed to serialize event: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Debug capture manager not initialized")]
    NotInitialized,

    #[error("Session file not set")]
    NoSessionFile,

    #[error("Failed to acquire lock")]
    LockError,
}

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

/// Default configuration values
const DEFAULT_CONTEXT_WINDOW: usize = 200000;
const DEFAULT_MAX_OUTPUT_TOKENS: usize = 16384;
const TIMELINE_EVENT_LIMIT: usize = 20;
const SUMMARY_TRUNCATION_LENGTH: usize = 30;

/// Headers that should be redacted for security
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "anthropic-api-key",
    "openai-api-key",
    "api-key",
];

/// Redact sensitive values from headers
fn sanitize_headers(headers: &Value) -> Value {
    if let Some(obj) = headers.as_object() {
        let mut redacted = serde_json::Map::new();
        for (key, value) in obj {
            if SENSITIVE_HEADERS.contains(&key.to_lowercase().as_str()) {
                redacted.insert(key.clone(), Value::String("[REDACTED]".to_string()));
            } else {
                redacted.insert(key.clone(), value.clone());
            }
        }
        Value::Object(redacted)
    } else {
        headers.clone()
    }
}

/// Singleton manager for debug capture sessions
pub struct DebugCaptureManager {
    enabled: bool,
    session_id: Option<String>,
    session_file: Option<PathBuf>,
    sequence: u32,
    turn_id: u32,
    start_time: Option<Instant>,
    start_datetime: Option<DateTime<Utc>>,
    event_count: u32,
    error_count: u32,
    warning_count: u32,
    session_metadata: SessionMetadata,
    debug_dir: PathBuf,
    codelet_dir: PathBuf,
}

impl DebugCaptureManager {
    /// Create a new manager
    fn new() -> Result<Self, DebugCaptureError> {
        let home_dir = dirs::home_dir().ok_or(DebugCaptureError::NoHomeDirectory)?;
        let codelet_dir = home_dir.join(".codelet");
        let debug_dir = codelet_dir.join("debug");

        Ok(Self {
            enabled: false,
            session_id: None,
            session_file: None,
            sequence: 0,
            turn_id: 0,
            start_time: None,
            start_datetime: None,
            event_count: 0,
            error_count: 0,
            warning_count: 0,
            session_metadata: SessionMetadata::default(),
            debug_dir,
            codelet_dir,
        })
    }

    /// Check if debug capture is currently enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set session metadata (provider, model, etc.)
    pub fn set_session_metadata(&mut self, metadata: SessionMetadata) {
        if let Some(provider) = metadata.provider {
            self.session_metadata.provider = Some(provider);
        }
        if let Some(model) = metadata.model {
            self.session_metadata.model = Some(model);
        }
        if let Some(context_window) = metadata.context_window {
            self.session_metadata.context_window = Some(context_window);
        }
        if let Some(max_output_tokens) = metadata.max_output_tokens {
            self.session_metadata.max_output_tokens = Some(max_output_tokens);
        }
    }

    /// Start a new debug capture session
    pub fn start_capture(&mut self) -> Result<String, DebugCaptureError> {
        // Ensure directory exists with secure permissions
        if !self.codelet_dir.exists() {
            fs::create_dir_all(&self.codelet_dir)?;
        }
        if !self.debug_dir.exists() {
            fs::create_dir_all(&self.debug_dir)?;
            // Set permissions to 0o700 on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(&self.debug_dir, perms)?;
            }
        }

        // Generate session filename
        self.session_id = Some(Uuid::new_v4().to_string());
        let now = Utc::now();
        let timestamp = now.format("%Y-%m-%dT%H-%M-%S").to_string();
        let filename = format!("session-{timestamp}.jsonl");
        self.session_file = Some(self.debug_dir.join(&filename));

        // Initialize empty file
        let session_path = self
            .session_file
            .as_ref()
            .ok_or(DebugCaptureError::NoSessionFile)?;
        File::create(session_path)?;

        self.sequence = 0;
        self.turn_id = 0;
        self.start_time = Some(Instant::now());
        self.start_datetime = Some(now);
        self.event_count = 0;
        self.error_count = 0;
        self.warning_count = 0;
        self.enabled = true;

        // Update latest symlink
        self.update_latest_symlink();

        // Write session start event
        self.capture_session_start();

        Ok(self
            .session_file
            .as_ref()
            .ok_or(DebugCaptureError::NoSessionFile)?
            .to_string_lossy()
            .to_string())
    }

    /// Stop the current debug capture session
    pub fn stop_capture(&mut self) -> Result<String, DebugCaptureError> {
        if !self.enabled {
            return Ok(String::new());
        }

        // Write session end event
        self.capture_session_end();

        self.enabled = false;

        let session_file = self
            .session_file
            .as_ref()
            .ok_or(DebugCaptureError::NoSessionFile)?
            .to_string_lossy()
            .to_string();

        // Generate summary
        self.generate_summary(&session_file);

        // Reset state
        self.session_id = None;
        self.session_file = None;
        self.start_time = None;
        self.start_datetime = None;

        Ok(session_file)
    }

    /// Capture a debug event
    ///
    /// This method silently fails if an error occurs to avoid disrupting the main application.
    pub fn capture(&mut self, event_type: &str, data: Value, options: Option<CaptureOptions>) {
        if !self.enabled {
            return;
        }

        // Silently return if session file is not set
        let session_path = match self.session_file.as_ref() {
            Some(path) => path,
            None => return,
        };

        // Auto-redact headers if present
        let processed_data = if let Some(headers) = data.get("headers") {
            let mut new_data = data.clone();
            if let Some(obj) = new_data.as_object_mut() {
                obj.insert("headers".to_string(), sanitize_headers(headers));
            }
            new_data
        } else {
            data
        };

        let event = DebugEvent {
            timestamp: Utc::now().to_rfc3339(),
            sequence: self.sequence,
            event_type: event_type.to_string(),
            turn_id: Some(self.turn_id),
            request_id: options.and_then(|o| o.request_id),
            data: processed_data,
        };

        self.sequence += 1;

        // Append to file - silently fail if file operations fail
        let file_result = OpenOptions::new().append(true).open(session_path);

        let mut file = match file_result {
            Ok(f) => f,
            Err(_) => return,
        };

        let line = match serde_json::to_string(&event) {
            Ok(l) => l,
            Err(_) => return,
        };

        if writeln!(file, "{line}").is_err() {
            return;
        }

        self.event_count += 1;

        // Track errors and warnings
        if event_type.contains("error") {
            self.error_count += 1;
        }
        if event_type == "log.entry" {
            if let Some(level) = event.data.get("level").and_then(|l| l.as_str()) {
                if level == "warn" || level == "warning" {
                    self.warning_count += 1;
                }
            }
        }
    }

    /// Increment the conversation turn counter
    pub fn increment_turn(&mut self) {
        self.turn_id += 1;
    }

    /// Update the latest.jsonl symlink
    fn update_latest_symlink(&self) {
        let latest_path = self.debug_dir.join("latest.jsonl");

        // Remove existing symlink if present
        if latest_path.exists() || latest_path.is_symlink() {
            let _ = fs::remove_file(&latest_path);
        }

        // Create new symlink (Unix only, ignore errors on other platforms)
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            if let Some(session_path) = self.session_file.as_ref() {
                let _ = symlink(session_path, &latest_path);
            }
        }
    }

    /// Capture session start event with metadata
    fn capture_session_start(&mut self) {
        let env_info = serde_json::json!({
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cwd": std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            "shell": std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
            "user": std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "unknown".to_string()),
        });

        let data = serde_json::json!({
            "sessionId": self.session_id,
            "startTime": self.start_datetime.map(|dt| dt.to_rfc3339()),
            "version": env!("CARGO_PKG_VERSION"),
            "provider": self.session_metadata.provider.as_deref().unwrap_or("unknown"),
            "model": self.session_metadata.model.as_deref().unwrap_or("unknown"),
            "environment": env_info,
            "contextWindow": self.session_metadata.context_window.unwrap_or(DEFAULT_CONTEXT_WINDOW),
            "maxOutputTokens": self.session_metadata.max_output_tokens.unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
        });

        self.capture("session.start", data, None);
    }

    /// Calculate session duration in milliseconds
    fn get_session_duration(&self) -> u64 {
        self.start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }

    /// Capture session end event with statistics
    fn capture_session_end(&mut self) {
        let end_time = Utc::now();
        let duration = self.get_session_duration();

        let data = serde_json::json!({
            "endTime": end_time.to_rfc3339(),
            "duration": duration,
            "turnCount": self.turn_id,
            "eventCount": self.event_count,
            "exitReason": "user",
        });

        self.capture("session.end", data, None);
    }

    /// Generate a markdown summary of the session
    fn generate_summary(&self, session_file: &str) {
        let summary_path = session_file.replace(".jsonl", ".summary.md");
        let duration = self.get_session_duration();
        let duration_formatted = self.format_duration(duration);

        // Read events for timeline
        let timeline = match fs::read_to_string(session_file) {
            Ok(content) => {
                let events: Vec<String> = content
                    .trim()
                    .lines()
                    .take(TIMELINE_EVENT_LIMIT)
                    .filter_map(|line| {
                        serde_json::from_str::<DebugEvent>(line).ok().map(|event| {
                            let time = event
                                .timestamp
                                .split('T')
                                .nth(1)
                                .and_then(|t| t.split('.').next())
                                .unwrap_or("-");
                            let details = self.summarize_event_data(&event);
                            format!("| {} | {} | {} |", time, event.event_type, details)
                        })
                    })
                    .collect();
                events.join("\n")
            }
            Err(_) => "| - | - | Unable to read events |".to_string(),
        };

        let summary = format!(
            r#"# Debug Session Summary

**Session ID:** {}
**Duration:** {} ({}ms)
**File:** {}

## Environment
- **Platform:** {} ({})
- **CWD:** {}
- **Provider:** {}
- **Model:** {}

## Session Statistics
- **Turns:** {}
- **Events:** {}
- **Errors:** {}
- **Warnings:** {}

## Event Timeline Summary
| Time | Event | Details |
|------|-------|---------|
{}

## Errors & Warnings
{}

## To Analyze This Session
Load the JSONL file and examine events:
```bash
cat {} | head -50
```
"#,
            self.session_id.as_deref().unwrap_or("unknown"),
            duration_formatted,
            duration,
            session_file,
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            self.session_metadata
                .provider
                .as_deref()
                .unwrap_or("unknown"),
            self.session_metadata.model.as_deref().unwrap_or("unknown"),
            self.turn_id,
            self.event_count,
            self.error_count,
            self.warning_count,
            timeline,
            if self.error_count > 0 || self.warning_count > 0 {
                format!(
                    "- {} error(s), {} warning(s) recorded",
                    self.error_count, self.warning_count
                )
            } else {
                "- No errors or warnings recorded".to_string()
            },
            session_file,
        );

        // Silently ignore errors when writing summary (non-critical)
        let _ = fs::write(&summary_path, summary);
    }

    /// Format duration in human-readable format
    fn format_duration(&self, ms: u64) -> String {
        let seconds = ms / 1000;
        let minutes = seconds / 60;
        let hours = minutes / 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes % 60, seconds % 60)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds % 60)
        } else {
            format!("{seconds}s")
        }
    }

    /// Summarize event data for timeline display
    fn summarize_event_data(&self, event: &DebugEvent) -> String {
        let data = &event.data;
        match event.event_type.as_str() {
            "session.start" => format!(
                "Provider: {}",
                data.get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            ),
            "session.end" => format!(
                "Duration: {}ms",
                data.get("duration")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            ),
            "api.request" => format!(
                "{} request",
                data.get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("api")
            ),
            "api.response.end" => format!(
                "Duration: {}ms",
                data.get("duration")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            ),
            "tool.call" => format!(
                "Tool: {}",
                data.get("toolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            ),
            "tool.result" => format!(
                "Tool: {}, {}ms",
                data.get("toolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown"),
                data.get("duration")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            ),
            "log.entry" => {
                let level = data.get("level").and_then(|v| v.as_str()).unwrap_or("info");
                let message = data.get("message").and_then(|v| v.as_str()).unwrap_or("");
                let truncated: String = message.chars().take(SUMMARY_TRUNCATION_LENGTH).collect();
                format!("[{level}] {truncated}...")
            }
            "user.input" => {
                let input = data.get("input").and_then(|v| v.as_str()).unwrap_or("");
                let truncated: String = input.chars().take(SUMMARY_TRUNCATION_LENGTH).collect();
                format!("{truncated}...")
            }
            _ => event.event_type.clone(),
        }
    }
}

/// A mutex wrapper that automatically recovers from poison errors
///
/// This is necessary because tests may panic while holding the lock, which poisons
/// a standard Mutex. For debug capture, we can safely recover since the worst case
/// is losing some debug events, which is acceptable.
pub struct PoisonRecoveryMutex<T> {
    inner: std::sync::Mutex<T>,
}

impl<T> PoisonRecoveryMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: std::sync::Mutex::new(value),
        }
    }

    /// Lock the mutex, recovering from poison if needed
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, T>, DebugCaptureError> {
        match self.inner.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                // Recover from poison - the data may be in an inconsistent state
                // but for debug capture this is acceptable (we might lose events)
                Ok(poisoned.into_inner())
            }
        }
    }
}

// Global singleton instance with poison recovery
static DEBUG_CAPTURE_MANAGER: OnceLock<Arc<PoisonRecoveryMutex<DebugCaptureManager>>> =
    OnceLock::new();

/// Get the singleton debug capture manager instance
///
/// This implementation handles mutex poisoning gracefully by clearing the poison
/// and continuing. For debug capture, this is acceptable since the worst case
/// is missing some debug events.
#[allow(clippy::expect_used)]
pub fn get_debug_capture_manager(
) -> Result<Arc<PoisonRecoveryMutex<DebugCaptureManager>>, DebugCaptureError> {
    // Note: get_or_init requires infallible initialization. If home directory
    // cannot be determined, this is a fundamental system issue and panic is appropriate.
    let manager = DEBUG_CAPTURE_MANAGER.get_or_init(|| {
        let mgr = DebugCaptureManager::new().expect("Failed to create DebugCaptureManager");
        Arc::new(PoisonRecoveryMutex::new(mgr))
    });

    Ok(manager.clone())
}

/// Handle the /debug command to toggle debug capture
pub fn handle_debug_command() -> DebugCommandResult {
    let manager_arc = match get_debug_capture_manager() {
        Ok(m) => m,
        Err(e) => {
            return DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to initialize debug capture: {e}"),
            }
        }
    };

    let mut manager = match manager_arc.lock() {
        Ok(m) => m,
        Err(_) => {
            return DebugCommandResult {
                enabled: false,
                session_file: None,
                message: "Failed to acquire lock on debug capture manager".to_string(),
            }
        }
    };

    if manager.is_enabled() {
        // Turn off
        match manager.stop_capture() {
            Ok(session_file) => DebugCommandResult {
                enabled: false,
                session_file: Some(session_file.clone()),
                message: format!("Debug capture stopped. Session saved to: {session_file}"),
            },
            Err(e) => DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to stop debug capture: {e}"),
            },
        }
    } else {
        // Turn on
        match manager.start_capture() {
            Ok(session_file) => DebugCommandResult {
                enabled: true,
                session_file: Some(session_file.clone()),
                message: format!("Debug capture started. Writing to: {session_file}"),
            },
            Err(e) => DebugCommandResult {
                enabled: false,
                session_file: None,
                message: format!("Failed to start debug capture: {e}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_headers_redacts_sensitive() {
        let headers = serde_json::json!({
            "authorization": "Bearer secret",
            "x-api-key": "key123",
            "content-type": "application/json"
        });

        let sanitized = sanitize_headers(&headers);

        assert_eq!(
            sanitized.get("authorization").unwrap().as_str().unwrap(),
            "[REDACTED]"
        );
        assert_eq!(
            sanitized.get("x-api-key").unwrap().as_str().unwrap(),
            "[REDACTED]"
        );
        assert_eq!(
            sanitized.get("content-type").unwrap().as_str().unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_format_duration() {
        let manager = DebugCaptureManager::new().expect("Failed to create manager");

        assert_eq!(manager.format_duration(500), "0s");
        assert_eq!(manager.format_duration(5000), "5s");
        assert_eq!(manager.format_duration(65000), "1m 5s");
        assert_eq!(manager.format_duration(3665000), "1h 1m 5s");
    }
}
