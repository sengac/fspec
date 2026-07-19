//! Session manager that owns multiple background sessions.
//!
//! Moved here by **RPC-040** from `codelet/napi/src/session_manager.rs`
//! (former lines 2135-3013). All `#[napi]` attributes were stripped and
//! the `napi::Result<T>` return types became `Result<T, String>`. The
//! handful of NAPI-private dependencies that the original methods
//! reached for (agent_loop spawning, the scheduler engine, the footer
//! poller free functions, and the `GLOBAL_CHUNK_CALLBACK` global) are
//! now injected through the new [`SessionManagerHooks`] trait so
//! `codelet-sessions` has zero transitive dependency on `codelet-napi`.
//!
//! ## Hooks design
//!
//! Six things stay napi-side:
//!
//! * `agent_loop` free function (still lives at
//!   `codelet/napi/src/session_manager.rs`)
//! * The scheduler engine (`crate::scheduler::spawn_scheduler` and
//!   `crate::scheduler::LoopStore`)
//! * `spawn_footer_poller` / `stop_footer_poller`
//! * `GLOBAL_CHUNK_CALLBACK`
//!
//! Each of these is reachable from `SessionManager` via a method on the
//! [`SessionManagerHooks`] trait object stored as
//! `hooks: ArcSwap<Arc<dyn SessionManagerHooks>>`. A
//! [`NoopSessionManagerHooks`] default impl makes
//! `SessionManager::new()` work out-of-the-box for the future
//! `fspec` binary (RPC-044); the napi side replaces it at startup with
//! a `NapiSessionManagerHooks` that delegates to the existing napi
//! helpers.
//!
//! ## RPC-041 deferral
//!
//! The new broadcast-sender fields (`chunks_tx`, `logs_tx`,
//! `status_changes_tx`) ARE wired up here but
//! `BackgroundSession::handle_output` is NOT yet plumbed to use the
//! manager-owned senders — that is explicitly RPC-041's deliverable.
//! For the duration of RPC-040 the napi side keeps driving
//! `GLOBAL_CHUNK_CALLBACK` via [`SessionManagerHooks::emit_isolation_state_change`].

#![allow(clippy::expect_used)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(dead_code)]

use std::sync::{Arc, OnceLock, RwLock, Weak};

use arc_swap::ArcSwap;
use indexmap::IndexMap;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use codelet_core::lifecycle_hooks::{load_lifecycle_hooks, run_pre_tool};
use codelet_rpc_types::{LogRecord, SessionInfo, SessionStatus, StreamChunk};
use codelet_tools::pre_tool_hook::{
    register_pre_tool_hook, unregister_pre_tool_hook, PreToolHookDecision, PreToolHookHandler,
};
use codelet_tools::McpInjection;

use crate::background_session::{BackgroundSession, PromptInput, SUPERVISOR_BROADCAST_CAPACITY};
use crate::session_creation_helper::{
    create_background_session_inner, ParsedModelInfo, SessionCreationParams,
};

/// Maximum concurrent sessions.
pub const MAX_SESSIONS: usize = 10;

/// Session ID type alias — the wire-portable `codelet_rpc_types::SessionId`
/// is used everywhere the SessionManager hands an id across the
/// broadcast or hook boundary so that cross-crate subscribers (napi
/// adapter, future Rust `fspec-tui`) see one consistent type.
pub type SessionId = codelet_rpc_types::SessionId;

/// Hook surface that the napi side installs at startup to keep existing
/// TS behaviour intact. The `fspec` binary in RPC-044 leaves the
/// default [`NoopSessionManagerHooks`] in place.
pub trait SessionManagerHooks: Send + Sync + 'static {
    /// Spawn the agent loop for a newly-created session. The default
    /// no-op impl does nothing; the napi side spawns
    /// `tokio::spawn(async move { agent_loop(session, input_rx, mcp_injection_rx).await })`.
    fn spawn_agent_loop(
        &self,
        session: Arc<BackgroundSession>,
        input_rx: mpsc::Receiver<PromptInput>,
        mcp_injection_rx: mpsc::Receiver<McpInjection>,
    );

    /// Start the scheduler for a project if it has not already been
    /// started for this manager.
    fn spawn_scheduler(&self, project: String, rt: tokio::runtime::Handle);

    /// Ensure the scheduler is running for `/loop` support.
    fn ensure_scheduler_running_for_loop(&self, project: String, rt: tokio::runtime::Handle);

    /// Spawn the per-session footer poller.
    fn spawn_footer_poller(&self, session_id: String, cwd: String, worktree_path: Option<String>);

    /// Stop the per-session footer poller.
    fn stop_footer_poller(&self, session_id: &str);

    /// Clean up scheduler loops on session destroy.
    fn cleanup_session_loops(&self, session_id: Uuid);
}

/// Default no-op implementation used by the `fspec` binary in RPC-044
/// where every NAPI subsystem is absent.
#[derive(Default)]
pub struct NoopSessionManagerHooks;

impl SessionManagerHooks for NoopSessionManagerHooks {
    fn spawn_agent_loop(
        &self,
        _session: Arc<BackgroundSession>,
        _input_rx: mpsc::Receiver<PromptInput>,
        _mcp_injection_rx: mpsc::Receiver<McpInjection>,
    ) {
    }

    fn spawn_scheduler(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn ensure_scheduler_running_for_loop(&self, _project: String, _rt: tokio::runtime::Handle) {}

    fn spawn_footer_poller(
        &self,
        _session_id: String,
        _cwd: String,
        _worktree_path: Option<String>,
    ) {
    }

    fn stop_footer_poller(&self, _session_id: &str) {}

    fn cleanup_session_loops(&self, _session_id: Uuid) {}
}

/// Background session manager.
///
/// Owns multiple [`BackgroundSession`] instances keyed by `Uuid`.
/// Implements RPC-040's two new responsibilities:
///
/// 1. Three new broadcast senders (`chunks_tx`, `logs_tx`,
///    `status_changes_tx`) that RPC-041 will plug into
///    `BackgroundSession::handle_output`.
/// 2. A pluggable [`SessionManagerHooks`] trait object so the napi
///    side keeps spawning the agent loop, the scheduler, the footer
///    poller, and the IsolationStateChange chunk fan-out unchanged.
pub struct SessionManager {
    sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,
    /// Tracks subordinate-supervisor relationships between sessions (WATCH-002)
    chain_of_command: crate::chain_of_command::ChainOfCommand,
    /// Tracks the currently active (attached) session for navigation (VIEWNV-001)
    active_session_id: RwLock<Option<Uuid>>,
    /// SCHED-003: Scheduler task handle for graceful shutdown
    scheduler_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
    /// SCHED-004: Default model string for scheduled session spawning
    default_model: RwLock<Option<String>>,
    /// RPC-040: per-manager broadcast of session chunk events.
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    /// RPC-040: per-manager broadcast of log records.
    logs_tx: broadcast::Sender<LogRecord>,
    /// RPC-040: per-manager broadcast of session status changes.
    status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
    /// RPC-385: per-manager broadcast of session-created events. Fires
    /// whenever any session is created (TUI-initiated, scheduled, or a
    /// spawned subordinate via AgentManager) so the embedded Rust TUI can
    /// append a tab for sessions it did not itself initiate. The payload is
    /// the new session's [`SessionInfo`] (its `.id` carries the SessionId).
    session_created_tx: broadcast::Sender<SessionInfo>,
    /// RPC-040: NAPI-side subsystems injected via the hooks trait.
    hooks: ArcSwap<Arc<dyn SessionManagerHooks>>,
    /// RPC-386: Weak self-reference, populated via [`SessionManager::init_self_weak`]
    /// / [`SessionManager::new_arc`] once the manager is wrapped in an `Arc`. Used
    /// to stamp each created session's owning-manager back-reference so the
    /// AgentManager handler binds to this manager instead of the global
    /// singleton. The singleton (`instance()`) never sets this, so its sessions
    /// carry an empty back-reference and fall back to `instance()`.
    self_weak: OnceLock<Weak<SessionManager>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Create a new session manager with default no-op hooks.
    pub fn new() -> Self {
        let (chunks_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let (logs_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let (status_changes_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        // RPC-385: session-creation events use the supervisor broadcast
        // capacity because subscribers must tolerate bursts (e.g. the
        // lag-recovery path can flood many events at once). A lagged receiver
        // recovers via RecvError::Lagged rather than losing correctness: the
        // TUI's append is idempotent, so a dropped/replayed event never
        // produces a duplicate or missing tab.
        let (session_created_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let default_hooks: Arc<dyn SessionManagerHooks> = Arc::new(NoopSessionManagerHooks);
        Self {
            sessions: RwLock::new(IndexMap::new()),
            chain_of_command: crate::chain_of_command::ChainOfCommand::new(),
            active_session_id: RwLock::new(None),
            scheduler_handle: RwLock::new(None),
            // PROV-119: pre-populate the default model from disk so a fresh
            // process starts with the user's last selection (an uninitialized
            // data dir / missing file loads as None — graceful degradation).
            default_model: RwLock::new(crate::default_model_persistence::load_default_model()),
            chunks_tx,
            logs_tx,
            status_changes_tx,
            session_created_tx,
            hooks: ArcSwap::from_pointee(default_hooks),
            self_weak: OnceLock::new(),
        }
    }

    /// RPC-386: Construct a `SessionManager` already wrapped in an `Arc` with its
    /// self-weak populated, so created sessions can carry an owning-manager
    /// back-reference. Prefer this (or [`SessionManager::init_self_weak`]) over
    /// `Arc::new(SessionManager::new())` for daemon-owned managers.
    pub fn new_arc() -> Arc<Self> {
        let manager = Arc::new(Self::new());
        manager.init_self_weak();
        manager
    }

    /// RPC-386: Populate the self-weak back-reference. Idempotent — only the
    /// first call wins (subsequent calls are ignored). Safe to call on an
    /// already-`Arc`-wrapped manager built via `Arc::new(SessionManager::new())`.
    pub fn init_self_weak(self: &Arc<Self>) {
        let _ = self.self_weak.set(Arc::downgrade(self));
    }

    /// Replace the [`SessionManagerHooks`] implementation. Called by
    /// the napi side at startup to install `NapiSessionManagerHooks`.
    pub fn set_hooks(&self, hooks: Arc<dyn SessionManagerHooks>) {
        self.hooks.store(Arc::new(hooks));
    }

    fn hooks(&self) -> Arc<dyn SessionManagerHooks> {
        let guard = self.hooks.load();
        (**guard).clone()
    }

    /// Accessor for the chunks broadcast sender.
    pub fn chunks_tx(&self) -> &broadcast::Sender<(SessionId, StreamChunk)> {
        &self.chunks_tx
    }

    /// Accessor for the logs broadcast sender.
    pub fn logs_tx(&self) -> &broadcast::Sender<LogRecord> {
        &self.logs_tx
    }

    /// Accessor for the status-changes broadcast sender.
    pub fn status_changes_tx(&self) -> &broadcast::Sender<(SessionId, SessionStatus)> {
        &self.status_changes_tx
    }

    /// RPC-385: accessor for the session-created broadcast sender. The
    /// embedded TUI transport subscribes to this so spawned subordinate
    /// sessions (created outside the TUI) become visible as tabs.
    pub fn session_created_tx(&self) -> &broadcast::Sender<SessionInfo> {
        &self.session_created_tx
    }

    /// SCHED-004: Set the default model for scheduled session spawning.
    /// PROV-119: also persists the non-empty choice to disk
    /// (`<data_dir>/default-model.json`) so it survives a process restart. The
    /// write is best-effort: a failure is logged and never propagates, so
    /// session creation is never blocked by a persistence error.
    /// PROV-122: additionally persist the canonical `tui.lastUsedModel` in
    /// `fspec-config.json` alongside the legacy store (kept for back-compat) so
    /// the no-session selection path produces the key PROV-120's read path
    /// prefers. Both writes are best-effort and non-fatal.
    pub fn set_default_model(&self, model: &str) {
        if !model.is_empty() {
            *self
                .default_model
                .write()
                .expect("default_model lock poisoned") = Some(model.to_string());
            // PROV-119: persist across restarts; best-effort and non-fatal.
            if let Err(e) = crate::default_model_persistence::save_default_model(model) {
                tracing::warn!(
                    error = %e,
                    model,
                    "set_default_model: failed to persist default model (non-fatal)"
                );
            }
            // PROV-122: also write the canonical fspec-config.json
            // tui.lastUsedModel; best-effort and non-fatal.
            if let Err(e) = crate::last_used_model_persistence::save_persisted_model_string(model) {
                tracing::warn!(
                    error = %e,
                    model,
                    "set_default_model: failed to persist lastUsedModel (non-fatal)"
                );
            }
        }
    }

    /// SCHED-004: Get the default model string for scheduled session spawning.
    pub fn get_default_model(&self) -> Option<String> {
        self.default_model
            .read()
            .expect("default_model lock poisoned")
            .clone()
    }

    /// SCHED-006: Get the current number of sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().expect("sessions lock poisoned").len()
    }

    /// SCHED-006: Get all live session IDs for sweep detection.
    pub async fn live_session_ids(&self) -> Vec<Uuid> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .keys()
            .copied()
            .collect()
    }

    /// SCHED-006: Find a session by its schedule_name metadata.
    pub async fn find_session_by_schedule_name(&self, schedule_name: &str) -> Option<Uuid> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        for (id, session) in sessions.iter() {
            let name = session.schedule_name.read().expect("schedule_name lock");
            if name.as_deref() == Some(schedule_name) {
                return Some(*id);
            }
        }
        None
    }

    /// Get singleton instance
    pub fn instance() -> &'static SessionManager {
        static INSTANCE: OnceLock<SessionManager> = OnceLock::new();
        INSTANCE.get_or_init(SessionManager::new)
    }

    /// Create a new background session (generates new UUID)
    pub async fn create_session(&self, model: &str, project: &str) -> Result<String, String> {
        let id = Uuid::new_v4();
        self.create_session_with_id(
            &id.to_string(),
            model,
            project,
            &format!("Session {}", &id.to_string()[..8]),
        )
        .await?;
        Ok(id.to_string())
    }

    /// List sessions, filtering persisted sessions by `project_path`.
    ///
    /// RPC-427: Added `project_path` parameter so `/resume` only shows sessions
    /// belonging to the current project. In-memory sessions are inherently
    /// project-scoped (created with `std::env::current_dir()`), so only
    /// persisted sessions require explicit filtering.
    pub fn list_sessions(&self, project_path: &str) -> Vec<SessionInfo> {
        let in_memory: Vec<SessionInfo> = self
            .sessions
            .read()
            .expect("sessions lock poisoned")
            .values()
            .map(|s| s.get_info())
            .collect();

        tracing::debug!(
            in_memory_count = in_memory.len(),
            "list_sessions: collected in-memory sessions"
        );

        // RPC-427: Filter persisted sessions by project path instead of loading all.
        let persisted: Vec<SessionInfo> =
            match codelet_core::persistence::list_sessions_for_project(
                std::path::Path::new(project_path),
            ) {
            Ok(manifests) => {
                tracing::info!(
                    persisted_on_disk = manifests.len(),
                    "list_sessions: loaded persisted session manifests from disk"
                );
                let in_memory_ids: std::collections::HashSet<String> = in_memory
                    .iter()
                    .map(|s| s.id.clone())
                    .collect();
                let result: Vec<SessionInfo> = manifests
                    .into_iter()
                    .filter(|m| !in_memory_ids.contains(&m.id.to_string()))
                    .map(|m| SessionInfo {
                        id: m.id.to_string(),
                        name: m.name,
                        status: "idle".to_string(),
                        project: m.project.to_string_lossy().to_string(),
                        message_count: m.messages.len() as u32,
                        provider_id: if m.provider.is_empty() {
                            None
                        } else {
                            Some(m.provider.split('/').next().unwrap_or(&m.provider).to_string())
                        },
                        model_id: if m.provider.is_empty() {
                            None
                        } else {
                            m.provider.split('/').nth(1).map(|s| s.to_string())
                        },
                        is_isolated: false,
                        worktree_path: None,
                        role: None,
                        updated_at_ms: Some(m.updated_at.timestamp_millis()),
                    })
                    .collect();
                tracing::info!(
                    persisted_new = result.len(),
                    "list_sessions: merged persisted sessions not in memory"
                );
                result
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "list_sessions: failed to load persisted sessions from disk"
                );
                Vec::new()
            }
        };

        let mut all = in_memory;
        all.extend(persisted);

        // TUI-099: Sort by updated_at_ms descending (most recent first),
        // with session ID as alphabetical tiebreaker. Sessions without
        // a timestamp (None) appear at the end.
        all.sort_by(|a, b| {
            match (a.updated_at_ms, b.updated_at_ms) {
                (Some(ts_a), Some(ts_b)) => {
                    ts_b.cmp(&ts_a).then_with(|| a.id.cmp(&b.id))
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.id.cmp(&b.id),
            }
        });

        tracing::debug!(
            total_count = all.len(),
            "list_sessions: returning merged session list"
        );

        // DEBUG: Log the IDs of all sessions returned so callers can trace
        // which sessions are included in the merge result.
        let ids: Vec<String> = all.iter().map(|s| s.id.clone()).collect();
        tracing::debug!(
            session_ids = ?ids,
            "list_sessions: session IDs in merged result"
        );

        all
    }

    /// VIEWNV-001: Set the active (currently viewed) session
    pub fn set_active_session(&self, id: Uuid) {
        *self
            .active_session_id
            .write()
            .expect("active_session lock poisoned") = Some(id);
    }

    /// VIEWNV-001: Clear the active session (when returning to board)
    pub fn clear_active_session(&self) {
        *self
            .active_session_id
            .write()
            .expect("active_session lock poisoned") = None;
    }

    /// VIEWNV-001: Get the active session ID
    pub fn get_active_session(&self) -> Option<Uuid> {
        *self
            .active_session_id
            .read()
            .expect("active_session lock poisoned")
    }

    /// SCHED-003: Start the scheduler if not already running.
    fn maybe_start_scheduler(&self, project: &str) {
        let mut handle = self
            .scheduler_handle
            .write()
            .expect("scheduler lock poisoned");
        if handle.is_some() {
            return;
        }
        let schedules_path = std::path::Path::new(project).join("spec/schedules.json");
        if schedules_path.exists() {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                tracing::info!("Starting scheduler for project: {}", project);
                self.hooks().spawn_scheduler(project.to_string(), rt);
                *handle = Some(tokio::spawn(async {}));
            } else {
                tracing::warn!("No Tokio runtime available for scheduler");
            }
        }
    }

    /// SCHED-011: Ensure the scheduler is running (for /loop support).
    pub fn ensure_scheduler_running(&self, project: &str, rt: &tokio::runtime::Handle) {
        let mut handle = self
            .scheduler_handle
            .write()
            .expect("scheduler lock poisoned");
        if handle.is_some() {
            return;
        }
        tracing::info!("Starting scheduler for /loop support: {}", project);
        self.hooks()
            .ensure_scheduler_running_for_loop(project.to_string(), rt.clone());
        *handle = Some(tokio::spawn(async {}));
    }

    /// Navigation: next session
    pub fn get_next_session(&self) -> Option<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let active = self
            .active_session_id
            .read()
            .expect("active_session lock poisoned");
        let nav_list = crate::navigation::build_navigation_list(&sessions, &self.chain_of_command);
        match crate::navigation::get_next_target(&nav_list, *active) {
            crate::navigation::NavigationTarget::Session(id) => Some(id.to_string()),
            _ => None,
        }
    }

    /// Navigation: previous session
    pub fn get_prev_session(&self) -> Option<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let active = self
            .active_session_id
            .read()
            .expect("active_session lock poisoned");
        let nav_list = crate::navigation::build_navigation_list(&sessions, &self.chain_of_command);
        match crate::navigation::get_prev_target(&nav_list, *active) {
            crate::navigation::NavigationTarget::Session(id) => Some(id.to_string()),
            _ => None,
        }
    }

    /// Navigation: first session
    pub fn get_first_session(&self) -> Option<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let nav_list = crate::navigation::build_navigation_list(&sessions, &self.chain_of_command);
        nav_list.first().map(|id| id.to_string())
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Arc<BackgroundSession>, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&uuid)
            .cloned()
            .ok_or_else(|| format!("Session not found: {}", id))
    }

    /// Create a background session with a specific ID (for persistence integration).
    pub async fn create_session_with_id(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;

        tracing::info!(
            session_id = %uuid,
            model = %model,
            project = %project,
            name = %name,
            "create_session_with_id: starting session creation"
        );

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            tracing::debug!(
                session_count = sessions.len(),
                max_sessions = MAX_SESSIONS,
                "create_session_with_id: checking session limits"
            );
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                drop(sessions);
                tracing::info!(
                    session_id = %uuid,
                    "create_session_with_id: session already exists in memory, reactivating"
                );
                self.set_active_session(uuid);
                return Ok(());
            }
        }

        let project_path = std::path::PathBuf::from(project);
        let mut manifest = codelet_core::persistence::SessionManifest::with_provider(
            name,
            project_path.clone(),
            model,
        );
        manifest.id = uuid;
        tracing::info!(
            session_id = %uuid,
            manifest_provider = %manifest.provider,
            manifest_name = %manifest.name,
            manifest_project = %manifest.project.display(),
            "create_session_with_id: constructed manifest, about to persist to disk"
        );
        codelet_core::persistence::save_session(&manifest)
            .map_err(|e| format!("Failed to persist session manifest: {}", e))?;
        tracing::info!(
            session_id = %uuid,
            "create_session_with_id: manifest persisted to disk successfully"
        );

        // RPC-424: Parse model string using shared helper
        let parsed = crate::model_parsing::parse_model_string(model)?;
        let registry_provider = parsed.registry_provider;
        let model_part = parsed.model_part;
        let is_profile_model = parsed.is_profile_model;
        let is_codex_model = parsed.is_codex_model;
        let is_custom_model = parsed.is_custom_model;

        let (provider_id, model_id) = (
            Some(registry_provider.to_string()),
            Some(model_part.to_string()),
        );

        // Resolve credentials internally using the lifted credentials module.
        let project_path = std::path::PathBuf::from(project);
        if let Err(e) = crate::credentials::resolve_and_set_env_var(
            registry_provider,
            Some(project_path.as_path()),
        ) {
            tracing::error!(
                "Failed to resolve credentials for provider {}: {}",
                registry_provider,
                e
            );
        }

        // RPC-425: Use shared session creation helper
        let parsed_model = ParsedModelInfo {
            model,
            registry_provider,
            is_profile_model,
            is_codex_model,
            is_custom_model,
        };
        let params = SessionCreationParams {
            uuid,
            name,
            project,
            project_path: project_path.as_path(),
            parsed_model,
            provider_id,
            model_id,
            worktree_path: None,
            base_commit: None,
            isolation: None,
            chunks_tx: self.chunks_tx.clone(),
            status_changes_tx: self.status_changes_tx.clone(),
        };

        let provider_manager = codelet_providers::ProviderManager::with_model_support()
            .await
            .map_err(|e| format!("Failed to create provider manager: {}", e))?;

        let result = create_background_session_inner(params, provider_manager).await?;
        let session = result.session;
        let input_rx = result.input_rx;
        let mcp_injection_rx = result.mcp_injection_rx;

        // BUG-154: stamp the owning-manager back-reference BEFORE spawning the
        // agent loop, so the AgentManager handler the loop registers binds to
        // THIS manager instead of the global singleton. Mirrors the call in
        // create_session_from_manifest (line 840) and
        // create_isolated_session_with_id (line 1050).
        session.set_owning_manager(self.self_weak.get().cloned().unwrap_or_default());

        // RPC-040: agent_loop spawning is delegated to the hooks impl so
        // codelet-sessions has no transitive napi dependency.
        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        // RPC-385: capture the SessionInfo BEFORE the insert moves `session`
        // into the map, so the session-created broadcast can carry it.
        let created_info = session.get_info();

        tracing::info!(
            session_id = %uuid,
            "create_session_with_id: inserting session into in-memory map"
        );
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        // RPC-385: fire the session-created broadcast next to the existing
        // metadata-update fan-out so the embedded TUI can append a tab for
        // sessions it did not itself initiate (spawned subordinates). A send
        // error (no subscribers) is benign and ignored.
        let _ = self.session_created_tx.send(created_info);
        tracing::debug!(
            session_id = %uuid,
            "create_session_with_id: session-created broadcast sent"
        );

        self.set_default_model(model);
        self.set_active_session(uuid);
        self.maybe_start_scheduler(project);

        // RPC-041: Emit IsolationStateChange directly on the manager-owned
        // chunks_tx (the previous `hooks().emit_isolation_state_change(...)`
        // delegation is gone — the hook has been removed from the trait).
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(id.to_string()),
            codelet_rpc_types::StreamChunk::isolation_state_change(false, None),
        ));

        // TUI-091: footer poller via the hooks.
        self.hooks()
            .spawn_footer_poller(id.to_string(), project.to_string(), None);

        codelet_tools::broadcast_metadata_update();

        tracing::info!(
            session_id = %uuid,
            model = %model,
            project = %project,
            name = %name,
            "create_session_with_id: session creation complete"
        );

        Ok(())
    }

    /// RPC-422: Create a BackgroundSession from an existing manifest WITHOUT
    /// persisting a blank manifest to disk.
    ///
    /// Used by `resume_session` to restore a session that was persisted on disk
    /// but is not currently in memory. This avoids the bug where
    /// `create_session_with_id` overwrites the manifest with 0 messages.
    pub async fn create_session_from_manifest(
        &self,
        manifest: &codelet_core::persistence::SessionManifest,
        model: &str,
    ) -> Result<(), String> {
        let uuid = manifest.id;

        tracing::info!(
            session_id = %uuid,
            model = %model,
            manifest_name = %manifest.name,
            manifest_message_count = manifest.messages.len(),
            "create_session_from_manifest: starting session creation from existing manifest"
        );

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                drop(sessions);
                tracing::info!(
                    session_id = %uuid,
                    "create_session_from_manifest: session already exists in memory, reactivating"
                );
                self.set_active_session(uuid);
                return Ok(());
            }
        }

        // RPC-422: DO NOT save a blank manifest to disk.
        // The manifest already exists on disk with the correct message references.
        // We only need to create the in-memory BackgroundSession.

        // RPC-424: Parse model string using shared helper
        let parsed = crate::model_parsing::parse_model_string(model)?;
        let registry_provider = parsed.registry_provider;
        let model_part = parsed.model_part;
        let is_profile_model = parsed.is_profile_model;
        let is_codex_model = parsed.is_codex_model;
        let is_custom_model = parsed.is_custom_model;

        let (provider_id, model_id) = (
            Some(registry_provider.to_string()),
            Some(model_part.to_string()),
        );

        // Resolve credentials internally using the lifted credentials module.
        let project_path = manifest.project.clone();
        if let Err(e) = crate::credentials::resolve_and_set_env_var(
            registry_provider,
            Some(project_path.as_path()),
        ) {
            tracing::error!(
                "Failed to resolve credentials for provider {}: {}",
                registry_provider,
                e
            );
        }

        let name = manifest.name.clone();
        let project = manifest.project.to_string_lossy().to_string();

        // RPC-425: Use shared session creation helper
        let parsed_model = ParsedModelInfo {
            model,
            registry_provider,
            is_profile_model,
            is_codex_model,
            is_custom_model,
        };
        let params = SessionCreationParams {
            uuid,
            name: &name,
            project: &project,
            project_path: project_path.as_path(),
            parsed_model,
            provider_id,
            model_id,
            worktree_path: None,
            base_commit: None,
            isolation: None,
            chunks_tx: self.chunks_tx.clone(),
            status_changes_tx: self.status_changes_tx.clone(),
        };

        let provider_manager = codelet_providers::ProviderManager::with_model_support()
            .await
            .map_err(|e| format!("Failed to create provider manager: {}", e))?;

        let result = create_background_session_inner(params, provider_manager).await?;
        let session = result.session;
        let input_rx = result.input_rx;
        let mcp_injection_rx = result.mcp_injection_rx;

        // RPC-386: stamp the owning-manager back-reference BEFORE spawning the
        // agent loop, so the AgentManager handler the loop registers binds to
        // THIS manager.
        session.set_owning_manager(self.self_weak.get().cloned().unwrap_or_default());

        // RPC-040: agent_loop spawning is delegated to the hooks impl
        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        // RPC-385: capture the SessionInfo BEFORE the insert
        let created_info = session.get_info();

        tracing::info!(
            session_id = %uuid,
            "create_session_from_manifest: inserting session into in-memory map"
        );
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        // RPC-385: fire the session-created broadcast
        let _ = self.session_created_tx.send(created_info);

        self.set_default_model(model);
        self.set_active_session(uuid);
        self.maybe_start_scheduler(&project);

        // RPC-041: Emit IsolationStateChange directly on the manager-owned chunks_tx
        let session_id_str = uuid.to_string();
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(session_id_str.clone()),
            codelet_rpc_types::StreamChunk::isolation_state_change(false, None),
        ));

        // TUI-091: footer poller via the hooks.
        self.hooks()
            .spawn_footer_poller(session_id_str, project.clone(), None);

        codelet_tools::broadcast_metadata_update();

        tracing::info!(
            session_id = %uuid,
            model = %model,
            project = %project,
            "create_session_from_manifest: session creation complete (manifest preserved on disk)"
        );

        Ok(())
    }

    /// GIT-028: Create an isolated session with a git worktree.
    pub async fn create_isolated_session_with_id(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
    ) -> Result<codelet_rpc_types::IsolatedSessionInfo, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                return Err(format!("Session {} already exists", id));
            }
        }

        let worktree_result = codelet_git::create_worktree(project, id)
            .map_err(|e| format!("Failed to create worktree: {}", e))?;

        let worktree_path = worktree_result.info.path.clone();
        let base_commit = worktree_result.base_commit.clone();

        codelet_git::create_session_manifest(
            id,
            project,
            Some(worktree_path.clone()),
            Some(base_commit.clone()),
        )
        .map_err(|e| format!("Failed to create session manifest: {}", e))?;

        let (input_tx, input_rx) = mpsc::channel::<PromptInput>(32);
        let _ = dotenvy::dotenv();

        // RPC-424: Parse model string using shared helper
        let parsed = crate::model_parsing::parse_model_string(model)?;
        let registry_provider = parsed.registry_provider;
        let model_part = parsed.model_part;
        let is_profile_model = parsed.is_profile_model;
        let is_codex_model = parsed.is_codex_model;

        let (provider_id, model_id) = (
            Some(registry_provider.to_string()),
            Some(model_part.to_string()),
        );

        let project_path = std::path::PathBuf::from(project);
        if let Err(e) = crate::credentials::resolve_and_set_env_var(
            registry_provider,
            Some(project_path.as_path()),
        ) {
            tracing::error!(
                "Failed to resolve credentials for provider {}: {}",
                registry_provider,
                e
            );
        }

        let provider_manager = if is_profile_model {
            tracing::info!(
                "PROV-007: Profile model detected, skipping registry validation for {}",
                model
            );
            // PROV-121: bridge the profile's stored baseUrl/apiKey into the
            // OPENAI_* env BEFORE constructing the provider manager, via the
            // SAME shared helper the resolver path uses so the two cannot
            // drift. `colon_idx`/`slash_idx` re-parse the profile name segment
            // (between ':' and '/').
            if let (Some(colon_idx), Some(slash_idx)) = (model.find(':'), model.find('/')) {
                let profile_name = &model[colon_idx + 1..slash_idx];
                if let Err(e) = crate::model_resolution::apply_profile_env_vars(
                    registry_provider,
                    profile_name,
                    model_part,
                ) {
                    tracing::warn!(
                        "PROV-121: apply_profile_env_vars failed for profile '{}': {}",
                        profile_name,
                        e
                    );
                }
            }
            codelet_providers::ProviderManager::with_provider_and_model(
                registry_provider,
                Some(model_part),
                None,
                None,
            )
            .map_err(|e| format!("Failed to create provider manager: {}", e))?
        } else if is_codex_model {
            tracing::info!(
                "PROV-018: Codex model detected, skipping registry validation for {}",
                model
            );
            codelet_providers::ProviderManager::with_provider_and_model(
                registry_provider,
                Some(model_part),
                None,
                None,
            )
            .map_err(|e| format!("Failed to create codex provider manager: {}", e))?
        } else {
            let mut pm = codelet_providers::ProviderManager::with_model_support()
                .await
                .map_err(|e| format!("Failed to create provider manager: {}", e))?;
            pm.select_model(model)
                .map_err(|e| format!("Failed to select model: {}", e))?;
            pm
        };

        let initial_context_window = provider_manager.context_window() as u32;
        let initial_max_output_tokens = provider_manager.max_output_tokens() as u32;

        let mut inner = codelet_cli::session::Session::from_provider_manager(provider_manager);

        let isolation = codelet_cli::session::context_gathering::IsolationContext {
            is_isolated: true,
            worktree_path: Some(
                worktree_path
                    .strip_prefix(&project_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| worktree_path.to_string_lossy().to_string()),
            ),
            base_commit: Some(base_commit.clone()),
        };

        inner.inject_context_reminders_with_isolation(Some(&isolation));

        let lifecycle_hooks =
            match load_lifecycle_hooks(Some(&project_path), dirs::home_dir().as_deref()) {
                Ok(Some(compiled)) => Some(Arc::new(compiled)),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(
                        "[HOOK-013] Failed to load lifecycle hooks for isolated session: {}",
                        e
                    );
                    None
                }
            };

        let session = Arc::new(BackgroundSession::new(
            uuid,
            name.to_string(),
            project.to_string(),
            provider_id,
            model_id,
            inner,
            input_tx,
            Some(worktree_path.clone()),
            Some(base_commit.clone()),
            lifecycle_hooks.clone(),
            self.chunks_tx.clone(),
            self.status_changes_tx.clone(),
        ));

        // RPC-386: stamp the owning-manager back-reference for isolated sessions
        // too (before spawn_agent_loop), so spawned isolated subordinates bind
        // their AgentManager handler to THIS manager rather than the singleton.
        session.set_owning_manager(self.self_weak.get().cloned().unwrap_or_default());

        // TUI-002: re-apply the persisted default thinking level to isolated
        // sessions too, so spawned/isolated sessions match the same idle badge
        // behaviour as the primary session-creation path.
        let isolated_thinking_level =
            crate::default_thinking_level_persistence::load_default_thinking_level();
        tracing::debug!(
            level = isolated_thinking_level as u8,
            "create_isolated_session: applying persisted default thinking level to isolated session"
        );
        session.set_base_thinking_level(isolated_thinking_level as u8);

        let isolated_model_id = session
            .model_id
            .read()
            .expect("model_id lock poisoned")
            .clone();
        let isolated_compaction_threshold =
            codelet_cli::compaction_threshold::resolve_compaction_threshold(
                initial_context_window as u64,
                initial_max_output_tokens as u64,
                isolated_model_id.as_deref(),
                None,
            ) as u32;
        session.set_model_limits(
            initial_context_window,
            initial_max_output_tokens,
            isolated_compaction_threshold,
        );

        if let Some(ref hooks) = lifecycle_hooks {
            if !hooks.pre_tool_use.is_empty() {
                let hooks_for_pre = hooks.clone();
                let session_for_pre = session.clone();
                let pre_handler: PreToolHookHandler = std::sync::Arc::new(
                    move |_sid, tool_name, tool_input| {
                        let ctx = session_for_pre.hook_context();
                        let hooks = hooks_for_pre.clone();
                        let name = tool_name.to_string();
                        let input = tool_input.clone();
                        let outcome = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current()
                                .block_on(run_pre_tool(&hooks, &ctx, &name, &input))
                        });
                        match outcome.decision {
                            codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Allow => {
                                PreToolHookDecision::Allow
                            }
                            codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Deny => {
                                PreToolHookDecision::Deny(
                                    outcome
                                        .reason
                                        .unwrap_or_else(|| "Denied by pre_tool_use hook".to_string()),
                                )
                            }
                            codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Continue => {
                                PreToolHookDecision::Continue
                            }
                            codelet_core::lifecycle_hooks::outcome::PreToolHookDecision::Ask => {
                                PreToolHookDecision::Continue
                            }
                        }
                    },
                );
                register_pre_tool_hook(uuid, pre_handler);
            }
        }

        let (mcp_injection_rx, _mcp_connections) = codelet_tools::init_mcp_session(uuid);

        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        // RPC-385: capture the SessionInfo BEFORE the insert moves `session`.
        let created_info = session.get_info();

        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        self.set_active_session(uuid);

        // RPC-385: fire the session-created broadcast for isolated/worktree
        // sessions too so spawned isolated subordinates are equally visible.
        let _ = self.session_created_tx.send(created_info);

        // RPC-041: Emit IsolationStateChange directly on the manager-owned
        // chunks_tx (the previous `hooks().emit_isolation_state_change(...)`
        // delegation is gone — the hook has been removed from the trait).
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(id.to_string()),
            codelet_rpc_types::StreamChunk::isolation_state_change(
                true,
                Some(worktree_path.to_string_lossy().to_string()),
            ),
        ));

        self.hooks().spawn_footer_poller(
            id.to_string(),
            worktree_path.to_string_lossy().to_string(),
            Some(worktree_path.to_string_lossy().to_string()),
        );

        codelet_tools::broadcast_metadata_update();

        Ok(codelet_rpc_types::IsolatedSessionInfo {
            session_id: codelet_rpc_types::SessionId::new(id.to_string()),
            worktree_path: worktree_path.to_string_lossy().to_string(),
            base_commit,
        })
    }

    /// SCHED-004: Spawn a session triggered by the scheduler.
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn_scheduled_session(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
        schedule_name: &str,
        role: Option<&str>,
        prompt: &str,
    ) -> Result<(), String> {
        self.create_session_with_id(id, model, project, name)
            .await?;

        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        if let Some(session) = sessions.get(&uuid) {
            session
                .schedule_triggered
                .store(true, std::sync::atomic::Ordering::Relaxed);
            *session.schedule_name.write().expect("schedule_name lock") =
                Some(schedule_name.to_string());

            if let Some(role_str) = role {
                if !role_str.is_empty() {
                    session.set_role(role_str.to_string());
                }
            }

            session.send_input(prompt.to_string(), None)?;
        }

        Ok(())
    }

    /// Destroy a session
    pub fn destroy_session(&self, id: &str) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;

        tracing::info!(
            session_id = %uuid,
            "destroy_session: starting session destruction"
        );

        self.chain_of_command.cleanup_subordinate(uuid);
        self.chain_of_command.remove_supervisor(uuid);

        let session = self
            .sessions
            .write()
            .expect("sessions lock poisoned")
            .shift_remove(&uuid);

        tracing::info!(
            session_id = %uuid,
            session_removed = session.is_some(),
            "destroy_session: shift_remove completed"
        );

        if let Some(session) = session {
            tracing::info!(
                session_id = %uuid,
                "destroy_session: session removed from in-memory map"
            );
            session.interrupt();
            self.hooks().stop_footer_poller(id);
            self.hooks().cleanup_session_loops(uuid);
            codelet_tools::cleanup_mcp_session(uuid);
            unregister_pre_tool_hook(uuid);
            codelet_tools::unregister_bash_abort_flag(uuid);
            codelet_tools::unregister_footer_cwd(uuid);
            codelet_tools::broadcast_metadata_update();

            // PARITY FIX: Do NOT delete the session manifest from disk.
            // The TypeScript reference implementation's "Close Session" (exit dialog)
            // calls sessionManagerDestroy() which only kills the in-memory session.
            // The manifest persists on disk so the user can resume later via /resume.
            //
            // Manifest deletion is a separate operation: "Delete This Session" from
            // the resume view calls persistenceDeleteSession() which maps to
            // backend.persistence_delete_session() → codelet_core::persistence::delete_session().
            //
            // See: src/tui/services/sessionService.ts destroySession() vs persistenceDeleteSession()

            tracing::info!(
                session_id = %uuid,
                "destroy_session: session destruction complete"
            );

            Ok(())
        } else {
            tracing::warn!(
                session_id = %uuid,
                "destroy_session: session not found in memory"
            );
            Err(format!("Session not found: {}", id))
        }
    }

    // === ChainOfCommand delegation methods (WATCH-002) ===

    pub fn add_supervisor(
        &self,
        subordinate_id: Uuid,
        supervisor_id: Uuid,
    ) -> std::result::Result<(), String> {
        self.chain_of_command
            .add_supervisor(subordinate_id, supervisor_id)
    }

    pub fn remove_supervisor(&self, supervisor_id: Uuid) {
        self.chain_of_command.remove_supervisor(supervisor_id)
    }

    pub fn get_supervisors(&self, subordinate_id: Uuid) -> Vec<Uuid> {
        self.chain_of_command.get_supervisors(subordinate_id)
    }

    pub fn get_subordinate(&self, supervisor_id: Uuid) -> Option<Uuid> {
        self.chain_of_command.get_subordinate(supervisor_id)
    }

    pub fn get_subordinates(&self, supervisor_id: Uuid) -> Vec<Uuid> {
        self.chain_of_command.get_subordinates(supervisor_id)
    }
}
