//! Event capture functionality for debug capture
//!
//! Handles capturing debug events to the session file.

use super::error::DebugCaptureError;
use super::manager::{sanitize_headers, DebugCaptureManager};
use super::types::{CaptureOptions, DebugEvent};
use chrono::Utc;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;

impl DebugCaptureManager {
    /// Capture a debug event.
    ///
    /// BUG-467: a failed write is counted with its error and reported by
    /// `stop_capture`; it cannot be logged here because the tracing
    /// `DebugCaptureLayer` calls this while holding the manager lock.
    pub fn capture(&mut self, event_type: &str, data: Value, options: Option<CaptureOptions>) {
        if !self.enabled {
            return;
        }
        if let Err(error) = self.write_event(event_type, data, options) {
            let count = self.lost_events.as_ref().map_or(0, |(count, _)| *count);
            self.lost_events = Some((count + 1, error.to_string()));
        }
    }

    fn write_event(
        &mut self,
        event_type: &str,
        data: Value,
        options: Option<CaptureOptions>,
    ) -> Result<(), DebugCaptureError> {
        let session_path = self
            .session_file
            .clone()
            .ok_or(DebugCaptureError::NoSessionFile)?;

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

        let line = serde_json::to_string(&event)?;
        let mut file = OpenOptions::new().append(true).open(&session_path)?;
        writeln!(file, "{line}")?;

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
        Ok(())
    }
}
