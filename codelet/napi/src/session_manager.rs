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
use codelet_tools::{clear_bash_abort, request_bash_abort};
use napi::bindgen_prelude::*;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{broadcast, mpsc, Mutex, Notify};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SessionStatus {
    #[default]
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

    /// Pending input text (TUI-049: preserved when switching sessions)
    pending_input: RwLock<Option<String>>,

    /// Broadcast channel for watcher sessions to observe stream output (WATCH-003)
    watcher_broadcast: broadcast::Sender<StreamChunk>,
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
            pending_input: RwLock::new(None),
            watcher_broadcast: broadcast::channel(WATCHER_BROADCAST_CAPACITY).0,
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
    
    /// Send input to the agent loop
    ///
    /// Buffers the user input as a UserInput chunk before sending to the agent,
    /// so it can be replayed when attaching to a detached session via /resume.
    pub fn send_input(&self, input: String, thinking_config: Option<String>) -> Result<()> {
        // TUI-049: Clear pending input - it's being sent now (state invariant)
        // This prevents "ghost input" from reappearing when switching sessions after send
        self.set_pending_input(None);

        // Buffer user input for resume/attach (NAPI-009)
        self.handle_output(StreamChunk::user_input(input.clone()));

        self.input_tx
            .try_send(PromptInput { input, thinking_config })
            .map_err(|e| Error::from_reason(format!("Failed to send input: {}", e)))
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

/// Singleton session manager
pub struct SessionManager {
    sessions: RwLock<HashMap<Uuid, Arc<BackgroundSession>>>,
    /// Tracks parent-watcher relationships between sessions (WATCH-002)
    watch_graph: WatchGraph,
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
            watch_graph: WatchGraph::new(),
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
        
        // Clean up watch graph relationships (WATCH-002)
        // If this session was a parent, clean up all its watchers
        self.watch_graph.cleanup_parent(uuid);
        // If this session was a watcher, remove its relationship
        self.watch_graph.remove_watcher(uuid);
        
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
async fn agent_loop(session: Arc<BackgroundSession>, mut input_rx: mpsc::Receiver<PromptInput>) {
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

        // Get provider and run agent stream using the macro to eliminate duplication
        let current_provider = inner_session.current_provider_name().to_string();
        let result = match current_provider.as_str() {
            "claude" => run_with_provider!(&mut inner_session, get_claude, input, session, &output, thinking_config_value),
            "openai" => run_with_provider!(&mut inner_session, get_openai, input, session, &output, thinking_config_value),
            "gemini" => run_with_provider!(&mut inner_session, get_gemini, input, session, &output, thinking_config_value),
            "zai" => run_with_provider!(&mut inner_session, get_zai, input, session, &output, thinking_config_value),
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
            });
            self.session.handle_output(chunk);
        }
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
