use std::io;
use thiserror::Error;

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

    /// BUG-467: stopping when no capture is running.
    #[error("debug capture is not running")]
    NotCapturing,

    /// BUG-467: events that could not be written to the session file.
    #[error("{count} debug event(s) could not be written to {session_file}: {last_error}")]
    EventsLost {
        count: u64,
        last_error: String,
        session_file: String,
    },

    /// BUG-467: the session summary could not be produced.
    #[error("the debug session summary could not be written: {0}")]
    Summary(String),
}
