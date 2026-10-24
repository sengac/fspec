#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    #[error("Codex checkpoint {0} lock is inaccessible")]
    Inaccessible(&'static str),
    #[error("Codex checkpoint version {0} is unsupported")]
    UnsupportedVersion(u32),
    #[error("Codex checkpoint persistence failed: {0}")]
    Persistence(String),
    #[error("Codex checkpoint requires explicit resolution after a failed operation")]
    Blocked,
    #[error("Codex checkpoint lineage mismatch: {0}")]
    Lineage(&'static str),
    #[error("Codex replay has no recorded outcome for call {0}")]
    MissingOutcome(String),
    #[error("Codex existing session has no required checkpoint")]
    Missing,
}

impl From<CheckpointError> for rig::completion::CompletionError {
    fn from(error: CheckpointError) -> Self {
        Self::RequestError(Box::new(error))
    }
}
