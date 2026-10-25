use super::*;

pub const SUPERVISOR_BROADCAST_CAPACITY: usize = 256;

pub struct PromptInput {
    pub input: String,
    pub thinking_config: Option<String>,
    pub compaction_epoch: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct CompactionProgress {
    pub phase: String,
    pub current: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Default)]
pub struct WorkUnitContext {
    pub id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
}

impl WorkUnitContext {
    pub fn new(id: String, title: String, status: String) -> Self {
        Self {
            id: Some(id),
            title: Some(title),
            status: Some(status),
        }
    }

    pub fn is_set(&self) -> bool {
        self.id.is_some()
    }

    pub fn format_for_environment(&self) -> Option<String> {
        self.id
            .as_ref()
            .map(|id| format!("Current work unit: {}", id))
    }
}

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub source_session_id: String,
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}

#[derive(Debug, Clone)]
pub struct BridgeImageData {
    pub data: String,
    pub media_type: String,
}

impl IncomingMessage {
    pub fn new(
        source_session_id: String,
        role_name: String,
        message: String,
    ) -> std::result::Result<Self, String> {
        if message.is_empty() {
            return Err("message cannot be empty".to_string());
        }
        Ok(Self {
            source_session_id,
            role_name,
            message,
            images: None,
        })
    }

    pub fn with_images(
        source_session_id: String,
        role_name: String,
        message: String,
        images: Option<Vec<BridgeImageData>>,
    ) -> std::result::Result<Self, String> {
        if message.is_empty() && images.as_ref().is_none_or(|v| v.is_empty()) {
            return Err("message cannot be empty when no images are provided".to_string());
        }
        Ok(Self {
            source_session_id,
            role_name,
            message,
            images,
        })
    }
}

pub fn format_incoming_message(input: &IncomingMessage) -> String {
    format!(
        "[SUPERVISOR: {} | Session: {}] {}",
        input.role_name, input.source_session_id, input.message
    )
}

#[derive(Debug)]
pub enum SessionError {
    NotIsolated,
    GitError(GitError),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::NotIsolated => {
                write!(f, "Session is not isolated - checkpoint operations require an isolated session with worktree")
            }
            SessionError::GitError(e) => write!(f, "Git error: {}", e),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<GitError> for SessionError {
    fn from(err: GitError) -> Self {
        SessionError::GitError(err)
    }
}
