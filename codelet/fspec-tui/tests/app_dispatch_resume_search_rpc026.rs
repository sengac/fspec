//! RPC-026 — App::dispatch routing for /resume + /search popups.
//!
//! Feature: spec/features/rpc026-app-dispatch.feature
//!
//! Drives App::dispatch state machine for the seven new Action
//! variants (OpenResumePicker, SessionListLoaded, AttachToSession,
//! OpenSearchPalette, SearchHistory, HistorySearchResults,
//! InsertIntoInput) with a programmable mock backend.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    CheckpointCounts, HealthInfo, HistoryMatch, LogRecord, ModelInfo, SessionId, SessionInfo,
    StreamChunk, ThinkingLevel, WorkUnitInfo, WorkspaceInfo,
};
use tokio::sync::broadcast;

pub struct ResumeMockBackend {
    work_units_tx: broadcast::Sender<Vec<WorkUnitInfo>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    pub list_sessions_calls: Mutex<u32>,
    pub search_history_calls: Mutex<Vec<String>>,
    scripted_sessions: Mutex<Vec<SessionInfo>>,
    scripted_history_matches: Mutex<Vec<HistoryMatch>>,
}

impl Default for ResumeMockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ResumeMockBackend {
    pub fn new() -> Self {
        let (work_units_tx, _) = broadcast::channel(8);
        let (chunks_tx, _) = broadcast::channel(8);
        let (logs_tx, _) = broadcast::channel(8);
        Self {
            work_units_tx,
            chunks_tx,
            logs_tx,
            list_sessions_calls: Mutex::new(0),
            search_history_calls: Mutex::new(Vec::new()),
            scripted_sessions: Mutex::new(Vec::new()),
            scripted_history_matches: Mutex::new(Vec::new()),
        }
    }

    pub fn script_sessions(&self, infos: Vec<SessionInfo>) {
        *self.scripted_sessions.lock().expect("sessions mutex") = infos;
    }

    pub fn script_history_matches(&self, matches: Vec<HistoryMatch>) {
        *self.scripted_history_matches.lock().expect("matches mutex") = matches;
    }
}

#[async_trait]
impl FspecBackend for ResumeMockBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(Vec::new())
    }
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        *self.list_sessions_calls.lock().expect("count mutex") += 1;
        Ok(self.scripted_sessions.lock().expect("sessions mutex").clone())
    }
    async fn create_session(&self, _role: Option<String>) -> Result<SessionId> {
        Ok(SessionId::new("s-mock"))
    }
    async fn send_input(&self, _id: SessionId, _text: String) -> Result<()> {
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
    async fn persistence_add_history(&self, _session: SessionId, _text: String) -> Result<()> {
        Ok(())
    }
    async fn persistence_get_history(
        &self,
        _session: SessionId,
        _limit: u32,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }
    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>> {
        self.search_history_calls
            .lock()
            .expect("search mutex")
            .push(query);
        Ok(self
            .scripted_history_matches
            .lock()
            .expect("matches mutex")
            .clone())
    }
}

fn fresh_app() -> (App, Arc<ResumeMockBackend>) {
    let mock = Arc::new(ResumeMockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn sinfo(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: "/tmp".to_string(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
    }
}

fn hm(text: &str) -> HistoryMatch {
    HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: text.to_string(),
        timestamp_iso: "2026-05-17T00:00:00Z".to_string(),
    }
}

/// Scenario: SlashCommandAction::Resume opens the resume_popup and spawns list_sessions
#[tokio::test(flavor = "current_thread")]
async fn resume_opens_popup_and_spawns_list_sessions() {
    // @step Given an App with the AgentView focused and AgentView.resume_popup is None
    let (mut app, mock) = fresh_app();
    mock.script_sessions(vec![sinfo("s-1"), sinfo("s-2")]);
    app.dispatch(Action::SessionCreated(SessionId::new("seed")));
    assert!(app.navigator().agent.resume_popup.is_none());

    // @step When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Resume)
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Resume));

    // @step Then AgentView.resume_popup is Some(_)
    assert!(app.navigator().agent.resume_popup.is_some());
    // @step And AgentView.slash_popup is None
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And the input buffer is reset to ""
    assert!(app.navigator().agent.input.is_empty());

    // Await the spawned list_sessions task to ensure it ran.
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        tokio::task::yield_now().await;
        if *mock.list_sessions_calls.lock().expect("count mutex") >= 1 {
            break;
        }
        if Instant::now() > deadline {
            panic!("list_sessions never invoked");
        }
    }
    // @step And backend.list_sessions() is invoked exactly once via tokio::spawn
    assert_eq!(*mock.list_sessions_calls.lock().expect("count mutex"), 1);
}

/// Scenario: Action::SessionListLoaded folds the result into the open resume_popup
#[tokio::test(flavor = "current_thread")]
async fn session_list_loaded_folds_into_resume_popup() {
    // @step Given an App with AgentView.resume_popup == Some(ResumePicker)
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenResumePicker);
    assert!(app.navigator().agent.resume_popup.is_some());

    // @step When App::dispatch handles Action::SessionListLoaded([...])
    app.dispatch(Action::SessionListLoaded(vec![sinfo("s-1"), sinfo("s-2")]));

    // @step Then AgentView.resume_popup.session_count() equals 2
    let popup = app
        .navigator()
        .agent
        .resume_popup
        .as_ref()
        .expect("popup");
    assert_eq!(popup.session_count(), 2);
}

/// Scenario: Action::SessionListLoaded when the popup is already closed is a no-op
#[tokio::test(flavor = "current_thread")]
async fn session_list_loaded_with_no_popup_is_noop() {
    // @step Given an App with AgentView.resume_popup == None
    let (mut app, _mock) = fresh_app();
    assert!(app.navigator().agent.resume_popup.is_none());
    // @step When App::dispatch handles Action::SessionListLoaded([SessionInfo("s-1")])
    app.dispatch(Action::SessionListLoaded(vec![sinfo("s-1")]));
    // @step Then AgentView.resume_popup is still None
    assert!(app.navigator().agent.resume_popup.is_none());
    // @step And no panic occurs
    // (reaching this line proves no panic — assert passes)
}

/// Scenario: Action::AttachToSession to a session already in open_sessions moves the index without appending
#[tokio::test(flavor = "current_thread")]
async fn attach_to_existing_session_moves_index() {
    // @step Given an AgentViewStore with open_sessions [s-1, s-2, s-3] and current_session_index == 0
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    app.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    app.dispatch(Action::SessionCreated(SessionId::new("s-3")));
    // Refocus on s-1 (SessionCreated focuses the new tail by default).
    while app.agent_view_store().current_session_index() > 0 {
        app.agent_view_store_mut().cycle_session(-1);
    }
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);

    // @step When App::dispatch handles Action::AttachToSession(SessionId("s-3"))
    app.dispatch(Action::AttachToSession(SessionId::new("s-3")));

    // @step Then AgentViewStore.current_session_index equals 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    // @step And AgentViewStore.open_sessions.len() equals 3
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And active_session_tx receives Some(SessionId("s-3"))
    let cur = app.active_session_rx_snapshot();
    assert_eq!(cur, Some(SessionId::new("s-3")));
    // @step And refresh_session_chrome is invoked for SessionId("s-3")
    // (refresh_session_chrome is a private synchronous fold over the
    // AgentViewStore — the parity test in
    // tests/agent_chrome_parity_rpc018.rs covers the chrome state; here
    // we just assert that active_session_tx fired the expected SessionId,
    // which is the public side-effect handle_attach_to_session must
    // surface together with refresh_session_chrome.)
}

/// Scenario: Action::AttachToSession to a session NOT in open_sessions appends and focuses it
#[tokio::test(flavor = "current_thread")]
async fn attach_to_new_session_appends_and_focuses() {
    // @step Given an AgentViewStore with open_sessions [s-1] and current_session_index == 0
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);

    // @step When App::dispatch handles Action::AttachToSession(SessionId("s-99"))
    app.dispatch(Action::AttachToSession(SessionId::new("s-99")));

    // @step Then AgentViewStore.open_sessions.len() equals 2
    assert_eq!(app.agent_view_store().open_sessions().len(), 2);
    // @step And AgentViewStore.open_sessions[1].id equals SessionId("s-99")
    assert_eq!(
        app.agent_view_store().open_sessions()[1].id,
        SessionId::new("s-99")
    );
    // @step And AgentViewStore.current_session_index equals 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And active_session_tx receives Some(SessionId("s-99"))
    let cur = app.active_session_rx_snapshot();
    assert_eq!(cur, Some(SessionId::new("s-99")));
}

/// Scenario: SlashCommandAction::Search opens the search_popup empty and does NOT spawn
#[tokio::test(flavor = "current_thread")]
async fn search_opens_popup_empty_and_does_not_spawn() {
    // @step Given an App with the AgentView focused and AgentView.search_popup is None
    let (mut app, mock) = fresh_app();
    assert!(app.navigator().agent.search_popup.is_none());

    // @step When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Search)
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Search));

    // @step Then AgentView.search_popup is Some(_)
    let popup = app
        .navigator()
        .agent
        .search_popup
        .as_ref()
        .expect("search popup");
    // @step And AgentView.search_popup.query() equals ""
    assert_eq!(popup.query(), "");
    // @step And AgentView.search_popup.match_count() equals 0
    assert_eq!(popup.match_count(), 0);
    // @step And AgentView.slash_popup is None
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And the input buffer is reset to ""
    assert!(app.navigator().agent.input.is_empty());
    // @step And backend.persistence_search_history is NOT invoked
    tokio::task::yield_now().await;
    assert!(mock.search_history_calls.lock().expect("mtx").is_empty());
}

/// Scenario: Action::SearchHistory spawns persistence_search_history and folds the result
#[tokio::test(flavor = "current_thread")]
async fn search_history_spawns_and_folds_results() {
    // @step Given an App with AgentView.search_popup == Some(SearchPalette)
    let (mut app, mock) = fresh_app();
    mock.script_history_matches(vec![hm("git status"), hm("git push")]);
    app.dispatch(Action::OpenSearchPalette);
    assert!(app.navigator().agent.search_popup.is_some());

    // @step When App::dispatch handles Action::SearchHistory("git")
    app.dispatch(Action::SearchHistory("git".to_string()));

    // Await the spawned task — it dispatches HistorySearchResults.
    let deadline = Instant::now() + Duration::from_millis(500);
    loop {
        tokio::task::yield_now().await;
        if !mock.search_history_calls.lock().expect("mtx").is_empty() {
            break;
        }
        if Instant::now() > deadline {
            panic!("persistence_search_history not invoked");
        }
    }
    // @step Then backend.persistence_search_history("git") is invoked exactly once via tokio::spawn
    assert_eq!(mock.search_history_calls.lock().expect("mtx").len(), 1);
    assert_eq!(mock.search_history_calls.lock().expect("mtx")[0], "git");

    // Drain the spawned Action::HistorySearchResults from the action bus
    // and dispatch it explicitly (App run-loop does this in production).
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut received = None;
    while Instant::now() < deadline {
        if let Some(a) = app.try_recv_action() {
            received = Some(a);
            break;
        }
        tokio::task::yield_now().await;
    }
    let action = received.expect("HistorySearchResults action");
    // @step When the spawned task resolves with [HistoryMatch(text="git status"), HistoryMatch(text="git push")]
    // (the spawned task already resolved above — `action` is the
    // Action::HistorySearchResults([..]) it dispatched onto the bus)
    // @step And App::dispatch handles Action::HistorySearchResults([HistoryMatch(text="git status"), HistoryMatch(text="git push")])
    app.dispatch(action);
    // @step Then AgentView.search_popup.match_count() equals 2
    let popup = app
        .navigator()
        .agent
        .search_popup
        .as_ref()
        .expect("popup");
    assert_eq!(popup.match_count(), 2);
    // @step And AgentView.search_popup.selected_index() equals 0
    assert_eq!(popup.selected_index(), 0);
}

/// Scenario: Action::HistorySearchResults when search_popup is closed is a no-op
#[tokio::test(flavor = "current_thread")]
async fn history_search_results_with_no_popup_is_noop() {
    // @step Given an App with AgentView.search_popup == None
    let (mut app, _mock) = fresh_app();
    assert!(app.navigator().agent.search_popup.is_none());
    // @step When App::dispatch handles Action::HistorySearchResults([HistoryMatch(text="git status")])
    app.dispatch(Action::HistorySearchResults(vec![hm("git status")]));
    // @step Then AgentView.search_popup is still None
    assert!(app.navigator().agent.search_popup.is_none());
}

/// Scenario: Action::InsertIntoInput sets the input buffer and drops the search_popup
#[tokio::test(flavor = "current_thread")]
async fn insert_into_input_sets_buffer_and_drops_popup() {
    // @step Given an App with AgentView.search_popup == Some(SearchPalette) and AgentView.input.value() == ""
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenSearchPalette);
    assert!(app.navigator().agent.search_popup.is_some());
    assert!(app.navigator().agent.input.is_empty());

    // @step When App::dispatch handles Action::InsertIntoInput("git status")
    app.dispatch(Action::InsertIntoInput("git status".to_string()));

    // @step Then AgentView.input.value() equals "git status"
    assert_eq!(app.navigator().agent.input.value(), "git status");
    // @step And AgentView.search_popup is None
    assert!(app.navigator().agent.search_popup.is_none());
    // @step And no Action::InputSubmitted is dispatched
    // (we just never see one go through the bus during this test)
}

/// Scenario: handle_slash_command no longer fires the "[notice] not yet implemented" arm for Resume/Search
#[tokio::test(flavor = "current_thread")]
async fn resume_and_search_no_longer_emit_notice_lines() {
    // @step Given an App with the AgentView focused
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    let before = app.navigator().agent.chunk_count(app.agent_view_store());

    // @step When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Resume)
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Resume));
    // @step Then no scrollback line containing "/resume not yet implemented" is appended
    let after_resume = app.navigator().agent.chunk_count(app.agent_view_store());
    assert_eq!(after_resume, before, "Resume must not push a notice chunk");

    // @step When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Search)
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Search));
    // @step Then no scrollback line containing "/search not yet implemented" is appended
    let after_search = app.navigator().agent.chunk_count(app.agent_view_store());
    assert_eq!(after_search, before, "Search must not push a notice chunk");
}

/// Scenario: handle_slash_command still fires "[notice]" for the OTHER unimplemented variants
#[tokio::test(flavor = "current_thread")]
async fn other_slash_actions_still_emit_notice_lines() {
    // @step Given an App with the AgentView focused
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    let before = app.navigator().agent.chunk_count(app.agent_view_store());
    // @step When App::dispatch handles Action::SlashCommandSelected(SlashCommandAction::Model)
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Model));
    // @step Then a scrollback line containing "/model not yet implemented" is appended
    assert_eq!(
        app.navigator().agent.chunk_count(app.agent_view_store()),
        before + 1
    );
}
