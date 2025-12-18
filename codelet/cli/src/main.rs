//! Codelet - A multi-provider AI coding agent CLI

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present)
    // Silently ignore if .env doesn't exist
    let _ = dotenvy::dotenv();

    // Initialize unified tracing-based logging system
    // Logs to ~/.codelet/logs/codelet-YYYY-MM-DD.log with JSON formatting
    codelet_common::logging::init_logging(false)?;

    // Install browser cleanup handler for SIGINT/SIGTERM
    // This ensures Chrome is properly terminated on Ctrl+C or kill
    codelet_tools::install_browser_cleanup_handler();

    // Parse CLI and dispatch to appropriate handler
    let result = codelet_cli::run().await;

    // Explicitly shutdown browser before exit (statics don't drop on exit)
    codelet_tools::shutdown_browser();

    result
}
