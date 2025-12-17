//! Summary generation for debug capture sessions
//!
//! Handles markdown summary generation and event summarization for timeline display.

use super::manager::DebugCaptureManager;
use super::types::DebugEvent;
use std::fs;

/// Default configuration values (duplicated from manager for now)
const TIMELINE_EVENT_LIMIT: usize = 20;
const SUMMARY_TRUNCATION_LENGTH: usize = 30;

impl DebugCaptureManager {
    /// Format duration in human-readable format
    pub fn format_duration(&self, ms: u64) -> String {
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
    pub(super) fn summarize_event_data(&self, event: &DebugEvent) -> String {
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

    /// Generate a markdown summary of the session
    pub(super) fn generate_summary(&self, session_file: &str) {
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
}
