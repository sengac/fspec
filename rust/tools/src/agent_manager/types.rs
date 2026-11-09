//! AgentManager types — action enum, args, and result structures
//!
//! Feature: spec/features/agent-manager-core.feature
//! Feature: spec/features/agent-manager-messaging.feature
//! Feature: spec/features/agent-manager-context-resolution.feature
//! Feature: spec/features/agent-manager-await-idle.feature
//!
//! Defines the data model for the AgentManager tool's seven core actions:
//! - `spawn`: Create a new subordinate session with optional role
//! - `list`: List all sessions with relationships
//! - `get_status`: Get detailed status of a specific session
//! - `close`: Terminate a subordinate session (spawner-only)
//! - `message`: Send a plain text message to any session by ID, with optional context references
//! - `set_role`: Set or replace the system prompt overlay on a session
//! - `await_idle`: Block until one or more sessions reach idle state

use serde::{Deserialize, Serialize};

/// Flexible session ID parameter — accepts a single string or array of strings (AMGR-015)
///
/// Allows the `await_idle` action to accept either:
/// - A single session ID: `"session_id": "abc-123"`
/// - Multiple session IDs: `"session_id": ["abc-123", "def-456"]`
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SessionIdParam {
    /// A single session ID string
    Single(String),
    /// An array of session ID strings
    Multiple(Vec<String>),
}

impl SessionIdParam {
    /// Convert to a Vec of session ID strings regardless of variant
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s],
            Self::Multiple(v) => v,
        }
    }
}

/// AgentManager action types — discriminated union via serde tag
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentManagerAction {
    /// Create a new subordinate session
    Spawn {
        /// Optional role string — system prompt overlay for the subordinate
        #[serde(default)]
        role: Option<String>,
    },
    /// List all sessions with their relationships
    List,
    /// Get detailed status of a specific session
    GetStatus {
        /// Session ID to query (required)
        session_id: String,
    },
    /// Close/terminate a subordinate session
    Close {
        /// Session ID to close (required)
        session_id: String,
    },
    /// Send a plain text message to any session by ID, with optional context references
    Message {
        /// Target session ID to send the message to (required)
        session_id: String,
        /// Message text content (required)
        message: String,
        /// Optional context references to resolve and include with the message (AMGR-011)
        #[serde(default)]
        context: Option<Vec<ContextReference>>,
    },
    /// Set or replace the role (system prompt overlay) on a session (AMGR-012)
    SetRole {
        /// Target session ID (optional — defaults to caller's own session)
        #[serde(default)]
        session_id: Option<String>,
        /// Role string to set (empty string clears the role)
        role: String,
    },
    /// Await one or more sessions reaching idle state (AMGR-015)
    ///
    /// Blocks efficiently using broadcast channel subscription rather than polling.
    /// Returns structured results showing which sessions became idle, timed out,
    /// were destroyed, or were interrupted.
    AwaitIdle {
        /// Target session ID(s) — accepts a single string or array of strings
        session_id: SessionIdParam,
        /// Optional maximum wait time in seconds. If omitted, waits indefinitely.
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u64")]
        timeout: Option<u64>,
    },
    /// Run a time-bounded runtime profiling window (AMGR-017)
    ///
    /// Flips the global `PROFILING_ACTIVE` atomic gate to `true`, resets per-scope counters,
    /// sleeps for `duration_secs` seconds (default 10, range 1..=60), captures process and
    /// runtime metrics at window boundaries, then returns an aggregated `ProfileResult`.
    ///
    /// This call BLOCKS the caller for the full duration — by design. Only one profile
    /// session may be active at a time; overlapping calls are rejected with
    /// `{ error: "profile_session_active", ... }` via the async handler.
    /// AMGR-021: answer a question the caller holds, by its request id.
    Answer {
        request_id: String,
        #[serde(default)]
        answers: Option<Vec<RequestAnswer>>,
        #[serde(default)]
        cancelled: Option<bool>,
    },
    /// AMGR-021: decide an approval the caller holds, by its request id.
    Approve {
        request_id: String,
        choice: ApproveChoice,
    },
    /// AMGR-021: pass a request the caller holds up to its own parent, or
    /// the user at the root.
    Escalate { request_id: String },
    Profile {
        /// Length of the profiling window in seconds (1..=60, default 10)
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u32")]
        duration_secs: Option<u32>,
        /// Cap for `scopes_by_calls` and `scopes_by_self_ms` lists (default 20, max 200)
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        top_n: Option<usize>,
        /// Optional label prefix filter; only scopes whose label starts with this string
        /// appear in the result
        #[serde(default)]
        label_prefix: Option<String>,
        /// AMGR-018: optional substring filter applied BEFORE per-leaf attribution.
        /// Any sample whose full stack does not contain a frame whose symbol contains
        /// this substring is dropped, so `hot_stacks`, `scopes_by_calls`,
        /// `scopes_by_self_ms`, and `samples_by_thread` all reflect the narrowed view.
        /// Use this to confirm or rule out a suspected hot function in one call.
        #[serde(default)]
        focus: Option<String>,
    },
}

/// AMGR-021: one question's answer in an `answer` action.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RequestAnswer {
    /// The question's id.
    pub id: String,
    /// Labels of the chosen options.
    #[serde(default)]
    pub selected: Vec<String>,
    /// Free text.
    #[serde(default)]
    pub other: Option<String>,
}

/// AMGR-021: the decision in an `approve` action.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApproveChoice {
    Approve,
    ApproveSession,
    Deny,
}

/// AMGR-021: a pending question or approval as AgentManager reports it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PendingRequestView {
    pub request_id: String,
    /// The sub-agent that asks and waits.
    pub origin_session_id: String,
    /// `question`, `approval` or `human_action`.
    pub kind: String,
    /// `user`, or the id of the session that holds it.
    pub holder: String,
    /// What a question asks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub questions: Option<Vec<crate::request_user_input::HitlQuestion>>,
    /// What an approval or human action asks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<ApprovalView>,
}

/// AMGR-021: the tool call an approval or human action is about.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalView {
    pub tool_name: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Context reference for quoting session history in messages (AMGR-011)
///
/// Three variants allow referencing conversation history by:
/// - Specific turn indices
/// - Contiguous turn range
/// - Search query (ripgrep regex)
///
/// References are resolved at send time in the handler. The resolved
/// content is appended to the message text as XML-style quoted-context blocks.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ContextReference {
    /// Reference specific turns by index: { session_id, turns: [0, 1, 2] }
    Turns {
        session_id: String,
        #[serde(deserialize_with = "crate::serde_coerce::deser_vec_usize")]
        turns: Vec<usize>,
    },
    /// Reference a contiguous range of turns: { session_id, start_turn: 0, end_turn: 5 }
    TurnRange {
        session_id: String,
        #[serde(deserialize_with = "crate::serde_coerce::deser_usize")]
        start_turn: usize,
        #[serde(deserialize_with = "crate::serde_coerce::deser_usize")]
        end_turn: usize,
    },
    /// Reference turns matching a search query: { session_id, query: "SQL injection" }
    Query { session_id: String, query: String },
}

/// Top-level args for the AgentManager tool
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentManagerArgs {
    /// The action to perform
    #[serde(flatten)]
    pub action: AgentManagerAction,
}

/// A session entry returned by the `list` action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    /// Session UUID
    pub session_id: String,
    /// Human-readable session name
    pub name: String,
    /// Session role (system prompt overlay)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Session status (idle, running, interrupted, etc.)
    pub status: String,
    /// ID of the session that spawned this one (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawner_id: Option<String>,
    /// IDs of subordinate sessions spawned by this session
    pub subordinate_ids: Vec<String>,
    /// AMGR-021: the question or approval the session waits on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_request: Option<PendingRequestView>,
}

/// Detailed status returned by `get_status`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    /// Session UUID
    pub session_id: String,
    /// Session role (system prompt overlay)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Session status (idle, running, interrupted, etc.)
    pub status: String,
    /// Model being used by this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ID of the session that spawned this one (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawner_id: Option<String>,
    /// IDs of subordinate sessions spawned by this session
    pub subordinate_ids: Vec<String>,
    /// Number of pending incoming messages
    pub pending_messages: usize,
    /// AMGR-021: the question or approval the session waits on (boxed: it
    /// is the largest result).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_request: Option<Box<PendingRequestView>>,
}

/// Result from any AgentManager action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentManagerResult {
    /// Result from the `spawn` action
    Spawned { session_id: String },
    /// Result from the `list` action
    Listed { sessions: Vec<SessionEntry> },
    /// Result from the `get_status` action
    Status(SessionStatus),
    /// Result from the `close` action
    Closed { closed: bool, session_id: String },
    /// Result from the `message` action (plain delivery, no context)
    MessageDelivered { delivered: bool, session_id: String },
    /// Result from the `message` action with context resolution (AMGR-011)
    MessageDeliveredWithContext {
        delivered: bool,
        session_id: String,
        /// Number of context references that resolved successfully
        context_resolved: usize,
    },
    /// Result from the `set_role` action (AMGR-012)
    RoleSet {
        session_id: String,
        /// New role value, or None if role was cleared
        role: Option<String>,
    },
    /// Result from the `await_idle` action (AMGR-015)
    AwaitResult {
        /// Per-session await outcomes
        results: Vec<AwaitSessionResult>,
    },
    /// AMGR-021: an `answer` or `approve` was accepted.
    RequestAnswered { answered: bool, request_id: String },
    /// AMGR-021: an `escalate` moved the request to `holder`.
    RequestEscalated { request_id: String, holder: String },
    /// Error result
    Error {
        error: bool,
        code: String,
        message: String,
    },
}

/// Per-session result from `await_idle` (AMGR-015)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AwaitSessionResult {
    /// Session UUID
    pub session_id: String,
    /// Outcome of the await for this session
    pub status: AwaitOutcome,
    /// AMGR-021: the request behind `needs_input` or `waiting_on_user`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<PendingRequestView>,
}

/// Outcome of awaiting a single session (AMGR-015)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AwaitOutcome {
    /// Session reached idle state
    Idle,
    /// Timeout expired before session became idle
    TimedOut,
    /// Session was destroyed during the wait
    Destroyed,
    /// Calling session was interrupted (Esc) during the wait
    Interrupted,
    /// AMGR-021: the session, or one below it, has a request the caller
    /// holds; answer, approve or escalate it.
    NeedsInput,
    /// AMGR-021: the session, or one below it, waits on the user for a
    /// human action; reported once.
    WaitingOnUser,
    /// AMGR-021: not finished when await_idle returned for another session.
    Pending,
}

impl AgentManagerResult {
    /// Create a session_not_found error
    pub fn session_not_found(session_id: &str) -> Self {
        Self::Error {
            error: true,
            code: "session_not_found".to_string(),
            message: format!("Session not found: {session_id}"),
        }
    }

    /// Create a permission_denied error
    pub fn permission_denied(message: &str) -> Self {
        Self::Error {
            error: true,
            code: "permission_denied".to_string(),
            message: message.to_string(),
        }
    }

    /// Create an invalid_parameter error
    pub fn invalid_parameter(message: &str) -> Self {
        Self::Error {
            error: true,
            code: "invalid_parameter".to_string(),
            message: message.to_string(),
        }
    }

    /// AMGR-021: an answer, approval or escalation that the request's id,
    /// kind or holder refused.
    pub fn request_rejected(message: &str) -> Self {
        Self::Error {
            error: true,
            code: "request_rejected".to_string(),
            message: message.to_string(),
        }
    }

    pub fn request_not_found(request_id: &str) -> Self {
        Self::Error {
            error: true,
            code: "request_not_found".to_string(),
            message: format!("No pending request {request_id}"),
        }
    }

    /// Create a delivery_failed error (incoming message channel full)
    pub fn delivery_failed(message: &str) -> Self {
        Self::Error {
            error: true,
            code: "delivery_failed".to_string(),
            message: message.to_string(),
        }
    }
}
