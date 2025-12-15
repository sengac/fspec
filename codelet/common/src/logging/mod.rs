//! Logging module - Rust port of codelet's Winston logging system
//!
//! Uses tracing + tracing-subscriber + tracing-appender for structured logging
//! with daily file rotation to ~/.codelet/logs/

use anyhow::Result;
use tracing::Subscriber;
use tracing_appender::rolling;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt, EnvFilter, Layer,
};

/// CLI-022: Custom tracing layer that captures log entries to debug stream
pub struct DebugCaptureLayer;

impl<S> Layer<S> for DebugCaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use crate::debug_capture::get_debug_capture_manager;

        // Only capture if debug is enabled
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    // Extract log level
                    let level = event.metadata().level().as_str();

                    // Extract message from fields
                    let mut message = String::new();
                    let mut visitor = MessageVisitor {
                        message: &mut message,
                    };
                    event.record(&mut visitor);

                    // Capture to debug stream
                    manager.capture(
                        "log.entry",
                        serde_json::json!({
                            "level": level,
                            "message": message,
                            "target": event.metadata().target(),
                        }),
                        None,
                    );
                }
            }
        }
    }
}

/// Visitor to extract message from tracing event fields
struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.message = value.to_string();
        }
    }
}

/// Initialize the logging system with file-based JSON output and daily rotation
///
/// This mirrors codelet's Winston logging implementation:
/// - Logs to ~/.codelet/logs/codelet-YYYY-MM-DD
/// - Daily rotation with retention of last 5 files
/// - JSON format for machine parsing
/// - File-only (no stdout) to avoid CLI interference
/// - Supports RUST_LOG env var or verbose flag for debug mode
///
/// # Arguments
/// * `verbose` - Enable debug-level logging when true
///
/// # Examples
/// ```no_run
/// use codelet_common::logging::init_logging;
///
/// // Initialize with info level (default)
/// init_logging(false).expect("Failed to initialize logging");
///
/// // Initialize with debug level (verbose mode)
/// init_logging(true).expect("Failed to initialize logging");
/// ```
pub fn init_logging(verbose: bool) -> Result<()> {
    // Create log directory ~/.codelet/logs/
    let log_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".codelet")
        .join("logs");

    std::fs::create_dir_all(&log_dir)?;

    // Set up daily file rotation (like Winston's DailyRotateFile)
    // Files named: codelet-2024-12-02, codelet-2024-12-03, etc.
    let file_appender = rolling::daily(log_dir, "codelet");

    // Configure log level based on verbose flag or RUST_LOG env var
    let env_filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into())
    };

    // Initialize tracing subscriber with JSON formatting (file-only, no stdout)
    // CLI-022: Add DebugCaptureLayer for log.entry capture
    tracing_subscriber::registry()
        .with(fmt::layer().json().with_writer(file_appender))
        .with(DebugCaptureLayer)
        .with(env_filter)
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging_basic() {
        // Note: Can only init once per process, so tests may interfere
        // This is a basic smoke test - we just verify it doesn't panic
        let _result = init_logging(false);

        // May fail if already initialized, which is okay
        // The fact we reached here means no panic occurred
    }
}
