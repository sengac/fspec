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

use std::sync::{Arc, RwLock};

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
    /// RPC-040: NAPI-side subsystems injected via the hooks trait.
    hooks: ArcSwap<Arc<dyn SessionManagerHooks>>,
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
            hooks: ArcSwap::from_pointee(default_hooks),
        }
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

    /// SCHED-004: Set the default model for scheduled session spawning.
    /// PROV-119: also persists the non-empty choice to disk
    /// (`<data_dir>/default-model.json`) so it survives a process restart. The
    /// write is best-effort: a failure is logged and never propagates, so
    /// session creation is never blocked by a persistence error.
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
        use std::sync::OnceLock;
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

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .values()
            .map(|s| s.get_info())
            .collect()
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

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                drop(sessions);
                self.set_active_session(uuid);
                return Ok(());
            }
        }

        let (input_tx, input_rx) = mpsc::channel::<PromptInput>(32);

        // Load environment variables from .env file (if present)
        let _ = dotenvy::dotenv();

        if !model.contains('/') || model.is_empty() {
            return Err(format!(
                "Invalid model string '{}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')",
                model
            ));
        }

        let is_profile_model = model.contains(':') && model.find(':') < model.find('/');
        let is_codex_model = model.starts_with("codex/");

        let (registry_provider, model_part) = if is_profile_model {
            let colon_idx = model
                .find(':')
                .ok_or_else(|| format!("Invalid profile model string '{}': missing ':'", model))?;
            let provider = &model[..colon_idx];
            let slash_idx = model
                .find('/')
                .ok_or_else(|| format!("Invalid profile model string '{}': missing '/'", model))?;
            let model_id = &model[slash_idx + 1..];
            (provider, model_id)
        } else {
            let parts: Vec<&str> = model.splitn(2, '/').collect();
            (parts[0], parts.get(1).copied().unwrap_or(""))
        };

        if registry_provider.is_empty() || model_part.is_empty() {
            return Err(format!(
                "Invalid model string '{}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')",
                model
            ));
        }

        let is_custom_model = !is_profile_model
            && !is_codex_model
            && codelet_providers::custom_provider_registered(registry_provider);

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

        let mut provider_manager = codelet_providers::ProviderManager::with_model_support()
            .await
            .map_err(|e| format!("Failed to create provider manager: {}", e))?;

        if is_profile_model {
            tracing::info!(
                "PROV-007: Profile model detected, using set_model_direct for {}",
                model
            );
        } else if is_codex_model {
            tracing::info!(
                "PROV-018: Codex model detected, using set_model_direct for {}",
                model
            );
        } else if is_custom_model {
            tracing::info!(
                "PROV-096: Custom provider '{}' detected, using set_model_direct for {}",
                registry_provider,
                model
            );
        }

        // RPC-343: apply the selection via the shared resolver so creation and
        // the mid-session set_model path can never drift.
        let resolved =
            crate::model_resolution::apply_model_selection(&mut provider_manager, model)?;

        let initial_context_window = resolved.context_window;
        let initial_max_output_tokens = resolved.max_output_tokens;

        let mut inner = codelet_cli::session::Session::from_provider_manager(provider_manager);
        inner.inject_context_reminders();

        let lifecycle_hooks =
            match load_lifecycle_hooks(Some(&project_path), dirs::home_dir().as_deref()) {
                Ok(Some(compiled)) => Some(Arc::new(compiled)),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!(
                        "[HOOK-013] Failed to load lifecycle hooks: {} - continuing without",
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
            None,
            None,
            lifecycle_hooks.clone(),
            self.chunks_tx.clone(),
            self.status_changes_tx.clone(),
        ));

        let initial_model_id = session
            .model_id
            .read()
            .expect("model_id lock poisoned")
            .clone();
        let initial_compaction_threshold =
            codelet_cli::compaction_threshold::resolve_compaction_threshold(
                initial_context_window as u64,
                initial_max_output_tokens as u64,
                initial_model_id.as_deref(),
                None,
            ) as u32;
        session.set_model_limits(
            initial_context_window,
            initial_max_output_tokens,
            initial_compaction_threshold,
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

        // RPC-040: agent_loop spawning is delegated to the hooks impl so
        // codelet-sessions has no transitive napi dependency.
        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

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

        if !model.contains('/') || model.is_empty() {
            return Err(format!(
                "Invalid model string '{}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')",
                model
            ));
        }

        let is_profile_model = model.contains(':') && model.find(':') < model.find('/');
        let is_codex_model = model.starts_with("codex/");

        let (registry_provider, model_part) = if is_profile_model {
            let colon_idx = model
                .find(':')
                .ok_or_else(|| format!("Invalid profile model string '{}': missing ':'", model))?;
            let provider = &model[..colon_idx];
            let slash_idx = model
                .find('/')
                .ok_or_else(|| format!("Invalid profile model string '{}': missing '/'", model))?;
            let model_id = &model[slash_idx + 1..];
            (provider, model_id)
        } else {
            let parts: Vec<&str> = model.splitn(2, '/').collect();
            (parts[0], parts.get(1).copied().unwrap_or(""))
        };

        if registry_provider.is_empty() || model_part.is_empty() {
            return Err(format!(
                "Invalid model string '{}': must be in 'provider/model-id' format (e.g., 'anthropic/claude-opus-4-5')",
                model
            ));
        }

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

        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        self.set_active_session(uuid);

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

        self.chain_of_command.cleanup_subordinate(uuid);
        self.chain_of_command.remove_supervisor(uuid);

        let session = self
            .sessions
            .write()
            .expect("sessions lock poisoned")
            .shift_remove(&uuid);

        if let Some(session) = session {
            session.interrupt();
            self.hooks().stop_footer_poller(id);
            self.hooks().cleanup_session_loops(uuid);
            codelet_tools::cleanup_mcp_session(uuid);
            unregister_pre_tool_hook(uuid);
            codelet_tools::unregister_bash_abort_flag(uuid);
            codelet_tools::unregister_footer_cwd(uuid);
            codelet_tools::broadcast_metadata_update();
            Ok(())
        } else {
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
