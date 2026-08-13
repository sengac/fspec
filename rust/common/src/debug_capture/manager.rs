use super::error::DebugCaptureError;
use super::types::SessionMetadata;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

/// Headers that should be redacted for security
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "x-api-key",
    "anthropic-api-key",
    "openai-api-key",
    "api-key",
];

/// Redact sensitive values from headers
pub(super) fn sanitize_headers(headers: &Value) -> Value {
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
    pub(super) enabled: bool,
    pub(super) session_id: Option<String>,
    pub(super) session_file: Option<PathBuf>,
    pub(super) sequence: u32,
    pub(super) turn_id: u32,
    start_time: Option<Instant>,
    pub(super) start_datetime: Option<DateTime<Utc>>,
    pub(super) event_count: u32,
    pub(super) error_count: u32,
    pub(super) warning_count: u32,
    pub(super) session_metadata: SessionMetadata,
    pub(super) debug_dir: PathBuf,
    data_dir: PathBuf,
    /// Whether the session.start event has been written to the current session file.
    ///
    /// BUG-fix: session.start emission is deferred until `set_session_metadata` supplies
    /// the provider/model/context-window values. Without this flag, callers that set
    /// metadata *after* `start_capture()` (e.g. `session_toggle_debug` in the NAPI layer)
    /// end up writing a `session.start` event with `provider: "unknown"` and
    /// `model: "unknown"`. If no metadata ever arrives, `stop_capture` emits a
    /// fallback `session.start` before the `session.end` event.
    pub(super) session_start_emitted: bool,
}

impl DebugCaptureManager {
    /// Create a new manager
    pub fn new() -> Result<Self, DebugCaptureError> {
        // Derive debug directory from global data directory
        let data_dir = crate::get_data_dir().map_err(|_| DebugCaptureError::NoHomeDirectory)?;
        let debug_dir = data_dir.join("debug");

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
            data_dir,
            session_start_emitted: false,
        })
    }

    /// Set a custom debug directory
    ///
    /// This should be called before starting capture if you want to use
    /// a custom directory. Appends `/debug/` to the base_dir.
    pub fn set_debug_directory(&mut self, base_dir: PathBuf) {
        self.data_dir = base_dir.clone();
        self.debug_dir = base_dir.join("debug");
    }

    /// Set the debug directory directly without appending "debug/"
    ///
    /// BUG-134: Used for per-session debug capture where the session id
    /// is already included in the path (e.g., `~/.fspec/debug/{session_id}/`).
    pub fn set_debug_directory_raw(&mut self, dir: PathBuf) {
        self.debug_dir = dir;
    }

    /// Check if debug capture is currently enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set session metadata (provider, model, etc.)
    ///
    /// If debug capture is currently enabled and the `session.start` event has not yet
    /// been written to the session file, this will emit it now using the updated
    /// metadata. This guarantees the recorded `session.start` contains the real
    /// provider/model values rather than the `"unknown"` defaults.
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

        // Lazily emit the deferred `session.start` event now that metadata is available.
        if self.enabled && !self.session_start_emitted {
            self.capture_session_start();
            self.session_start_emitted = true;
        }
    }

    /// Start a new debug capture session
    pub fn start_capture(&mut self) -> Result<String, DebugCaptureError> {
        // Ensure debug directory exists with secure permissions
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
        self.session_start_emitted = false;

        // Update latest symlink
        self.update_latest_symlink();

        // Write session start event IMMEDIATELY only if metadata is already present.
        // Otherwise, defer emission until `set_session_metadata` is called so that the
        // recorded event contains real provider/model values instead of "unknown".
        // If metadata never arrives, `stop_capture` emits a fallback before session.end.
        if self.session_metadata.provider.is_some() || self.session_metadata.model.is_some() {
            self.capture_session_start();
            self.session_start_emitted = true;
        }

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

        // Fallback: if metadata was never supplied, emit a `session.start` now using
        // whatever (possibly default) values we have. This ensures every session file
        // contains exactly one `session.start` and one `session.end` event, preserving
        // the ordering contract for downstream consumers.
        if !self.session_start_emitted {
            self.capture_session_start();
            self.session_start_emitted = true;
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
        self.session_start_emitted = false;

        Ok(session_file)
    }

    /// Increment the conversation turn counter
    pub fn increment_turn(&mut self) {
        self.turn_id += 1;
    }

    /// Calculate session duration in milliseconds
    pub(super) fn get_session_duration(&self) -> u64 {
        self.start_time
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    }
}
