/// Record of a merge operation for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeRecord {
    /// Session messages were merged from.
    pub source_session_id: Uuid,
    /// Which message indices were imported.
    pub source_indices: Vec<usize>,
    /// Where they were inserted (None = appended).
    pub inserted_at: Option<usize>,
    /// When the merge occurred.
    pub merged_at: DateTime<Utc>,
}

/// Records when and where a session was forked from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkPoint {
    /// The session this was forked from.
    pub source_session_id: Uuid,
    /// The message index at which the fork occurred (inclusive).
    pub fork_after_index: usize,
    /// When the fork happened.
    pub forked_at: DateTime<Utc>,
}

/// Tracks compaction state for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactionState {
    /// Summary of compacted messages.
    pub summary: String,
    /// Messages 0..compacted_before_index are compacted.
    pub compacted_before_index: usize,
    /// When compaction occurred.
    pub compacted_at: DateTime<Utc>,
}

/// A session manifest — ordered list of message references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    #[serde(default)]
    pub work_unit_authorization: Option<codelet_rpc_types::WorkUnitContext>,
    #[serde(default)]
    pub work_unit_focus: codelet_rpc_types::WorkUnitFocus,
    #[serde(default)]
    pub latest_request_usage: Option<rig::completion::Usage>,
    #[serde(default)]
    pub request_settlements: std::collections::BTreeMap<String, rig::completion::Usage>,
    #[serde(default)]
    pub compaction_commit: Option<super::compaction_boundary::CommittedGeneration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_state: Option<super::provider_state::ProviderStateRef>,
    #[serde(default)]
    pub revision: u64,
    /// Unique session identifier.
    pub id: Uuid,
    /// Human-readable session name.
    pub name: String,
    /// Project path this session belongs to.
    pub project: PathBuf,
    /// Provider used (claude, openai, gemini, codex, etc.).
    #[serde(default)]
    pub provider: String,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last updated.
    pub updated_at: DateTime<Utc>,
    /// Ordered list of message references.
    pub messages: Vec<MessageRef>,
    /// If this session was forked, records the fork point.
    pub forked_from: Option<ForkPoint>,
    /// Record of merges from other sessions.
    #[serde(default)]
    pub merged_from: Vec<MergeRecord>,
    /// If this session has been compacted, records the state.
    pub compaction: Option<CompactionState>,
    /// Token usage statistics.
    #[serde(default)]
    pub token_usage: TokenUsage,
}

impl SessionManifest {
    /// Create a new empty session.
    pub fn new(name: &str, project: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            work_unit_authorization: None,
            work_unit_focus: Default::default(),
            latest_request_usage: None,
            request_settlements: Default::default(),
            compaction_commit: None,
            provider_state: None,
            revision: 0,
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

    /// Create a new session with provider.
    pub fn with_provider(name: &str, project: PathBuf, provider: &str) -> Self {
        let mut session = Self::new(name, project);
        session.provider = provider.to_string();
        session
    }

    /// Add a message reference to this session.
    pub fn add_message(&mut self, message_id: Uuid, source: MessageSource) {
        self.messages.push(MessageRef { message_id, source });
        self.updated_at = Utc::now();
    }

    /// Record a merge operation.
    pub fn record_merge(&mut self, record: MergeRecord) {
        self.merged_from.push(record);
        self.updated_at = Utc::now();
    }

    /// Number of messages in this session.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Update token usage with dual metrics (CTX-003).
    pub fn update_token_usage(
        &mut self,
        input: u64,
        output: u64,
        cache_read: u64,
        cache_create: u64,
    ) {
        self.token_usage.current_context_tokens = input;
        self.token_usage.cumulative_billed_input += input;
        self.token_usage.cumulative_billed_output += output;
        self.token_usage.cache_read_tokens = cache_read;
        self.token_usage.cache_creation_tokens = cache_create;
    }
}

/// Session lineage information returned by [`get_session_lineage`].
#[derive(Debug, Clone)]
pub struct SessionLineage {
    pub session_id: Uuid,
    pub forked_from: Option<ForkPoint>,
    pub merged_from: Vec<MergeRecord>,
}
