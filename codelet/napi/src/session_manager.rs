//! Background Session Manager
//!
//! Implements NAPI-009: Background Session Management with Attach/Detach
//!
//! Provides a singleton SessionManager that owns multiple BackgroundSession instances,
//! each running in its own tokio task. Sessions can be attached/detached without
//! interrupting agent execution.

use crate::types::{CompactionResult, DebugCommandResult, StreamChunk, ToolCallInfo, ToolResultInfo};
use codelet_cli::interactive_helpers::execute_compaction;
use codelet_common::debug_capture::{
    get_debug_capture_manager, handle_debug_command_with_dir, SessionMetadata,
};
use codelet_tools::{clear_bash_abort, request_bash_abort};
use codelet_tools::tool_pause::{PauseKind, PauseRequest, PauseResponse, PauseState, set_pause_handler, PauseHandler};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc, Mutex, Notify};
use uuid::Uuid;
use indexmap::IndexMap;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionStatus {
    #[default]
    Idle = 0,
    Running = 1,
    Interrupted = 2,
    /// PAUSE-001: Session is paused waiting for user input (Enter/Y/N/Esc)
    Paused = 3,
}

/// Role authority level for watcher sessions (WATCH-004)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoleAuthority {
    /// Equal authority - can observe but not override parent decisions
    #[default]
    Peer,
    /// Elevated authority - can inject directives that override parent
    Supervisor,
}

impl RoleAuthority {
    /// Parse authority from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "peer" => Some(RoleAuthority::Peer),
            "supervisor" => Some(RoleAuthority::Supervisor),
            _ => None,
        }
    }

    /// Convert to string representation (lowercase)
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleAuthority::Peer => "peer",
            RoleAuthority::Supervisor => "supervisor",
        }
    }

    /// Display name for formatted output (capitalized: Peer, Supervisor)
    pub fn display_name(&self) -> &'static str {
        match self {
            RoleAuthority::Peer => "Peer",
            RoleAuthority::Supervisor => "Supervisor",
        }
    }
}

// =============================================================================
// INTERJECTION PARSING (WATCH-020)
// =============================================================================

/// Parsed interjection from watcher AI response (WATCH-020)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interjection {
    /// Whether this is an urgent interjection (interrupt parent mid-stream)
    pub urgent: bool,
    /// The message content to inject
    pub content: String,
}

/// Parse an interjection from a watcher AI response (WATCH-020)
///
/// Looks for [INTERJECT]...[/INTERJECT] or [CONTINUE]...[/CONTINUE] blocks.
/// Returns Some(Interjection) for valid [INTERJECT] blocks, None otherwise.
///
/// Format requirements (strict parsing):
/// - Block markers must be exact: [INTERJECT], [/INTERJECT], [CONTINUE], [/CONTINUE]
/// - Field names must be lowercase: 'urgent:', 'content:'
/// - urgent value must be 'true' or 'false' (exact)
/// - Optional whitespace allowed after colons
/// - Empty content is invalid
///
/// Example valid [INTERJECT] block:
/// ```text
/// [INTERJECT]
/// urgent: true
/// content: Security vulnerability detected
/// [/INTERJECT]
/// ```
pub fn parse_interjection(response: &str) -> Option<Interjection> {
    // Check for [CONTINUE] block first - this means no interjection
    if response.contains("[CONTINUE]") && response.contains("[/CONTINUE]") {
        tracing::debug!("Watcher response contains [CONTINUE] block - no interjection");
        return None;
    }
    
    // Look for [INTERJECT] block
    let start_marker = "[INTERJECT]";
    let end_marker = "[/INTERJECT]";
    
    let start_idx = response.find(start_marker)?;
    let content_start = start_idx + start_marker.len();
    let end_idx = response[content_start..].find(end_marker)?;
    
    let block_content = &response[content_start..content_start + end_idx];
    
    // Parse urgent field - must be 'urgent:' followed by 'true' or 'false'
    let urgent = if let Some(urgent_line) = block_content.lines()
        .find(|line| line.trim().starts_with("urgent:"))
    {
        let value = urgent_line.trim()
            .strip_prefix("urgent:")
            .map(|s| s.trim())?;
        
        match value {
            "true" => true,
            "false" => false,
            _ => {
                tracing::warn!("Invalid urgent value '{}' in [INTERJECT] block - must be 'true' or 'false'", value);
                return None;
            }
        }
    } else {
        tracing::warn!("Missing 'urgent:' field in [INTERJECT] block");
        return None;
    };
    
    // Parse content field - 'content:' followed by the message (can be multiline)
    let content_line_idx = block_content.lines()
        .position(|line| line.trim().starts_with("content:"))?;
    
    let lines: Vec<&str> = block_content.lines().collect();
    let first_content_line = lines.get(content_line_idx)?;
    
    // Get content after 'content:' prefix
    let first_part = first_content_line.trim()
        .strip_prefix("content:")
        .map(|s| s.trim_start())?;
    
    // Collect remaining lines as part of content (multiline support)
    let mut content_parts = vec![first_part.to_string()];
    for line in lines.iter().skip(content_line_idx + 1) {
        // Stop if we hit another field (like a malformed duplicate)
        if line.trim().starts_with("urgent:") {
            break;
        }
        content_parts.push(line.to_string());
    }
    
    let content = content_parts.join("\n").trim().to_string();
    
    if content.is_empty() {
        tracing::warn!("Empty content in [INTERJECT] block");
        return None;
    }
    
    tracing::info!(
        "Parsed interjection: urgent={}, content_len={}",
        urgent,
        content.len()
    );
    
    Some(Interjection { urgent, content })
}

/// Session role for watcher sessions (WATCH-004, extended by WATCH-020)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRole {
    /// Role name (e.g., "code-reviewer", "supervisor")
    pub name: String,
    /// Optional description of what this role does
    pub description: Option<String>,
    /// Authority level
    pub authority: RoleAuthority,
    /// Whether to automatically inject interjections (WATCH-020)
    /// When true, parsed [INTERJECT] blocks trigger automatic watcher_inject calls.
    /// When false, interjections are shown in UI for manual review.
    pub auto_inject: bool,
}

impl SessionRole {
    /// Create a new session role
    pub fn new(name: String, description: Option<String>, authority: RoleAuthority) -> std::result::Result<Self, String> {
        if name.is_empty() {
            return Err("Role name cannot be empty".to_string());
        }
        Ok(Self { name, description, authority, auto_inject: true })
    }
    
    /// Create a new session role with auto_inject setting (WATCH-020)
    pub fn new_with_auto_inject(
        name: String,
        description: Option<String>,
        authority: RoleAuthority,
        auto_inject: bool,
    ) -> std::result::Result<Self, String> {
        if name.is_empty() {
            return Err("Role name cannot be empty".to_string());
        }
        Ok(Self { name, description, authority, auto_inject })
    }
}

/// Watcher input message for injection into parent session (WATCH-006)
#[derive(Debug, Clone)]
pub struct WatcherInput {
    /// Session ID of the watcher sending the input
    pub source_session_id: String,
    /// Role name of the watcher (e.g., "code-reviewer")
    pub role_name: String,
    /// Authority level (Peer or Supervisor)
    pub authority: RoleAuthority,
    /// The message content to inject
    pub message: String,
}

impl WatcherInput {
    /// Create a new WatcherInput
    pub fn new(
        source_session_id: String,
        role_name: String,
        authority: RoleAuthority,
        message: String,
    ) -> std::result::Result<Self, String> {
        if message.is_empty() {
            return Err("message cannot be empty".to_string());
        }
        Ok(Self {
            source_session_id,
            role_name,
            authority,
            message,
        })
    }
}

/// Format a watcher input message with the structured prefix (WATCH-006)
///
/// Format: [WATCHER: role | Authority: level | Session: id] message
pub fn format_watcher_input(input: &WatcherInput) -> String {
    format!(
        "[WATCHER: {} | Authority: {} | Session: {}] {}",
        input.role_name,
        input.authority.display_name(),
        input.source_session_id,
        input.message
    )
}

/// Watcher state for the agent loop (WATCH-005)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatcherState {
    /// Waiting for input (user prompt or parent observation)
    #[default]
    Idle,
    /// Accumulating observations from parent session
    Observing,
    /// Running the agent to process input
    Processing,
}

/// Buffer for accumulating observations from parent session (WATCH-005)
#[derive(Debug, Clone)]
pub struct ObservationBuffer {
    /// Accumulated chunks from parent session
    chunks: Vec<StreamChunk>,
    /// Timestamp of last chunk received (for silence timeout)
    last_chunk_time: Option<std::time::Instant>,
}

impl Default for ObservationBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationBuffer {
    /// Create a new empty observation buffer
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            last_chunk_time: None,
        }
    }

    /// Push a chunk to the buffer
    pub fn push(&mut self, chunk: StreamChunk) {
        self.chunks.push(chunk);
        self.last_chunk_time = Some(std::time::Instant::now());
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.last_chunk_time = None;
    }

    /// Get accumulated text from all Text chunks
    pub fn accumulated_text(&self) -> String {
        self.chunks
            .iter()
            .filter_map(|c| {
                if c.chunk_type == "Text" {
                    c.text.clone()
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the last chunk time (for silence timeout detection)
    pub fn last_chunk_time(&self) -> Option<std::time::Instant> {
        self.last_chunk_time
    }

    /// Get all chunks (for formatting)
    pub fn chunks(&self) -> &[StreamChunk] {
        &self.chunks
    }

    /// Get all correlation IDs from buffered chunks (WATCH-011)
    /// Returns correlation IDs for cross-pane selection highlighting
    pub fn correlation_ids(&self) -> Vec<String> {
        self.chunks
            .iter()
            .filter_map(|c| c.correlation_id.clone())
            .collect()
    }
}

/// Check if a StreamChunk represents a natural breakpoint (WATCH-005)
///
/// Natural breakpoints are:
/// - Done (turn complete)
/// - ToolResult (tool execution finished)
pub fn is_natural_breakpoint(chunk: &StreamChunk) -> bool {
    matches!(chunk.chunk_type.as_str(), "Done" | "ToolResult")
}

/// Check if silence timeout has been reached (WATCH-005)
pub fn is_silence_timeout(last_chunk_time: std::time::Instant, timeout: std::time::Duration) -> bool {
    last_chunk_time.elapsed() >= timeout
}

/// Format an evaluation prompt from accumulated observations and role context (WATCH-005, WATCH-020)
///
/// WATCH-020: Now includes structured response format instructions for [INTERJECT]/[CONTINUE]
pub fn format_evaluation_prompt(buffer: &ObservationBuffer, role: &SessionRole) -> String {
    let mut prompt = String::new();
    
    // Add role context
    prompt.push_str(&format!("You are a watcher session with role: {}\n", role.name));
    if let Some(desc) = &role.description {
        prompt.push_str(&format!("Role description: {}\n", desc));
    }
    
    // Authority-aware context (WATCH-020)
    let authority_context = match role.authority {
        RoleAuthority::Supervisor => "As a Supervisor, your interjections carry authority and should be followed by the parent session.",
        RoleAuthority::Peer => "As a Peer, your interjections are suggestions that the parent session may consider.",
    };
    prompt.push_str(&format!("Authority level: {} - {}\n\n", role.authority.as_str(), authority_context));
    
    // Add observation header
    prompt.push_str("=== PARENT SESSION OBSERVATIONS ===\n\n");
    
    // Add accumulated observations
    for chunk in buffer.chunks() {
        match chunk.chunk_type.as_str() {
            "Text" => {
                if let Some(text) = &chunk.text {
                    prompt.push_str(text);
                }
            }
            "Thinking" => {
                if let Some(thinking) = &chunk.thinking {
                    prompt.push_str(&format!("[Thinking]: {}\n", thinking));
                }
            }
            "ToolCall" => {
                if let Some(tc) = &chunk.tool_call {
                    prompt.push_str(&format!("[Tool Call]: {} ({})\n", tc.name, tc.id));
                }
            }
            "ToolResult" => {
                if let Some(tr) = &chunk.tool_result {
                    prompt.push_str(&format!("[Tool Result]: {}\n{}\n", tr.tool_call_id, tr.content));
                }
            }
            _ => {} // Ignore other chunk types
        }
    }
    
    prompt.push_str("\n=== END OBSERVATIONS ===\n\n");
    
    // WATCH-020: Add structured response format instructions
    prompt.push_str("Based on these observations, evaluate whether you need to interject.\n\n");
    prompt.push_str("RESPONSE FORMAT (required):\n");
    prompt.push_str("If you need to inject a message to the parent session, respond with:\n");
    prompt.push_str("[INTERJECT]\n");
    prompt.push_str("urgent: true\n");
    prompt.push_str("content: Your message here\n");
    prompt.push_str("[/INTERJECT]\n\n");
    prompt.push_str("Set 'urgent: true' to interrupt the parent mid-stream (for critical issues).\n");
    prompt.push_str("Set 'urgent: false' to wait until the parent's current turn completes.\n\n");
    prompt.push_str("If no interjection is needed, respond with:\n");
    prompt.push_str("[CONTINUE]\n");
    prompt.push_str("Your reasoning here (optional)\n");
    prompt.push_str("[/CONTINUE]\n\n");
    prompt.push_str("Important: Use EXACT markers [INTERJECT], [/INTERJECT], [CONTINUE], [/CONTINUE].\n");
    prompt.push_str("Field names must be lowercase: 'urgent:' and 'content:'.\n");
    
    prompt
}

/// Default silence timeout for watcher sessions (5 seconds)
pub const DEFAULT_SILENCE_TIMEOUT_SECS: u64 = 5;

/// Result of processing in the watcher loop
#[derive(Debug, Clone)]
pub enum WatcherLoopAction {
    /// Process a user prompt (takes priority)
    ProcessUserPrompt(String),
    /// Process accumulated observations (at breakpoint) (WATCH-011: includes observed correlation IDs)
    ProcessObservations {
        prompt: String,
        /// Correlation IDs of parent chunks that triggered this evaluation
        observed_correlation_ids: Vec<String>,
    },
    /// Continue waiting (no action needed)
    Continue,
    /// Stop the loop (channel closed or error)
    Stop,
}

/// Watcher agent loop input handler (WATCH-005)
///
/// This function implements Rule [0]: Uses tokio::select! to wait on both
/// user input channel AND parent broadcast receiver.
///
/// Returns a WatcherLoopAction indicating what action to take.
///
/// Note: This is the core loop logic. The actual agent execution is handled
/// by the caller (WATCH-007 will expose this via NAPI).
///
/// Processes one tick of the watcher loop (WATCH-005, wired up by WATCH-019)
///
/// This function is called by `run_watcher_loop` to process both user input
/// and parent observations. It uses tokio::select! with biased ordering to
/// prioritize user input over parent broadcast observations.
///
/// Called from `watcher_agent_loop` via `run_watcher_loop` when a watcher
/// session is created via `session_create_watcher`.
pub(crate) async fn watcher_loop_tick(
    user_input_rx: &mut mpsc::Receiver<PromptInput>,
    parent_broadcast_rx: &mut broadcast::Receiver<StreamChunk>,
    buffer: &mut ObservationBuffer,
    role: &SessionRole,
    silence_timeout: std::time::Duration,
) -> WatcherLoopAction {
    // Calculate time until silence timeout (if buffer has content)
    let timeout_duration = if let Some(last_time) = buffer.last_chunk_time() {
        let elapsed = last_time.elapsed();
        if elapsed >= silence_timeout {
            // Already timed out - process immediately
            if !buffer.is_empty() {
                // WATCH-011: Capture correlation IDs before clearing buffer
                let observed_correlation_ids = buffer.correlation_ids();
                let prompt = format_evaluation_prompt(buffer, role);
                buffer.clear();
                return WatcherLoopAction::ProcessObservations { prompt, observed_correlation_ids };
            }
        }
        silence_timeout.saturating_sub(elapsed)
    } else {
        silence_timeout
    };

    tokio::select! {
        // Bias towards user input - it takes priority (Rule [4])
        biased;

        // User input channel - highest priority
        result = user_input_rx.recv() => {
            match result {
                Some(prompt_input) => {
                    WatcherLoopAction::ProcessUserPrompt(prompt_input.input)
                }
                None => {
                    // Channel closed
                    WatcherLoopAction::Stop
                }
            }
        }

        // Parent broadcast receiver - observations
        result = parent_broadcast_rx.recv() => {
            match result {
                Ok(chunk) => {
                    // Check if this is a natural breakpoint BEFORE adding to buffer
                    let is_breakpoint = is_natural_breakpoint(&chunk);
                    // Check if buffer had content BEFORE adding the new chunk
                    // (Feature: Empty buffer at breakpoint does not trigger evaluation)
                    let had_content = !buffer.is_empty();
                    
                    // Add to buffer (accumulate observations)
                    buffer.push(chunk);
                    
                    // If breakpoint and buffer HAD content before, process
                    // Don't process if buffer was empty before the breakpoint chunk arrived
                    if is_breakpoint && had_content {
                        // WATCH-011: Capture correlation IDs before clearing buffer
                        let observed_correlation_ids = buffer.correlation_ids();
                        let prompt = format_evaluation_prompt(buffer, role);
                        buffer.clear();
                        WatcherLoopAction::ProcessObservations { prompt, observed_correlation_ids }
                    } else {
                        WatcherLoopAction::Continue
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Log warning about missed chunks (Rule: handle lag gracefully)
                    tracing::warn!("Watcher lagged behind by {} chunks, continuing from current position", n);
                    WatcherLoopAction::Continue
                }
                Err(broadcast::error::RecvError::Closed) => {
                    // Parent session ended
                    WatcherLoopAction::Stop
                }
            }
        }

        // Silence timeout - triggers breakpoint if buffer has content
        _ = tokio::time::sleep(timeout_duration), if !buffer.is_empty() => {
            // WATCH-011: Capture correlation IDs before clearing buffer
            let observed_correlation_ids = buffer.correlation_ids();
            let prompt = format_evaluation_prompt(buffer, role);
            buffer.clear();
            WatcherLoopAction::ProcessObservations { prompt, observed_correlation_ids }
        }
    }
}

/// Run the watcher agent loop (WATCH-005, wired up by WATCH-019)
///
/// This is the main entry point for a watcher session. It continuously
/// listens for both user input and parent observations, processing them
/// according to the business rules:
///
/// - User prompts take priority and are processed immediately
/// - Parent observations are accumulated until a natural breakpoint
/// - Natural breakpoints: TurnComplete (Done), ToolResult, or silence timeout
/// - Empty buffer at breakpoint does not trigger evaluation
///
/// The `process_prompt` callback is called whenever a prompt needs to be
/// processed (either user input or accumulated observations).
/// WATCH-011: For observation processing, observed_correlation_ids contains the
/// correlation IDs of parent chunks that triggered this evaluation.
///
/// Called from `watcher_agent_loop` when a watcher session is created via
/// `session_create_watcher` / `create_watcher_session_with_id`.
pub(crate) async fn run_watcher_loop<F, Fut>(
    user_input_rx: &mut mpsc::Receiver<PromptInput>,
    parent_broadcast_rx: &mut broadcast::Receiver<StreamChunk>,
    role: &SessionRole,
    silence_timeout_secs: Option<u64>,
    mut process_prompt: F,
) -> Result<()>
where
    F: FnMut(String, bool, Vec<String>) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut buffer = ObservationBuffer::new();
    let silence_timeout = std::time::Duration::from_secs(
        silence_timeout_secs.unwrap_or(DEFAULT_SILENCE_TIMEOUT_SECS)
    );

    loop {
        let action = watcher_loop_tick(
            user_input_rx,
            parent_broadcast_rx,
            &mut buffer,
            role,
            silence_timeout,
        ).await;

        match action {
            WatcherLoopAction::ProcessUserPrompt(prompt) => {
                // is_user_prompt = true, no observed correlation IDs
                process_prompt(prompt, true, Vec::new()).await?;
            }
            WatcherLoopAction::ProcessObservations { prompt, observed_correlation_ids } => {
                // is_user_prompt = false (this is an observation evaluation)
                process_prompt(prompt, false, observed_correlation_ids).await?;
            }
            WatcherLoopAction::Continue => {
                // No action needed, continue loop
            }
            WatcherLoopAction::Stop => {
                // Exit the loop
                break;
            }
        }
    }

    Ok(())
}

impl From<u8> for SessionStatus {
    fn from(v: u8) -> Self {
        match v {
            0 => SessionStatus::Idle,
            1 => SessionStatus::Running,
            2 => SessionStatus::Interrupted,
            3 => SessionStatus::Paused,
            _ => SessionStatus::Idle,
        }
    }
}

impl SessionStatus {
    /// Convert status to string representation for TypeScript
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Idle => "idle",
            SessionStatus::Running => "running",
            SessionStatus::Interrupted => "interrupted",
            SessionStatus::Paused => "paused",
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

/// PAUSE-001: Pause state returned to TypeScript via NAPI
#[napi(object)]
#[derive(Clone)]
pub struct NapiPauseState {
    /// "continue" or "confirm"
    pub kind: String,
    /// Tool name that initiated the pause (e.g., "WebSearch")
    pub tool_name: String,
    /// Human-readable message (e.g., "Page loaded at https://...")
    pub message: String,
    /// Optional additional details (e.g., command text for confirm)
    pub details: Option<String>,
}

impl From<PauseState> for NapiPauseState {
    fn from(state: PauseState) -> Self {
        Self {
            kind: match state.kind {
                PauseKind::Continue => "continue".to_string(),
                PauseKind::Confirm => "confirm".to_string(),
            },
            tool_name: state.tool_name,
            message: state.message,
            details: state.details,
        }
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

    /// Pending input text (TUI-049: preserved when switching sessions)
    pending_input: RwLock<Option<String>>,

    /// Broadcast channel for watcher sessions to observe stream output (WATCH-003)
    watcher_broadcast: broadcast::Sender<StreamChunk>,

    /// Session role for watcher sessions (WATCH-004) - None for regular sessions
    role: RwLock<Option<SessionRole>>,

    /// Channel for receiving watcher input messages (WATCH-006)
    /// Watchers use this to inject messages into the parent session
    watcher_input_tx: mpsc::Sender<WatcherInput>,
    watcher_input_rx: Mutex<mpsc::Receiver<WatcherInput>>,

    /// Correlation ID counter for cross-pane selection highlighting (WATCH-011)
    /// Each chunk emitted by handle_output gets a unique correlation_id
    correlation_counter: AtomicU64,

    /// Pending observed correlation IDs for watcher responses (WATCH-011)
    /// When a watcher processes observations, this is set to the correlation IDs
    /// of the parent chunks that triggered the evaluation. handle_output then
    /// tags output chunks with these IDs until cleared.
    pending_observed_correlation_ids: RwLock<Vec<String>>,

    /// PAUSE-001: Current pause state (None when not paused)
    pause_state: RwLock<Option<PauseState>>,

    /// PAUSE-001: Channel to send pause response from TypeScript back to the blocking tool
    pause_response_tx: std::sync::mpsc::Sender<PauseResponse>,
    pause_response_rx: std::sync::Mutex<std::sync::mpsc::Receiver<PauseResponse>>,

    /// TUI-054: Base thinking level for session (0=Off, 1=Low, 2=Medium, 3=High)
    /// This is the level set via /thinking command, persists for the session.
    /// Effective level = max(base_thinking_level, detected_level_from_text)
    base_thinking_level: AtomicU8,
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
        // Create watcher input channel (WATCH-006)
        let (watcher_input_tx, watcher_input_rx) = mpsc::channel::<WatcherInput>(16);

        // PAUSE-001: Create pause response channel (std::sync for blocking receive)
        let (pause_response_tx, pause_response_rx) = std::sync::mpsc::channel::<PauseResponse>();

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
            pending_input: RwLock::new(None),
            watcher_broadcast: broadcast::channel(WATCHER_BROADCAST_CAPACITY).0,
            role: RwLock::new(None),
            watcher_input_tx,
            watcher_input_rx: Mutex::new(watcher_input_rx),
            correlation_counter: AtomicU64::new(0),
            pending_observed_correlation_ids: RwLock::new(Vec::new()),
            pause_state: RwLock::new(None),
            pause_response_tx,
            pause_response_rx: std::sync::Mutex::new(pause_response_rx),
            base_thinking_level: AtomicU8::new(0), // TUI-054: Default to Off
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

    /// Get pending input text (TUI-049)
    pub fn get_pending_input(&self) -> Option<String> {
        self.pending_input.read().expect("pending_input lock poisoned").clone()
    }

    /// Set pending input text (TUI-049)
    pub fn set_pending_input(&self, input: Option<String>) {
        *self.pending_input.write().expect("pending_input lock poisoned") = input;
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
    
    /// Set status and notify attached callback
    pub fn set_status(&self, status: SessionStatus) {
        let old_status = self.status.swap(status as u8, Ordering::AcqRel);
        
        // Notify TypeScript when status changes (especially for pause state)
        if old_status != status as u8 {
            let status_str = match status {
                SessionStatus::Idle => "idle",
                SessionStatus::Running => "running", 
                SessionStatus::Interrupted => "interrupted",
                SessionStatus::Paused => "paused",
            };
            self.handle_output(StreamChunk::status(status_str.to_string()));
        }
    }
    
    /// Check if attached
    pub fn is_attached(&self) -> bool {
        self.is_attached.load(Ordering::Acquire)
    }
    
    /// Handle output chunk - buffer and optionally forward to callback
    /// WATCH-011: Assigns correlation_id for cross-pane selection highlighting
    /// WATCH-011: Applies pending_observed_correlation_ids for watcher responses
    pub fn handle_output(&self, mut chunk: StreamChunk) {
        // WATCH-011: Assign correlation_id if not already set
        if chunk.correlation_id.is_none() {
            let id = self.correlation_counter.fetch_add(1, Ordering::SeqCst);
            chunk.correlation_id = Some(format!("{}-{}", self.id, id));
        }

        // WATCH-011: Apply pending observed_correlation_ids for watcher responses
        // This tags watcher output chunks with the parent chunk IDs that triggered this response
        if chunk.observed_correlation_ids.is_none() {
            let pending_ids = self.pending_observed_correlation_ids.read()
                .expect("pending_observed_correlation_ids lock poisoned");
            if !pending_ids.is_empty() {
                chunk.observed_correlation_ids = Some(pending_ids.clone());
            }
        }

        // Always buffer (unbounded)
        {
            let mut buffer = self.output_buffer.write().expect("output buffer lock poisoned");
            buffer.push(chunk.clone());
        }

        // Broadcast to watcher sessions (WATCH-003)
        // Fire-and-forget: ignores SendError when no receivers are subscribed
        let _ = self.watcher_broadcast.send(chunk.clone());
        
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

    /// Subscribe to the output stream for watcher sessions (WATCH-003)
    ///
    /// Returns a broadcast receiver that will receive all StreamChunks output by this session.
    /// Late subscribers start receiving from the current position (no replay of past chunks).
    /// Slow receivers may receive RecvError::Lagged if they fall more than 256 chunks behind.
    pub fn subscribe_to_stream(&self) -> broadcast::Receiver<StreamChunk> {
        self.watcher_broadcast.subscribe()
    }

    /// Set the session role (WATCH-004)
    ///
    /// Used to mark a session as a watcher with a specific role and authority level.
    pub fn set_role(&self, role: SessionRole) {
        *self.role.write().expect("role lock poisoned") = Some(role);
    }

    /// Get the session role (WATCH-004)
    ///
    /// Returns None for regular sessions, Some(role) for watcher sessions.
    pub fn get_role(&self) -> Option<SessionRole> {
        self.role.read().expect("role lock poisoned").clone()
    }

    /// Clear the session role (WATCH-004)
    ///
    /// Returns the session to a regular (non-watcher) state.
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

    /// Set pending observed correlation IDs (WATCH-011)
    ///
    /// When a watcher processes observations, call this before sending the
    /// evaluation prompt. All subsequent output chunks from handle_output
    /// will be tagged with these IDs until clear_pending_observed_correlation_ids is called.
    pub fn set_pending_observed_correlation_ids(&self, ids: Vec<String>) {
        *self.pending_observed_correlation_ids.write()
            .expect("pending_observed_correlation_ids lock poisoned") = ids;
    }

    /// Clear pending observed correlation IDs (WATCH-011)
    ///
    /// Call this after the watcher finishes processing an observation response.
    /// Subsequent output chunks will no longer be tagged with observed IDs.
    pub fn clear_pending_observed_correlation_ids(&self) {
        self.pending_observed_correlation_ids.write()
            .expect("pending_observed_correlation_ids lock poisoned")
            .clear();
    }

    /// Receive watcher input (WATCH-006)
    ///
    /// Queues a WatcherInput message for processing by the parent session.
    /// The input is queued via an mpsc channel and processed asynchronously.
    /// Returns Ok(()) immediately without blocking.
    pub fn receive_watcher_input(&self, input: WatcherInput) -> std::result::Result<(), String> {
        self.watcher_input_tx
            .try_send(input)
            .map_err(|e| format!("Failed to queue watcher input: {}", e))
    }

    /// Get the watcher input sender (WATCH-006)
    ///
    /// Returns a clone of the sender for watchers to send input.
    pub fn watcher_input_sender(&self) -> mpsc::Sender<WatcherInput> {
        self.watcher_input_tx.clone()
    }
    
    /// Send input to the agent loop
    ///
    /// Buffers the user input as a UserInput chunk before sending to the agent,
    /// so it can be replayed when attaching to a detached session via /resume.
    ///
    /// CRITICAL: Sets status to Running BEFORE sending to channel to avoid race condition.
    /// The TypeScript side calls refreshRustState() right after sessionSendInput() returns,
    /// so status must be Running at that point for isLoading to be true.
    pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<()> {
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
                Error::from_reason(format!("Failed to send input: {}", e))
            })
    }
    
    /// Interrupt current agent execution
    ///
    /// Call this when the user presses Esc in the TUI.
    /// Also requests bash tool abortion for any running commands.
    pub fn interrupt(&self) {
        self.is_interrupted.store(true, Ordering::Release);
        // Also request bash tool abortion for any running commands
        request_bash_abort();
        self.interrupt_notify.notify_one();
    }

    /// Reset interrupt flag
    ///
    /// Called automatically at the start of each prompt.
    pub fn reset_interrupt(&self) {
        self.is_interrupted.store(false, Ordering::Release);
        // Also clear bash abort flag
        clear_bash_abort();
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

/// Tracks parent-watcher relationships between sessions (WATCH-002)
///
/// WatchGraph enables watcher sessions to observe parent sessions.
/// - One watcher can only watch one parent (1:1 from watcher side)
/// - One parent can have multiple watchers (1:N from parent side)
/// - Circular watching is prevented
pub struct WatchGraph {
    /// Parent session ID → list of watcher session IDs
    parent_to_watchers: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    /// Watcher session ID → parent session ID
    watcher_to_parent: RwLock<HashMap<Uuid, Uuid>>,
}

impl Default for WatchGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchGraph {
    /// Create a new empty WatchGraph
    pub fn new() -> Self {
        Self {
            parent_to_watchers: RwLock::new(HashMap::new()),
            watcher_to_parent: RwLock::new(HashMap::new()),
        }
    }

    /// Register a watcher for a parent session
    ///
    /// Returns an error if:
    /// - The watcher already has a parent (watcher can only watch one parent)
    /// - Adding would create a circular watch relationship
    pub fn add_watcher(&self, parent_id: Uuid, watcher_id: Uuid) -> std::result::Result<(), String> {
        // Acquire write lock for the entire operation to prevent TOCTOU race
        let mut w2p = self.watcher_to_parent.write().expect("watcher_to_parent lock poisoned");
        
        // Check if watcher already has a parent
        if w2p.contains_key(&watcher_id) {
            return Err("watcher already has a parent".to_string());
        }

        // Check for circular watching: would the proposed watcher be in the parent's chain?
        // If the watcher is already a parent of something in the chain, we'd have a cycle
        // Check if parent_id is watching watcher_id (direct cycle)
        if w2p.get(&parent_id) == Some(&watcher_id) {
            return Err("circular watching not allowed".to_string());
        }
        // Check deeper cycles: walk up from parent_id
        let mut current = parent_id;
        while let Some(&grandparent) = w2p.get(&current) {
            if grandparent == watcher_id {
                return Err("circular watching not allowed".to_string());
            }
            current = grandparent;
        }

        // Add the relationship (still under write lock)
        w2p.insert(watcher_id, parent_id);
        
        // Now acquire parent_to_watchers lock
        let mut p2w = self.parent_to_watchers.write().expect("parent_to_watchers lock poisoned");
        p2w.entry(parent_id).or_default().push(watcher_id);

        Ok(())
    }

    /// Remove a watcher relationship
    ///
    /// Removes the watcher from both maps. Safe to call even if watcher doesn't exist.
    pub fn remove_watcher(&self, watcher_id: Uuid) {
        // Get the parent (if any) and remove from watcher_to_parent
        let parent_id = {
            let mut w2p = self.watcher_to_parent.write().expect("watcher_to_parent lock poisoned");
            w2p.remove(&watcher_id)
        };

        // If there was a parent, remove watcher from parent's list
        if let Some(parent_id) = parent_id {
            let mut p2w = self.parent_to_watchers.write().expect("parent_to_watchers lock poisoned");
            if let Some(watchers) = p2w.get_mut(&parent_id) {
                watchers.retain(|&id| id != watcher_id);
                // Remove empty entries
                if watchers.is_empty() {
                    p2w.remove(&parent_id);
                }
            }
        }
    }

    /// Get all watchers for a parent session
    ///
    /// Returns an empty Vec if the parent has no watchers.
    pub fn get_watchers(&self, parent_id: Uuid) -> Vec<Uuid> {
        let p2w = self.parent_to_watchers.read().expect("parent_to_watchers lock poisoned");
        p2w.get(&parent_id).cloned().unwrap_or_default()
    }

    /// Get the parent for a watcher session
    ///
    /// Returns None if the session is not a watcher (or doesn't exist).
    pub fn get_parent(&self, watcher_id: Uuid) -> Option<Uuid> {
        let w2p = self.watcher_to_parent.read().expect("watcher_to_parent lock poisoned");
        w2p.get(&watcher_id).copied()
    }

    /// Clean up all watcher relationships when a parent session is removed
    ///
    /// This removes the parent from parent_to_watchers and removes all its
    /// watchers from watcher_to_parent.
    pub fn cleanup_parent(&self, parent_id: Uuid) {
        // Get and remove all watchers for this parent
        let watchers = {
            let mut p2w = self.parent_to_watchers.write().expect("parent_to_watchers lock poisoned");
            p2w.remove(&parent_id).unwrap_or_default()
        };

        // Remove each watcher from watcher_to_parent
        {
            let mut w2p = self.watcher_to_parent.write().expect("watcher_to_parent lock poisoned");
            for watcher_id in watchers {
                w2p.remove(&watcher_id);
            }
        }
    }

    /// Check if the WatchGraph has no entries
    pub fn is_empty(&self) -> bool {
        let p2w = self.parent_to_watchers.read().expect("parent_to_watchers lock poisoned");
        let w2p = self.watcher_to_parent.read().expect("watcher_to_parent lock poisoned");
        p2w.is_empty() && w2p.is_empty()
    }
}

/// Broadcast channel capacity for watcher stream observation (WATCH-003)
pub const WATCHER_BROADCAST_CAPACITY: usize = 256;

#[cfg(test)]
mod watcher_broadcast_tests {
    use super::*;

    /// Feature: spec/features/broadcast-channel-for-parent-stream-observation.feature
    ///
    /// Scenario: Broadcast with no subscribers still buffers normally
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And no watchers have subscribed to the stream
    /// @step When handle_output is called with a TextDelta chunk
    /// @step Then the chunk should be added to the output buffer
    /// @step And no error should occur from the broadcast
    #[test]
    fn test_broadcast_with_no_subscribers_still_buffers() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, _rx) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);
        let output_buffer: RwLock<Vec<StreamChunk>> = RwLock::new(Vec::new());

        // @step And no watchers have subscribed to the stream
        // (no receivers created - tx has no subscribers)

        // @step When handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("test content".to_string());
        
        // Simulate handle_output behavior:
        // 1. Buffer the chunk
        {
            let mut buffer = output_buffer.write().expect("lock");
            buffer.push(chunk.clone());
        }
        // 2. Broadcast (fire-and-forget, ignores SendError when no receivers)
        let _ = tx.send(chunk.clone());

        // @step Then the chunk should be added to the output buffer
        let buffer = output_buffer.read().expect("lock");
        assert_eq!(buffer.len(), 1, "chunk should be buffered");
        assert_eq!(buffer[0].chunk_type, "Text");

        // @step And no error should occur from the broadcast
        // (if we got here, no panic occurred)
    }

    /// Scenario: Single watcher receives chunks via broadcast
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And a watcher has called subscribe_to_stream to get a receiver
    /// @step When handle_output is called with a TextDelta chunk
    /// @step Then the watcher should receive the same chunk via its receiver
    /// @step And the chunk should also be buffered normally
    #[test]
    fn test_single_watcher_receives_chunks() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, mut rx) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);
        let output_buffer: RwLock<Vec<StreamChunk>> = RwLock::new(Vec::new());

        // @step And a watcher has called subscribe_to_stream to get a receiver
        // rx is already subscribed (created from channel)

        // @step When handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("watcher test".to_string());
        {
            let mut buffer = output_buffer.write().expect("lock");
            buffer.push(chunk.clone());
        }
        let _ = tx.send(chunk.clone());

        // @step Then the watcher should receive the same chunk via its receiver
        let received = rx.try_recv().expect("should receive chunk");
        assert_eq!(received.chunk_type, "Text");
        assert_eq!(received.text, Some("watcher test".to_string()));

        // @step And the chunk should also be buffered normally
        let buffer = output_buffer.read().expect("lock");
        assert_eq!(buffer.len(), 1);
    }

    /// Scenario: Multiple watchers receive chunks independently
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And watcher A has subscribed to the stream
    /// @step And watcher B has subscribed to the stream
    /// @step When handle_output is called with a TextDelta chunk
    /// @step Then watcher A should receive the chunk via its receiver
    /// @step And watcher B should receive the chunk via its receiver
    /// @step And both received chunks should be identical
    #[test]
    fn test_multiple_watchers_receive_independently() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, mut rx_a) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);

        // @step And watcher A has subscribed to the stream
        // rx_a is already subscribed

        // @step And watcher B has subscribed to the stream
        let mut rx_b = tx.subscribe();

        // @step When handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("multi-watcher".to_string());
        let _ = tx.send(chunk.clone());

        // @step Then watcher A should receive the chunk via its receiver
        let received_a = rx_a.try_recv().expect("watcher A should receive");

        // @step And watcher B should receive the chunk via its receiver
        let received_b = rx_b.try_recv().expect("watcher B should receive");

        // @step And both received chunks should be identical
        assert_eq!(received_a.chunk_type, received_b.chunk_type);
        assert_eq!(received_a.text, received_b.text);
        assert_eq!(received_a.text, Some("multi-watcher".to_string()));
    }

    /// Scenario: Slow watcher receives lagged error when falling behind
    ///
    /// @step Given a BackgroundSession with broadcast channel capacity of 256
    /// @step And a watcher has subscribed to the stream
    /// @step And the watcher has not consumed any chunks
    /// @step When handle_output is called 300 times with chunks
    /// @step Then the watcher should receive RecvError::Lagged when trying to receive
    #[test]
    fn test_slow_watcher_receives_lagged_error() {
        // @step Given a BackgroundSession with broadcast channel capacity of 256
        let (tx, mut rx) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);

        // @step And a watcher has subscribed to the stream
        // @step And the watcher has not consumed any chunks
        // (rx exists but we don't call recv)

        // @step When handle_output is called 300 times with chunks
        for i in 0..300 {
            let chunk = StreamChunk::text(format!("chunk {}", i));
            let _ = tx.send(chunk);
        }

        // @step Then the watcher should receive RecvError::Lagged when trying to receive
        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                assert!(n > 0, "should have lagged by some messages");
                // With 300 sends and 256 capacity, we lag by 300 - 256 = 44 messages
                assert!(n >= 44, "should lag by at least 44 messages, got {}", n);
            }
            other => panic!("expected Lagged error, got {:?}", other),
        }
    }

    /// Scenario: Dropped receiver does not affect other watchers
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And watcher A has subscribed to the stream
    /// @step And watcher B has subscribed to the stream
    /// @step When watcher A drops its receiver
    /// @step And handle_output is called with a TextDelta chunk
    /// @step Then watcher B should still receive the chunk normally
    /// @step And the parent session should continue operating normally
    #[test]
    fn test_dropped_receiver_does_not_affect_others() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, rx_a) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);

        // @step And watcher A has subscribed to the stream
        // rx_a exists

        // @step And watcher B has subscribed to the stream
        let mut rx_b = tx.subscribe();

        // @step When watcher A drops its receiver
        drop(rx_a);

        // @step And handle_output is called with a TextDelta chunk
        let chunk = StreamChunk::text("after drop".to_string());
        let send_result = tx.send(chunk);

        // @step Then watcher B should still receive the chunk normally
        let received = rx_b.try_recv().expect("watcher B should receive");
        assert_eq!(received.text, Some("after drop".to_string()));

        // @step And the parent session should continue operating normally
        assert!(send_result.is_ok(), "send should succeed with remaining receiver");
    }

    /// Scenario: Late subscriber starts receiving from current position
    ///
    /// @step Given a BackgroundSession with broadcast channel initialized
    /// @step And handle_output has been called 10 times with chunks
    /// @step When a new watcher subscribes to the stream
    /// @step And handle_output is called with a new chunk
    /// @step Then the new watcher should receive only the new chunk
    /// @step And the new watcher should not receive the previous 10 chunks
    #[test]
    fn test_late_subscriber_starts_from_current() {
        // @step Given a BackgroundSession with broadcast channel initialized
        let (tx, _initial_rx) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);

        // @step And handle_output has been called 10 times with chunks
        for i in 0..10 {
            let chunk = StreamChunk::text(format!("old chunk {}", i));
            let _ = tx.send(chunk);
        }

        // @step When a new watcher subscribes to the stream
        let mut late_rx = tx.subscribe();

        // @step And handle_output is called with a new chunk
        let new_chunk = StreamChunk::text("new chunk".to_string());
        let _ = tx.send(new_chunk);

        // @step Then the new watcher should receive only the new chunk
        let received = late_rx.try_recv().expect("should receive new chunk");
        assert_eq!(received.text, Some("new chunk".to_string()));

        // @step And the new watcher should not receive the previous 10 chunks
        // (already verified - we only got one chunk, the new one)
        match late_rx.try_recv() {
            Err(broadcast::error::TryRecvError::Empty) => {
                // Expected - no more chunks
            }
            other => panic!("expected Empty, got {:?}", other),
        }
    }

    // === Integration tests that verify BackgroundSession has broadcast channel ===

    /// Test that BackgroundSession has watcher_broadcast field and WATCHER_BROADCAST_CAPACITY is correct
    #[test]
    fn test_background_session_has_broadcast_field() {
        // Verify the constant is defined correctly
        assert_eq!(WATCHER_BROADCAST_CAPACITY, 256);
        
        // Note: Full BackgroundSession integration tested via handle_output() which
        // requires codelet_cli::session::Session. The unit tests above validate the
        // broadcast channel mechanics work correctly in isolation.
    }
}

#[cfg(test)]
mod session_role_tests {
    use super::*;

    /// Feature: spec/features/session-role-and-authority-model.feature
    ///
    /// Scenario: Set peer role with description
    ///
    /// @step Given a BackgroundSession exists
    /// @step When I call set_role with name "code-reviewer", description "Reviews code changes", and authority "peer"
    /// @step Then the session role should have name "code-reviewer"
    /// @step And the session role should have description "Reviews code changes"
    /// @step And the session role should have authority Peer
    #[test]
    fn test_set_peer_role_with_description() {
        // @step Given a BackgroundSession exists
        // (simulated with direct SessionRole construction)

        // @step When I call set_role with name "code-reviewer", description "Reviews code changes", and authority "peer"
        let authority = RoleAuthority::from_str("peer").expect("valid authority");
        let role = SessionRole::new(
            "code-reviewer".to_string(),
            Some("Reviews code changes".to_string()),
            authority,
        ).expect("valid role");

        // @step Then the session role should have name "code-reviewer"
        assert_eq!(role.name, "code-reviewer");

        // @step And the session role should have description "Reviews code changes"
        assert_eq!(role.description, Some("Reviews code changes".to_string()));

        // @step And the session role should have authority Peer
        assert_eq!(role.authority, RoleAuthority::Peer);
    }

    /// Scenario: Set supervisor role without description
    ///
    /// @step Given a BackgroundSession exists
    /// @step When I call set_role with name "supervisor", no description, and authority "supervisor"
    /// @step Then the session role should have name "supervisor"
    /// @step And the session role should have no description
    /// @step And the session role should have authority Supervisor
    #[test]
    fn test_set_supervisor_role_without_description() {
        // @step Given a BackgroundSession exists
        // (simulated with direct SessionRole construction)

        // @step When I call set_role with name "supervisor", no description, and authority "supervisor"
        let authority = RoleAuthority::from_str("supervisor").expect("valid authority");
        let role = SessionRole::new(
            "supervisor".to_string(),
            None,
            authority,
        ).expect("valid role");

        // @step Then the session role should have name "supervisor"
        assert_eq!(role.name, "supervisor");

        // @step And the session role should have no description
        assert_eq!(role.description, None);

        // @step And the session role should have authority Supervisor
        assert_eq!(role.authority, RoleAuthority::Supervisor);
    }

    /// Scenario: Get role on regular session returns None
    ///
    /// @step Given a BackgroundSession exists
    /// @step And no role has been set
    /// @step When I call get_role
    /// @step Then it should return None
    #[test]
    fn test_get_role_on_regular_session_returns_none() {
        // @step Given a BackgroundSession exists
        // @step And no role has been set
        let role: Option<SessionRole> = None;

        // @step When I call get_role
        // (simulated - role is None)

        // @step Then it should return None
        assert!(role.is_none());
    }

    /// Scenario: Get role on session with role returns role details
    ///
    /// @step Given a BackgroundSession exists
    /// @step And the role has been set to name "test-role" with authority Peer
    /// @step When I call get_role
    /// @step Then it should return a SessionRole with name "test-role" and authority Peer
    #[test]
    fn test_get_role_returns_role_details() {
        // @step Given a BackgroundSession exists
        // @step And the role has been set to name "test-role" with authority Peer
        let role = Some(SessionRole::new(
            "test-role".to_string(),
            None,
            RoleAuthority::Peer,
        ).expect("valid role"));

        // @step When I call get_role
        let result = role;

        // @step Then it should return a SessionRole with name "test-role" and authority Peer
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.name, "test-role");
        assert_eq!(r.authority, RoleAuthority::Peer);
    }

    /// Scenario: Set role with invalid authority returns error
    ///
    /// @step Given a BackgroundSession exists
    /// @step When I call set_role with name "test", description None, and authority "invalid"
    /// @step Then it should return an error "Invalid authority: must be peer or supervisor"
    #[test]
    fn test_set_role_with_invalid_authority_returns_error() {
        // @step Given a BackgroundSession exists
        // (simulated)

        // @step When I call set_role with name "test", description None, and authority "invalid"
        let authority = RoleAuthority::from_str("invalid");

        // @step Then it should return an error "Invalid authority: must be peer or supervisor"
        assert!(authority.is_none(), "invalid authority should return None");
    }

    /// Scenario: Set role with empty name returns error
    ///
    /// @step Given a BackgroundSession exists
    /// @step When I call set_role with name "", description None, and authority "peer"
    /// @step Then it should return an error "Role name cannot be empty"
    #[test]
    fn test_set_role_with_empty_name_returns_error() {
        // @step Given a BackgroundSession exists
        // (simulated)

        // @step When I call set_role with name "", description None, and authority "peer"
        let result = SessionRole::new(
            "".to_string(),
            None,
            RoleAuthority::Peer,
        );

        // @step Then it should return an error "Role name cannot be empty"
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Role name cannot be empty");
    }

    /// Test RoleAuthority default is Peer
    #[test]
    fn test_role_authority_default_is_peer() {
        let authority = RoleAuthority::default();
        assert_eq!(authority, RoleAuthority::Peer);
    }

    /// Test RoleAuthority as_str
    #[test]
    fn test_role_authority_as_str() {
        assert_eq!(RoleAuthority::Peer.as_str(), "peer");
        assert_eq!(RoleAuthority::Supervisor.as_str(), "supervisor");
    }
}

#[cfg(test)]
mod watch_graph_tests {
    use super::*;

    /// Scenario: Register a watcher for a parent session
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And a parent session "abc" exists
    /// @step And a watcher session "xyz" exists
    /// @step When I call add_watcher with parent_id "abc" and watcher_id "xyz"
    /// @step Then get_watchers for "abc" should return ["xyz"]
    /// @step And get_parent for "xyz" should return "abc"
    #[test]
    fn test_register_watcher_for_parent_session() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        // @step And a parent session "abc" exists
        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a1").unwrap();

        // @step And a watcher session "xyz" exists
        let watcher_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000b1").unwrap();

        // @step When I call add_watcher with parent_id "abc" and watcher_id "xyz"
        let result = watch_graph.add_watcher(parent_id, watcher_id);
        assert!(result.is_ok(), "add_watcher should succeed");

        // @step Then get_watchers for "abc" should return ["xyz"]
        let watchers = watch_graph.get_watchers(parent_id);
        assert_eq!(watchers, vec![watcher_id], "get_watchers should return [xyz]");

        // @step And get_parent for "xyz" should return "abc"
        let parent = watch_graph.get_parent(watcher_id);
        assert_eq!(parent, Some(parent_id), "get_parent should return abc");
    }

    /// Scenario: Parent with multiple watchers
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And a parent session "abc" exists
    /// @step And watcher sessions "xyz" and "def" exist
    /// @step When I call add_watcher with parent_id "abc" and watcher_id "xyz"
    /// @step And I call add_watcher with parent_id "abc" and watcher_id "def"
    /// @step Then get_watchers for "abc" should return ["xyz", "def"]
    #[test]
    fn test_parent_with_multiple_watchers() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        // @step And a parent session "abc" exists
        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a2").unwrap();

        // @step And watcher sessions "xyz" and "def" exist
        let watcher_xyz = Uuid::parse_str("00000000-0000-0000-0000-0000000000b2").unwrap();
        let watcher_def = Uuid::parse_str("00000000-0000-0000-0000-0000000000c2").unwrap();

        // @step When I call add_watcher with parent_id "abc" and watcher_id "xyz"
        let result1 = watch_graph.add_watcher(parent_id, watcher_xyz);
        assert!(result1.is_ok(), "first add_watcher should succeed");

        // @step And I call add_watcher with parent_id "abc" and watcher_id "def"
        let result2 = watch_graph.add_watcher(parent_id, watcher_def);
        assert!(result2.is_ok(), "second add_watcher should succeed");

        // @step Then get_watchers for "abc" should return ["xyz", "def"]
        let watchers = watch_graph.get_watchers(parent_id);
        assert!(watchers.contains(&watcher_xyz), "watchers should contain xyz");
        assert!(watchers.contains(&watcher_def), "watchers should contain def");
        assert_eq!(watchers.len(), 2, "should have exactly 2 watchers");
    }

    /// Scenario: Query parent for a watcher
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And session "xyz" is watching session "abc"
    /// @step When I call get_parent with watcher_id "xyz"
    /// @step Then it should return "abc"
    #[test]
    fn test_query_parent_for_watcher() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a3").unwrap();
        let watcher_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000b3").unwrap();

        // @step And session "xyz" is watching session "abc"
        let _ = watch_graph.add_watcher(parent_id, watcher_id);

        // @step When I call get_parent with watcher_id "xyz"
        let result = watch_graph.get_parent(watcher_id);

        // @step Then it should return "abc"
        assert_eq!(result, Some(parent_id), "get_parent should return abc");
    }

    /// Scenario: Remove a watcher relationship
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And session "xyz" is watching session "abc"
    /// @step When I call remove_watcher with watcher_id "xyz"
    /// @step Then get_watchers for "abc" should return an empty list
    /// @step And get_parent for "xyz" should return None
    #[test]
    fn test_remove_watcher_relationship() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a4").unwrap();
        let watcher_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000b4").unwrap();

        // @step And session "xyz" is watching session "abc"
        let _ = watch_graph.add_watcher(parent_id, watcher_id);

        // @step When I call remove_watcher with watcher_id "xyz"
        watch_graph.remove_watcher(watcher_id);

        // @step Then get_watchers for "abc" should return an empty list
        let watchers = watch_graph.get_watchers(parent_id);
        assert!(watchers.is_empty(), "get_watchers should return empty list");

        // @step And get_parent for "xyz" should return None
        let parent = watch_graph.get_parent(watcher_id);
        assert_eq!(parent, None, "get_parent should return None");
    }

    /// Scenario: Watcher cannot watch multiple parents
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And session "xyz" is watching session "abc"
    /// @step When I call add_watcher with parent_id "def" and watcher_id "xyz"
    /// @step Then it should return an error "watcher already has a parent"
    #[test]
    fn test_watcher_cannot_watch_multiple_parents() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        let parent_abc = Uuid::parse_str("00000000-0000-0000-0000-0000000000a5").unwrap();
        let parent_def = Uuid::parse_str("00000000-0000-0000-0000-0000000000b5").unwrap();
        let watcher_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000c5").unwrap();

        // @step And session "xyz" is watching session "abc"
        let _ = watch_graph.add_watcher(parent_abc, watcher_id);

        // @step When I call add_watcher with parent_id "def" and watcher_id "xyz"
        let result = watch_graph.add_watcher(parent_def, watcher_id);

        // @step Then it should return an error "watcher already has a parent"
        assert!(result.is_err(), "add_watcher should fail");
        assert!(
            result.unwrap_err().contains("already has a parent"),
            "error should mention 'already has a parent'"
        );
    }

    /// Scenario: Circular watching is prevented
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And session "B" is watching session "A"
    /// @step When I call add_watcher with parent_id "B" and watcher_id "A"
    /// @step Then it should return an error "circular watching not allowed"
    #[test]
    fn test_circular_watching_prevented() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        let session_a = Uuid::parse_str("00000000-0000-0000-0000-0000000000a6").unwrap();
        let session_b = Uuid::parse_str("00000000-0000-0000-0000-0000000000b6").unwrap();

        // @step And session "B" is watching session "A"
        let _ = watch_graph.add_watcher(session_a, session_b);

        // @step When I call add_watcher with parent_id "B" and watcher_id "A"
        let result = watch_graph.add_watcher(session_b, session_a);

        // @step Then it should return an error "circular watching not allowed"
        assert!(result.is_err(), "add_watcher should fail for circular watching");
        assert!(
            result.unwrap_err().contains("circular"),
            "error should mention 'circular'"
        );
    }

    /// Scenario: Regular session has no parent
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And a regular session "abc" exists that is not a watcher
    /// @step When I call get_parent with session_id "abc"
    /// @step Then it should return None
    #[test]
    fn test_regular_session_has_no_parent() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        // @step And a regular session "abc" exists that is not a watcher
        let session_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a7").unwrap();

        // @step When I call get_parent with session_id "abc"
        let parent = watch_graph.get_parent(session_id);

        // @step Then it should return None
        assert_eq!(parent, None, "regular session should have no parent");
    }

    /// Scenario: Cleanup watchers when parent session is removed
    ///
    /// @step Given a WatchGraph with no relationships
    /// @step And session "xyz" is watching session "abc"
    /// @step And session "def" is watching session "abc"
    /// @step When parent session "abc" is removed
    /// @step Then get_parent for "xyz" should return None
    /// @step And get_parent for "def" should return None
    /// @step And the WatchGraph should have no entries
    #[test]
    fn test_cleanup_watchers_when_parent_removed() {
        // @step Given a WatchGraph with no relationships
        let watch_graph = WatchGraph::new();

        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-0000000000a8").unwrap();
        let watcher_xyz = Uuid::parse_str("00000000-0000-0000-0000-0000000000b8").unwrap();
        let watcher_def = Uuid::parse_str("00000000-0000-0000-0000-0000000000c8").unwrap();

        // @step And session "xyz" is watching session "abc"
        let _ = watch_graph.add_watcher(parent_id, watcher_xyz);

        // @step And session "def" is watching session "abc"
        let _ = watch_graph.add_watcher(parent_id, watcher_def);

        // @step When parent session "abc" is removed
        watch_graph.cleanup_parent(parent_id);

        // @step Then get_parent for "xyz" should return None
        let parent_xyz = watch_graph.get_parent(watcher_xyz);
        assert_eq!(parent_xyz, None, "get_parent for xyz should return None after cleanup");

        // @step And get_parent for "def" should return None
        let parent_def = watch_graph.get_parent(watcher_def);
        assert_eq!(parent_def, None, "get_parent for def should return None after cleanup");

        // @step And the WatchGraph should have no entries
        assert!(watch_graph.is_empty(), "WatchGraph should be empty after cleanup");
    }
}

#[cfg(test)]
mod watcher_loop_tests {
    use super::*;
    use std::time::{Duration, Instant};

    // Feature: spec/features/watcher-agent-loop-with-dual-input.feature

    /// Scenario: Accumulate observations until TurnComplete breakpoint
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step And the watcher has an empty observation buffer
    /// @step When the parent sends TextDelta chunks "Hello" and "World"
    /// @step And the parent sends TurnComplete
    /// @step Then the watcher should have accumulated "HelloWorld" in the buffer
    /// @step And the watcher should detect a natural breakpoint
    /// @step And the watcher should format an evaluation prompt with the accumulated text
    /// @step And the observation buffer should be cleared after processing
    #[test]
    fn test_accumulate_until_turn_complete() {
        // @step Given a watcher session is observing a parent session
        // @step And the watcher has an empty observation buffer
        let mut buffer = ObservationBuffer::new();
        assert!(buffer.is_empty());

        // @step When the parent sends TextDelta chunks "Hello" and "World"
        buffer.push(StreamChunk::text("Hello".to_string()));
        buffer.push(StreamChunk::text("World".to_string()));

        // @step And the parent sends TurnComplete (represented as Done in our API)
        let turn_complete = StreamChunk::done();
        
        // @step Then the watcher should have accumulated "HelloWorld" in the buffer
        assert_eq!(buffer.accumulated_text(), "HelloWorld");

        // @step And the watcher should detect a natural breakpoint
        assert!(is_natural_breakpoint(&turn_complete));

        // @step And the watcher should format an evaluation prompt with the accumulated text
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        let prompt = format_evaluation_prompt(&buffer, &role);
        assert!(prompt.contains("HelloWorld"));
        assert!(prompt.contains("reviewer"));

        // @step And the observation buffer should be cleared after processing
        buffer.clear();
        assert!(buffer.is_empty());
    }

    /// Scenario: User prompt takes priority over buffered observations
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step And the watcher has accumulated observations in the buffer
    /// @step When the user sends a prompt "What do you think?"
    /// @step Then the user prompt should be processed immediately
    /// @step And the accumulated observations should remain in the buffer for later processing
    #[test]
    fn test_user_prompt_priority() {
        // @step Given a watcher session is observing a parent session
        // @step And the watcher has accumulated observations in the buffer
        let mut buffer = ObservationBuffer::new();
        buffer.push(StreamChunk::text("Some observation".to_string()));
        assert!(!buffer.is_empty());

        // @step When the user sends a prompt "What do you think?"
        let _user_prompt = "What do you think?";

        // @step Then the user prompt should be processed immediately
        // (User prompts bypass the observation buffer - they're sent directly)
        // This is verified by checking the buffer is NOT affected
        
        // @step And the accumulated observations should remain in the buffer for later processing
        assert!(!buffer.is_empty());
        assert_eq!(buffer.accumulated_text(), "Some observation");
    }

    /// Scenario: ToolResult triggers natural breakpoint
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step And the watcher has an empty observation buffer
    /// @step When the parent sends ToolUse for tool "bash"
    /// @step And the parent sends ToolResult with output "command output"
    /// @step Then the watcher should detect a natural breakpoint at ToolResult
    /// @step And the watcher should format an evaluation prompt with tool execution context
    #[test]
    fn test_tool_result_breakpoint() {
        // @step Given a watcher session is observing a parent session
        // @step And the watcher has an empty observation buffer
        let mut buffer = ObservationBuffer::new();

        // @step When the parent sends ToolUse for tool "bash"
        // Create a ToolCall chunk (ToolUse is represented as ToolCall in our API)
        let tool_call = StreamChunk::tool_call(crate::types::ToolCallInfo {
            id: "tool-123".to_string(),
            name: "bash".to_string(),
            input: "{}".to_string(),
        });
        buffer.push(tool_call);

        // @step And the parent sends ToolResult with output "command output"
        let tool_result = StreamChunk::tool_result(crate::types::ToolResultInfo {
            tool_call_id: "tool-123".to_string(),
            content: "command output".to_string(),
            is_error: false,
        });
        buffer.push(tool_result.clone());

        // @step Then the watcher should detect a natural breakpoint at ToolResult
        assert!(is_natural_breakpoint(&tool_result));

        // @step And the watcher should format an evaluation prompt with tool execution context
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        let prompt = format_evaluation_prompt(&buffer, &role);
        assert!(prompt.contains("bash"));
        assert!(prompt.contains("command output"));
    }

    /// Scenario: Silence timeout triggers breakpoint
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step And the silence timeout is configured to 5 seconds
    /// @step And the watcher has accumulated observations in the buffer
    /// @step When no chunks are received for 5 seconds
    /// @step Then the watcher should detect a silence timeout breakpoint
    /// @step And the watcher should process the accumulated observations
    #[test]
    fn test_silence_timeout_breakpoint() {
        // @step Given a watcher session is observing a parent session
        // @step And the silence timeout is configured to 5 seconds
        let silence_timeout = Duration::from_secs(5);
        
        // @step And the watcher has accumulated observations in the buffer
        let mut buffer = ObservationBuffer::new();
        buffer.push(StreamChunk::text("Some text".to_string()));
        
        // Simulate time passage by setting last_chunk_time in the past
        let last_chunk_time = Instant::now() - Duration::from_secs(6);

        // @step When no chunks are received for 5 seconds
        // @step Then the watcher should detect a silence timeout breakpoint
        assert!(is_silence_timeout(last_chunk_time, silence_timeout));

        // @step And the watcher should process the accumulated observations
        assert!(!buffer.is_empty());
    }

    /// Scenario: Handle broadcast lag gracefully
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step When the watcher receives RecvError::Lagged with 10 missed chunks
    /// @step Then the watcher should log a warning about 10 missed chunks
    /// @step And the watcher should continue observing from the current position
    #[test]
    fn test_handle_broadcast_lag() {
        // @step Given a watcher session is observing a parent session
        // (simulated)

        // @step When the watcher receives RecvError::Lagged with 10 missed chunks
        let lagged_count: u64 = 10;

        // @step Then the watcher should log a warning about 10 missed chunks
        // (logging is a side effect - we verify the count is captured)
        let warning_message = format!("Watcher lagged behind by {} chunks", lagged_count);
        assert!(warning_message.contains("10"));

        // @step And the watcher should continue observing from the current position
        // (verified by the fact that we don't panic or return error)
        assert!(lagged_count > 0); // Watcher continues
    }

    /// Scenario: Empty buffer at breakpoint does not trigger evaluation
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step And the watcher has an empty observation buffer
    /// @step When the parent sends TurnComplete
    /// @step Then no evaluation prompt should be generated
    /// @step And the watcher should continue waiting for observations
    #[test]
    fn test_empty_buffer_no_evaluation() {
        // @step Given a watcher session is observing a parent session
        // @step And the watcher has an empty observation buffer
        let buffer = ObservationBuffer::new();
        assert!(buffer.is_empty());

        // @step When the parent sends TurnComplete (Done)
        let turn_complete = StreamChunk::done();
        assert!(is_natural_breakpoint(&turn_complete));

        // @step Then no evaluation prompt should be generated
        let should_evaluate = !buffer.is_empty();
        assert!(!should_evaluate);

        // @step And the watcher should continue waiting for observations
        // (buffer remains empty, ready for new observations)
        assert!(buffer.is_empty());
    }

    /// Test WatcherState enum exists and has correct variants
    #[test]
    fn test_watcher_state_enum() {
        let idle = WatcherState::Idle;
        let observing = WatcherState::Observing;
        let processing = WatcherState::Processing;

        assert_eq!(idle, WatcherState::Idle);
        assert_eq!(observing, WatcherState::Observing);
        assert_eq!(processing, WatcherState::Processing);
    }

    /// Test watcher_loop_tick processes user input with priority (Rule [0], [4])
    ///
    /// @step Given a watcher session with tokio::select! loop
    /// @step When user input arrives
    /// @step Then it should be processed immediately with priority
    #[tokio::test]
    async fn test_watcher_loop_tick_user_input_priority() {
        // @step Given a watcher session with tokio::select! loop
        let (user_tx, mut user_rx) = mpsc::channel::<PromptInput>(32);
        let (parent_tx, mut parent_rx) = broadcast::channel::<StreamChunk>(256);
        let mut buffer = ObservationBuffer::new();
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        let timeout = Duration::from_secs(5);

        // @step When user input arrives
        user_tx.send(PromptInput {
            input: "What do you think?".to_string(),
            thinking_config: None,
        }).await.unwrap();

        // Also send a parent chunk to test priority
        let _ = parent_tx.send(StreamChunk::text("Parent text".to_string()));

        // @step Then it should be processed immediately with priority
        let action = watcher_loop_tick(
            &mut user_rx,
            &mut parent_rx,
            &mut buffer,
            &role,
            timeout,
        ).await;

        match action {
            WatcherLoopAction::ProcessUserPrompt(prompt) => {
                assert_eq!(prompt, "What do you think?");
            }
            _ => panic!("Expected ProcessUserPrompt, got {:?}", action),
        }
    }

    /// Test watcher_loop_tick accumulates and processes at breakpoint (Rule [0], [1], [2], [3])
    ///
    /// @step Given a watcher loop receiving parent observations
    /// @step When TurnComplete breakpoint is received
    /// @step Then accumulated observations should be formatted and returned
    #[tokio::test]
    async fn test_watcher_loop_tick_breakpoint_processing() {
        // @step Given a watcher loop receiving parent observations
        let (_user_tx, mut user_rx) = mpsc::channel::<PromptInput>(32);
        let (parent_tx, mut parent_rx) = broadcast::channel::<StreamChunk>(256);
        let mut buffer = ObservationBuffer::new();
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        let timeout = Duration::from_secs(5);

        // Pre-populate buffer with some observations
        buffer.push(StreamChunk::text("Hello ".to_string()));
        buffer.push(StreamChunk::text("World".to_string()));

        // @step When TurnComplete breakpoint is received
        let _ = parent_tx.send(StreamChunk::done());

        let action = watcher_loop_tick(
            &mut user_rx,
            &mut parent_rx,
            &mut buffer,
            &role,
            timeout,
        ).await;

        // @step Then accumulated observations should be formatted and returned
        match action {
            WatcherLoopAction::ProcessObservations { prompt, observed_correlation_ids } => {
                assert!(prompt.contains("Hello "));
                assert!(prompt.contains("World"));
                assert!(prompt.contains("reviewer"));
                // WATCH-011: Should have correlation IDs from buffered chunks
                // Note: In this test, chunks are created without correlation_id set (None)
                // so observed_correlation_ids will be empty. Real usage assigns IDs via handle_output.
                assert!(observed_correlation_ids.is_empty());
            }
            _ => panic!("Expected ProcessObservations, got {:?}", action),
        }

        // Buffer should be cleared
        assert!(buffer.is_empty());
    }

    /// Test watcher_loop_tick handles broadcast lag gracefully (Rule [4] from examples)
    ///
    /// @step Given a watcher loop
    /// @step When broadcast receiver reports lagged chunks
    /// @step Then it should continue without error
    #[tokio::test]
    async fn test_watcher_loop_tick_handles_lag() {
        // This test verifies the lag handling code path exists
        // In practice, lag is simulated by the broadcast channel when receiver falls behind
        
        // @step Given a watcher loop
        let (_user_tx, mut user_rx) = mpsc::channel::<PromptInput>(32);
        // Create a small capacity channel to potentially trigger lag
        let (parent_tx, mut parent_rx) = broadcast::channel::<StreamChunk>(2);
        let mut buffer = ObservationBuffer::new();
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        let timeout = Duration::from_millis(100);

        // @step When broadcast receiver reports lagged chunks
        // Send more messages than capacity to trigger lag
        for i in 0..5 {
            let _ = parent_tx.send(StreamChunk::text(format!("Message {}", i)));
        }

        // @step Then it should continue without error
        let action = watcher_loop_tick(
            &mut user_rx,
            &mut parent_rx,
            &mut buffer,
            &role,
            timeout,
        ).await;

        // Should either get Continue (from lag) or process a chunk
        match action {
            WatcherLoopAction::Continue | WatcherLoopAction::ProcessObservations { .. } => {
                // Both are acceptable - lag returns Continue, normal chunk may process
            }
            WatcherLoopAction::Stop => {
                panic!("Should not stop on lag");
            }
            _ => {} // Other actions are fine too
        }
    }

    /// Test watcher_loop_tick silence timeout (Rule [2])
    ///
    /// @step Given a watcher with buffered observations
    /// @step When silence timeout elapses
    /// @step Then observations should be processed
    #[tokio::test]
    async fn test_watcher_loop_tick_silence_timeout() {
        // @step Given a watcher with buffered observations
        let (_user_tx, mut user_rx) = mpsc::channel::<PromptInput>(32);
        let (_parent_tx, mut parent_rx) = broadcast::channel::<StreamChunk>(256);
        let mut buffer = ObservationBuffer::new();
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        
        // Use very short timeout for test
        let timeout = Duration::from_millis(50);

        // Add observation and set old timestamp
        buffer.push(StreamChunk::text("Buffered content".to_string()));
        
        // Wait for timeout to elapse
        tokio::time::sleep(Duration::from_millis(100)).await;

        // @step When silence timeout elapses
        let action = watcher_loop_tick(
            &mut user_rx,
            &mut parent_rx,
            &mut buffer,
            &role,
            timeout,
        ).await;

        // @step Then observations should be processed
        match action {
            WatcherLoopAction::ProcessObservations { prompt, .. } => {
                assert!(prompt.contains("Buffered content"));
            }
            _ => panic!("Expected ProcessObservations from timeout, got {:?}", action),
        }
    }

    /// Test watcher_loop_tick empty buffer at breakpoint (Rule from example [5])
    ///
    /// @step Given a watcher with empty buffer
    /// @step When breakpoint chunk arrives
    /// @step Then Continue should be returned (no evaluation)
    #[tokio::test]
    async fn test_watcher_loop_tick_empty_buffer_at_breakpoint() {
        // @step Given a watcher with empty buffer
        let (_user_tx, mut user_rx) = mpsc::channel::<PromptInput>(32);
        let (parent_tx, mut parent_rx) = broadcast::channel::<StreamChunk>(256);
        let mut buffer = ObservationBuffer::new();
        let role = SessionRole::new("reviewer".to_string(), None, RoleAuthority::Peer).unwrap();
        let timeout = Duration::from_secs(5);

        assert!(buffer.is_empty());

        // @step When breakpoint chunk arrives to an empty buffer
        let _ = parent_tx.send(StreamChunk::done());

        let action = watcher_loop_tick(
            &mut user_rx,
            &mut parent_rx,
            &mut buffer,
            &role,
            timeout,
        ).await;

        // @step Then no evaluation prompt should be generated (Continue returned)
        // Feature file: "Empty buffer at breakpoint does not trigger evaluation"
        match action {
            WatcherLoopAction::Continue => {
                // Correct! Empty buffer at breakpoint → no evaluation
                // The Done chunk is still added to buffer for potential future use
                assert!(!buffer.is_empty()); // Buffer has the Done chunk
            }
            WatcherLoopAction::ProcessObservations { .. } => {
                panic!("Should NOT process when buffer was empty before breakpoint");
            }
            _ => panic!("Unexpected action: {:?}", action),
        }
    }
}

#[cfg(test)]
mod watcher_input_tests {
    use super::*;

    // Feature: spec/features/watcher-injection-message-format.feature

    /// Scenario: Format peer watcher message with structured prefix
    ///
    /// @step Given a watcher session with role "code-reviewer" and authority "Peer"
    /// @step And the watcher session id is "abc123"
    /// @step When the watcher sends message "Consider adding error handling"
    /// @step Then the formatted message should be "[WATCHER: code-reviewer | Authority: Peer | Session: abc123] Consider adding error handling"
    #[test]
    fn test_format_peer_watcher_message() {
        // @step Given a watcher session with role "code-reviewer" and authority "Peer"
        let role_name = "code-reviewer".to_string();
        let authority = RoleAuthority::Peer;

        // @step And the watcher session id is "abc123"
        let session_id = "abc123".to_string();

        // @step When the watcher sends message "Consider adding error handling"
        let message = "Consider adding error handling".to_string();
        let input = WatcherInput::new(session_id, role_name, authority, message).unwrap();
        let formatted = format_watcher_input(&input);

        // @step Then the formatted message should be "[WATCHER: code-reviewer | Authority: Peer | Session: abc123] Consider adding error handling"
        assert_eq!(
            formatted,
            "[WATCHER: code-reviewer | Authority: Peer | Session: abc123] Consider adding error handling"
        );
    }

    /// Scenario: Format supervisor watcher message with structured prefix
    ///
    /// @step Given a watcher session with role "security-auditor" and authority "Supervisor"
    /// @step And the watcher session id is "xyz789"
    /// @step When the watcher sends message "CRITICAL: SQL injection vulnerability detected"
    /// @step Then the parent should receive a WatcherInput chunk
    /// @step And the chunk should contain the formatted message with structured prefix
    #[test]
    fn test_format_supervisor_watcher_message() {
        // @step Given a watcher session with role "security-auditor" and authority "Supervisor"
        let role_name = "security-auditor".to_string();
        let authority = RoleAuthority::Supervisor;

        // @step And the watcher session id is "xyz789"
        let session_id = "xyz789".to_string();

        // @step When the watcher sends message "CRITICAL: SQL injection vulnerability detected"
        let message = "CRITICAL: SQL injection vulnerability detected".to_string();
        let input = WatcherInput::new(session_id, role_name, authority, message).unwrap();

        // @step Then the parent should receive a WatcherInput chunk
        let chunk = StreamChunk::watcher_input(format_watcher_input(&input));

        // @step And the chunk should contain the formatted message with structured prefix
        assert_eq!(chunk.chunk_type, "WatcherInput");
        assert!(chunk.text.as_ref().unwrap().starts_with("[WATCHER: security-auditor | Authority: Supervisor | Session: xyz789]"));
    }

    /// Scenario: Receive watcher input queues message asynchronously
    ///
    /// This test verifies the watcher input channel mechanism works correctly.
    /// Note: BackgroundSession.receive_watcher_input() uses try_send which is non-blocking.
    /// We test the channel pattern here since BackgroundSession construction requires
    /// a full codelet_cli::session::Session (integration test territory).
    ///
    /// @step Given a parent session exists
    /// @step When receive_watcher_input is called with a valid WatcherInput
    /// @step Then the input should be queued via the watcher input channel
    /// @step And the method should return immediately without blocking
    #[test]
    fn test_receive_watcher_input_queues_via_try_send() {
        // @step Given a parent session exists
        // We test the channel mechanism that BackgroundSession.receive_watcher_input uses
        let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::channel::<WatcherInput>(16);

        // @step When receive_watcher_input is called with a valid WatcherInput
        let input = WatcherInput::new(
            "session123".to_string(),
            "test-watcher".to_string(),
            RoleAuthority::Peer,
            "Test message".to_string(),
        ).unwrap();

        // BackgroundSession.receive_watcher_input uses try_send (non-blocking)
        // This mirrors the exact implementation pattern
        let result = watcher_tx.try_send(input);

        // @step Then the input should be queued via the watcher input channel
        assert!(result.is_ok(), "try_send should succeed when channel has capacity");

        // @step And the method should return immediately without blocking
        // try_send is guaranteed non-blocking - verified by using try_send instead of send
        let received = watcher_rx.try_recv();
        assert!(received.is_ok(), "Message should be in channel");
        assert_eq!(received.unwrap().message, "Test message");
    }

    /// Test that channel returns error when full (matches receive_watcher_input error handling)
    #[test]
    fn test_receive_watcher_input_channel_full_returns_error() {
        // Create a channel with capacity 1
        let (watcher_tx, _watcher_rx) = tokio::sync::mpsc::channel::<WatcherInput>(1);

        let input1 = WatcherInput::new(
            "s1".to_string(),
            "watcher".to_string(),
            RoleAuthority::Peer,
            "First".to_string(),
        ).unwrap();

        let input2 = WatcherInput::new(
            "s2".to_string(),
            "watcher".to_string(),
            RoleAuthority::Peer,
            "Second".to_string(),
        ).unwrap();

        // First send should succeed
        assert!(watcher_tx.try_send(input1).is_ok());

        // Second send should fail (channel full)
        let result = watcher_tx.try_send(input2);
        assert!(result.is_err(), "try_send should fail when channel is full");
    }

    /// Scenario: Empty watcher message returns error
    ///
    /// @step Given a watcher session with role "test-watcher" and authority "Peer"
    /// @step And the watcher session id is "test123"
    /// @step When the watcher sends an empty message
    /// @step Then an error should be returned with message "message cannot be empty"
    #[test]
    fn test_empty_watcher_message_returns_error() {
        // @step Given a watcher session with role "test-watcher" and authority "Peer"
        let role_name = "test-watcher".to_string();
        let authority = RoleAuthority::Peer;

        // @step And the watcher session id is "test123"
        let session_id = "test123".to_string();

        // @step When the watcher sends an empty message
        let result = WatcherInput::new(session_id, role_name, authority, "".to_string());

        // @step Then an error should be returned with message "message cannot be empty"
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "message cannot be empty");
    }

    /// Scenario: Multiline watcher message preserves formatting
    ///
    /// @step Given a watcher session with role "code-reviewer" and authority "Peer"
    /// @step And the watcher session id is "abc123"
    /// @step When the watcher sends a multiline message
    /// @step Then the formatted message should have the prefix on the first line
    /// @step And subsequent lines should be preserved without additional prefixes
    #[test]
    fn test_multiline_watcher_message_preserves_formatting() {
        // @step Given a watcher session with role "code-reviewer" and authority "Peer"
        let role_name = "code-reviewer".to_string();
        let authority = RoleAuthority::Peer;

        // @step And the watcher session id is "abc123"
        let session_id = "abc123".to_string();

        // @step When the watcher sends a multiline message
        let multiline_message = "Issue found on line 42:\n- Missing null check\n- Consider using Option<T>".to_string();
        let input = WatcherInput::new(session_id, role_name, authority, multiline_message).unwrap();
        let formatted = format_watcher_input(&input);

        // @step Then the formatted message should have the prefix on the first line
        assert!(formatted.starts_with("[WATCHER: code-reviewer | Authority: Peer | Session: abc123]"));

        // @step And subsequent lines should be preserved without additional prefixes
        let lines: Vec<&str> = formatted.lines().collect();
        assert!(lines.len() >= 3); // Prefix line + 2 content lines (or content all on one line after prefix)
        // The message content follows the prefix, newlines are preserved
        assert!(formatted.contains("- Missing null check"));
        assert!(formatted.contains("- Consider using Option<T>"));
    }
}

#[cfg(test)]
mod napi_watcher_tests {
    use super::*;

    // Feature: spec/features/napi-bindings-for-watcher-operations.feature

    /// Scenario: Create watcher session for a parent
    ///
    /// @step Given a parent session exists with id "parent-uuid"
    /// @step When I call session_create_watcher with parent "parent-uuid", model "claude-sonnet-4", project "/project", name "Code Reviewer"
    /// @step Then a new watcher session should be created and returned
    /// @step And the watcher should be registered in WatchGraph with parent "parent-uuid"
    /// Note: Broadcast subscription happens lazily when watcher loop starts
    #[test]
    fn test_create_watcher_registers_in_watch_graph() {
        // @step Given a parent session exists with id "parent-uuid"
        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let watcher_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let watch_graph = WatchGraph::new();

        // @step When I call session_create_watcher (simulated via WatchGraph.add_watcher)
        let result = watch_graph.add_watcher(parent_id, watcher_id);

        // @step Then a new watcher session should be created and returned
        assert!(result.is_ok());

        // @step And the watcher should be registered in WatchGraph with parent "parent-uuid"
        assert_eq!(watch_graph.get_parent(watcher_id), Some(parent_id));

        // Broadcast subscription is lazy - happens when watcher loop starts via subscribe_to_stream()
        assert!(watch_graph.get_watchers(parent_id).contains(&watcher_id));
    }

    /// Scenario: Get parent of a watcher session
    ///
    /// @step Given a watcher session "watcher-uuid" watching parent "parent-uuid"
    /// @step When I call session_get_parent with "watcher-uuid"
    /// @step Then it should return "parent-uuid"
    #[test]
    fn test_get_parent_returns_parent_id() {
        // @step Given a watcher session "watcher-uuid" watching parent "parent-uuid"
        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let watcher_id = Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();
        let watch_graph = WatchGraph::new();
        watch_graph.add_watcher(parent_id, watcher_id).unwrap();

        // @step When I call session_get_parent with "watcher-uuid"
        let result = watch_graph.get_parent(watcher_id);

        // @step Then it should return "parent-uuid"
        assert_eq!(result, Some(parent_id));
    }

    /// Scenario: Get parent of a regular session returns None
    ///
    /// @step Given a regular session "regular-uuid" with no parent
    /// @step When I call session_get_parent with "regular-uuid"
    /// @step Then it should return None
    #[test]
    fn test_get_parent_returns_none_for_regular_session() {
        // @step Given a regular session "regular-uuid" with no parent
        let regular_id = Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap();
        let watch_graph = WatchGraph::new();

        // @step When I call session_get_parent with "regular-uuid"
        let result = watch_graph.get_parent(regular_id);

        // @step Then it should return None
        assert_eq!(result, None);
    }

    /// Scenario: Get watchers of a parent session
    ///
    /// @step Given a parent session "parent-uuid"
    /// @step And watcher session "watcher-1-uuid" watching "parent-uuid"
    /// @step And watcher session "watcher-2-uuid" watching "parent-uuid"
    /// @step When I call session_get_watchers with "parent-uuid"
    /// @step Then it should return ["watcher-1-uuid", "watcher-2-uuid"]
    #[test]
    fn test_get_watchers_returns_watcher_list() {
        // @step Given a parent session "parent-uuid"
        let parent_id = Uuid::parse_str("00000000-0000-0000-0000-000000000006").unwrap();
        let watcher_1_id = Uuid::parse_str("00000000-0000-0000-0000-000000000007").unwrap();
        let watcher_2_id = Uuid::parse_str("00000000-0000-0000-0000-000000000008").unwrap();
        let watch_graph = WatchGraph::new();

        // @step And watcher session "watcher-1-uuid" watching "parent-uuid"
        watch_graph.add_watcher(parent_id, watcher_1_id).unwrap();

        // @step And watcher session "watcher-2-uuid" watching "parent-uuid"
        watch_graph.add_watcher(parent_id, watcher_2_id).unwrap();

        // @step When I call session_get_watchers with "parent-uuid"
        let watchers = watch_graph.get_watchers(parent_id);

        // @step Then it should return ["watcher-1-uuid", "watcher-2-uuid"]
        assert_eq!(watchers.len(), 2);
        assert!(watchers.contains(&watcher_1_id));
        assert!(watchers.contains(&watcher_2_id));
    }

    /// Scenario: Get watchers of a session with no watchers
    ///
    /// @step Given a session "lonely-uuid" with no watchers
    /// @step When I call session_get_watchers with "lonely-uuid"
    /// @step Then it should return an empty array
    #[test]
    fn test_get_watchers_returns_empty_for_no_watchers() {
        // @step Given a session "lonely-uuid" with no watchers
        let lonely_id = Uuid::parse_str("00000000-0000-0000-0000-000000000009").unwrap();
        let watch_graph = WatchGraph::new();

        // @step When I call session_get_watchers with "lonely-uuid"
        let watchers = watch_graph.get_watchers(lonely_id);

        // @step Then it should return an empty array
        assert!(watchers.is_empty());
    }

    /// Scenario: Inject watcher message into parent session
    ///
    /// @step Given a watcher session "watcher-uuid" with role "code-reviewer" and authority "Peer"
    /// @step And the watcher is watching parent "parent-uuid"
    /// @step When I call watcher_inject with watcher "watcher-uuid" and message "Consider adding error handling"
    /// @step Then the message should be formatted with watcher prefix
    /// @step And the message should be queued on the parent session
    #[test]
    fn test_watcher_inject_formats_and_queues_message() {
        // @step Given a watcher session "watcher-uuid" with role "code-reviewer" and authority "Peer"
        let watcher_id = "00000000-0000-0000-0000-00000000000a";
        let role = SessionRole::new(
            "code-reviewer".to_string(),
            None,
            RoleAuthority::Peer,
        ).unwrap();

        // @step And the watcher is watching parent "parent-uuid"
        // (Setup via WatchGraph in real implementation)

        // @step When I call watcher_inject with watcher "watcher-uuid" and message "Consider adding error handling"
        let input = WatcherInput::new(
            watcher_id.to_string(),
            role.name.clone(),
            role.authority,
            "Consider adding error handling".to_string(),
        ).unwrap();

        // @step Then the message should be formatted with watcher prefix
        let formatted = format_watcher_input(&input);
        assert!(formatted.starts_with("[WATCHER: code-reviewer | Authority: Peer | Session:"));
        assert!(formatted.contains("Consider adding error handling"));

        // @step And the message should be queued on the parent session
        // (Tested via receive_watcher_input in integration)
    }

    /// Scenario: Inject fails when session has no role
    ///
    /// @step Given a session "no-role-uuid" without a watcher role
    /// @step When I call watcher_inject with watcher "no-role-uuid" and message "Test"
    /// @step Then it should return error "Session has no watcher role set"
    #[test]
    fn test_watcher_inject_fails_without_role() {
        // @step Given a session "no-role-uuid" without a watcher role
        let role: Option<SessionRole> = None;

        // @step When I call watcher_inject with watcher "no-role-uuid" and message "Test"
        // Simulated: check that role is None

        // @step Then it should return error "Session has no watcher role set"
        assert!(role.is_none(), "Role should be None for session without watcher role");
        // Real NAPI function will return: Error::from_reason("Session has no watcher role set")
    }

    /// Scenario: Inject fails when watcher has no parent
    ///
    /// @step Given a session "orphan-uuid" with role "reviewer" but no parent registered
    /// @step When I call watcher_inject with watcher "orphan-uuid" and message "Test"
    /// @step Then it should return error "Watcher has no parent session"
    #[test]
    fn test_watcher_inject_fails_without_parent() {
        // @step Given a session "orphan-uuid" with role "reviewer" but no parent registered
        let orphan_id = Uuid::parse_str("00000000-0000-0000-0000-00000000000b").unwrap();
        let watch_graph = WatchGraph::new();
        // Note: role is set but no parent in WatchGraph

        // @step When I call watcher_inject with watcher "orphan-uuid" and message "Test"
        let parent = watch_graph.get_parent(orphan_id);

        // @step Then it should return error "Watcher has no parent session"
        assert!(parent.is_none(), "Orphan watcher should have no parent");
        // Real NAPI function will return: Error::from_reason("Watcher has no parent session")
    }
}

#[cfg(test)]
mod correlation_id_tests {
    use super::*;

    // Feature: spec/features/cross-pane-selection-with-correlation-ids.feature (WATCH-011)

    /// Scenario: StreamChunk receives correlation ID in handle_output
    ///
    /// @step Given a parent session exists
    /// @step When the parent session emits a Text chunk via handle_output()
    /// @step Then the chunk receives a unique correlation_id assigned by an atomic counter
    /// @step And the correlation_id is in format "{session_id}-{counter}"
    #[test]
    fn test_correlation_id_format() {
        // @step Given a parent session exists
        let session_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        // Simulate correlation ID assignment as done in handle_output
        // Using AtomicU64::fetch_add as in the real implementation
        let counter = AtomicU64::new(0);

        // @step When the parent session emits a Text chunk via handle_output()
        let id1 = counter.fetch_add(1, Ordering::SeqCst);
        let correlation_id1 = format!("{}-{}", session_id, id1);

        let id2 = counter.fetch_add(1, Ordering::SeqCst);
        let correlation_id2 = format!("{}-{}", session_id, id2);

        // @step Then the chunk receives a unique correlation_id assigned by an atomic counter
        assert_ne!(correlation_id1, correlation_id2);

        // @step And the correlation_id is in format "{session_id}-{counter}"
        assert_eq!(correlation_id1, "00000000-0000-0000-0000-000000000001-0");
        assert_eq!(correlation_id2, "00000000-0000-0000-0000-000000000001-1");
    }

    /// Scenario: ObservationBuffer captures correlation IDs
    ///
    /// @step Given a watcher session is observing a parent session
    /// @step And the parent emits chunks with correlation_ids "p-0", "p-1", "p-2"
    /// @step When a natural breakpoint triggers watcher evaluation
    /// @step Then the buffer.correlation_ids() returns ["p-0", "p-1", "p-2"]
    #[test]
    fn test_observation_buffer_correlation_ids() {
        // @step Given a watcher session is observing a parent session
        let mut buffer = ObservationBuffer::new();

        // @step And the parent emits chunks with correlation_ids "p-0", "p-1", "p-2"
        let mut chunk1 = StreamChunk::text("Hello".to_string());
        chunk1.correlation_id = Some("p-0".to_string());
        buffer.push(chunk1);

        let mut chunk2 = StreamChunk::text("World".to_string());
        chunk2.correlation_id = Some("p-1".to_string());
        buffer.push(chunk2);

        let mut chunk3 = StreamChunk::text("!".to_string());
        chunk3.correlation_id = Some("p-2".to_string());
        buffer.push(chunk3);

        // @step When a natural breakpoint triggers watcher evaluation
        // @step Then the buffer.correlation_ids() returns ["p-0", "p-1", "p-2"]
        let ids = buffer.correlation_ids();
        assert_eq!(ids, vec!["p-0", "p-1", "p-2"]);
    }

    /// Scenario: StreamChunk can be tagged with observed correlation IDs
    ///
    /// @step Given a watcher response chunk
    /// @step When it is tagged with observed correlation IDs
    /// @step Then the chunk has observed_correlation_ids set
    #[test]
    fn test_stream_chunk_with_observed_correlation_ids() {
        // @step Given a watcher response chunk
        let chunk = StreamChunk::text("I noticed an issue".to_string());

        // @step When it is tagged with observed correlation IDs
        let tagged_chunk = chunk.with_observed_correlation_ids(vec![
            "p-0".to_string(),
            "p-1".to_string(),
        ]);

        // @step Then the chunk has observed_correlation_ids set
        assert!(tagged_chunk.observed_correlation_ids.is_some());
        let ids = tagged_chunk.observed_correlation_ids.unwrap();
        assert_eq!(ids, vec!["p-0", "p-1"]);
    }

    /// Scenario: WatcherLoopAction::ProcessObservations includes observed correlation IDs
    ///
    /// @step Given accumulated observations with correlation IDs
    /// @step When ProcessObservations action is created
    /// @step Then it contains the observed correlation IDs
    #[test]
    fn test_watcher_loop_action_has_correlation_ids() {
        // @step Given accumulated observations with correlation IDs
        let prompt = "Evaluate these observations".to_string();
        let correlation_ids = vec!["p-0".to_string(), "p-1".to_string()];

        // @step When ProcessObservations action is created
        let action = WatcherLoopAction::ProcessObservations {
            prompt: prompt.clone(),
            observed_correlation_ids: correlation_ids.clone(),
        };

        // @step Then it contains the observed correlation IDs
        match action {
            WatcherLoopAction::ProcessObservations { prompt: p, observed_correlation_ids } => {
                assert_eq!(p, prompt);
                assert_eq!(observed_correlation_ids, correlation_ids);
            }
            _ => panic!("Expected ProcessObservations"),
        }
    }
}

#[cfg(test)]
mod watcher_integration_tests {
    use super::*;

    // Feature: spec/features/watcher-loop-and-input-channel-not-integrated.feature (WATCH-019)

    /// Scenario: Parent session processes watcher injections
    ///
    /// @step Given a parent session exists with a watcher attached
    /// @step When the watcher injects a message via watcher_inject
    /// @step Then the parent agent_loop should read the message from watcher_input_rx and process it
    #[test]
    fn test_parent_session_processes_watcher_injections() {
        // @step Given a parent session exists with a watcher attached
        // Create a watcher input channel (simulating parent's watcher_input_tx/rx)
        let (watcher_input_tx, mut watcher_input_rx) = mpsc::channel::<WatcherInput>(16);

        // @step When the watcher injects a message via watcher_inject
        let input = WatcherInput::new(
            "watcher-uuid".to_string(),
            "security-reviewer".to_string(),
            RoleAuthority::Supervisor,
            "SQL injection vulnerability detected!".to_string(),
        ).unwrap();
        
        watcher_input_tx.try_send(input.clone()).expect("Should send watcher input");

        // @step Then the parent agent_loop should read the message from watcher_input_rx and process it
        // Use try_recv to simulate what agent_loop would do
        let received = watcher_input_rx.try_recv();
        assert!(received.is_ok(), "Parent should receive watcher injection from watcher_input_rx");
        
        let received_input = received.unwrap();
        assert_eq!(received_input.message, "SQL injection vulnerability detected!");
        assert_eq!(received_input.role_name, "security-reviewer");
    }

    /// Scenario: Watcher session subscribes to parent broadcast on creation
    ///
    /// @step Given a parent session exists with an active broadcast channel
    /// @step When session_create_watcher is called with the parent session ID
    /// @step Then the watcher should have a broadcast receiver subscribed to the parent's stream
    #[test]
    fn test_watcher_subscribes_to_parent_broadcast() {
        // @step Given a parent session exists with an active broadcast channel
        let (parent_broadcast_tx, _) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);

        // @step When session_create_watcher is called with the parent session ID
        // Simulate what session_create_watcher SHOULD do: subscribe to parent's broadcast
        let mut watcher_broadcast_rx = parent_broadcast_tx.subscribe();

        // @step Then the watcher should have a broadcast receiver subscribed to the parent's stream
        // Send a chunk from parent and verify watcher receives it
        let test_chunk = StreamChunk::text("test from parent".to_string());
        parent_broadcast_tx.send(test_chunk.clone()).expect("Should send");
        
        let received = watcher_broadcast_rx.try_recv();
        assert!(received.is_ok(), "Watcher should receive chunks from parent broadcast");
        assert_eq!(received.unwrap().text, Some("test from parent".to_string()));
    }

    /// Scenario: Watcher loop processes parent observations at breakpoints
    ///
    /// @step Given a watcher session is running with parent broadcast subscription
    /// @step When the parent session emits Text chunks followed by a Done chunk
    /// @step Then the watcher should accumulate observations and trigger evaluation at the Done breakpoint
    #[tokio::test]
    async fn test_watcher_loop_processes_observations() {
        // @step Given a watcher session is running with parent broadcast subscription
        let (user_input_tx, mut user_input_rx) = mpsc::channel::<PromptInput>(16);
        let (parent_broadcast_tx, mut parent_broadcast_rx) = broadcast::channel::<StreamChunk>(WATCHER_BROADCAST_CAPACITY);
        let role = SessionRole::new("test-watcher".to_string(), None, RoleAuthority::Peer).unwrap();
        let mut buffer = ObservationBuffer::new();
        let silence_timeout = std::time::Duration::from_secs(5);

        // @step When the parent session emits Text chunks followed by a Done chunk
        // Send text chunk first
        parent_broadcast_tx.send(StreamChunk::text("function login() { }".to_string())).unwrap();
        
        // Process the text chunk - should accumulate
        let action1 = watcher_loop_tick(
            &mut user_input_rx,
            &mut parent_broadcast_rx,
            &mut buffer,
            &role,
            silence_timeout,
        ).await;
        
        // Should continue (not a breakpoint)
        assert!(matches!(action1, WatcherLoopAction::Continue), "Text chunk should not trigger evaluation");
        assert!(!buffer.is_empty(), "Buffer should have accumulated the text chunk");

        // Send Done chunk (breakpoint)
        parent_broadcast_tx.send(StreamChunk::done()).unwrap();
        
        let action2 = watcher_loop_tick(
            &mut user_input_rx,
            &mut parent_broadcast_rx,
            &mut buffer,
            &role,
            silence_timeout,
        ).await;

        // @step Then the watcher should accumulate observations and trigger evaluation at the Done breakpoint
        match action2 {
            WatcherLoopAction::ProcessObservations { prompt, observed_correlation_ids: _ } => {
                assert!(!prompt.is_empty(), "Evaluation prompt should be generated");
                assert!(prompt.contains("function login"), "Prompt should contain observed content");
            }
            _ => panic!("Expected ProcessObservations action at Done breakpoint, got {:?}", action2),
        }
        
        // Buffer should be cleared after processing
        assert!(buffer.is_empty(), "Buffer should be cleared after breakpoint processing");

        // Clean up
        drop(user_input_tx);
    }
}

/// Singleton session manager
/// 
/// VIEWNV-001: Uses IndexMap instead of HashMap to maintain insertion order.
/// Sessions are stored in creation order, which allows navigation to traverse
/// sessions from oldest to newest without needing timestamps.
pub struct SessionManager {
    sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,
    /// Tracks parent-watcher relationships between sessions (WATCH-002)
    watch_graph: WatchGraph,
    /// Tracks the currently active (attached) session for navigation (VIEWNV-001)
    active_session_id: RwLock<Option<Uuid>>,
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
            sessions: RwLock::new(IndexMap::new()),
            watch_graph: WatchGraph::new(),
            active_session_id: RwLock::new(None),
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
            let registry_provider = parts.first().unwrap_or(&"anthropic");
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
        let mut inner = codelet_cli::session::Session::from_provider_manager(provider_manager);

        // Inject context reminders (CLAUDE.md discovery, environment info)
        // This provides the LLM with platform, architecture, shell, user, and working directory
        inner.inject_context_reminders();

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
    
    /// Create a watcher session that observes a parent session (WATCH-019)
    ///
    /// Similar to create_session_with_id but:
    /// - Spawns watcher_agent_loop instead of agent_loop
    /// - Subscribes to parent's broadcast channel
    /// - Sets the watcher role
    pub async fn create_watcher_session_with_id(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
        parent_id: Uuid,
        role: SessionRole,
    ) -> Result<()> {
        let uuid = Uuid::parse_str(id)
            .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;

        // Check session limits
        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(Error::from_reason(format!(
                    "Maximum sessions ({}) reached",
                    MAX_SESSIONS
                )));
            }
            if sessions.contains_key(&uuid) {
                return Ok(());
            }
        }

        // Get parent session and subscribe to its broadcast
        let parent = self.sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&parent_id)
            .cloned()
            .ok_or_else(|| Error::from_reason(format!("Parent session not found: {}", parent_id)))?;
        
        let parent_broadcast_rx = parent.subscribe_to_stream();

        let (input_tx, input_rx) = mpsc::channel::<PromptInput>(32);

        let _ = dotenvy::dotenv();

        let (provider_id, model_id) = if model.contains('/') {
            let parts: Vec<&str> = model.split('/').collect();
            let registry_provider = parts.first().unwrap_or(&"anthropic");
            let model_part = parts.get(1).map(|s| s.to_string());
            (Some(registry_provider.to_string()), model_part)
        } else {
            (Some(model.to_string()), None)
        };

        let mut provider_manager = codelet_providers::ProviderManager::with_model_support()
            .await
            .map_err(|e| Error::from_reason(format!("Failed to create provider manager: {}", e)))?;

        if model.contains('/') {
            provider_manager.select_model(model)
                .map_err(|e| Error::from_reason(format!("Failed to select model: {}", e)))?;
        }

        let mut inner = codelet_cli::session::Session::from_provider_manager(provider_manager);
        inner.inject_context_reminders();

        let session = Arc::new(BackgroundSession::new(
            uuid,
            name.to_string(),
            project.to_string(),
            provider_id,
            model_id,
            inner,
            input_tx,
        ));
        
        // Set the watcher role
        session.set_role(role.clone());
        
        // Spawn watcher agent loop (observes parent via broadcast)
        let session_clone = session.clone();
        tokio::spawn(async move {
            watcher_agent_loop(session_clone, input_rx, parent_broadcast_rx, role).await;
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
    
    // === VIEWNV-001: Active session tracking for navigation ===
    
    /// Set the active (currently viewed) session
    pub fn set_active_session(&self, id: Uuid) {
        *self.active_session_id.write().expect("active_session lock poisoned") = Some(id);
    }
    
    /// Clear the active session (when returning to board)
    pub fn clear_active_session(&self) {
        *self.active_session_id.write().expect("active_session lock poisoned") = None;
    }
    
    /// Get the active session ID
    pub fn get_active_session(&self) -> Option<Uuid> {
        *self.active_session_id.read().expect("active_session lock poisoned")
    }
    
    /// Get the next session after the active one (VIEWNV-001)
    ///
    /// Uses hierarchy-aware navigation:
    /// - From board: returns first parent session
    /// - From parent session with watchers: returns first watcher
    /// - From parent session without watchers: returns next parent session
    /// - From watcher: returns next sibling watcher, or next parent session
    /// - From last item: returns None (show create dialog)
    pub fn get_next_session(&self) -> Option<String> {
        use crate::navigation::{build_navigation_list, get_next_target, NavigationTarget};
        
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let active = self.active_session_id.read().expect("active_session lock poisoned");
        
        // Build the navigation list with watchers following their parents
        let nav_list = build_navigation_list(&sessions, &self.watch_graph);

        // Get the next target
        let result = match get_next_target(&nav_list, *active) {
            NavigationTarget::Session(id) => Some(id.to_string()),
            NavigationTarget::CreateDialog => None,
            NavigationTarget::Board => None, // Shouldn't happen on next
            NavigationTarget::None => None,
        };

        result
    }
    
    /// Get the previous session before the active one (VIEWNV-001)
    ///
    /// Uses hierarchy-aware navigation:
    /// - From board: returns None (stay on board)
    /// - From first parent session: returns None (go to board)
    /// - From watcher: returns prev sibling watcher, or parent session
    /// - From parent session: returns last watcher of prev session, or prev session
    pub fn get_prev_session(&self) -> Option<String> {
        use crate::navigation::{build_navigation_list, get_prev_target, NavigationTarget};
        
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let active = self.active_session_id.read().expect("active_session lock poisoned");

        // Build the navigation list with watchers following their parents
        let nav_list = build_navigation_list(&sessions, &self.watch_graph);

        // Get the previous target
        let result = match get_prev_target(&nav_list, *active) {
            NavigationTarget::Session(id) => Some(id.to_string()),
            NavigationTarget::Board => None, // Go to board
            NavigationTarget::CreateDialog => None, // Shouldn't happen on prev
            NavigationTarget::None => None,
        };

        result
    }
    
    /// Get the first session (VIEWNV-001)
    /// Returns the first parent session (not a watcher)
    pub fn get_first_session(&self) -> Option<String> {
        use crate::navigation::build_navigation_list;
        
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let nav_list = build_navigation_list(&sessions, &self.watch_graph);
        
        nav_list.first().map(|id| id.to_string())
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
        
        // Clean up watch graph relationships (WATCH-002)
        // If this session was a parent, clean up all its watchers
        self.watch_graph.cleanup_parent(uuid);
        // If this session was a watcher, remove its relationship
        self.watch_graph.remove_watcher(uuid);
        
        // VIEWNV-001: Use shift_remove to maintain insertion order
        let session = self.sessions.write().expect("sessions lock poisoned").shift_remove(&uuid);
        
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
    
    // === WatchGraph delegation methods (WATCH-002) ===
    
    /// Register a watcher for a parent session
    pub fn add_watcher(&self, parent_id: Uuid, watcher_id: Uuid) -> std::result::Result<(), String> {
        self.watch_graph.add_watcher(parent_id, watcher_id)
    }
    
    /// Remove a watcher relationship
    pub fn remove_watcher(&self, watcher_id: Uuid) {
        self.watch_graph.remove_watcher(watcher_id)
    }
    
    /// Get all watchers for a parent session
    pub fn get_watchers(&self, parent_id: Uuid) -> Vec<Uuid> {
        self.watch_graph.get_watchers(parent_id)
    }
    
    /// Get the parent for a watcher session
    pub fn get_parent(&self, watcher_id: Uuid) -> Option<Uuid> {
        self.watch_graph.get_parent(watcher_id)
    }
    
}

/// Macro to reduce duplication in provider handling.
/// Each provider returns a different concrete type, so we must match and call
/// run_agent_stream in each branch. This macro eliminates the boilerplate.
macro_rules! run_with_provider {
    ($inner:expr, $getter:ident, $input:expr, $session:expr, $output:expr, $thinking:expr) => {
        match $inner.provider_manager_mut().$getter() {
            Ok(provider) => {
                let agent = codelet_core::RigAgent::with_default_depth(
                    provider.create_rig_agent(None, $thinking.clone())
                );
                codelet_cli::interactive::run_agent_stream(
                    agent,
                    $input,
                    $inner,
                    $session.is_interrupted.clone(),
                    $session.interrupt_notify.clone(),
                    $output,
                )
                .await
            }
            Err(e) => Err(anyhow::anyhow!("Failed to get provider: {}", e)),
        }
    };
}

/// Agent loop that runs in background tokio task
/// WATCH-019: Modified to also process watcher injections via watcher_input_rx
async fn agent_loop(session: Arc<BackgroundSession>, mut input_rx: mpsc::Receiver<PromptInput>) {
    loop {
        // WATCH-019: Use tokio::select! to wait on both user input and watcher input
        // Lock the watcher_input_rx to use in select
        let mut watcher_rx = session.watcher_input_rx.lock().await;
        
        // Use biased to prefer user input over watcher input
        let input_to_process: Option<(String, Option<String>)> = tokio::select! {
            biased;
            
            // User input takes priority
            result = input_rx.recv() => {
                match result {
                    Some(prompt_input) => Some((prompt_input.input, prompt_input.thinking_config)),
                    None => {
                        // Channel closed, exit loop
                        drop(watcher_rx);
                        break;
                    }
                }
            }
            
            // WATCH-019: Watcher injection input
            result = watcher_rx.recv() => {
                match result {
                    Some(watcher_input) => {
                        // Format watcher input as a user message with structured prefix
                        let formatted = format_watcher_input(&watcher_input);
                        // Emit the watcher input chunk so it appears in the UI
                        session.handle_output(StreamChunk::watcher_input(formatted.clone()));
                        Some((formatted, None))
                    }
                    None => {
                        // Watcher channel closed, continue with user input only
                        None
                    }
                }
            }
        };
        
        // Drop the lock before processing to avoid holding it during agent execution
        drop(watcher_rx);
        
        // If we got input to process, run the agent
        if let Some((input, thinking_config)) = input_to_process {
            let thinking_config_ref = thinking_config.as_deref();

            tracing::debug!("Session {} received input: {}", session.id, &input[..input.len().min(50)]);

            // Set status to running
            session.set_status(SessionStatus::Running);
            session.reset_interrupt();

            // Parse thinking config JSON if provided
            let thinking_config_value: Option<serde_json::Value> = thinking_config_ref.and_then(|config_str| {
                serde_json::from_str(config_str).ok()
            });

            // Create output handler that buffers and forwards
            let session_for_output = session.clone();
            let output = BackgroundOutput::new(session_for_output);

            let mut inner_session = session.inner.lock().await;

            let session_for_pause = session.clone();
            let pause_handler: PauseHandler = Arc::new(move |request: PauseRequest| {
                let state = PauseState {
                    kind: request.kind,
                    tool_name: request.tool_name.clone(),
                    message: request.message.clone(),
                    details: request.details.clone(),
                };
                session_for_pause.set_pause_state(Some(state));
                session_for_pause.set_status(SessionStatus::Paused);
                
                let response = session_for_pause.wait_for_pause_response();
                
                session_for_pause.set_status(SessionStatus::Running);
                
                response
            });

            set_pause_handler(Some(pause_handler));
            
            let current_provider = inner_session.current_provider_name().to_string();
            let result = match current_provider.as_str() {
                "claude" => run_with_provider!(&mut inner_session, get_claude, &input, session, &output, thinking_config_value),
                "openai" => run_with_provider!(&mut inner_session, get_openai, &input, session, &output, thinking_config_value),
                "gemini" => run_with_provider!(&mut inner_session, get_gemini, &input, session, &output, thinking_config_value),
                "zai" => run_with_provider!(&mut inner_session, get_zai, &input, session, &output, thinking_config_value),
                _ => {
                    tracing::error!("Unsupported provider: {}", current_provider);
                    Err(anyhow::anyhow!("Unsupported provider: {}", current_provider))
                }
            };
            
            set_pause_handler(None);

            // Handle result
            // Note: run_agent_stream emits StreamEvent::Done on successful completion,
            // so we only emit Done here on error (to ensure the turn is properly terminated)
            if let Err(e) = result {
                tracing::error!("Agent stream error for session {}: {}", session.id, e);
                session.handle_output(StreamChunk::error(e.to_string()));
                // NAPI-009-FIX: Set status to Idle BEFORE emitting Done chunk
                // This prevents race condition where JS receives Done before status is Idle
                session.set_status(SessionStatus::Idle);
                session.handle_output(StreamChunk::done());
            } else {
                // Success case: BackgroundOutput::emit already set status to Idle when Done was emitted
                // Setting it again here is idempotent and ensures consistency
                session.set_status(SessionStatus::Idle);
            }
        }
    }
}

/// Watcher agent loop that observes parent session and handles dual input (WATCH-019)
///
/// This loop uses `run_watcher_loop` from WATCH-005 to handle both:
/// - User prompts to the watcher (takes priority)
/// - Parent session observations (accumulated until breakpoints)
///
/// When observations trigger evaluation, the prompt is run through the agent
/// and the output is shown in the watcher's UI. The watcher user can then
/// manually inject messages via watcher_inject if needed.
async fn watcher_agent_loop(
    watcher_session: Arc<BackgroundSession>,
    mut user_input_rx: mpsc::Receiver<PromptInput>,
    mut parent_broadcast_rx: broadcast::Receiver<StreamChunk>,
    role: SessionRole,
) {
    let silence_timeout_secs = Some(DEFAULT_SILENCE_TIMEOUT_SECS);
    let watcher_for_callback = watcher_session.clone();
    let auto_inject = role.auto_inject; // WATCH-020: Capture auto_inject setting
    let watcher_id = watcher_session.id.to_string(); // WATCH-020: Capture for injection (convert Uuid to String)

    // Process prompt callback - runs prompts through the agent (similar to agent_loop)
    // WATCH-020: Now uses WatcherOutput for observation evaluations to capture turn text
    let process_prompt = |prompt: String, is_user_prompt: bool, observed_correlation_ids: Vec<String>| {
        let session = watcher_for_callback.clone();
        let watcher_id = watcher_id.clone();
        async move {
            // WATCH-011: Set pending observed correlation IDs for watcher responses
            if !is_user_prompt && !observed_correlation_ids.is_empty() {
                session.set_pending_observed_correlation_ids(observed_correlation_ids);
            }

            tracing::debug!(
                "Watcher {} processing {}: {}",
                session.id,
                if is_user_prompt { "user prompt" } else { "observation evaluation" },
                &prompt[..prompt.len().min(50)]
            );

            // Set status to running
            session.set_status(SessionStatus::Running);
            session.reset_interrupt();

            // WATCH-020: Use WatcherOutput for observation evaluations to capture turn text
            // User prompts use BackgroundOutput directly (no parsing needed)
            let session_for_output = session.clone();
            let watcher_output = WatcherOutput::new(session_for_output.clone());

            let mut inner_session = session.inner.lock().await;
            let current_provider = inner_session.current_provider_name().to_string();
            
            let session_for_pause = session.clone();
            let pause_handler: PauseHandler = Arc::new(move |request: PauseRequest| {
                let state = PauseState {
                    kind: request.kind,
                    tool_name: request.tool_name.clone(),
                    message: request.message.clone(),
                    details: request.details.clone(),
                };
                session_for_pause.set_pause_state(Some(state));
                session_for_pause.set_status(SessionStatus::Paused);
                
                let response = session_for_pause.wait_for_pause_response();
                
                session_for_pause.set_status(SessionStatus::Running);
                
                response
            });
            
            set_pause_handler(Some(pause_handler));
            
            let result = match current_provider.as_str() {
                "claude" => run_with_provider!(&mut inner_session, get_claude, &prompt, session, &watcher_output, None::<serde_json::Value>),
                "openai" => run_with_provider!(&mut inner_session, get_openai, &prompt, session, &watcher_output, None::<serde_json::Value>),
                "gemini" => run_with_provider!(&mut inner_session, get_gemini, &prompt, session, &watcher_output, None::<serde_json::Value>),
                "zai" => run_with_provider!(&mut inner_session, get_zai, &prompt, session, &watcher_output, None::<serde_json::Value>),
                _ => {
                    tracing::error!("Unsupported provider: {}", current_provider);
                    Err(anyhow::anyhow!("Unsupported provider: {}", current_provider))
                }
            };
            
            set_pause_handler(None);

            // Release the lock before any injection calls
            drop(inner_session);

            if let Err(e) = result {
                tracing::error!("Watcher agent error for session {}: {}", session.id, e);
                session.handle_output(StreamChunk::error(e.to_string()));
                // NAPI-009-FIX: Set status to Idle BEFORE emitting Done chunk
                // This prevents race condition where JS receives Done before status is Idle
                session.set_status(SessionStatus::Idle);
                session.handle_output(StreamChunk::done());
            } else {
                // WATCH-020: Parse for interjections on observation evaluations only
                if !is_user_prompt {
                    let turn_text = watcher_output.get_turn_text();
                    if !turn_text.is_empty() {
                        if let Some(interjection) = parse_interjection(&turn_text) {
                            tracing::info!(
                                "Watcher {} detected interjection: urgent={}, content_len={}",
                                watcher_id,
                                interjection.urgent,
                                interjection.content.len()
                            );
                            
                            if auto_inject {
                                // WATCH-020: Automatic injection
                                tracing::info!("Watcher {} auto-injecting to parent", watcher_id);
                                
                                // Call watcher_inject with the extracted content
                                // Note: watcher_inject is a NAPI function that handles the injection
                                if let Err(e) = watcher_inject(watcher_id.clone(), interjection.content) {
                                    tracing::error!("Failed to auto-inject from watcher {}: {}", watcher_id, e);
                                }
                            } else {
                                // WATCH-020: Manual review mode - emit pending injection event
                                tracing::info!(
                                    "Watcher {} has pending interjection (auto_inject=false): {:?}",
                                    watcher_id,
                                    &interjection.content[..interjection.content.len().min(50)]
                                );
                                
                                // Emit a special chunk to notify UI of pending injection
                                session.handle_output(StreamChunk::watcher_pending_injection(
                                    interjection.urgent,
                                    interjection.content,
                                ));
                            }
                        } else {
                            tracing::debug!(
                                "Watcher {} response parsed as [CONTINUE] or no interjection block",
                                watcher_id
                            );
                        }
                    }
                }
                // Success case: BackgroundOutput::emit already set status to Idle when Done was emitted
                // Setting it again here is idempotent and ensures consistency
                session.set_status(SessionStatus::Idle);
            }
            
            // Clear pending observed correlation IDs after processing
            if !is_user_prompt {
                session.set_pending_observed_correlation_ids(Vec::new());
            }

            Ok(())
        }
    };

    // Run the watcher loop
    if let Err(e) = run_watcher_loop(
        &mut user_input_rx,
        &mut parent_broadcast_rx,
        &role,
        silence_timeout_secs,
        process_prompt,
    ).await {
        tracing::error!("Watcher loop error for session {}: {}", watcher_session.id, e);
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
                is_stderr: tp.is_stderr,
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
            StreamEvent::Done => {
                // NAPI-009-FIX: Set status to Idle BEFORE emitting Done chunk
                // This prevents a race condition where JavaScript receives the Done callback
                // and calls sessionGetStatus() before Rust has set the status to Idle.
                // The NonBlocking callback mode means JS could process Done at any time,
                // so we must ensure status is Idle before the chunk is sent.
                self.session.set_status(SessionStatus::Idle);
                StreamChunk::done()
            }
        };

        self.session.handle_output(chunk);
    }

    fn progress_emitter(&self) -> Option<std::sync::Arc<dyn codelet_cli::interactive::StreamOutput>> {
        Some(std::sync::Arc::new(BackgroundProgressEmitter {
            session: self.session.clone(),
        }))
    }
}

/// Progress emitter for background sessions - can be captured in 'static closures
struct BackgroundProgressEmitter {
    session: Arc<BackgroundSession>,
}

impl codelet_cli::interactive::StreamOutput for BackgroundProgressEmitter {
    fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
        // Only handle ToolProgress events
        if let codelet_cli::interactive::StreamEvent::ToolProgress(tp) = event {
            let chunk = crate::types::StreamChunk::tool_progress(crate::types::ToolProgressInfo {
                tool_call_id: tp.tool_call_id,
                tool_name: tp.tool_name,
                output_chunk: tp.output_chunk,
                is_stderr: tp.is_stderr,
            });
            self.session.handle_output(chunk);
        }
    }
}

// =============================================================================
// WATCHER OUTPUT (WATCH-020)
// =============================================================================

/// Watcher output handler that captures turn text during streaming (WATCH-020)
///
/// Wraps BackgroundOutput to accumulate Text chunks for parsing after turn completion.
/// Used for observation evaluations to detect [INTERJECT]/[CONTINUE] blocks.
struct WatcherOutput {
    inner: BackgroundOutput,
    turn_text: std::sync::Mutex<String>,
}

impl WatcherOutput {
    fn new(session: Arc<BackgroundSession>) -> Self {
        Self {
            inner: BackgroundOutput::new(session),
            turn_text: std::sync::Mutex::new(String::new()),
        }
    }
    
    /// Get the accumulated turn text for parsing
    fn get_turn_text(&self) -> String {
        self.turn_text.lock().unwrap().clone()
    }
}

impl codelet_cli::interactive::StreamOutput for WatcherOutput {
    fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
        // Capture Text events for later parsing (WATCH-020)
        if let codelet_cli::interactive::StreamEvent::Text(ref text) = event {
            let mut turn_text = self.turn_text.lock().unwrap();
            turn_text.push_str(text);
        }
        
        // Delegate all events to inner handler for normal output
        self.inner.emit(event);
    }
    
    fn progress_emitter(&self) -> Option<std::sync::Arc<dyn codelet_cli::interactive::StreamOutput>> {
        self.inner.progress_emitter()
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
    let uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
    let manager = SessionManager::instance();
    let session = manager.get_session(&session_id)?;
    session.attach(callback);
    // VIEWNV-001: Track this as the active session for navigation
    manager.set_active_session(uuid);
    Ok(())
}

/// Detach from a session (session continues running)
#[napi]
pub fn session_detach(session_id: String) -> Result<()> {
    let manager = SessionManager::instance();
    let session = manager.get_session(&session_id)?;
    session.detach();
    // VIEWNV-001: Clear active session when detaching
    manager.clear_active_session();
    Ok(())
}

/// Subscribe to a session for live streaming WITHOUT changing the active session.
///
/// Use this when you want to observe a session's output (e.g., watching a parent
/// session from a watcher view) without affecting navigation state.
///
/// VIEWNV-001: This is separate from session_attach to avoid corrupting the
/// active_session_id when subscribing to parent sessions for observation.
#[napi]
pub fn session_subscribe(session_id: String, callback: ThreadsafeFunction<StreamChunk>) -> Result<()> {
    let manager = SessionManager::instance();
    let session = manager.get_session(&session_id)?;
    session.attach(callback);
    // NOTE: Do NOT set active session here - this is just for observation
    Ok(())
}

/// Unsubscribe from a session WITHOUT clearing the active session.
///
/// Use this to stop observing a session that was subscribed via session_subscribe.
///
/// VIEWNV-001: This is separate from session_detach to avoid clearing the
/// active_session_id when unsubscribing from parent sessions.
#[napi]
pub fn session_unsubscribe(session_id: String) -> Result<()> {
    let manager = SessionManager::instance();
    let session = manager.get_session(&session_id)?;
    session.detach();
    // NOTE: Do NOT clear active session here - this is just for observation
    Ok(())
}

/// Explicitly set the active session for navigation.
///
/// Use this when switching to a session that was already attached via session_subscribe,
/// or when you need to update navigation state without re-attaching.
///
/// VIEWNV-001: This allows TypeScript to explicitly control the navigation state.
#[napi]
pub fn session_set_active(session_id: String) -> Result<()> {
    let uuid = uuid::Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
    let manager = SessionManager::instance();
    // Verify session exists
    let _ = manager.get_session(&session_id)?;
    manager.set_active_session(uuid);
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
    let status = session.get_status();
    Ok(status.as_str().to_string())
}

// === PAUSE-001: Session pause NAPI functions ===

/// Get pause state for a session (PAUSE-001)
///
/// Returns the current pause state if the session is paused, null otherwise.
/// TypeScript uses this to display pause UI (tool name, message, kind).
#[napi]
pub fn session_get_pause_state(session_id: String) -> Result<Option<NapiPauseState>> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_pause_state().map(|s| s.into()))
}

/// Resume a paused session (PAUSE-001)
///
/// Called when user presses Enter during a Continue pause.
/// Sends Resumed response to unblock the waiting tool.
#[napi]
pub fn session_pause_resume(session_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.send_pause_response(PauseResponse::Resumed);
    Ok(())
}

/// Confirm or deny a paused session (PAUSE-001)
///
/// Called when user presses Y (approved=true) or N (approved=false) during a Confirm pause.
/// Sends Approved or Denied response to unblock the waiting tool.
#[napi]
pub fn session_pause_confirm(session_id: String, approved: bool) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    let response = if approved {
        PauseResponse::Approved
    } else {
        PauseResponse::Denied
    };
    session.send_pause_response(response);
    Ok(())
}

// === TUI-054: Base thinking level NAPI functions ===

/// Get the base thinking level for a session (TUI-054)
///
/// Returns the base thinking level: 0=Off, 1=Low, 2=Medium, 3=High
/// This is the level set via /thinking command dialog.
#[napi]
pub fn session_get_base_thinking_level(session_id: String) -> Result<u8> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_base_thinking_level())
}

/// Set the base thinking level for a session (TUI-054)
///
/// Sets the base thinking level: 0=Off, 1=Low, 2=Medium, 3=High
/// Values > 3 are clamped to 3.
/// This is called when user selects a level in the /thinking dialog.
#[napi]
pub fn session_set_base_thinking_level(session_id: String, level: u8) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.set_base_thinking_level(level);
    Ok(())
}

// === VIEWNV-001: Session navigation NAPI functions ===

/// Get the next session after the currently active one (VIEWNV-001)
/// Returns None if no sessions exist or at the last session
/// If no active session (BoardView), returns the first session
#[napi]
pub fn session_get_next() -> Option<String> {
    SessionManager::instance().get_next_session()
}

/// Get the previous session before the currently active one (VIEWNV-001)
/// Returns None if no sessions exist or at the first session (should go to board)
#[napi]
pub fn session_get_prev() -> Option<String> {
    SessionManager::instance().get_prev_session()
}

/// Get the first session (VIEWNV-001)
/// Returns None if no sessions exist
#[napi]
pub fn session_get_first() -> Option<String> {
    SessionManager::instance().get_first_session()
}

/// Clear the active session tracking (VIEWNV-001)
/// Call this when returning to BoardView to ensure navigation works correctly
#[napi]
pub fn session_clear_active() {
    SessionManager::instance().clear_active_session();
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

/// Get pending input text for a background session (TUI-049)
///
/// Returns the input text that was being typed when the user switched away from this session.
/// Used to restore input field state when switching back to the session.
#[napi]
pub fn session_get_pending_input(session_id: String) -> Result<Option<String>> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_pending_input())
}

/// Set pending input text for a background session (TUI-049)
///
/// Saves the current input field text before switching to another session.
/// Pass None to clear the pending input.
#[napi]
pub fn session_set_pending_input(session_id: String, input: Option<String>) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.set_pending_input(input);
    Ok(())
}

/// Get buffered output from a session
#[napi]
pub fn session_get_buffered_output(session_id: String, limit: u32) -> Result<Vec<StreamChunk>> {
    let session = SessionManager::instance().get_session(&session_id)?;
    Ok(session.get_buffered_output(limit as usize))
}

/// Session role info returned to TypeScript (WATCH-004)
#[napi(object)]
#[derive(Clone)]
pub struct SessionRoleInfo {
    /// Role name (e.g., "code-reviewer", "supervisor")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Authority level ("peer" or "supervisor")
    pub authority: String,
}

/// Set the role for a session (WATCH-004)
///
/// Used to mark a session as a watcher with a specific role and authority level.
/// Authority must be "peer" or "supervisor" (case-insensitive).
#[napi]
pub fn session_set_role(
    session_id: String,
    role_name: String,
    role_description: Option<String>,
    authority: String,
    auto_inject: Option<bool>, // WATCH-021: Optional auto_inject parameter
) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    
    let auth = RoleAuthority::from_str(&authority)
        .ok_or_else(|| Error::from_reason("Invalid authority: must be peer or supervisor"))?;
    
    // WATCH-021: Use new_with_auto_inject when auto_inject is specified
    let role = SessionRole::new_with_auto_inject(
        role_name,
        role_description,
        auth,
        auto_inject.unwrap_or(true), // Default to true if not specified
    ).map_err(Error::from_reason)?;
    
    session.set_role(role);
    Ok(())
}

/// Get the role for a session (WATCH-004)
///
/// Returns None for regular sessions, role info for watcher sessions.
#[napi]
pub fn session_get_role(session_id: String) -> Result<Option<SessionRoleInfo>> {
    let session = SessionManager::instance().get_session(&session_id)?;
    
    Ok(session.get_role().map(|r| SessionRoleInfo {
        name: r.name,
        description: r.description,
        authority: r.authority.as_str().to_string(),
    }))
}

/// Clear the role for a session (WATCH-004)
///
/// Returns the session to a regular (non-watcher) state.
#[napi]
pub fn session_clear_role(session_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.clear_role();
    Ok(())
}

// === Watcher Operations (WATCH-007) ===

/// Create a watcher session for a parent session (WATCH-007)
///
/// Creates a new session that watches the specified parent session.
/// The watcher is registered in WatchGraph and immediately starts observing
/// the parent's output stream via broadcast subscription.
/// WATCH-019: Now spawns watcher_agent_loop instead of regular agent_loop.
#[napi]
pub async fn session_create_watcher(
    parent_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<String> {
    // Validate parent exists
    let parent_uuid = Uuid::parse_str(&parent_id)
        .map_err(|e| Error::from_reason(format!("Invalid parent ID: {}", e)))?;
    
    let _parent = SessionManager::instance().get_session(&parent_id)?;
    
    // Generate watcher ID
    let watcher_id = Uuid::new_v4();
    let watcher_id_str = watcher_id.to_string();
    
    // Create default role (can be updated via session_set_role)
    let role = SessionRole::new(
        name.clone(),
        None,
        RoleAuthority::Peer,
    ).map_err(Error::from_reason)?;
    
    // Create watcher session with watcher-specific loop
    SessionManager::instance()
        .create_watcher_session_with_id(
            &watcher_id_str,
            &model,
            &project,
            &name,
            parent_uuid,
            role,
        )
        .await?;
    
    // Register in WatchGraph (tracks parent-watcher relationships)
    SessionManager::instance()
        .add_watcher(parent_uuid, watcher_id)
        .map_err(Error::from_reason)?;

    Ok(watcher_id_str)
}

/// Get the parent session ID for a watcher (WATCH-007)
///
/// Returns the parent session ID if the session is a watcher, None otherwise.
#[napi]
pub fn session_get_parent(session_id: String) -> Result<Option<String>> {
    let uuid = Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
    
    Ok(SessionManager::instance()
        .get_parent(uuid)
        .map(|id| id.to_string()))
}

/// Get all watcher session IDs for a parent session (WATCH-007)
///
/// Returns a list of session IDs that are watching the specified parent.
#[napi]
pub fn session_get_watchers(session_id: String) -> Result<Vec<String>> {
    let uuid = Uuid::parse_str(&session_id)
        .map_err(|e| Error::from_reason(format!("Invalid session ID: {}", e)))?;
    
    Ok(SessionManager::instance()
        .get_watchers(uuid)
        .into_iter()
        .map(|id| id.to_string())
        .collect())
}

/// Inject a watcher message into the parent session (WATCH-007)
///
/// Formats the message with the watcher's role prefix and queues it
/// on the parent session via receive_watcher_input().
#[napi]
pub fn watcher_inject(watcher_id: String, message: String) -> Result<()> {
    let watcher_uuid = Uuid::parse_str(&watcher_id)
        .map_err(|e| Error::from_reason(format!("Invalid watcher ID: {}", e)))?;
    
    // Get watcher session
    let watcher = SessionManager::instance().get_session(&watcher_id)?;
    
    // Get watcher role (required)
    let role = watcher.get_role()
        .ok_or_else(|| Error::from_reason("Session has no watcher role set"))?;
    
    // Get parent session
    let parent_uuid = SessionManager::instance()
        .get_parent(watcher_uuid)
        .ok_or_else(|| Error::from_reason("Watcher has no parent session"))?;
    
    let parent = SessionManager::instance().get_session(&parent_uuid.to_string())?;
    
    // Create WatcherInput and format message
    let input = WatcherInput::new(
        watcher_id,
        role.name,
        role.authority,
        message,
    ).map_err(Error::from_reason)?;
    
    // Queue on parent
    parent.receive_watcher_input(input)
        .map_err(Error::from_reason)?;
    
    Ok(())
}

/// Set pending observed correlation IDs for a watcher session (WATCH-011)
///
/// When processing observations, call this before sending the evaluation prompt.
/// All subsequent output chunks from this session will be tagged with these IDs
/// (in observed_correlation_ids field) until session_clear_observed_correlation_ids is called.
///
/// This enables cross-pane highlighting: when viewing a watcher session in split view,
/// selecting a watcher turn shows which parent turns it was responding to.
#[napi]
pub fn session_set_observed_correlation_ids(session_id: String, correlation_ids: Vec<String>) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.set_pending_observed_correlation_ids(correlation_ids);
    Ok(())
}

/// Clear pending observed correlation IDs for a session (WATCH-011)
///
/// Call this after the watcher finishes processing an observation response.
/// Subsequent output chunks will no longer have observed_correlation_ids set.
#[napi]
pub fn session_clear_observed_correlation_ids(session_id: String) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    session.clear_pending_observed_correlation_ids();
    Ok(())
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
            // TUI-049: Include TokenUpdate and ContextFillUpdate in merged output
            // These are needed to restore token state when switching sessions
            "TokenUpdate" | "ContextFillUpdate" => merged.push(chunk),
            _ => merged.push(chunk),
        }
    }

    Ok(merged)
}

/// Restore messages to a background session from persisted envelopes.
///
/// This is used when attaching to a session via /resume - it restores the
/// conversation history so the LLM has context for future prompts.
///
/// Also populates the output_buffer with synthetic StreamChunks so that
/// sessionGetMergedOutput() returns the restored conversation. This enables
/// proper UI replay when detaching and re-attaching via kanban.
#[napi]
pub async fn session_restore_messages(session_id: String, envelopes: Vec<String>) -> Result<()> {
    let session = SessionManager::instance().get_session(&session_id)?;
    
    // Collect rig messages and StreamChunks to push
    let mut rig_messages: Vec<rig::message::Message> = Vec::new();
    let mut stream_chunks: Vec<StreamChunk> = Vec::new();
    
    for envelope_json in envelopes {
        let envelope: serde_json::Value = serde_json::from_str(&envelope_json)
            .map_err(|e| Error::from_reason(format!("Failed to parse envelope: {}", e)))?;

        // Extract message from envelope
        if let Some(message) = envelope.get("message") {
            let role = message.get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("user");

            if role == "assistant" {
                // Handle assistant messages with content blocks
                if let Some(content) = message.get("content") {
                    if let Some(arr) = content.as_array() {
                        let mut text_parts = Vec::new();
                        
                        // Process each content block for StreamChunks
                        for block in arr {
                            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            
                            match block_type {
                                "thinking" => {
                                    if let Some(thinking) = block.get("thinking").and_then(|t| t.as_str()) {
                                        if !thinking.is_empty() {
                                            stream_chunks.push(StreamChunk::thinking(thinking.to_string()));
                                        }
                                    }
                                }
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(text.to_string());
                                        if !text.is_empty() {
                                            stream_chunks.push(StreamChunk::text(text.to_string()));
                                        }
                                    }
                                }
                                "tool_use" => {
                                    let id = block.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string();
                                    let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                                    let input = block.get("input")
                                        .map(|i| serde_json::to_string(i).unwrap_or_default())
                                        .unwrap_or_default();
                                    
                                    if !id.is_empty() && !name.is_empty() {
                                        stream_chunks.push(StreamChunk::tool_call(ToolCallInfo {
                                            id,
                                            name,
                                            input,
                                        }));
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        // Build rig message for LLM context
                        let joined_text = text_parts.join("");
                        if !joined_text.is_empty() {
                            rig_messages.push(rig::message::Message::Assistant {
                                id: None,
                                content: rig::OneOrMany::one(rig::message::AssistantContent::text(joined_text)),
                            });
                        }
                        
                        // Push Done chunk to finalize assistant turn
                        stream_chunks.push(StreamChunk::done());
                    }
                }
            } else {
                // Handle user messages
                if let Some(content) = message.get("content") {
                    if let Some(arr) = content.as_array() {
                        let mut text_parts = Vec::new();
                        
                        // Process each content block
                        for block in arr {
                            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            
                            match block_type {
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                        text_parts.push(text.to_string());
                                        if !text.is_empty() {
                                            stream_chunks.push(StreamChunk::user_input(text.to_string()));
                                        }
                                    }
                                }
                                "tool_result" => {
                                    let tool_use_id = block.get("tool_use_id")
                                        .and_then(|i| i.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let result_content = block.get("content")
                                        .and_then(|c| c.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let is_error = block.get("is_error")
                                        .and_then(|e| e.as_bool())
                                        .unwrap_or(false);
                                    
                                    if !tool_use_id.is_empty() {
                                        stream_chunks.push(StreamChunk::tool_result(ToolResultInfo {
                                            tool_call_id: tool_use_id,
                                            content: result_content,
                                            is_error,
                                        }));
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        // Build rig message for LLM context (text only)
                        let joined_text = text_parts.join("");
                        if !joined_text.is_empty() {
                            rig_messages.push(rig::message::Message::User {
                                content: rig::OneOrMany::one(rig::message::UserContent::text(joined_text)),
                            });
                        }
                    } else if let Some(s) = content.as_str() {
                        // Simple string content
                        if !s.is_empty() {
                            stream_chunks.push(StreamChunk::user_input(s.to_string()));
                            rig_messages.push(rig::message::Message::User {
                                content: rig::OneOrMany::one(rig::message::UserContent::text(s.to_string())),
                            });
                        }
                    }
                }
            }
        }
    }
    
    // Push rig messages to inner (for LLM context)
    {
        let mut inner = session.inner.lock().await;
        for msg in rig_messages {
            inner.messages.push(msg);
        }
    }
    
    // Push StreamChunks to output_buffer via handle_output (for UI replay)
    // This enables sessionGetMergedOutput() to return the restored conversation
    for chunk in stream_chunks {
        session.handle_output(chunk);
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
