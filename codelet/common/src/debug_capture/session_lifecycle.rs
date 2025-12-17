//! Session lifecycle events for debug capture
//!
//! Handles session start/end event capture and symlink management.

use super::manager::DebugCaptureManager;
use chrono::Utc;
use std::fs;

/// Default configuration values
const DEFAULT_CONTEXT_WINDOW: usize = 200000;
const DEFAULT_MAX_OUTPUT_TOKENS: usize = 16384;

impl DebugCaptureManager {
    /// Update the latest.jsonl symlink
    pub(super) fn update_latest_symlink(&self) {
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
    pub(super) fn capture_session_start(&mut self) {
        let env_info = serde_json::json!({
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cwd": std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            "shell": std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
            "user": std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "unknown".to_string()),
        });

        let data = serde_json::json!({
            "sessionId": self.session_id,
            "startTime": self.start_datetime.map(|dt: chrono::DateTime<Utc>| dt.to_rfc3339()),
            "version": env!("CARGO_PKG_VERSION"),
            "provider": self.session_metadata.provider.as_deref().unwrap_or("unknown"),
            "model": self.session_metadata.model.as_deref().unwrap_or("unknown"),
            "environment": env_info,
            "contextWindow": self.session_metadata.context_window.unwrap_or(DEFAULT_CONTEXT_WINDOW),
            "maxOutputTokens": self.session_metadata.max_output_tokens.unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS),
        });

        self.capture("session.start", data, None);
    }

    /// Capture session end event with statistics
    pub(super) fn capture_session_end(&mut self) {
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
}
