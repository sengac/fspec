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
}
