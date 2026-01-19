//! Background Session Manager
//!
//! Implements NAPI-009: Background Session Management with Attach/Detach
//!
//! Provides a singleton SessionManager that owns multiple BackgroundSession instances,
//! each running in its own tokio task. Sessions can be attached/detached without
//! interrupting agent execution.

use crate::types::{CompactionResult, DebugCommandResult, StreamChunk};
use codelet_cli::interactive_helpers::execute_compaction;
use codelet_common::debug_capture::{
    get_debug_capture_manager, handle_debug_command_with_dir, SessionMetadata,
};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, Mutex, Notify};
use uuid::Uuid;

/// Maximum concurrent sessions
const MAX_SESSIONS: usize = 10;

/// Input message sent to the agent loop via channel
pub(crate) struct PromptInput {
    /// The user's prompt text
    input: String,
    /// Optional thinking config JSON (for extended thinking)
    thinking_config: Option<String>,
}

/// Session status values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Idle = 0,
    Running = 1,
    Interrupted = 2,
}

impl From<u8> for SessionStatus {
    fn from(v: u8) -> Self {
        match v {
            0 => SessionStatus::Idle,
            1 => SessionStatus::Running,
            2 => SessionStatus::Interrupted,
            _ => SessionStatus::Idle,
        }
    }
}

impl Default for SessionStatus {
    fn default() -> Self {
        SessionStatus::Idle
    }
}

impl SessionStatus {
    /// Convert status to string representation for TypeScript
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Idle => "idle",
            SessionStatus::Running => "running",
            SessionStatus::Interrupted => "interrupted",
        }
    }
}

/// Session info returned to TypeScript
#[napi(object)]
#[derive(Clone)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub project: String,
    pub message_count: u32,
    /// Provider ID (e.g., "anthropic", "openai")
    pub provider_id: Option<String>,
    /// Model ID (e.g., "claude-sonnet-4", "gpt-4o")
    pub model_id: Option<String>,
}

/// Model info returned by session_get_model
#[napi(object)]
#[derive(Clone)]
pub struct SessionModel {
    /// Provider ID (e.g., "anthropic", "openai")
    pub provider_id: Option<String>,
    /// Model ID (e.g., "claude-sonnet-4", "gpt-4o")
    pub model_id: Option<String>,
}

/// Token info returned by session_get_tokens
#[napi(object)]
#[derive(Clone)]
pub struct SessionTokens {
    /// Input tokens (context size)
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
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
    cached_input_tokens: AtomicU32,
    cached_output_tokens: AtomicU32,

    /// Inner codelet session (protected by async mutex for agent operations)
    pub inner: Arc<Mutex<codelet_cli::session::Session>>,

    /// Current status (lock-free)
    status: AtomicU8,

    /// Whether a UI is currently attached
    is_attached: AtomicBool,

    /// Channel to send input prompts to the agent loop
    input_tx: mpsc::Sender<PromptInput>,

    /// Buffered output chunks (unbounded - keeps all output for session lifetime)
    output_buffer: RwLock<Vec<StreamChunk>>,

    /// Callback for attached UI (None if detached)
    attached_callback: RwLock<Option<ThreadsafeFunction<StreamChunk>>>,

    /// Interrupt flag for stopping agent execution
    is_interrupted: Arc<AtomicBool>,

    /// Notify for immediate interrupt wake-up
    interrupt_notify: Arc<Notify>,

    /// Debug capture enabled for this session
    is_debug_enabled: AtomicBool,
}

impl BackgroundSession {
    /// Create a new background session
    pub(crate) fn new(
        id: Uuid,
        name: String,
        project: String,
        provider_id: Option<String>,
        model_id: Option<String>,
        inner: codelet_cli::session::Session,
        input_tx: mpsc::Sender<PromptInput>,
    ) -> Self {
        Self {
            id,
            name: RwLock::new(name),
            project,
            provider_id: RwLock::new(provider_id),
            model_id: RwLock::new(model_id),
            cached_input_tokens: AtomicU32::new(0),
            cached_output_tokens: AtomicU32::new(0),
            inner: Arc::new(Mutex::new(inner)),
            status: AtomicU8::new(SessionStatus::Idle as u8),
            is_attached: AtomicBool::new(false),
            input_tx,
            output_buffer: RwLock::new(Vec::new()),
            attached_callback: RwLock::new(None),
            is_interrupted: Arc::new(AtomicBool::new(false)),
            interrupt_notify: Arc::new(Notify::new()),
            is_debug_enabled: AtomicBool::new(false),
        }
    }

    /// Get debug enabled state
    pub fn get_debug_enabled(&self) -> bool {
        self.is_debug_enabled.load(Ordering::Acquire)
    }

    /// Set debug enabled state
    pub fn set_debug_enabled(&self, enabled: bool) {
        self.is_debug_enabled.store(enabled, Ordering::Release);
    }

    /// Update cached token counts (called when TokenUpdate events are emitted)
    pub fn update_tokens(&self, input_tokens: u32, output_tokens: u32) {
        self.cached_input_tokens.store(input_tokens, Ordering::Release);
        self.cached_output_tokens.store(output_tokens, Ordering::Release);
    }

    /// Get cached token counts
    pub fn get_tokens(&self) -> (u32, u32) {
        (
            self.cached_input_tokens.load(Ordering::Acquire),
            self.cached_output_tokens.load(Ordering::Acquire),
        )
    }

    /// Update the model info (called when model is changed mid-session)
    pub fn set_model(&self, provider_id: Option<String>, model_id: Option<String>) {
        *self.provider_id.write().expect("provider_id lock poisoned") = provider_id;
        *self.model_id.write().expect("model_id lock poisoned") = model_id;
    }
    
    /// Get current status
    pub fn get_status(&self) -> SessionStatus {
        SessionStatus::from(self.status.load(Ordering::Acquire))
    }
    
    /// Set status
    pub fn set_status(&self, status: SessionStatus) {
        self.status.store(status as u8, Ordering::Release);
    }
    
    /// Check if attached
    pub fn is_attached(&self) -> bool {
        self.is_attached.load(Ordering::Acquire)
    }
    
    /// Handle output chunk - buffer and optionally forward to callback
    pub fn handle_output(&self, chunk: StreamChunk) {
        // Always buffer (unbounded)
        {
            let mut buffer = self.output_buffer.write().expect("output buffer lock poisoned");
            buffer.push(chunk.clone());
        }
        
        // If attached, forward to callback
        // Note: We check is_attached first, but callback may be None during detach transition.
        // This is safe because detach() clears callback before setting is_attached to false.
        if self.is_attached() {
            if let Some(cb) = self.attached_callback.read().expect("callback lock poisoned").as_ref() {
                let _ = cb.call(Ok(chunk), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }
    }
    
    /// Get buffered output
    pub fn get_buffered_output(&self, limit: usize) -> Vec<StreamChunk> {
        let buffer = self.output_buffer.read().expect("output buffer lock poisoned");
        buffer.iter().take(limit).cloned().collect()
    }
    
    /// Attach a callback for live streaming
    pub fn attach(&self, callback: ThreadsafeFunction<StreamChunk>) {
        *self.attached_callback.write().expect("callback lock poisoned") = Some(callback);
        self.is_attached.store(true, Ordering::Release);
    }
    
    /// Detach - session continues running but stops forwarding to callback
    /// Note: We clear the callback first, then set is_attached to false to avoid
    /// a race where handle_output sees is_attached=true but callback is None
    pub fn detach(&self) {
        *self.attached_callback.write().expect("callback lock poisoned") = None;
        self.is_attached.store(false, Ordering::Release);
    }
    
    /// Send input to the agent loop
    ///
    /// Buffers the user input as a UserInput chunk before sending to the agent,
    /// so it can be replayed when attaching to a detached session via /resume.
    pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<()> {
        // Buffer user input for resume/attach (NAPI-009)
        self.handle_output(StreamChunk::user_input(input.clone()));

        self.input_tx
            .try_send(PromptInput { input, thinking_config })
            .map_err(|e| Error::from_reason(format!("Failed to send input: {}", e)))
    }
    
    /// Interrupt current agent execution
    pub fn interrupt(&self) {
        self.is_interrupted.store(true, Ordering::Release);
        self.interrupt_notify.notify_one();
    }
    
    /// Reset interrupt flag
    pub fn reset_interrupt(&self) {
        self.is_interrupted.store(false, Ordering::Release);
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
            .filter(|c| c.chunk_type == "Done")
            .count() as u32;

        SessionInfo {
            id: self.id.to_string(),
            name: self.name.read().expect("name lock poisoned").clone(),
            status: self.get_status().as_str().to_string(),
            project: self.project.clone(),
            message_count,
            provider_id: self.provider_id.read().expect("provider_id lock poisoned").clone(),
            model_id: self.model_id.read().expect("model_id lock poisoned").clone(),
        }
    }
}

/// Singleton session manager
pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, Arc<BackgroundSession>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Create new session manager
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
    
    /// Get singleton instance
    pub fn instance() -> &'static SessionManager {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<SessionManager> = OnceLock::new();
        INSTANCE.get_or_init(SessionManager::new)
    }
    
    /// Create a new background session (generates new UUID)
    pub async fn create_session(&self, _model: &str, project: &str) -> Result<String> {
        let id = Uuid::new_v4();
        self.create_session_with_id(&id.to_string(), _model, project, &format!("Session {}", &id.to_string()[..8])).await?;
        Ok(id.to_string())
    }
    
    /// Create a background session with a specific ID (for persistence integration).
    ///
    /// This is the core session creation method. The ID should match the persistence
    /// session ID so that ESC + Detach and /resume can find the session.
    pub async fn create_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<()> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;

        // Check session limits in a block to ensure lock is dropped before async operations
        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(Error::from_reason(format!(
                    "Maximum sessions ({}) reached",
                    MAX_SESSIONS
                )));
            }
            if sessions.contains_key(&uuid) {
                // Already registered - this is fine, session exists
                return Ok(());
            }
        }

        let (input_tx, input_rx) = mpsc::channel::<PromptInput>(32);

        // Load environment variables from .env file (if present)
        // This is required for API keys to be available when running from Node.js
        let _ = dotenvy::dotenv();

        // Parse model string to extract provider_id and model_id for storage
        let (provider_id, model_id) = if model.contains('/') {
            let parts: Vec<&str> = model.split('/').collect();
            let registry_provider = parts.get(0).unwrap_or(&"anthropic");
            let model_part = parts.get(1).map(|s| s.to_string());
            (Some(registry_provider.to_string()), model_part)
        } else {
            (Some(model.to_string()), None)
        };

        // Create ProviderManager with model registry support and select the model
        let mut provider_manager = codelet_providers::ProviderManager::with_model_support()
            .await
            .map_err(|e| Error::from_reason(format!("Failed to create provider manager: {}", e)))?;

        // Select the model (validates against registry)
        if model.contains('/') {
            provider_manager.select_model(model)
                .map_err(|e| Error::from_reason(format!("Failed to select model: {}", e)))?;
        }

        // Create session from the configured provider manager
        let inner = codelet_cli::session::Session::from_provider_manager(provider_manager);

        let session = Arc::new(BackgroundSession::new(
            uuid,
            name.to_string(),
            project.to_string(),
            provider_id,
            model_id,
            inner,
            input_tx,
        ));
        
        // Spawn agent loop task
        let session_clone = session.clone();
        tokio::spawn(async move {
            agent_loop(session_clone, input_rx).await;
        });
        
        // Store session
        self.sessions.write().expect("sessions lock poisoned").insert(uuid, session);
        
        Ok(())
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
    
    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Arc<BackgroundSession>> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
        
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&uuid)
            .cloned()
            .ok_or_else(|| Error::from_reason(format!("Session not found: {}", id)))
    }
    
    /// Destroy a session
    pub fn destroy_session(&self, id: &str) -> Result<()> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
        
        let session = self.sessions.write().expect("sessions lock poisoned").remove(&uuid);
        
        if let Some(session) = session {
            // Interrupt to stop the agent loop
            session.interrupt();
            // Drop the input sender to signal the loop to exit
            // (happens automatically when session is dropped)
            Ok(())
        } else {
            Err(Error::from_reason(format!("Session not found: {}", id)))
        }
    }
    
}

/// Agent loop that runs in background tokio task
async fn agent_loop(session: Arc<BackgroundSession>, mut input_rx: mpsc::Receiver<PromptInput>) {
    use codelet_cli::interactive::run_agent_stream;
    use codelet_core::RigAgent;
    
    loop {
        // Wait for input
        let prompt_input = match input_rx.recv().await {
            Some(input) => input,
            None => {
                break;
            }
        };
        
        let input = &prompt_input.input;
        let thinking_config = prompt_input.thinking_config.as_deref();
        
        tracing::debug!("Session {} received input: {}", session.id, &input[..input.len().min(50)]);
        
        // Set status to running
        session.set_status(SessionStatus::Running);
        session.reset_interrupt();
        
        // Parse thinking config JSON if provided
        let thinking_config_value: Option<serde_json::Value> = thinking_config.and_then(|config_str| {
            serde_json::from_str(config_str).ok()
        });
        
        // Create output handler that buffers and forwards
        let session_for_output = session.clone();
        let output = BackgroundOutput::new(session_for_output);
        
        // Lock the inner session and run agent stream
        let mut inner_session = session.inner.lock().await;
        
        // Get provider and run agent stream
        // Note: Each provider type returns a different concrete type, so we must match
        // and call run_agent_stream in each branch. This is a Rust limitation with
        // heterogeneous types that share a trait but need to be used with generics.
        let current_provider = inner_session.current_provider_name().to_string();
        let result = match current_provider.as_str() {
            "claude" => match inner_session.provider_manager_mut().get_claude() {
                Ok(provider) => {
                    let agent = RigAgent::with_default_depth(provider.create_rig_agent(None, thinking_config_value.clone()));
                    run_agent_stream(
                        agent,
                        input,
                        &mut inner_session,
                        session.is_interrupted.clone(),
                        session.interrupt_notify.clone(),
                        &output,
                    )
                    .await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to get provider: {}", e)),
            },
            "openai" => match inner_session.provider_manager_mut().get_openai() {
                Ok(provider) => {
                    let agent = RigAgent::with_default_depth(provider.create_rig_agent(None, thinking_config_value.clone()));
                    run_agent_stream(
                        agent,
                        input,
                        &mut inner_session,
                        session.is_interrupted.clone(),
                        session.interrupt_notify.clone(),
                        &output,
                    )
                    .await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to get provider: {}", e)),
            },
            "gemini" => match inner_session.provider_manager_mut().get_gemini() {
                Ok(provider) => {
                    let agent = RigAgent::with_default_depth(provider.create_rig_agent(None, thinking_config_value.clone()));
                    run_agent_stream(
                        agent,
                        input,
                        &mut inner_session,
                        session.is_interrupted.clone(),
                        session.interrupt_notify.clone(),
                        &output,
                    )
                    .await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to get provider: {}", e)),
            },
            "zai" => match inner_session.provider_manager_mut().get_zai() {
                Ok(provider) => {
                    let agent = RigAgent::with_default_depth(provider.create_rig_agent(None, thinking_config_value));
                    run_agent_stream(
                        agent,
                        input,
                        &mut inner_session,
                        session.is_interrupted.clone(),
                        session.interrupt_notify.clone(),
                        &output,
                    )
                    .await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to get provider: {}", e)),
            },
            _ => {
                tracing::error!("Unsupported provider: {}", current_provider);
                Err(anyhow::anyhow!("Unsupported provider: {}", current_provider))
            }
        };
        
        // Handle result
        // Note: run_agent_stream emits StreamEvent::Done on successful completion,
        // so we only emit Done here on error (to ensure the turn is properly terminated)
        if let Err(e) = result {
            tracing::error!("Agent stream error for session {}: {}", session.id, e);
            session.handle_output(StreamChunk::error(e.to_string()));
            session.handle_output(StreamChunk::done());
        }
        
        // Set status back to idle
        session.set_status(SessionStatus::Idle);
    }
}

/// Output handler for background sessions that implements StreamOutput
struct BackgroundOutput {
    session: Arc<BackgroundSession>,
}

impl BackgroundOutput {
    fn new(session: Arc<BackgroundSession>) -> Self {
        Self { session }
    }
}

impl codelet_cli::interactive::StreamOutput for BackgroundOutput {
    fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
        use codelet_cli::interactive::StreamEvent;
        use crate::types::{
            ContextFillInfo, StreamChunk, TokenTracker, ToolCallInfo, ToolProgressInfo,
            ToolResultInfo,
        };

        let chunk = match event {
            StreamEvent::Text(text) => StreamChunk::text(text),
            StreamEvent::Thinking(thinking) => StreamChunk::thinking(thinking),
            StreamEvent::ToolCall(tc) => StreamChunk::tool_call(ToolCallInfo {
                id: tc.id,
                name: tc.name,
                input: tc.args.to_string(),
            }),
            StreamEvent::ToolResult(tr) => StreamChunk::tool_result(ToolResultInfo {
                tool_call_id: tr.id,
                content: tr.content,
                is_error: tr.is_error,
            }),
            StreamEvent::ToolProgress(tp) => StreamChunk::tool_progress(ToolProgressInfo {
                tool_call_id: tp.tool_call_id,
                tool_name: tp.tool_name,
                output_chunk: tp.output_chunk,
            }),
            StreamEvent::Status(status) => StreamChunk::status(status),
            StreamEvent::Tokens(info) => {
                // Update cached tokens for sync access
                self.session.update_tokens(info.input_tokens as u32, info.output_tokens as u32);
                StreamChunk::token_update(TokenTracker {
                    input_tokens: info.input_tokens as u32,
                    output_tokens: info.output_tokens as u32,
                    cache_read_input_tokens: info.cache_read_input_tokens.map(|v| v as u32),
                    cache_creation_input_tokens: info.cache_creation_input_tokens.map(|v| v as u32),
                    tokens_per_second: info.tokens_per_second,
                    cumulative_billed_input: None,
                    cumulative_billed_output: None,
                })
            }
            StreamEvent::ContextFill(info) => StreamChunk::context_fill_update(ContextFillInfo {
                fill_percentage: info.fill_percentage,
                effective_tokens: info.effective_tokens as f64,
                threshold: info.threshold as f64,
                context_window: info.context_window as f64,
            }),
            StreamEvent::Error(error) => StreamChunk::error(error),
            StreamEvent::Interrupted(queued) => StreamChunk::interrupted(queued),
            StreamEvent::Done => StreamChunk::done(),
        };

        self.session.handle_output(chunk);
    }
}

// =============================================================================
// NAPI Bindings
// =============================================================================

/// Create a new background session (generates new UUID)
#[napi]
pub async fn session_manager_create(model: String, project: String) -> Result<String> {
    SessionManager::instance().create_session(&model, &project).await
}

/// Create a background session with a specific ID (for persistence integration).
///
/// This is used when AgentView creates a session - the ID comes from persistence
/// so that detach/attach can find the session by the same ID used for persistence.
///
/// Note: This must be async because it uses tokio::spawn internally, which requires
/// a Tokio runtime context. NAPI-RS provides this context for async functions.
#[napi]
pub async fn session_manager_create_with_id(
    session_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<()> {
    SessionManager::instance().create_session_with_id(&session_id, &model, &project, &name).await
}

/// List all background sessions
#[napi]
pub fn session_manager_list() -> Vec<SessionInfo> {
    SessionManager::instance().list_sessions()
}

/// Destroy a background session
#[napi]
pub fn session_manager_destroy(session_id: String) -> Result<()> {
    SessionManager::instance().destroy_session(&session_id)
}

/// Attach to a session for live streaming
#[napi]
pub fn session_attach(session_id: String, callback: ThreadsafeFunction<StreamChunk>) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.attach(callback);
    Ok(())
}

/// Detach from a session (session continues running)
#[napi]
pub fn session_detach(session_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.detach();
    Ok(())
}

/// Send input to a session with optional thinking config
#[napi]
pub fn session_send_input(session_id: String, input: String, thinking_config: Option<String>) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.send_input(input, thinking_config)
}

/// Interrupt a session
#[napi]
pub fn session_interrupt(session_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.interrupt();
    Ok(())
}

/// Get session status
#[napi]
pub fn session_get_status(session_id: String) -> Result<String> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_status().as_str().to_string())
}

/// Update the model for a background session
#[napi]
pub async fn session_set_model(session_id: String, provider_id: String, model_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;

    // Update metadata for display
    session.set_model(Some(provider_id.clone()), Some(model_id.clone()));

    // Construct model string and update the inner ProviderManager
    let model_string = format!("{}/{}", provider_id, model_id);
    let mut inner = session.inner.lock().await;
    inner.provider_manager_mut().select_model(&model_string)
        .map_err(|e| Error::from_reason(format!("Failed to select model: {}", e)))?;

    Ok(())
}

/// Get the model info for a background session
#[napi]
pub fn session_get_model(session_id: String) -> Result<SessionModel> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let provider_id = session.provider_id.read().unwrap().clone();
    let model_id = session.model_id.read().unwrap().clone();
    Ok(SessionModel {
        provider_id,
        model_id,
    })
}

/// Get cached token counts for a background session
#[napi]
pub fn session_get_tokens(session_id: String) -> Result<SessionTokens> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let (input_tokens, output_tokens) = session.get_tokens();
    Ok(SessionTokens {
        input_tokens,
        output_tokens,
    })
}

/// Get debug enabled state for a background session
#[napi]
pub fn session_get_debug_enabled(session_id: String) -> Result<bool> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_debug_enabled())
}

/// Set debug enabled state for a background session (without toggling global state)
#[napi]
pub fn session_set_debug_enabled(session_id: String, enabled: bool) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.set_debug_enabled(enabled);
    Ok(())
}

/// Get buffered output from a session
#[napi]
pub fn session_get_buffered_output(session_id: String, limit: u32) -> Result<Vec<StreamChunk>> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_buffered_output(limit as usize))
}

/// Get buffered output with consecutive Text/Thinking chunks merged.
/// This is more efficient for reattachment - JS can process fewer chunks.
#[napi]
pub fn session_get_merged_output(session_id: String) -> Result<Vec<StreamChunk>> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let chunks = session.get_buffered_output(usize::MAX);

    let mut merged: Vec<StreamChunk> = Vec::new();

    for chunk in chunks {
        match chunk.chunk_type.as_str() {
            "Text" => {
                // Merge consecutive Text chunks
                if let Some(last) = merged.last_mut() {
                    if last.chunk_type == "Text" {
                        if let (Some(existing), Some(new)) = (&mut last.text, &chunk.text) {
                            existing.push_str(new);
                            continue;
                        }
                    }
                }
                merged.push(chunk);
            }
            "Thinking" => {
                // Merge consecutive Thinking chunks
                if let Some(last) = merged.last_mut() {
                    if last.chunk_type == "Thinking" {
                        if let (Some(existing), Some(new)) = (&mut last.thinking, &chunk.thinking) {
                            existing.push_str(new);
                            continue;
                        }
                    }
                }
                merged.push(chunk);
            }
            // Skip metadata chunks that don't affect display
            "TokenUpdate" | "ContextFillUpdate" => continue,
            _ => merged.push(chunk),
        }
    }

    Ok(merged)
}

/// Restore messages to a background session from persisted envelopes.
///
/// This is used when attaching to a session via /resume - it restores the
/// conversation history so the LLM has context for future prompts.
#[napi]
pub async fn session_restore_messages(session_id: String, envelopes: Vec<String>) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let mut inner = session.inner.lock().await;

    // Use the existing restore_messages_from_envelopes logic from codelet_cli
    for envelope_json in envelopes {
        let envelope: serde_json::Value = serde_json::from_str(&envelope_json)
            .map_err(|e| Error::from_reason(format!("Failed to parse envelope: {}", e)))?;

        // Extract message from envelope
        if let Some(message) = envelope.get("message") {
            let role = message.get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");

            // Build rig message from envelope content
            let rig_message = if role == "assistant" {
                // Handle assistant messages with content blocks
                if let Some(content) = message.get("content") {
                    if let Some(arr) = content.as_array() {
                        let mut text_parts = Vec::new();
                        for block in arr {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                text_parts.push(text.to_string());
                            }
                        }
                        let joined_text = text_parts.join("");
                        // Skip empty messages to avoid API error "text content blocks must be non-empty"
                        if joined_text.is_empty() {
                            continue;
                        }
                        rig::message::Message::Assistant {
                            id: None,
                            content: rig::OneOrMany::one(rig::message::AssistantContent::text(joined_text)),
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            } else {
                // Handle user messages
                if let Some(content) = message.get("content") {
                    let text = if let Some(arr) = content.as_array() {
                        arr.iter()
                            .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("")
                    } else if let Some(s) = content.as_str() {
                        s.to_string()
                    } else {
                        continue;
                    };
                    // Skip empty messages to avoid API error "text content blocks must be non-empty"
                    if text.is_empty() {
                        continue;
                    }
                    rig::message::Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(text)),
                    }
                } else {
                    continue;
                }
            };

            inner.messages.push(rig_message);
        }
    }

    Ok(())
}

/// Restore token state to a background session from persisted values.
///
/// This is used when attaching to a session via /resume - it restores the
/// token tracking state so context fill percentage and token counts are accurate.
#[napi]
pub async fn session_restore_token_state(
    session_id: String,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_creation_tokens: u32,
    cumulative_billed_input: u32,
    cumulative_billed_output: u32,
) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;

    // Update cached tokens for sync access
    session.update_tokens(input_tokens, output_tokens);

    let mut inner = session.inner.lock().await;

    inner.token_tracker.input_tokens = input_tokens as u64;
    inner.token_tracker.output_tokens = output_tokens as u64;
    inner.token_tracker.cache_read_input_tokens = Some(cache_read_tokens as u64);
    inner.token_tracker.cache_creation_input_tokens = Some(cache_creation_tokens as u64);
    inner.token_tracker.cumulative_billed_input = cumulative_billed_input as u64;
    inner.token_tracker.cumulative_billed_output = cumulative_billed_output as u64;

    Ok(())
}

/// Toggle debug capture mode without requiring a session.
///
/// Can be called before a session exists. Session metadata will not be set.
/// Use session_update_debug_metadata after creating a session to add metadata.
///
/// If debug_dir is provided, debug files will be written to `{debug_dir}/debug/`
/// instead of the default directory. For fspec, pass `~/.fspec` to write to
/// `~/.fspec/debug/`.
#[napi]
pub fn toggle_debug(debug_dir: Option<String>) -> DebugCommandResult {
    let result = handle_debug_command_with_dir(debug_dir.as_deref());
    DebugCommandResult {
        enabled: result.enabled,
        session_file: result.session_file,
        message: result.message,
    }
}

/// Update debug capture metadata with session info.
///
/// Call this after creating a session if debug was enabled before the session existed.
#[napi]
pub async fn session_update_debug_metadata(session_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let inner = session.inner.lock().await;

    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.set_session_metadata(SessionMetadata {
                    provider: Some(inner.current_provider_name().to_string()),
                    model: Some(inner.current_provider_name().to_string()),
                    context_window: Some(inner.provider_manager().context_window()),
                    max_output_tokens: None,
                });
            }
        }
    }

    Ok(())
}

/// Toggle debug capture mode for a background session (NAPI-009 + AGENT-021)
///
/// Mirrors CodeletSession::toggle_debug() behavior but works with background sessions.
/// When enabling, sets session metadata (provider, model, context_window).
/// When disabling, stops capture and returns path to saved session file.
///
/// If debug_dir is provided, debug files will be written to `{debug_dir}/debug/`
/// instead of the default directory. For fspec, pass `~/.fspec` to write to
/// `~/.fspec/debug/`.
#[napi]
pub async fn session_toggle_debug(
    session_id: String,
    debug_dir: Option<String>,
) -> Result<DebugCommandResult> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let result = handle_debug_command_with_dir(debug_dir.as_deref());

    // Store debug state in BackgroundSession for persistence across detach/attach
    session.set_debug_enabled(result.enabled);

    // If debug was just enabled, set session metadata
    if result.enabled {
        let inner = session.inner.lock().await;
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                manager.set_session_metadata(SessionMetadata {
                    provider: Some(inner.current_provider_name().to_string()),
                    model: Some(inner.current_provider_name().to_string()),
                    context_window: Some(inner.provider_manager().context_window()),
                    max_output_tokens: None,
                });
            }
        }
    }

    Ok(DebugCommandResult {
        enabled: result.enabled,
        session_file: result.session_file,
        message: result.message,
    })
}

/// Manually trigger context compaction for a background session (NAPI-009 + NAPI-005)
///
/// Mirrors CodeletSession::compact() behavior but works with background sessions.
/// Calls execute_compaction from interactive_helpers to compress context.
///
/// Returns CompactionResult with metrics about the compaction operation.
/// Returns error if session is empty (nothing to compact).
#[napi]
pub async fn session_compact(session_id: String) -> Result<CompactionResult> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let mut inner = session.inner.lock().await;

    // Check if there's anything to compact
    if inner.messages.is_empty() {
        return Err(Error::from_reason("Nothing to compact - no messages yet"));
    }

    // Get current token count for reporting
    let original_tokens = inner.token_tracker.input_tokens;

    // Capture compaction.manual.start event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "compaction.manual.start",
                    serde_json::json!({
                        "command": "/compact",
                        "originalTokens": original_tokens,
                        "messageCount": inner.messages.len(),
                    }),
                    None,
                );
            }
        }
    }

    // Execute compaction
    let metrics = execute_compaction(&mut inner).await.map_err(|e| {
        // Capture compaction.manual.failed event
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    manager.capture(
                        "compaction.manual.failed",
                        serde_json::json!({
                            "command": "/compact",
                            "error": e.to_string(),
                        }),
                        None,
                    );
                }
            }
        }
        Error::from_reason(format!("Compaction failed: {e}"))
    })?;

    // Capture compaction.manual.complete event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "compaction.manual.complete",
                    serde_json::json!({
                        "command": "/compact",
                        "originalTokens": metrics.original_tokens,
                        "compactedTokens": metrics.compacted_tokens,
                        "compressionRatio": metrics.compression_ratio,
                        "turnsSummarized": metrics.turns_summarized,
                        "turnsKept": metrics.turns_kept,
                    }),
                    None,
                );
            }
        }
    }

    Ok(CompactionResult {
        original_tokens: metrics.original_tokens as u32,
        compacted_tokens: metrics.compacted_tokens as u32,
        compression_ratio: metrics.compression_ratio * 100.0, // Convert to percentage
        turns_summarized: metrics.turns_summarized as u32,
        turns_kept: metrics.turns_kept as u32,
    })
}
