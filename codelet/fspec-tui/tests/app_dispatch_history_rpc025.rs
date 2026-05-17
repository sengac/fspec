//! RPC-025 — App::dispatch routing for Shift+↑/↓ history recall and
//! fire-and-forget submission persistence.
//!
//! Feature: spec/features/rpc025-app-dispatch.feature
//!
//! Drives the App::dispatch state machine with a programmable
//! HistoryMockBackend that scripts persistence_get_history return values
//! and records persistence_add_history call captures with optional
//! latches.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    CheckpointCounts, HealthInfo, HistoryMatch, LogRecord, ModelInfo, SessionId, SessionInfo,
    StreamChunk, ThinkingLevel, WorkUnitInfo, WorkspaceInfo,
};
use tokio::sync::{broadcast, Notify};

/// Programmable fake backend for App::dispatch history tests.
/// Captures call counts and lets tests script the
/// `persistence_get_history` return value per-session.
pub struct HistoryMockBackend {
    work_units_tx: broadcast::Sender<Vec<WorkUnitInfo>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    pub send_input_calls: Mutex<Vec<(SessionId, String)>>,
    pub persistence_add_history_calls: Mutex<Vec<(SessionId, String)>>,
    pub persistence_get_history_calls: Mutex<Vec<(SessionId, u32)>>,
    /// Per-session scripted return for persistence_get_history.
    scripted_history: Mutex<HashMap<SessionId, Vec<String>>>,
    /// When `Some(notify)`, persistence_add_history blocks on `notify.notified().await`
    /// before completing — lets the latch test prove fire-and-forget.
    add_history_latch: Mutex<Option<Arc<Notify>>>,
}

impl Default for HistoryMockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryMockBackend {
    pub fn new() -> Self {
        let (work_units_tx, _) = broadcast::channel(8);
        let (chunks_tx, _) = broadcast::channel(8);
        let (logs_tx, _) = broadcast::channel(8);
        Self {
            work_units_tx,
            chunks_tx,
            logs_tx,
            send_input_calls: Mutex::new(Vec::new()),
            persistence_add_history_calls: Mutex::new(Vec::new()),
            persistence_get_history_calls: Mutex::new(Vec::new()),
            scripted_history: Mutex::new(HashMap::new()),
            add_history_latch: Mutex::new(None),
        }
    }

    pub fn script_history(&self, session: SessionId, entries: Vec<String>) {
        self.scripted_history
            .lock()
            .expect("scripted_history mutex")
            .insert(session, entries);
    }

    pub fn install_add_history_latch(&self, latch: Arc<Notify>) {
        *self.add_history_latch.lock().expect("latch mutex") = Some(latch);
    }

    pub fn get_history_call_count(&self) -> usize {
        self.persistence_get_history_calls
            .lock()
            .expect("get_history calls mutex")
            .len()
    }
}

#[async_trait]
impl FspecBackend for HistoryMockBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(Vec::new())
    }
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        Ok(Vec::new())
    }
    async fn create_session(&self, _role: Option<String>) -> Result<SessionId> {
        Ok(SessionId::new("s-mock"))
    }
    async fn send_input(&self, id: SessionId, text: String) -> Result<()> {
        self.send_input_calls
            .lock()
            .expect("send_input mutex")
            .push((id, text));
        Ok(())
    }
    async fn interrupt(&self, _id: SessionId) -> Result<()> {
        Ok(())
    }
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.work_units_tx.subscribe()
    }
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }
    async fn health(&self) -> Result<HealthInfo> {
        Ok(HealthInfo {
            uptime_secs: 0,
            connected_clients: 0,
            last_watcher_event_secs_ago: None,
            lag_chunks: 0,
            lag_logs: 0,
            lag_work_units: 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    async fn checkpoint_counts(&self) -> Result<CheckpointCounts> {
        Ok(CheckpointCounts::default())
    }
    async fn move_work_unit_up(&self, _id: String) -> Result<()> {
        Ok(())
    }
    async fn move_work_unit_down(&self, _id: String) -> Result<()> {
        Ok(())
    }
    async fn get_model_info(&self, _session_id: SessionId) -> Result<ModelInfo> {
        Ok(ModelInfo::default())
    }
    async fn get_thinking_level(&self, _session_id: SessionId) -> Result<ThinkingLevel> {
        Ok(ThinkingLevel::Off)
    }
    async fn get_workspace_info(&self) -> Result<WorkspaceInfo> {
        Ok(WorkspaceInfo::default())
    }
    async fn search_files(&self, _prefix: String, _limit: u32) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()> {
        let latch = self
            .add_history_latch
            .lock()
            .expect("latch mutex")
            .clone();
        if let Some(notify) = latch {
            notify.notified().await;
        }
        self.persistence_add_history_calls
            .lock()
            .expect("add_history mutex")
            .push((session, text));
        Ok(())
    }
    async fn persistence_get_history(
        &self,
        session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>> {
        self.persistence_get_history_calls
            .lock()
            .expect("get_history mutex")
            .push((session.clone(), limit));
        let scripted = self
            .scripted_history
            .lock()
            .expect("scripted_history mutex")
            .get(&session)
            .cloned()
            .unwrap_or_default();
        Ok(scripted)
    }
    async fn persistence_search_history(&self, _query: String) -> Result<Vec<HistoryMatch>> {
        Ok(Vec::new())
    }
}

fn fresh_app() -> (App, Arc<HistoryMockBackend>) {
    let mock = Arc::new(HistoryMockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Scenario: First Shift+↑ caches the live draft, loads a fresh snapshot, and replaces input with history[0]
#[tokio::test(flavor = "current_thread")]
async fn first_shift_up_caches_draft_loads_snapshot_replaces_input() {
    // @step Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And history_state_by_session is empty
    // @step And cached_history_snapshot is empty
    // @step And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return ["third", "second", "first"]
    mock.script_history(
        sid("s-1"),
        vec!["third".to_string(), "second".to_string(), "first".to_string()],
    );
    // @step And the MultiLineInput buffer is "live draft"
    app.navigator_mut().agent.input.set_value("live draft");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    let state = app
        .agent_view_store()
        .history_state_for(&sid("s-1"))
        .expect("history state must exist after first HistoryPrev");
    assert_eq!(state.recall_index, Some(0));
    // @step And history_state_by_session[SessionId("s-1")].cached_draft equals Some("live draft")
    assert_eq!(state.cached_draft.as_deref(), Some("live draft"));
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["third", "second", "first"]
    let snap = app
        .agent_view_store()
        .cached_history_snapshot(&sid("s-1"))
        .expect("snapshot must be cached after first HistoryPrev");
    assert_eq!(
        snap,
        &vec!["third".to_string(), "second".to_string(), "first".to_string()]
    );
    // @step And the MultiLineInput buffer equals "third"
    assert_eq!(app.navigator().agent.input.value(), "third");
}

/// Scenario: Subsequent Shift+↑ walks backwards through the cached snapshot and clamps at len-1
#[tokio::test(flavor = "current_thread")]
async fn subsequent_shift_up_walks_back_and_clamps() {
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.script_history(sid("s-1"), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    app.navigator_mut().agent.input.set_value("live");
    // @step Given an AgentViewStore where history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(0), cached_draft: Some("live") }
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    let calls_after_first = mock.get_history_call_count();
    assert_eq!(calls_after_first, 1);
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(1)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(1)
    );
    // @step And the MultiLineInput buffer equals "b"
    assert_eq!(app.navigator().agent.input.value(), "b");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(2)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(2)
    );
    // @step And the MultiLineInput buffer equals "c"
    assert_eq!(app.navigator().agent.input.value(), "c");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index stays at Some(2)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(2)
    );
    // @step And the MultiLineInput buffer is still "c"
    assert_eq!(app.navigator().agent.input.value(), "c");
    // @step And the backend's persistence_get_history was called exactly 0 additional times (snapshot is cached)
    assert_eq!(mock.get_history_call_count(), calls_after_first);
}

/// Scenario: Shift+↓ walks forwards and exits recall on the final step restoring the cached_draft
#[tokio::test(flavor = "current_thread")]
async fn shift_down_walks_forward_and_exits_recall() {
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.script_history(sid("s-1"), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    app.navigator_mut().agent.input.set_value("live");
    // @step Given an AgentViewStore where history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(2), cached_draft: Some("live") }
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(2)
    );
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(1)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(1)
    );
    // @step And the MultiLineInput buffer equals "b"
    assert_eq!(app.navigator().agent.input.value(), "b");
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And the MultiLineInput buffer equals "a"
    assert_eq!(app.navigator().agent.input.value(), "a");
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals None
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        None
    );
    // @step And the MultiLineInput buffer equals "live"
    assert_eq!(app.navigator().agent.input.value(), "live");
}

/// Scenario: Shift+↓ from live-draft mode is a no-op
#[tokio::test(flavor = "current_thread")]
async fn shift_down_from_live_draft_mode_is_a_noop() {
    // @step Given an AgentViewStore where history_state_by_session[SessionId("s-1")].recall_index is None
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And the MultiLineInput buffer is "current draft"
    app.navigator_mut().agent.input.set_value("current draft");
    // @step When App::dispatch handles Action::HistoryNext
    app.dispatch(Action::HistoryNext);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index stays None
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        None
    );
    // @step And the MultiLineInput buffer is unchanged at "current draft"
    assert_eq!(app.navigator().agent.input.value(), "current draft");
}

/// Scenario: Shift+↑ when history is empty is a no-op
#[tokio::test(flavor = "current_thread")]
async fn shift_up_with_empty_history_is_a_noop() {
    // @step Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return an empty Vec
    mock.script_history(sid("s-1"), Vec::new());
    // @step And the MultiLineInput buffer is "untouched"
    app.navigator_mut().agent.input.set_value("untouched");
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index stays at None
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        None
    );
    // @step And cached_history_snapshot[SessionId("s-1")] is empty
    let snap = app.agent_view_store().cached_history_snapshot(&sid("s-1"));
    assert!(snap.map(std::vec::Vec::is_empty).unwrap_or(true));
    // @step And the MultiLineInput buffer is unchanged at "untouched"
    assert_eq!(app.navigator().agent.input.value(), "untouched");
}

/// Scenario: Each open session keeps an independent recall position across Shift+←/→ cycling
#[tokio::test(flavor = "current_thread")]
async fn each_session_keeps_independent_recall_across_cycling() {
    // @step Given an AgentViewStore with open_sessions [SessionId("s-1"), SessionId("s-2")] and current_session_index == 0
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionPrev); // focus s-1
    // @step And the backend's persistence_get_history(SessionId("s-1"), 100) is stubbed to return ["a", "b"]
    mock.script_history(sid("s-1"), vec!["a".to_string(), "b".to_string()]);
    // @step And the backend's persistence_get_history(SessionId("s-2"), 100) is stubbed to return ["x", "y", "z"]
    mock.script_history(sid("s-2"), vec!["x".to_string(), "y".to_string(), "z".to_string()]);
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-1")].recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And the MultiLineInput buffer equals "a"
    assert_eq!(app.navigator().agent.input.value(), "a");
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then current_session_index equals 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And the MultiLineInput buffer is restored from open_sessions[1].input_draft (NOT from history)
    let s2_draft = app.agent_view_store().open_sessions()[1].input_draft.clone();
    assert_eq!(app.navigator().agent.input.value(), s2_draft);
    // @step When App::dispatch handles Action::HistoryPrev
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then history_state_by_session[SessionId("s-2")].recall_index equals Some(0)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-2")).and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And the MultiLineInput buffer equals "x"
    assert_eq!(app.navigator().agent.input.value(), "x");
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then history_state_by_session[SessionId("s-1")].recall_index is preserved at Some(0)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(0)
    );
    // @step And history_state_by_session[SessionId("s-2")].recall_index is preserved at Some(0)
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-2")).and_then(|s| s.recall_index),
        Some(0)
    );
}

/// Scenario: Submitting an input fires persistence_add_history via tokio::spawn and resets the session's history state
#[tokio::test(flavor = "current_thread")]
async fn submitting_input_fires_add_history_and_resets_state() {
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    mock.script_history(sid("s-1"), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    app.navigator_mut().agent.input.set_value("live");
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    app.dispatch(Action::HistoryPrev);
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Given an AgentViewStore with open_sessions[0].id == SessionId("s-1") and current_session_index == 0
    // @step And history_state_by_session[SessionId("s-1")] is HistoryNavState { recall_index: Some(1), cached_draft: Some("live") }
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        Some(1)
    );
    // @step And cached_history_snapshot[SessionId("s-1")] equals ["a", "b", "c"]
    assert_eq!(
        app.agent_view_store().cached_history_snapshot(&sid("s-1")).expect("snap"),
        &vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    // @step When App::dispatch handles Action::InputSubmitted("hello") for SessionId("s-1")
    app.dispatch(Action::InputSubmitted("hello".to_string()));
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    // @step Then backend.send_input(SessionId("s-1"), "hello") is invoked
    let sent = mock.send_input_calls.lock().expect("send_input").clone();
    assert!(sent.iter().any(|(s, t)| s == &sid("s-1") && t == "hello"));
    // @step And backend.persistence_add_history(SessionId("s-1"), "hello") is invoked via tokio::spawn
    let added = mock.persistence_add_history_calls.lock().expect("add_history").clone();
    assert!(added.iter().any(|(s, t)| s == &sid("s-1") && t == "hello"));
    // @step And history_state_by_session[SessionId("s-1")].recall_index is reset to None
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.recall_index),
        None
    );
    // @step And history_state_by_session[SessionId("s-1")].cached_draft is reset to None
    assert_eq!(
        app.agent_view_store().history_state_for(&sid("s-1")).and_then(|s| s.cached_draft.clone()),
        None
    );
    // @step And cached_history_snapshot[SessionId("s-1")] is cleared
    let snap = app.agent_view_store().cached_history_snapshot(&sid("s-1"));
    assert!(snap.map(std::vec::Vec::is_empty).unwrap_or(true));
    // @step And the next Action::HistoryPrev triggers a fresh persistence_get_history round-trip
    let before = mock.get_history_call_count();
    app.dispatch(Action::HistoryPrev);
    tokio::task::yield_now().await;
    while let Some(action) = app.try_recv_action() { app.dispatch(action); }
    assert!(mock.get_history_call_count() > before);
}

/// Scenario: persistence_add_history is fire-and-forget — a slow disk write never blocks input submission
#[tokio::test(flavor = "current_thread")]
async fn persistence_add_history_is_fire_and_forget() {
    // @step Given a fake FspecBackend whose persistence_add_history blocks indefinitely on an internal latch
    let (mut app, mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    let latch = Arc::new(Notify::new());
    mock.install_add_history_latch(latch.clone());
    // @step When App::dispatch handles Action::InputSubmitted("hello") for SessionId("s-1")
    let start = Instant::now();
    app.dispatch(Action::InputSubmitted("hello".to_string()));
    let elapsed = start.elapsed();
    // @step Then the dispatch call returns within 50 milliseconds without awaiting persistence_add_history
    assert!(elapsed < Duration::from_millis(50), "dispatch took {elapsed:?}");
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    // @step And backend.send_input(SessionId("s-1"), "hello") is invoked exactly once
    let sent = mock.send_input_calls.lock().expect("send_input").clone();
    assert_eq!(
        sent.iter().filter(|(s, t)| s == &sid("s-1") && t == "hello").count(),
        1
    );
    // @step And the spawned persistence_add_history task is still pending (the latch has not been released)
    let added = mock.persistence_add_history_calls.lock().expect("add_history").clone();
    assert!(added.is_empty(), "add_history must still be latched");
    latch.notify_one();
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
}
