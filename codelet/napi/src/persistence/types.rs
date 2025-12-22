//! Core types for session persistence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Token usage tracking for a session
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TokenUsage {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}

/// Record of a merge operation for audit trail
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRecord {
    /// Session messages were merged from
    pub source_session_id: Uuid,
    /// Which message indices were imported
    pub source_indices: Vec<usize>,
    /// Where they were inserted (None = appended)
    pub inserted_at: Option<usize>,
    /// When the merge occurred
    pub merged_at: DateTime<Utc>,
}

/// Content that may be pasted by user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PastedContent {
    /// Small pasted content, stored inline
    Inline(String),
    /// Large pasted content, stored as blob
    BlobRef { hash: String, size_bytes: u64 },
}

/// A stored message in the content-addressed message store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    /// Unique identifier for this message
    pub id: Uuid,
    /// SHA-256 hash of the content (for integrity verification)
    pub content_hash: String,
    /// When this message was created
    pub created_at: DateTime<Utc>,
    /// Role: "user" or "assistant"
    pub role: String,
    /// The message content (or preview if blob_refs is populated)
    pub content: String,
    /// Approximate token count for context tracking
    pub token_count: Option<u32>,
    /// References to blob storage for large content
    pub blob_refs: Vec<String>,
    /// Provider-specific metadata (model used, stop reason, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tracks where a message reference came from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageSource {
    /// Created natively in this session
    Native,
    /// Inherited from a forked session
    Forked { from_session: Uuid },
    /// Imported via merge or cherry-pick
    Imported {
        from_session: Uuid,
        original_index: usize,
    },
}

/// A reference to a message in a session manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRef {
    /// The ID of the stored message
    pub message_id: Uuid,
    /// How this message got into this session
    pub source: MessageSource,
}

/// Records when and where a session was forked from
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkPoint {
    /// The session this was forked from
    pub source_session_id: Uuid,
    /// The message index at which the fork occurred (inclusive)
    pub fork_after_index: usize,
    /// When the fork happened
    pub forked_at: DateTime<Utc>,
}

/// Tracks compaction state for a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactionState {
    /// Summary of compacted messages
    pub summary: String,
    /// Messages 0 to (compacted_before_index - 1) are compacted
    pub compacted_before_index: usize,
    /// When compaction occurred
    pub compacted_at: DateTime<Utc>,
}

/// A session manifest - ordered list of message references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    /// Unique session identifier
    pub id: Uuid,
    /// Human-readable session name
    pub name: String,
    /// Project path this session belongs to
    pub project: PathBuf,
    /// Provider used (claude, openai, gemini, codex, etc.)
    #[serde(default)]
    pub provider: String,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Ordered list of message references
    pub messages: Vec<MessageRef>,
    /// If this session was forked, records the fork point
    pub forked_from: Option<ForkPoint>,
    /// Record of merges from other sessions
    #[serde(default)]
    pub merged_from: Vec<MergeRecord>,
    /// If this session has been compacted, records the state
    pub compaction: Option<CompactionState>,
    /// Token usage statistics
    #[serde(default)]
    pub token_usage: TokenUsage,
}

impl SessionManifest {
    /// Create a new empty session
    pub fn new(name: &str, project: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            project,
            provider: String::new(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            forked_from: None,
            merged_from: Vec::new(),
            compaction: None,
            token_usage: TokenUsage::default(),
        }
    }

    /// Create a new session with provider
    pub fn with_provider(name: &str, project: PathBuf, provider: &str) -> Self {
        let mut session = Self::new(name, project);
        session.provider = provider.to_string();
        session
    }

    /// Add a message reference to this session
    pub fn add_message(&mut self, message_id: Uuid, source: MessageSource) {
        self.messages.push(MessageRef { message_id, source });
        self.updated_at = Utc::now();
    }

    /// Record a merge operation
    pub fn record_merge(&mut self, record: MergeRecord) {
        self.merged_from.push(record);
        self.updated_at = Utc::now();
    }

    /// Get the number of messages in this session
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Update token usage
    pub fn update_token_usage(
        &mut self,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_create: u64,
    ) {
        self.token_usage.total_input_tokens += input;
        self.token_usage.total_output_tokens += output;
        self.token_usage.cache_read_tokens += cache_read;
        self.token_usage.cache_creation_tokens += cache_create;
    }
}

/// A command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// The command/input that was entered
    pub display: String,
    /// When the command was entered
    pub timestamp: DateTime<Utc>,
    /// Which project this was entered in
    pub project: PathBuf,
    /// Which session this was entered in
    pub session_id: Uuid,
    /// Any pasted content (stored separately if large)
    #[serde(default)]
    pub pasted_content: Option<PastedContent>,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(display: String, project: PathBuf, session_id: Uuid) -> Self {
        Self {
            display,
            timestamp: Utc::now(),
            project,
            session_id,
            pasted_content: None,
        }
    }

    /// Create a new history entry with pasted content
    pub fn with_pasted_content(
        display: String,
        project: PathBuf,
        session_id: Uuid,
        pasted: PastedContent,
    ) -> Self {
        Self {
            display,
            timestamp: Utc::now(),
            project,
            session_id,
            pasted_content: Some(pasted),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manifest_new() {
        let session = SessionManifest::new("Test", PathBuf::from("/test"));
        assert_eq!(session.name, "Test");
        assert_eq!(session.project, PathBuf::from("/test"));
        assert!(session.messages.is_empty());
        assert!(session.forked_from.is_none());
        assert!(session.compaction.is_none());
        assert!(session.merged_from.is_empty());
        assert_eq!(session.token_usage.total_input_tokens, 0);
    }

    #[test]
    fn test_session_with_provider() {
        let session = SessionManifest::with_provider("Test", PathBuf::from("/test"), "claude");
        assert_eq!(session.provider, "claude");
    }

    #[test]
    fn test_session_add_message() {
        let mut session = SessionManifest::new("Test", PathBuf::from("/test"));
        let msg_id = Uuid::new_v4();
        session.add_message(msg_id, MessageSource::Native);
        assert_eq!(session.message_count(), 1);
        assert_eq!(session.messages[0].message_id, msg_id);
    }

    #[test]
    fn test_token_usage_update() {
        let mut session = SessionManifest::new("Test", PathBuf::from("/test"));
        session.update_token_usage(100, 50, 10, 5);
        assert_eq!(session.token_usage.total_input_tokens, 100);
        assert_eq!(session.token_usage.total_output_tokens, 50);
        session.update_token_usage(100, 50, 10, 5);
        assert_eq!(session.token_usage.total_input_tokens, 200);
    }

    #[test]
    fn test_merge_record() {
        let mut session = SessionManifest::new("Test", PathBuf::from("/test"));
        let record = MergeRecord {
            source_session_id: Uuid::new_v4(),
            source_indices: vec![1, 2, 3],
            inserted_at: None,
            merged_at: Utc::now(),
        };
        session.record_merge(record.clone());
        assert_eq!(session.merged_from.len(), 1);
        assert_eq!(session.merged_from[0].source_indices, vec![1, 2, 3]);
    }
}
