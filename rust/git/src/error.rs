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

    /// Failed to discover repository (walking up parent directories)
    #[error("Failed to discover repository from '{path}': {source}")]
    DiscoverRepository {
        path: String,
        #[source]
        source: Box<gix::discover::Error>,
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

    /// WT-010: conflict detected when restoring a ghost-commit checkpoint
    /// with force=false — the working tree diverges from the checkpoint.
    /// No files were written or deleted.
    #[error("Restore conflict: {files:?} have changed since checkpoint (use force to overwrite)")]
    RestoreConflict { files: Vec<String> },

    /// Git index is corrupted or missing
    #[error("Corrupted git index: {message}")]
    CorruptedIndex { message: String },

    /// WT-008: the requested merge strategy is not supported. Only
    /// FastForward is implemented; Squash/ThreeWay are rejected up
    /// front so a client never silently gets a different algorithm than
    /// it asked for. No files were copied or committed.
    #[error("Unsupported merge strategy: only FastForward is supported")]
    UnsupportedMergeStrategy,

    /// WT-017: the worktree's copy of shared spec state (board, ID
    /// counters, epics, tags, foundation, config, ...) differs from the
    /// session's base. The session reads and writes that state at the main
    /// checkout, so a changed copy means something wrote around the
    /// state-root resolver. Nothing was copied or committed.
    #[error(
        "Merge refused: the session changed its worktree copy of shared spec state {files:?}, \
         which lives only in the main checkout; restore these files in the worktree to their \
         content at the session's base commit and merge again"
    )]
    SharedStateModified { files: Vec<String> },

    /// Other error
    #[error("{0}")]
    Other(String),
}
