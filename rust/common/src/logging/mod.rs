//! Logging module - Rust port of rust's Winston logging system
//!
//! Uses tracing + tracing-subscriber + tracing-appender for structured logging
//! with private daily file rotation to {data_dir}/logs/private/.

use anyhow::Result;
pub mod privacy;
pub mod private_files;
use tracing::Subscriber;
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
                    let message =
                        privacy::event_fields(event, privacy::PrivacyFields::default()).render();

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

/// Initialize privacy-aware file diagnostics with daily rotation
///
/// This mirrors rust's Winston logging implementation:
/// - Logs to {data_dir}/logs/private/rust.YYYY-MM-DD
/// - Daily rotation with retention of last 5 files
/// - Privacy-aware text format shared with the fspec binary
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
    // Derive log directory from global data directory
    let log_dir = crate::get_data_dir()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .join("logs/private");

    // Set up daily file rotation (like Winston's DailyRotateFile)
    // Files named: codelet-2024-12-02, codelet-2024-12-03, etc.
    let file_appender = private_files::appender(&log_dir, "rust")?;

    // Configure log level based on verbose flag or RUST_LOG env var
    let env_filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into())
    };

    // Initialize tracing subscriber with JSON formatting (file-only, no stdout)
    // CLI-022: Add DebugCaptureLayer for log.entry capture
    tracing_subscriber::registry()
        .with(privacy::PrivacySpanLayer(privacy::PrivacyFields::default()))
        .with(
            fmt::layer()
                .fmt_fields(privacy::PrivacyFields::default())
                .event_format(privacy::PrivacyFields::default())
                .with_writer(file_appender),
        )
        .with(DebugCaptureLayer)
        .with(env_filter)
        .init();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_data_dir() -> anyhow::Result<tempfile::TempDir> {
        let temp_dir = tempfile::tempdir()?;
        crate::set_data_directory(temp_dir.path().to_path_buf()).map_err(|e| anyhow::anyhow!(e))?;
        Ok(temp_dir)
    }

    #[test]
    fn test_init_logging_basic() -> anyhow::Result<()> {
        let _temp_dir = setup_test_data_dir()?;

        let _result = init_logging(false);

        Ok(())
    }
}
