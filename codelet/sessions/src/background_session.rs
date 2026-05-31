//! Background session that runs the agent loop in a tokio task.
//!
//! Moved here by **RPC-039** from `codelet/napi/src/session_manager.rs`
//! (former lines 459–1356). The agent loop (`SessionManager::run_session_loop`),
//! `ChainOfCommand`, and the `#[napi]` free functions stay napi-side for
//! this card — they are populated into `codelet-sessions` by RPC-040.
//!
//! ## What changed compared to the napi-side original
//!
//! * `napi::bindgen_prelude::Result<()>` on
//!   [`BackgroundSession::send_input`] became `Result<(), String>`. The
//!   napi free function `session_send_input` (in
//!   `codelet/napi/src/session_manager.rs`) maps the new string error
//!   back to `napi::Error::from_reason(...)` at the wire boundary so
//!   the TypeScript `Promise<void>` shape is preserved verbatim.
//! * `FspecResult` is now `codelet_rpc_types::FspecResult`
//!   (lifted in RPC-036).
//! * The pre-existing call to the napi-side `GLOBAL_CHUNK_CALLBACK`
//!   global was removed from [`BackgroundSession::handle_output`] (the
//!   global itself stays alive in napi for this card — its deletion is
//!   explicitly RPC-041). In its place an optional
//!   `chunks_tx: Option<broadcast::Sender<(SessionId, StreamChunk)>>`
//!   field is added. RPC-039 leaves the field defaulted to `None` and
//!   the napi shell continues to drive the global callback from the
//!   agent_loop sites (which still live napi-side); chunks therefore
//!   keep reaching the TS frontend unchanged.

#![allow(clippy::too_many_arguments)]
// RPC-039: `pub(crate)` items in this module appear unused inside
// codelet-sessions because the only callers (the agent_loop +
// SessionManager free functions) still live in codelet-napi and reach
// them via the `pub use codelet_sessions::background_session::*;`
// re-exports declared at the top of codelet/napi/src/session_manager.rs.
// Rust's dead-code lint can't see those callers because they live in a
// downstream crate.
#![allow(dead_code)]
// RPC-039: this file was moved verbatim from codelet-napi (which does
// not inherit workspace clippy lints) into codelet-sessions (which
// does). The lints relaxed below were never checked against the moved
// code by the napi build. Cleanup is tracked by RPC-040 (which moves
// SessionManager) and RPC-042 (which implements SessionManagerHandle on
// the extracted types); both cards refactor enough of this file that
// re-tightening these lints is a natural next step.
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::uninlined_format_args)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use tokio::sync::{Mutex, Notify, broadcast, mpsc};
use uuid::Uuid;

// Lifecycle hooks live in codelet-core (HOOK-013).
use codelet_core::lifecycle_hooks::{CompiledLifecycleHooks, HookContext};

// Per-session debug capture (BUG-134) lives in codelet-common, already NAPI-free.
use codelet_common::debug_capture::{DebugCaptureManager, PoisonRecoveryMutex};

// Gather-environment helper lives in codelet-cli, already NAPI-free.
use codelet_cli::session::context_gathering::gather_environment_info;

// Ghost-commit / worktree helpers live in codelet-git (GIT-019, GIT-021).
use codelet_git::GitError;
use codelet_git::ghost_commit::{
    GhostCheckpoint, RestoreResult, create_ghost_commit, list_ghost_checkpoints,
    restore_ghost_commit,
};

// Bash-abort + pause types live in codelet-tools, already NAPI-free.
// HitlRequest / HitlResponse are referenced via their fully-qualified
// `codelet_tools::request_user_input::*` path inside field signatures
// (matching the pre-move idiom), so no `use` import is needed for them.
use codelet_tools::tool_pause::{PauseResponse, PauseState};
use codelet_tools::{clear_bash_abort, request_bash_abort};

// Wire types lifted in RPC-036.
use codelet_rpc_types::{FspecResult, SessionInfo, SessionState, SessionStatus, StreamChunk};

/// Capacity of the per-session supervisor broadcast channel (WATCH-003).
///
/// Late subscribers start receiving from the current position; slow
/// receivers may see `RecvError::Lagged` if they fall more than this
/// many chunks behind.
pub const SUPERVISOR_BROADCAST_CAPACITY: usize = 256;

/// Input message sent to the agent loop via channel.
///
/// Fields are `pub` (was `pub(crate)`) because the agent_loop in
/// `codelet/napi/src/session_manager.rs` reads `prompt_input.input` and
/// `prompt_input.thinking_config` from outside this crate. The wider
/// visibility is unobservable from the JS side (the type is never
/// exposed through napi) but is required for the move to compile while
/// keeping the agent_loop napi-side.
pub struct PromptInput {
    /// The user's prompt text
    pub input: String,
    /// Optional thinking config JSON (for extended thinking)
    pub thinking_config: Option<String>,
}

/// PERF-002: Compaction progress information.
///
/// Local to BackgroundSession. The wire-side equivalent is
/// `codelet_rpc_types::CompactionProgress`; conversion impls are
/// deferred to RPC-042 (Implement SessionManagerHandle).
#[derive(Debug, Clone, Default)]
pub struct CompactionProgress {
    /// Current compaction phase (e.g., "Preparing compaction", "Analyzing context")
    pub phase: String,
    /// Current progress count (e.g., current turn being processed)
    pub current: u32,
    /// Total items to process (e.g., total turns to analyze)
    pub total: u32,
}

/// TUI-059: Work unit context for session.
///
/// Local to BackgroundSession with optional fields + the
/// `is_set()` / `format_for_environment()` helpers. The wire-side
/// equivalent is `codelet_rpc_types::WorkUnitContext` with required
/// fields; conversion impls are deferred to RPC-042.
#[derive(Debug, Clone, Default)]
pub struct WorkUnitContext {
    /// Work unit ID (e.g., "AUTH-001")
    pub id: Option<String>,
    /// Work unit title (e.g., "User Authentication")
    pub title: Option<String>,
    /// Current status (e.g., "specifying", "testing")
    pub status: Option<String>,
}

impl WorkUnitContext {
    /// Create a new work unit context
    pub fn new(id: String, title: String, status: String) -> Self {
        Self {
            id: Some(id),
            title: Some(title),
            status: Some(status),
        }
    }

    /// Check if context is set
    pub fn is_set(&self) -> bool {
        self.id.is_some()
    }

    /// Format work unit context for environment information
    /// Returns "Current work unit: ID" or None if not set
    /// TUI-059: Only includes ID, not title or status
    pub fn format_for_environment(&self) -> Option<String> {
        self.id.as_ref().map(|id| format!("Current work unit: {}", id))
    }
}

/// Incoming message for injection into a session (WATCH-006).
///
/// AMGR-008: Renamed from `SupervisorInput`. BRIDGE-007: Extended with
/// optional images from the Telegram bridge.
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    /// Session ID of the supervisor sending the input
    pub source_session_id: String,
    /// Role name of the supervisor (e.g., "code-reviewer")
    pub role_name: String,
    /// The message content to inject
    pub message: String,
    /// Optional images for multimodal input (BRIDGE-007)
    pub images: Option<Vec<BridgeImageData>>,
}

/// Image data from bridge (BRIDGE-007).
///
/// Matches the `ImageData` struct from `codelet_tools::bridge_relay`.
#[derive(Debug, Clone)]
pub struct BridgeImageData {
    /// Base64-encoded image data
    pub data: String,
    /// Media type (e.g., "image/jpeg", "image/png")
    pub media_type: String,
}

impl IncomingMessage {
    /// Create a new IncomingMessage (backward compatible - no images)
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

    /// Create a new IncomingMessage with images (BRIDGE-007)
    pub fn with_images(
        source_session_id: String,
        role_name: String,
        message: String,
        images: Option<Vec<BridgeImageData>>,
    ) -> std::result::Result<Self, String> {
        // Allow empty message if images are present
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

/// Format an incoming message with the structured prefix.
///
/// Format: `[SUPERVISOR: role | Session: id] message`
pub fn format_incoming_message(input: &IncomingMessage) -> String {
    format!(
        "[SUPERVISOR: {} | Session: {}] {}",
        input.role_name,
        input.source_session_id,
        input.message
    )
}

/// GIT-021: Error type for session checkpoint operations.
#[derive(Debug)]
pub enum SessionError {
    /// Session is not isolated - checkpoint operations require an isolated session with worktree
    NotIsolated,
    /// Git operation failed
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

/// Background session that runs agent in a tokio task.
///
/// The `id` field serves as the persistence identifier - TypeScript stores this ID
/// in its persistence system (persistenceStoreMessageEnvelope), and on restart can
/// use a future `restore_session(id)` function to recreate sessions with their original IDs.
pub struct BackgroundSession {
    /// Session ID - also serves as the persistence identifier for session recovery
    pub id: Uuid,
    pub name: RwLock<String>,
    pub project: String,

    /// Provider ID (e.g., "anthropic", "openai") - stored for quick access
    pub provider_id: RwLock<Option<String>>,
    /// Model ID (e.g., "claude-sonnet-4") - stored for quick access
    pub model_id: RwLock<Option<String>>,

    /// Cached token counts for quick sync access (updated on each TokenUpdate event)
    pub cached_input_tokens: AtomicU32,
    pub cached_output_tokens: AtomicU32,
    pub cached_reasoning_tokens: AtomicU32,

    /// CTX-006: Cached model limits for quick sync access
    /// Updated when model is set via session_set_model/session_set_model_profile
    /// 0 means not yet resolved (Option<u32> is not available in AtomicU32)
    pub cached_context_window: AtomicU32,
    pub cached_max_output_tokens: AtomicU32,
    /// CTX-007: Cached resolved compaction threshold
    pub cached_compaction_threshold: AtomicU32,

    /// Inner codelet session (protected by async mutex for agent operations)
    pub inner: Arc<Mutex<codelet_cli::session::Session>>,

    /// Current status (lock-free)
    status: AtomicU8,

    /// Channel to send input prompts to the agent loop
    input_tx: mpsc::Sender<PromptInput>,

    /// Buffered output chunks (unbounded - keeps all output for session lifetime)
    output_buffer: RwLock<Vec<StreamChunk>>,

    /// Interrupt flag for stopping agent execution
    pub is_interrupted: Arc<AtomicBool>,

    /// Notify for immediate interrupt wake-up
    pub interrupt_notify: Arc<Notify>,

    /// Debug capture enabled for this session
    is_debug_enabled: AtomicBool,

    /// BUG-134: Per-session debug capture manager
    /// Each session owns its own DebugCaptureManager instead of sharing a global singleton.
    /// This ensures toggling debug in one session doesn't affect another session's capture.
    pub debug_capture: Arc<PoisonRecoveryMutex<DebugCaptureManager>>,

    /// Pending input text (TUI-049: preserved when switching sessions)
    pending_input: RwLock<Option<String>>,

    /// Broadcast channel for supervisor sessions to observe stream output (WATCH-003)
    pub supervisor_broadcast: broadcast::Sender<StreamChunk>,

    /// RPC-041: Mandatory broadcast sender used by `handle_output` to fan
    /// every emitted chunk out to the cross-frontend `FspecBackend`
    /// subscribers (both the embedded backend in `codelet-fspec-tui` and
    /// the WebSocket backend that mirrors the same stream to remote
    /// clients). The napi adapter subscribes here to drive its
    /// JS `ThreadsafeFunction` fan-out, and the `SessionManager`
    /// populates this field at construction time. Non-Option:
    /// every BackgroundSession must have a live sender.
    chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>,

    /// RPC-041: Mandatory broadcast sender used by `set_status` to fan
    /// every status change out to typed `(SessionId, SessionStatus)`
    /// subscribers. The future Rust `fspec-tui` (RPC-045+) consumes this
    /// directly without parsing a `SessionStateChange` `StreamChunk`.
    /// Populated by `SessionManager` via constructor injection.
    status_changes_tx: broadcast::Sender<(codelet_rpc_types::SessionId, codelet_rpc_types::SessionStatus)>,

    /// Session role - simple string overlay for system prompt (AMGR-008: simplified from SupervisorRole struct)
    role: RwLock<Option<String>>,

    /// Channel for receiving supervisor input messages (WATCH-006)
    /// Supervisors use this to inject messages into the subordinate session
    incoming_message_tx: mpsc::Sender<IncomingMessage>,
    pub incoming_message_rx: Mutex<mpsc::Receiver<IncomingMessage>>,

    /// FIX-6: Counter for pending incoming messages in the channel.
    /// Incremented on send (receive_incoming_message), decremented on recv (agent_loop).
    /// mpsc::Receiver doesn't expose len(), so we track it with an atomic counter.
    pub incoming_message_pending: Arc<AtomicUsize>,

    /// Correlation ID counter for cross-pane selection highlighting (WATCH-011)
    /// Each chunk emitted by handle_output gets a unique correlation_id
    correlation_counter: AtomicU64,

    /// Pending observed correlation IDs for supervisor responses (WATCH-011)
    /// When a supervisor processes observations, this is set to the correlation IDs
    /// of the subordinate chunks that triggered the evaluation. handle_output then
    /// tags output chunks with these IDs until cleared.
    pending_observed_correlation_ids: RwLock<Vec<String>>,

    /// PAUSE-001: Current pause state (None when not paused)
    pause_state: RwLock<Option<PauseState>>,

    /// PAUSE-001: Channel to send pause response from TypeScript back to the blocking tool
    pause_response_tx: std::sync::mpsc::Sender<PauseResponse>,
    pause_response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<PauseResponse>>,

    /// CODE-009: Channel to send fspec command result from TypeScript back to the blocking session
    fspec_response_tx: std::sync::mpsc::Sender<FspecResult>,
    fspec_response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<FspecResult>>,

    /// BUG-117: Channel to send HITL response from TypeScript back to the blocking handler
    hitl_response_tx: std::sync::mpsc::Sender<codelet_tools::request_user_input::HitlResponse>,
    hitl_response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<codelet_tools::request_user_input::HitlResponse>>,

    /// BUG-117: HITL request state — stores questions while waiting for user response
    /// TypeScript polls this via session_get_hitl_request NAPI getter (like pause_state)
    hitl_request: RwLock<Option<codelet_tools::request_user_input::HitlRequest>>,

    /// TUI-054: Base thinking level for session (0=Off, 1=Low, 2=Medium, 3=High)
    /// This is the level set via /thinking command, persists for the session.
    /// Effective level = max(base_thinking_level, detected_level_from_text)
    base_thinking_level: AtomicU8,
    
    /// PERF-002: Current compaction progress information
    compaction_progress: RwLock<Option<CompactionProgress>>,
    
    /// TUI-059: Work unit context for session
    /// Tracks which work unit this session is currently working on
    work_unit_context: RwLock<Option<WorkUnitContext>>,
    
    /// TUI-059: Base environment content (without work unit)
    /// Stored so we can compose full environment info when work unit changes
    base_environment_content: RwLock<String>,
    
    /// GIT-019: Path to worktree for isolated sessions
    /// Only set when session was created with isolated=true
    pub worktree_path: Option<PathBuf>,
    
    /// GIT-019: Base commit SHA for isolated sessions
    /// The commit the worktree was created from
    pub base_commit: Option<String>,

    /// Flag controlling Layer 0 trimming in SessionSearch results.
    pub compaction_in_progress: Arc<AtomicBool>,

    /// Pending DAG content from inject_summary tool call.
    /// Stored here because the handler cannot lock session.inner during streaming.
    /// Applied by agent_loop after the stream completes.
    pub pending_dag_content: Arc<std::sync::Mutex<Option<String>>>,

    /// Pre-compaction token count for accurate CompactionComplete metrics.
    pub pre_compaction_tokens: AtomicU32,

    /// SCHED-004: Whether this session was spawned by the scheduler
    pub schedule_triggered: AtomicBool,

    /// SCHED-004: Name of the schedule that triggered this session (if any)
    pub schedule_name: RwLock<Option<String>>,

    /// HOOK-013: Compiled lifecycle hooks (None = no agent hooks configured → zero overhead)
    pub lifecycle_hooks: Option<Arc<CompiledLifecycleHooks>>,
}

impl BackgroundSession {
    /// Create a new background session
    /// 
    /// GIT-019: Added worktree_path and base_commit parameters for isolated session support
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Uuid,
        name: String,
        project: String,
        provider_id: Option<String>,
        model_id: Option<String>,
        inner: codelet_cli::session::Session,
        input_tx: mpsc::Sender<PromptInput>,
        worktree_path: Option<PathBuf>,
        base_commit: Option<String>,
        lifecycle_hooks: Option<Arc<CompiledLifecycleHooks>>,
        chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>,
        status_changes_tx: broadcast::Sender<(
            codelet_rpc_types::SessionId,
            codelet_rpc_types::SessionStatus,
        )>,
    ) -> Self {
        // Create supervisor input channel (WATCH-006)
        let (incoming_message_tx, incoming_message_rx) = mpsc::channel::<IncomingMessage>(16);

        // PAUSE-001: Create pause response channel (std::sync for blocking receive)
        let (pause_response_tx, pause_response_rx) = std::sync::mpsc::channel::<PauseResponse>();

        // CODE-009: Create fspec response channel (std::sync for blocking receive)
        let (fspec_response_tx, fspec_response_rx) = std::sync::mpsc::channel::<FspecResult>();

        // BUG-117: Create HITL response channel (std::sync for blocking receive)
        let (hitl_response_tx, hitl_response_rx) = std::sync::mpsc::channel::<codelet_tools::request_user_input::HitlResponse>();

        Self {
            id,
            name: RwLock::new(name),
            project,
            provider_id: RwLock::new(provider_id),
            model_id: RwLock::new(model_id),
            cached_input_tokens: AtomicU32::new(0),
            cached_output_tokens: AtomicU32::new(0),
            cached_reasoning_tokens: AtomicU32::new(0),
            cached_context_window: AtomicU32::new(0),
            cached_max_output_tokens: AtomicU32::new(0),
            cached_compaction_threshold: AtomicU32::new(0),
            inner: Arc::new(Mutex::new(inner)),
            status: AtomicU8::new(SessionStatus::Idle as u8),
            input_tx,
            output_buffer: RwLock::new(Vec::new()),
            is_interrupted: Arc::new(AtomicBool::new(false)),
            interrupt_notify: Arc::new(Notify::new()),
            is_debug_enabled: AtomicBool::new(false),
            pending_input: RwLock::new(None),
            supervisor_broadcast: broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY).0,
            chunks_tx,
            status_changes_tx,
            role: RwLock::new(None),
            incoming_message_tx,
            incoming_message_rx: Mutex::new(incoming_message_rx),
            incoming_message_pending: Arc::new(AtomicUsize::new(0)),
            correlation_counter: AtomicU64::new(0),
            pending_observed_correlation_ids: RwLock::new(Vec::new()),
            pause_state: RwLock::new(None),
            pause_response_tx,
            pause_response_rx: std::sync::Mutex::new(pause_response_rx),
            fspec_response_tx,
            fspec_response_rx: std::sync::Mutex::new(fspec_response_rx),
            hitl_response_tx,
            hitl_response_rx: std::sync::Mutex::new(hitl_response_rx),
            hitl_request: RwLock::new(None),
            base_thinking_level: AtomicU8::new(0), // TUI-054: Default to Off
            compaction_progress: RwLock::new(None), // PERF-002: No compaction in progress initially
            work_unit_context: RwLock::new(None), // TUI-059: No work unit context initially
            // TUI-059: Store base environment content for composing with work unit later
            base_environment_content: RwLock::new(gather_environment_info().to_reminder_content()),
            // GIT-019: Worktree path and base commit for isolated sessions
            worktree_path,
            base_commit,
            compaction_in_progress: Arc::new(AtomicBool::new(false)),
            pending_dag_content: Arc::new(std::sync::Mutex::new(None)),
            pre_compaction_tokens: AtomicU32::new(0),
            // SCHED-004: Default to non-scheduled
            schedule_triggered: AtomicBool::new(false),
            schedule_name: RwLock::new(None),
            // HOOK-013: Lifecycle hooks (compiled once at session creation)
            lifecycle_hooks,
            // BUG-134: Per-session debug capture manager
            debug_capture: {
                let mgr = DebugCaptureManager::new()
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to create per-session DebugCaptureManager: {e}");
                        // Fallback: create with default - will fail on capture but won't crash
                        DebugCaptureManager::new()
                            .expect("DebugCaptureManager::new() failed twice - data dir not set")
                    });
                Arc::new(PoisonRecoveryMutex::new(mgr))
            },
        }
    }

    /// GIT-019: Returns the effective working directory for this session
    /// 
    /// - For isolated sessions: returns the worktree path
    /// - For non-isolated sessions: returns the project root
    pub fn effective_cwd(&self) -> PathBuf {
        self.worktree_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.project))
    }

    /// HOOK-013: Build a HookContext for lifecycle hook execution.
    pub fn hook_context(&self) -> HookContext {
        HookContext {
            session_id: self.id.to_string(),
            cwd: self.effective_cwd().to_string_lossy().to_string(),
            transcript_path: format!(
                "{}/.fspec/sessions/{}/transcript.json",
                self.project, self.id
            ),
        }
    }

    /// GIT-034: Build isolation context for environment reminder injection
    /// 
    /// Returns Some(IsolationContext) if session is isolated, None otherwise.
    /// The worktree path is converted to a relative path from project root.
    fn build_isolation_context(&self) -> Option<codelet_cli::session::context_gathering::IsolationContext> {
        if let Some(ref worktree_path) = self.worktree_path {
            // Convert worktree path to relative path from project root
            let project_root = PathBuf::from(&self.project);
            let relative_path = worktree_path
                .strip_prefix(&project_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| worktree_path.to_string_lossy().to_string());
            
            Some(codelet_cli::session::context_gathering::IsolationContext {
                is_isolated: true,
                worktree_path: Some(relative_path),
                base_commit: self.base_commit.clone(),
            })
        } else {
            None
        }
    }

    /// GIT-021: Create a checkpoint capturing current worktree state
    /// 
    /// Uses ghost commits to capture all working tree state (staged, unstaged, untracked files).
    /// Checkpoints are stored at refs/fspec-checkpoints/<session-id>/<label>
    /// 
    /// # Arguments
    /// * `label` - Name for the checkpoint
    /// 
    /// # Errors
    /// * `SessionError::NotIsolated` - Session is not isolated (no worktree)
    /// * `SessionError::GitError` - Git operation failed
    pub fn checkpoint(&self, label: &str) -> std::result::Result<GhostCheckpoint, SessionError> {
        let worktree_path = self.worktree_path.as_ref()
            .ok_or(SessionError::NotIsolated)?;
        
        // Use session ID as the work_unit_id for checkpoint namespace
        create_ghost_commit(worktree_path, &self.id.to_string(), label)
            .map_err(SessionError::from)
    }

    /// GIT-021: Restore worktree to checkpoint state
    /// 
    /// Restores all files from the specified checkpoint, deleting files that
    /// were created after the checkpoint.
    /// 
    /// # Arguments
    /// * `label` - Name of the checkpoint to restore
    /// 
    /// # Errors
    /// * `SessionError::NotIsolated` - Session is not isolated (no worktree)
    /// * `SessionError::GitError` - Git operation failed
    pub fn restore(&self, label: &str) -> std::result::Result<RestoreResult, SessionError> {
        let worktree_path = self.worktree_path.as_ref()
            .ok_or(SessionError::NotIsolated)?;
        
        restore_ghost_commit(worktree_path, &self.id.to_string(), label, true)
            .map_err(SessionError::from)
    }

    /// GIT-021: List all checkpoints for this session
    /// 
    /// Returns all checkpoint labels that have been created for this session.
    /// 
    /// # Errors
    /// * `SessionError::NotIsolated` - Session is not isolated (no worktree)
    /// * `SessionError::GitError` - Git operation failed
    pub fn list_checkpoints(&self) -> std::result::Result<Vec<String>, SessionError> {
        let worktree_path = self.worktree_path.as_ref()
            .ok_or(SessionError::NotIsolated)?;
        
        list_ghost_checkpoints(worktree_path, &self.id.to_string())
            .map_err(SessionError::from)
    }

    /// Get debug enabled state
    pub fn get_debug_enabled(&self) -> bool {
        self.is_debug_enabled.load(Ordering::Acquire)
    }

    /// Set debug enabled state
    pub fn set_debug_enabled(&self, enabled: bool) {
        self.is_debug_enabled.store(enabled, Ordering::Release);
    }

    /// Get pending input text (TUI-049)
    pub fn get_pending_input(&self) -> Option<String> {
        self.pending_input.read().expect("pending_input lock poisoned").clone()
    }

    /// Set pending input text (TUI-049)
    pub fn set_pending_input(&self, input: Option<String>) {
        *self.pending_input.write().expect("pending_input lock poisoned") = input;
    }

    /// TUI-059: Get work unit context
    pub fn get_work_unit_context(&self) -> Option<WorkUnitContext> {
        self.work_unit_context.read().expect("work_unit_context lock poisoned").clone()
    }

    /// TUI-059: Set work unit context
    /// Updates the Environment system reminder to include work unit info
    pub fn set_work_unit_context(&self, id: Option<String>, title: Option<String>, status: Option<String>) {
        use codelet_cli::session::SystemReminderType;
        
        // Scope the locks so they're dropped before broadcast_metadata_update(),
        // which calls back into get_work_unit_context() on all sessions.
        // Without this scoping, the write lock on work_unit_context would deadlock
        // when the broadcast tries to read-lock the same session's work_unit_context.
        {
            let mut ctx = self.work_unit_context.write().expect("work_unit_context lock poisoned");
            let base_env = self.base_environment_content.read().expect("base_environment_content lock poisoned");
            
            if let (Some(id_val), Some(title_val), Some(status_val)) = (id.clone(), title.clone(), status.clone()) {
                *ctx = Some(WorkUnitContext::new(id_val, title_val, status_val));
                
                // Compose full environment info with work unit
                // TUI-059: Add "Current work unit: ID" to the environment info (alongside Platform, Shell, etc.)
                let work_unit_line = ctx.as_ref()
                    .and_then(|c| c.format_for_environment())
                    .unwrap_or_default();
                
                let full_env = if work_unit_line.is_empty() {
                    base_env.clone()
                } else {
                    format!("{}\n{}", base_env, work_unit_line)
                };
                
                // Update the Environment reminder (supersedes the original)
                if let Ok(mut inner) = self.inner.try_lock() {
                    inner.add_system_reminder(SystemReminderType::Environment, &full_env);
                }
            } else {
                *ctx = None;
                // Restore base environment without work unit
                if let Ok(mut inner) = self.inner.try_lock() {
                    inner.add_system_reminder(SystemReminderType::Environment, &base_env);
                }
            }
        } // locks dropped here

        // Broadcast metadata update so relay clients see work unit changes
        codelet_tools::broadcast_metadata_update();
    }

    /// Update cached token counts (called when TokenUpdate events are emitted)
    pub fn update_tokens(&self, input_tokens: u32, output_tokens: u32) {
        self.cached_input_tokens.store(input_tokens, Ordering::Release);
        self.cached_output_tokens.store(output_tokens, Ordering::Release);
    }

    /// Update cached reasoning token count
    pub fn update_reasoning_tokens(&self, reasoning_tokens: u32) {
        self.cached_reasoning_tokens.store(reasoning_tokens, Ordering::Release);
    }

    /// Get cached token counts
    pub fn get_tokens(&self) -> (u32, u32, Option<u32>) {
        let reasoning = self.cached_reasoning_tokens.load(Ordering::Acquire);
        (
            self.cached_input_tokens.load(Ordering::Acquire),
            self.cached_output_tokens.load(Ordering::Acquire),
            if reasoning > 0 { Some(reasoning) } else { None },
        )
    }

    /// Update the model info (called when model is changed mid-session)
    pub fn set_model(&self, provider_id: Option<String>, model_id: Option<String>) {
        *self.provider_id.write().expect("provider_id lock poisoned") = provider_id;
        *self.model_id.write().expect("model_id lock poisoned") = model_id;
    }

    /// CTX-006: Update the cached model limits from Rust-resolved ProviderManager values
    /// CTX-007: Also caches the resolved compaction threshold
    pub fn set_model_limits(&self, context_window: u32, max_output_tokens: u32, compaction_threshold: u32) {
        self.cached_context_window.store(context_window, Ordering::Release);
        self.cached_max_output_tokens.store(max_output_tokens, Ordering::Release);
        self.cached_compaction_threshold.store(compaction_threshold, Ordering::Release);
    }
    
    /// Get current status
    pub fn get_status(&self) -> SessionStatus {
        SessionStatus::from(self.status.load(Ordering::Acquire))
    }
    
    /// Set status and notify attached callback
    pub fn set_status(&self, status: SessionStatus) {
        let old_status = self.status.swap(status as u8, Ordering::AcqRel);
        
        // NAPI-010: Notify TypeScript when status changes via SessionStateChange chunk
        // This is an internal state update - NOT added to conversation
        if old_status != status as u8 {
            // RPC-041: Emit typed (SessionId, SessionStatus) status change
            // on the manager-owned `status_changes_tx` broadcast. Future
            // Rust subscribers (fspec-tui) can listen here without
            // parsing a `SessionStateChange` `StreamChunk`.
            let _ = self.status_changes_tx.send((
                codelet_rpc_types::SessionId::from(self.id.to_string()),
                status,
            ));

            let state = match status {
                SessionStatus::Idle => SessionState::Idle,
                SessionStatus::Running => SessionState::Running, 
                SessionStatus::Interrupted => SessionState::Interrupted,
                SessionStatus::Paused => SessionState::Paused,
                SessionStatus::Compacting => SessionState::Compacting,
                SessionStatus::Cleared => SessionState::Cleared,
            };
            self.handle_output(StreamChunk::session_state_change(state));
            
            // BRIDGE-SESSION: Broadcast metadata update on session state change
            // so relay clients see updated session status in real-time.
            codelet_tools::broadcast_metadata_update();
        }
    }
    
    /// Handle output chunk - buffer and optionally forward to callback
    /// WATCH-011: Assigns correlation_id for cross-pane selection highlighting
    /// WATCH-011: Applies pending_observed_correlation_ids for supervisor responses
    pub fn handle_output(&self, chunk: StreamChunk) {
        // WATCH-011: Assign correlation_id if not already set (for variants that support it)
        let id = self.correlation_counter.fetch_add(1, Ordering::SeqCst);
        let correlation_id = format!("{}-{}", self.id, id);
        let chunk = chunk.with_correlation_id(correlation_id);

        // WATCH-011: Apply pending observed_correlation_ids for supervisor responses
        // This tags supervisor output chunks with the subordinate chunk IDs that triggered this response
        let chunk = {
            let pending_ids = self.pending_observed_correlation_ids.read()
                .expect("pending_observed_correlation_ids lock poisoned");
            if !pending_ids.is_empty() {
                chunk.with_observed_correlation_ids(pending_ids.clone())
            } else {
                chunk
            }
        };

        // Always buffer (unbounded)
        {
            let mut buffer = self.output_buffer.write().expect("output buffer lock poisoned");
            buffer.push(chunk.clone());
        }

        // Broadcast to supervisor sessions (WATCH-003)
        // Fire-and-forget: ignores SendError when no receivers are subscribed
        let _ = self.supervisor_broadcast.send(chunk.clone());

        // RPC-041: Forward the chunk on the mandatory SessionManager-owned
        // broadcast channel. The napi adapter subscribes to this channel
        // and fans into the JS ThreadsafeFunction (the old
        // `GLOBAL_CHUNK_CALLBACK` static is gone). The future Rust
        // `fspec-tui` frontend subscribes here as a peer.
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(self.id.to_string()),
            chunk.clone(),
        ));
    }
    
    /// Get buffered output
    pub fn get_buffered_output(&self, limit: usize) -> Vec<StreamChunk> {
        let buffer = self.output_buffer.read().expect("output buffer lock poisoned");
        buffer.iter().take(limit).cloned().collect()
    }

    /// Subscribe to the output stream for supervisor sessions (WATCH-003)
    ///
    /// Returns a broadcast receiver that will receive all StreamChunks output by this session.
    /// Late subscribers start receiving from the current position (no replay of past chunks).
    /// Slow receivers may receive RecvError::Lagged if they fall more than 256 chunks behind.
    pub fn subscribe_to_stream(&self) -> broadcast::Receiver<StreamChunk> {
        self.supervisor_broadcast.subscribe()
    }

    /// Set the session role (WATCH-004)
    ///
    /// Used to mark a session as a supervisor with a specific role and brief.
    pub fn set_role(&self, role: String) {
        *self.role.write().expect("role lock poisoned") = Some(role);
    }

    /// Get the session role (WATCH-004)
    ///
    /// Returns None for regular sessions, Some(role) for supervisor sessions.
    pub fn get_role(&self) -> Option<String> {
        self.role.read().expect("role lock poisoned").clone()
    }

    /// Clear the session role (WATCH-004)
    ///
    /// Returns the session to a regular (non-supervisor) state.
    pub fn clear_role(&self) {
        *self.role.write().expect("role lock poisoned") = None;
    }

    // =========================================================================
    // PAUSE-001: Pause state methods
    // =========================================================================

    /// Get the current pause state (PAUSE-001)
    ///
    /// Returns None if session is not paused, Some(PauseState) if paused.
    pub fn get_pause_state(&self) -> Option<PauseState> {
        self.pause_state.read().expect("pause_state lock poisoned").clone()
    }

    /// Set the pause state (PAUSE-001)
    ///
    /// Called by the pause handler when a tool requests a pause.
    /// Also sets status to Paused.
    pub fn set_pause_state(&self, state: Option<PauseState>) {
        let is_paused = state.is_some();
        *self.pause_state.write().expect("pause_state lock poisoned") = state;
        if is_paused {
            self.set_status(SessionStatus::Paused);
        }
    }

    /// Clear pause state (PAUSE-001)
    ///
    /// Called when resuming from pause. Sets status back to Running.
    pub fn clear_pause_state(&self) {
        *self.pause_state.write().expect("pause_state lock poisoned") = None;
        self.set_status(SessionStatus::Running);
    }

    /// Wait for pause response (PAUSE-001) - BLOCKS until TypeScript sends response
    ///
    /// Called by the pause handler to block until the UI sends a response.
    pub fn wait_for_pause_response(&self) -> PauseResponse {
        let rx = self.pause_response_rx.lock().expect("pause_response_rx lock poisoned");
        // Block until we receive a response
        rx.recv().unwrap_or(PauseResponse::Interrupted)
    }

    /// Send pause response (PAUSE-001)
    ///
    /// Called by NAPI functions (sessionPauseResume, sessionPauseConfirm) when
    /// TypeScript sends the user's response.
    ///
    /// Order is critical: Send response FIRST to unblock the waiting tool,
    /// THEN clear pause state. This prevents a race condition where TypeScript
    /// might poll status and see "running" before the tool has received its response.
    pub fn send_pause_response(&self, response: PauseResponse) {
        // Send response first to unblock the waiting tool
        let _ = self.pause_response_tx.send(response);
        // Then clear pause state (tool is already unblocked and will continue)
        self.clear_pause_state();
    }

    /// Get pause response sender clone (PAUSE-001)
    ///
    /// Used by stream loop to create pause handler with session context.
    pub fn get_pause_response_tx(&self) -> std::sync::mpsc::Sender<PauseResponse> {
        self.pause_response_tx.clone()
    }

    // =========================================================================
    // CODE-009: Fspec command response methods
    // =========================================================================

    /// Wait for fspec command response (CODE-009) - BLOCKS until TypeScript sends result
    ///
    /// Called by session loop when FspecTool is invoked. Blocks until TypeScript
    /// executes the command and sends the result back via sessionSendFspecResult.
    pub fn wait_for_fspec_response(&self) -> FspecResult {
        let rx = self.fspec_response_rx.lock().expect("fspec_response_rx lock poisoned");
        // Block until we receive a response
        rx.recv().unwrap_or_else(|_| {
            FspecResult {
                success: false,
                data: String::new(),
                error: Some("Fspec response channel closed unexpectedly".to_string()),
                system_reminder: None,
                tool_call_id: String::new(),
            }
        })
    }

    /// Send fspec command result (CODE-009)
    ///
    /// Called by NAPI function (sessionSendFspecResult) when TypeScript
    /// has finished executing the fspec command and wants to send the result back.
    pub fn send_fspec_result(&self, result: FspecResult) {
        if let Err(e) = self.fspec_response_tx.send(result) {
            tracing::error!("[FSPEC_SESSION] Failed to send fspec result: {:?}", e);
        }
    }

    // =========================================================================
    // BUG-117: HITL response methods
    // =========================================================================

    /// Wait for HITL response (BUG-117) - BLOCKS until TypeScript sends user's answers
    ///
    /// Called by the HITL handler closure when request_user_input is invoked.
    /// Blocks until the TUI renders the modal and the user responds.
    pub fn wait_for_hitl_response(&self) -> codelet_tools::request_user_input::HitlResponse {
        let rx = self.hitl_response_rx.lock().expect("hitl_response_rx lock poisoned");
        // Block until we receive a response
        rx.recv().unwrap_or(codelet_tools::request_user_input::HitlResponse::Cancelled {
            cancelled: true,
        })
    }

    /// Send HITL response (BUG-117)
    ///
    /// Called by NAPI function (sessionSendHitlResponse) when TypeScript
    /// has collected the user's answers from the HITL input.
    pub fn send_hitl_response(&self, response: codelet_tools::request_user_input::HitlResponse) {
        if let Err(e) = self.hitl_response_tx.send(response) {
            tracing::error!("[HITL_SESSION] Failed to send HITL response: {:?}", e);
        }
    }

    /// Set HITL request state (BUG-117)
    ///
    /// Called by the HITL handler closure to store the questions for TypeScript to poll.
    /// Pass None to clear when done.
    pub fn set_hitl_request(&self, request: Option<codelet_tools::request_user_input::HitlRequest>) {
        if let Ok(mut guard) = self.hitl_request.write() {
            *guard = request;
        }
    }

    /// Get HITL request state (BUG-117)
    ///
    /// Called by NAPI getter (session_get_hitl_request) for TypeScript to poll.
    pub fn get_hitl_request(&self) -> Option<codelet_tools::request_user_input::HitlRequest> {
        self.hitl_request.read().ok().and_then(|guard| guard.clone())
    }

    // =========================================================================
    // TUI-054: Base thinking level methods
    // =========================================================================

    /// Get the base thinking level (TUI-054)
    ///
    /// Returns 0=Off, 1=Low, 2=Medium, 3=High
    pub fn get_base_thinking_level(&self) -> u8 {
        self.base_thinking_level.load(Ordering::Acquire)
    }

    /// Set the base thinking level (TUI-054)
    ///
    /// Values: 0=Off, 1=Low, 2=Medium, 3=High
    /// Values > 3 are clamped to 3 (High)
    pub fn set_base_thinking_level(&self, level: u8) {
        let clamped = level.min(3);
        self.base_thinking_level.store(clamped, Ordering::Release);
    }

    /// PERF-002: Get current compaction progress information
    pub fn get_compaction_progress(&self) -> Option<CompactionProgress> {
        self.compaction_progress.read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// PERF-002: Set compaction progress information
    pub fn set_compaction_progress(&self, progress: Option<CompactionProgress>) {
        *self.compaction_progress.write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = progress;
    }

    /// PERF-002: Update compaction progress phase and counts
    pub fn update_compaction_progress(&self, phase: String, current: u32, total: u32) {
        let progress = CompactionProgress { phase, current, total };
        *self.compaction_progress.write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(progress);
    }

    /// Set pending observed correlation IDs (WATCH-011)
    ///
    /// When a supervisor processes observations, call this before sending the
    /// evaluation prompt. All subsequent output chunks from handle_output
    /// will be tagged with these IDs until clear_pending_observed_correlation_ids is called.
    pub fn set_pending_observed_correlation_ids(&self, ids: Vec<String>) {
        *self.pending_observed_correlation_ids.write()
            .expect("pending_observed_correlation_ids lock poisoned") = ids;
    }

    /// Clear pending observed correlation IDs (WATCH-011)
    ///
    /// Call this after the supervisor finishes processing an observation response.
    /// Subsequent output chunks will no longer be tagged with observed IDs.
    pub fn clear_pending_observed_correlation_ids(&self) {
        self.pending_observed_correlation_ids.write()
            .expect("pending_observed_correlation_ids lock poisoned")
            .clear();
    }

    /// Receive supervisor input (WATCH-006)
    ///
    /// Queues a IncomingMessage message for processing by the subordinate session.
    /// The input is queued via an mpsc channel and processed asynchronously.
    /// Returns Ok(()) immediately without blocking.
    pub fn receive_incoming_message(&self, input: IncomingMessage) -> std::result::Result<(), String> {
        self.incoming_message_tx
            .try_send(input)
            .map_err(|e| format!("Failed to queue supervisor input: {}", e))?;
        // FIX-6: Increment pending counter after successful send
        self.incoming_message_pending.fetch_add(1, Ordering::Release);
        Ok(())
    }

    /// Get the model identifier for this session (AMGR-009)
    pub fn get_model_id(&self) -> Option<String> {
        self.model_id.read().expect("model_id lock poisoned").clone()
    }

    /// Get the count of pending incoming messages (AMGR-009)
    ///
    /// Returns the number of messages waiting in the incoming_message channel.
    /// FIX-6: Uses an AtomicUsize counter that is incremented on send and
    /// decremented on receive in the agent loop.
    pub fn pending_incoming_message_count(&self) -> usize {
        self.incoming_message_pending.load(Ordering::Acquire)
    }

    /// Send input to the agent loop
    ///
    /// Buffers the user input as a UserInput chunk before sending to the agent,
    /// so it can be replayed when attaching to a detached session via /resume.
    ///
    /// CRITICAL: Sets status to Running BEFORE sending to channel to avoid race condition.
    /// The TypeScript side calls refreshRustState() right after sessionSendInput() returns,
    /// so status must be Running at that point for isLoading to be true.
    pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<(), String> {
        // TUI-049: Clear pending input - it's being sent now (state invariant)
        // This prevents "ghost input" from reappearing when switching sessions after send
        self.set_pending_input(None);

        // Buffer user input for resume/attach (NAPI-009)
        self.handle_output(StreamChunk::user_input(input.clone()));

        // NAPI-009: Set status to Running BEFORE sending to channel.
        // This ensures sessionGetStatus() returns "running" when called immediately after
        // sessionSendInput(), allowing the UI to show loading state without race conditions.
        // The agent_loop will also set this (idempotent), and will set back to Idle when done.
        self.set_status(SessionStatus::Running);
        self.reset_interrupt();

        self.input_tx
            .try_send(PromptInput { input, thinking_config })
            .map_err(|e| {
                // If send fails, revert status to Idle since no processing will occur
                self.set_status(SessionStatus::Idle);
                // The String error is mapped back into napi::Error::from_reason
                // at the wire boundary by the napi free function
                // session_send_input. The internal type is `Result<(), String>`.
                format!("Failed to send input: {}", e)
            })
    }
    
    /// Interrupt current agent execution
    ///
    /// Call this when the user presses Esc in the TUI.
    /// Also requests bash tool abortion for any running commands.
    pub fn interrupt(&self) {
        self.is_interrupted.store(true, Ordering::Release);
        // Also request bash tool abortion for any running commands
        request_bash_abort(self.id);
        self.interrupt_notify.notify_one();
    }

    /// Reset interrupt flag
    ///
    /// Called automatically at the start of each prompt.
    pub fn reset_interrupt(&self) {
        self.is_interrupted.store(false, Ordering::Release);
        // Also clear bash abort flag
        clear_bash_abort(self.id);
    }

    /// Get a clone of the interrupt notify handle (AMGR-015)
    ///
    /// Used by `await_idle` to cancel waiting when the calling session
    /// is interrupted (Esc).
    pub fn get_interrupt_notify(&self) -> Arc<Notify> {
        self.interrupt_notify.clone()
    }
    
    /// TUI-065: Clear session history and reinject context reminders
    ///
    /// This method clears the session's messages, turns, and token tracker,
    /// then reinjects the context reminders (CLAUDE.md, environment info) so
    /// the AI retains project context after clearing.
    ///
    /// DRY: This is the single source of truth for clear functionality.
    /// Both TUI /clear command (via NAPI) and Telegram bridge /clear use this.
    ///
    /// Note: Caller is responsible for wrapping in `tokio::task::block_in_place()`
    /// if calling from an async context (like the bridge control handler).
    pub fn clear_history(&self) {
        // Clear the output buffer (conversation history display)
        if let Ok(mut buffer) = self.output_buffer.write() {
            buffer.clear();
        }
        
        // Clear actual session state (messages, turns, tokens)
        let mut inner = self.inner.blocking_lock();
        inner.messages.clear();
        inner.turns.clear();
        inner.token_tracker = codelet_core::compaction::TokenTracker::default();
        
        // CRITICAL: Reinject context reminders so AI retains project context
        // Without this, the AI loses CLAUDE.md and environment info
        // GIT-034: Include isolation context so AI knows about worktree
        let isolation = self.build_isolation_context();
        inner.inject_context_reminders_with_isolation(isolation.as_ref());
        drop(inner);
        
        // Reset the interrupt flag
        self.reset_interrupt();
        
        // TUI-066: Emit chunk so React updates state as side effect
        self.handle_output(StreamChunk::session_state_change(SessionState::Cleared));
    }
    
    /// Get session info for listing
    pub fn get_info(&self) -> SessionInfo {
        // Get message count from output buffer (each turn produces multiple chunks,
        // but Done chunks mark the end of a turn response)
        let message_count = self
            .output_buffer
            .read()
            .expect("output buffer lock poisoned")
            .iter()
            .filter(|c| matches!(c, StreamChunk::Done))
            .count() as u32;

        SessionInfo {
            id: self.id.to_string(),
            name: self.name.read().expect("name lock poisoned").clone(),
            status: self.get_status().as_str().to_string(),
            project: self.project.clone(),
            message_count,
            provider_id: self.provider_id.read().expect("provider_id lock poisoned").clone(),
            model_id: self.model_id.read().expect("model_id lock poisoned").clone(),
            // GIT-029: Isolation state
            is_isolated: self.worktree_path.is_some(),
            worktree_path: self.worktree_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            // RPC-007: role surface for the session manager handle. NAPI
            // sessions don't currently track a role at construction; emit
            // None so the lifted shape is satisfied without changing TS
            // behaviour.
            role: None,
        }
    }
}
