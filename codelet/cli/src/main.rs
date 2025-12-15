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

    // Parse CLI and dispatch to appropriate handler
    codelet_cli::run().await
}
