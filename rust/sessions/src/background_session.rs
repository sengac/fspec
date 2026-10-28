//! Background session that runs the agent loop in a tokio task.
//!
//! Moved here by **RPC-039** from `rust/napi/src/session_manager.rs`
//! (former lines 459–1356). The agent loop (`SessionManager::run_session_loop`),
//! `ChainOfCommand`, and the `#[napi]` free functions stay napi-side for
//! this card — they are populated into `codelet-sessions` by RPC-040.
//!
//! ## What changed compared to the napi-side original
//!
//! * `napi::bindgen_prelude::Result<()>` on
//!   [`BackgroundSession::send_input`] became `Result<(), String>`. The
//!   napi free function `session_send_input` (in
//!   `rust/napi/src/session_manager.rs`) maps the new string error
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
#![allow(dead_code)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::uninlined_format_args)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::{broadcast, mpsc, Mutex, Notify};
use uuid::Uuid;

use codelet_cli::session::context_gathering::gather_environment_info;
use codelet_common::debug_capture::{DebugCaptureManager, PoisonRecoveryMutex};
use codelet_core::lifecycle_hooks::{CompiledLifecycleHooks, HookContext};
use codelet_git::ghost_commit::{
    create_ghost_commit, list_ghost_checkpoints, restore_ghost_commit, GhostCheckpoint,
    RestoreResult,
};
use codelet_git::GitError;
use codelet_rpc_types::{FspecResult, SessionInfo, SessionState, SessionStatus, StreamChunk};
use codelet_tools::tool_pause::{PauseResponse, PauseState};
use codelet_tools::{clear_bash_abort, request_bash_abort};

mod agent_message;
mod compaction_handoff;
mod completion_contract;
mod construction;
mod exec_stdin;
mod focus;
mod history;
mod interactions;
mod interruption;
mod messaging;
mod metrics;
mod output;
mod progress;
mod transcript;
mod types;
mod workspace;

pub use agent_message::AgentMessageError;
pub use types::{
    format_incoming_message, BridgeImageData, CompactionProgress, IncomingMessage, PromptInput,
    SessionError, WorkUnitContext, SUPERVISOR_BROADCAST_CAPACITY,
};

pub struct BackgroundSession {
    pub(crate) provider_status:
        std::sync::Mutex<Option<(String, codelet_providers::session::ProviderSession)>>,
    pub id: Uuid,
    pub name: RwLock<String>,
    pub project: String,
    pub provider_id: RwLock<Option<String>>,
    pub model_id: RwLock<Option<String>>,
    pub cached_input_tokens: AtomicU32,
    pub cached_output_tokens: AtomicU32,
    pub cached_reasoning_tokens: AtomicU32,
    pub(crate) committed_context: std::sync::Mutex<crate::committed_context::ContextProjection>,
    pub cached_context_window: AtomicU32,
    pub cached_max_output_tokens: AtomicU32,
    pub cached_compaction_threshold: AtomicU32,
    pub inner: Arc<Mutex<codelet_cli::session::Session>>,
    settlement_gate: Arc<Mutex<()>>,
    status: AtomicU8,
    input_tx: mpsc::Sender<PromptInput>,
    output_buffer: RwLock<crate::transcript_journal::TranscriptJournal>,
    transcript_seed: std::sync::Mutex<()>,
    manifest_restore: AtomicBool,
    relay_output: crate::subordinate_output::RelayOutput,
    pub is_interrupted: Arc<AtomicBool>,
    interrupt_epoch: AtomicU64,
    interruption_reason: std::sync::Mutex<Option<codelet_rpc_types::InterruptionReason>>,
    pub interrupt_notify: Arc<Notify>,
    is_debug_enabled: AtomicBool,
    pub debug_capture: Arc<PoisonRecoveryMutex<DebugCaptureManager>>,
    pending_input: RwLock<Option<String>>,
    pub supervisor_broadcast: broadcast::Sender<StreamChunk>,
    chunks_tx: broadcast::Sender<(codelet_rpc_types::SessionId, StreamChunk)>,
    status_changes_tx: broadcast::Sender<(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::SessionStatus,
    )>,
    session_cleared_tx: broadcast::Sender<codelet_rpc_types::SessionId>,
    role: RwLock<Option<String>>,
    incoming_message_tx: mpsc::Sender<IncomingMessage>,
    pub incoming_message_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
    pub incoming_message_pending: Arc<AtomicUsize>,
    correlation_counter: AtomicU64,
    pending_observed_correlation_ids: RwLock<Vec<String>>,
    pause_state: crate::interaction_slot::InteractionSlot<PauseState>,
    pause_response_tx: std::sync::mpsc::Sender<PauseResponse>,
    pause_response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<PauseResponse>>,
    fspec_response_tx: std::sync::mpsc::Sender<FspecResult>,
    fspec_response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<FspecResult>>,
    hitl_response_tx: std::sync::mpsc::Sender<codelet_tools::request_user_input::HitlResponse>,
    hitl_response_rx: std::sync::Mutex<
        std::sync::mpsc::Receiver<codelet_tools::request_user_input::HitlResponse>,
    >,
    hitl_request:
        crate::interaction_slot::InteractionSlot<codelet_tools::request_user_input::HitlRequest>,
    exec_stdin_request: RwLock<Option<codelet_tools::unified_exec::ExecStdinRequest>>,
    base_thinking_level: AtomicU8,
    continue_enabled: AtomicBool,
    continue_budget: AtomicU32,
    continue_explicit: AtomicBool,
    goal_state: std::sync::Mutex<Option<(String, Option<String>)>>,
    goal_generation: AtomicU64,
    goal_synced_generation: AtomicU64,
    compaction_progress: RwLock<Option<CompactionProgress>>,
    pub(crate) work_unit_context: RwLock<Option<WorkUnitContext>>,
    pub(crate) work_unit_focus: Arc<std::sync::Mutex<codelet_rpc_types::WorkUnitFocus>>,
    pub(crate) focus_reminder_revision: Arc<AtomicU64>,
    pub(crate) focus_admission: std::sync::Mutex<u64>,
    pub(crate) focus_runtime: tokio::runtime::Handle,
    base_environment_content: RwLock<String>,
    pub worktree_path: RwLock<Option<PathBuf>>,
    pub base_commit: RwLock<Option<String>>,
    pub compaction_in_progress: Arc<AtomicBool>,
    pub pending_dag_content: Arc<std::sync::Mutex<Option<String>>>,
    pub pre_compaction_tokens: AtomicU32,
    pub schedule_triggered: AtomicBool,
    pub schedule_name: RwLock<Option<String>>,
    pub lifecycle_hooks: Option<Arc<CompiledLifecycleHooks>>,
    owning_manager: RwLock<std::sync::Weak<crate::session_manager::SessionManager>>,
    pending_loop_abort_note: std::sync::Mutex<Option<String>>,
}
