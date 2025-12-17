//! Event capture functionality for debug capture
//!
//! Handles capturing debug events to the session file.

use super::manager::{sanitize_headers, DebugCaptureManager};
use super::types::{CaptureOptions, DebugEvent};
use chrono::Utc;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;

impl DebugCaptureManager {
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
}
