//! Error types for git operations

use thiserror::Error;

/// Result type for git operations
pub type Result<T> = std::result::Result<T, GitError>;

/// Errors that can occur during git operations
#[derive(Error, Debug)]
pub enum GitError {
    /// Failed to open repository
    #[error("Failed to open repository at '{path}': {source}")]
    OpenRepository {
        path: String,
        #[source]
        source: Box<gix::open::Error>,
    },

    /// Failed to get repository status
    #[error("Failed to get repository status: {0}")]
    Status(String),

    /// Failed to read HEAD reference
    #[error("Failed to read HEAD: {0}")]
    Head(String),

    /// Failed to read file from tree
    #[error("Failed to read file '{path}' from tree: {source}")]
    ReadBlob {
        path: String,
        #[source]
        source: anyhow::Error,
    },

    /// File not found in repository
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Worktree already exists for session
    #[error("Worktree already exists for session '{session_id}'")]
    WorktreeExists { session_id: String },

    /// Worktree not found for session
    #[error("Worktree not found for session '{session_id}'")]
    WorktreeNotFound { session_id: String },

    /// Failed to create worktree
    #[error("Failed to create worktree: {message}")]
    WorktreeCreate { message: String },

    /// Not a git repository
    #[error("Not a git repository: {path}")]
    NotARepository { path: String },

    /// Invalid commit reference
    #[error("Invalid commit reference: {commit_ref}")]
    InvalidCommitRef { commit_ref: String },

    /// Conflict detected when applying session changes
    #[error("Conflict detected: {files:?} have been modified in both session and main worktree")]
    ConflictError { files: Vec<String> },

    /// Git index is corrupted or missing
    #[error("Corrupted git index: {message}")]
    CorruptedIndex { message: String },

    /// Other error
    #[error("{0}")]
    Other(String),
}
