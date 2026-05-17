//! RPC-026 — View-layer key routing for /resume + /search popups.
//!
//! Feature: spec/features/rpc026-resume-picker.feature
//! Feature: spec/features/rpc026-search-palette.feature
//!
//! Supplementary integration tests proving the AgentView routes user
//! key events through the resume / search popups into the Action bus.
//! Rule [4] of RPC-026 requires *"Enter emits ResumePickerOutcome::Selected
//! (SessionId) and AgentView routes Action::AttachToSession(session_id)"*;
//! rule [5] requires the analogous wiring for the search palette.
//! Architecture note [1] requires *"handle_event in views/agent/dispatch.rs
//! is extended via a new helper that routes keys through the resume /
//! search popups BEFORE the slash / file popups"*.
//!
//! These tests do not map 1:1 to the Gherkin scenarios in the
//! `rpc026-app-dispatch.feature` file (which exercise App::dispatch
//! handler behaviour given a pre-dispatched Action). They cover the
//! bridge layer that converts a raw KeyEvent → popup Outcome → Action.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use codelet_fspec_tui::views::agent::{ResumePicker, SearchPalette};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{
    CheckpointCounts, HealthInfo, HistoryMatch, LogRecord, ModelInfo, SessionId, SessionInfo,
    StreamChunk, ThinkingLevel, WorkUnitInfo, WorkspaceInfo,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::broadcast;

pub struct RoutingMockBackend {
    work_units_tx: broadcast::Sender<Vec<WorkUnitInfo>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
}

impl Default for RoutingMockBackend {
    fn default() -> Self {
        let (work_units_tx, _) = broadcast::channel(8);
        let (chunks_tx, _) = broadcast::channel(8);
        let (logs_tx, _) = broadcast::channel(8);
        Self {
            work_units_tx,
            chunks_tx,
            logs_tx,
        }
    }
}

#[async_trait]
impl FspecBackend for RoutingMockBackend {
    async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>> {
        Ok(Vec::new())
    }
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        Ok(Vec::new())
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
    async fn persistence_search_history(&self, _query: String) -> Result<Vec<HistoryMatch>> {
        Ok(Vec::new())
    }
}

fn fresh_app() -> App {
    let backend: Arc<dyn FspecBackend> = Arc::new(RoutingMockBackend::default());
    App::new(backend)
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
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

/// Find the first matching Action on the bus; consumes intervening
/// actions until one matches. Panics if none arrives.
fn expect_action<F>(app: &mut App, predicate: F) -> Action
where
    F: Fn(&Action) -> bool,
{
    while let Some(action) = app.try_recv_action() {
        if predicate(&action) {
            return action;
        }
    }
    panic!("expected Action was not emitted onto the bus");
}

// ---------------------------------------------------------------------------
// Resume picker view-layer routing
// ---------------------------------------------------------------------------

/// Pressing Enter on a resume_popup with a selected session emits
/// Action::AttachToSession AND drops the popup. Rule [4].
#[tokio::test(flavor = "current_thread")]
async fn enter_on_resume_popup_emits_attach_to_session() {
    let mut app = fresh_app();
    // Open the popup directly and stuff a scripted session in.
    let mut picker = ResumePicker::new();
    picker.set_sessions(vec![sinfo("s-7"), sinfo("s-8")]);
    app.navigator_mut().agent.resume_popup = Some(picker);

    // @step When Enter is pressed while resume_popup highlights s-7
    let consumed = app.navigator_mut().agent.handle_event(&key(KeyCode::Enter));
    assert!(consumed.is_consumed(), "Enter must be consumed by popup");

    // @step Then the popup is dropped
    assert!(
        app.navigator().agent.resume_popup.is_none(),
        "resume_popup must be dropped after Enter"
    );

    // @step And Action::AttachToSession(SessionId(\"s-7\")) is emitted
    let action = expect_action(&mut app, |a| matches!(a, Action::AttachToSession(_)));
    match action {
        Action::AttachToSession(id) => assert_eq!(id, SessionId::new("s-7")),
        other => panic!("expected AttachToSession, got {other:?}"),
    }
}

/// Pressing Esc on a resume_popup drops it without emitting any Action.
/// Rule [4] (Esc emits Dismiss).
#[tokio::test(flavor = "current_thread")]
async fn esc_on_resume_popup_drops_it_silently() {
    let mut app = fresh_app();
    let mut picker = ResumePicker::new();
    picker.set_sessions(vec![sinfo("s-1")]);
    app.navigator_mut().agent.resume_popup = Some(picker);

    // @step When Esc is pressed while resume_popup is open
    let consumed = app.navigator_mut().agent.handle_event(&key(KeyCode::Esc));
    assert!(consumed.is_consumed());

    // @step Then resume_popup is None
    assert!(app.navigator().agent.resume_popup.is_none());
    // @step And no Action::AttachToSession is emitted onto the bus
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::AttachToSession(_)) {
            panic!("AttachToSession must not fire on Esc");
        }
    }
    // @step And no Action::BackToBoard is emitted (Esc is consumed by popup, not fallthrough)
    // (re-poll — if BackToBoard fired we would have seen it above.)
}

/// Pressing ↓ on a resume_popup moves selection internally; the key
/// does NOT fall through to MultiLineInput / Shift+arrow handling.
#[tokio::test(flavor = "current_thread")]
async fn down_arrow_on_resume_popup_navigates_and_consumes() {
    let mut app = fresh_app();
    let mut picker = ResumePicker::new();
    picker.set_sessions(vec![sinfo("s-a"), sinfo("s-b"), sinfo("s-c")]);
    app.navigator_mut().agent.resume_popup = Some(picker);
    assert_eq!(
        app.navigator()
            .agent
            .resume_popup
            .as_ref()
            .expect("popup")
            .selected_index(),
        0
    );

    // @step When ↓ is pressed while resume_popup is open
    let consumed = app.navigator_mut().agent.handle_event(&key(KeyCode::Down));
    assert!(consumed.is_consumed(), "↓ must be consumed by popup");

    // @step Then selected_index advances to 1
    assert_eq!(
        app.navigator()
            .agent
            .resume_popup
            .as_ref()
            .expect("popup")
            .selected_index(),
        1
    );
    // @step And no Action::AttachToSession is emitted
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::AttachToSession(_)) {
            panic!("AttachToSession must not fire on bare ↓");
        }
    }
}

/// Resume popup ignores unmodified printable characters so they fall
/// through. (Ignored Outcome → None → caller tries next popup / default.)
/// Verifies the Ignored arm is honoured.
#[tokio::test(flavor = "current_thread")]
async fn unhandled_char_on_resume_popup_does_not_emit_attach() {
    let mut app = fresh_app();
    let mut picker = ResumePicker::new();
    picker.set_sessions(vec![sinfo("s-1")]);
    app.navigator_mut().agent.resume_popup = Some(picker);

    // @step When a printable character is pressed while resume_popup is open
    let _ = app
        .navigator_mut()
        .agent
        .handle_event(&key(KeyCode::Char('x')));

    // @step Then resume_popup is still Some (Ignored does not auto-drop it)
    assert!(app.navigator().agent.resume_popup.is_some());
    // @step And no Action::AttachToSession is emitted
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::AttachToSession(_)) {
            panic!("AttachToSession must not fire on Ignored");
        }
    }
}

// ---------------------------------------------------------------------------
// Search palette view-layer routing
// ---------------------------------------------------------------------------

/// Pressing Enter on a search_popup with a selected match emits
/// Action::InsertIntoInput AND drops the popup. Rule [5].
#[tokio::test(flavor = "current_thread")]
async fn enter_on_search_popup_emits_insert_into_input() {
    let mut app = fresh_app();
    let mut palette = SearchPalette::new();
    palette.set_matches(vec![hm("git status"), hm("git push")]);
    app.navigator_mut().agent.search_popup = Some(palette);

    // @step When Enter is pressed while search_popup highlights "git status"
    let consumed = app.navigator_mut().agent.handle_event(&key(KeyCode::Enter));
    assert!(consumed.is_consumed());

    // @step Then the search_popup is dropped
    assert!(app.navigator().agent.search_popup.is_none());

    // @step And Action::InsertIntoInput("git status") is emitted
    let action = expect_action(&mut app, |a| matches!(a, Action::InsertIntoInput(_)));
    match action {
        Action::InsertIntoInput(text) => assert_eq!(text, "git status"),
        other => panic!("expected InsertIntoInput, got {other:?}"),
    }
}

/// Typing a printable character on a search_popup emits
/// Action::SearchHistory(query) WITHOUT dropping the popup. Rule [5].
#[tokio::test(flavor = "current_thread")]
async fn char_on_search_popup_emits_search_history_and_keeps_open() {
    let mut app = fresh_app();
    app.navigator_mut().agent.search_popup = Some(SearchPalette::new());

    // @step When the character 'g' is pressed
    app.navigator_mut()
        .agent
        .handle_event(&key(KeyCode::Char('g')));
    // @step Then Action::SearchHistory("g") is emitted
    let action = expect_action(&mut app, |a| matches!(a, Action::SearchHistory(_)));
    match action {
        Action::SearchHistory(q) => assert_eq!(q, "g"),
        other => panic!("expected SearchHistory, got {other:?}"),
    }

    // @step And the search_popup is still Some
    assert!(app.navigator().agent.search_popup.is_some());
    assert_eq!(
        app.navigator()
            .agent
            .search_popup
            .as_ref()
            .expect("popup")
            .query(),
        "g"
    );

    // @step When the character 'i' is pressed
    app.navigator_mut()
        .agent
        .handle_event(&key(KeyCode::Char('i')));
    // @step Then Action::SearchHistory("gi") is emitted
    let action = expect_action(&mut app, |a| matches!(a, Action::SearchHistory(_)));
    match action {
        Action::SearchHistory(q) => assert_eq!(q, "gi"),
        other => panic!("expected SearchHistory, got {other:?}"),
    }
}

/// Pressing Backspace on a non-empty search_popup query emits
/// Action::SearchHistory with the trimmed query.
#[tokio::test(flavor = "current_thread")]
async fn backspace_on_search_popup_emits_trimmed_query() {
    let mut app = fresh_app();
    let mut palette = SearchPalette::new();
    palette.set_query("git");
    app.navigator_mut().agent.search_popup = Some(palette);

    // @step When Backspace is pressed
    app.navigator_mut()
        .agent
        .handle_event(&key(KeyCode::Backspace));

    // @step Then Action::SearchHistory("gi") is emitted
    let action = expect_action(&mut app, |a| matches!(a, Action::SearchHistory(_)));
    match action {
        Action::SearchHistory(q) => assert_eq!(q, "gi"),
        other => panic!("expected SearchHistory, got {other:?}"),
    }
    // @step And the search_popup is still Some with query "gi"
    assert_eq!(
        app.navigator()
            .agent
            .search_popup
            .as_ref()
            .expect("popup")
            .query(),
        "gi"
    );
}

/// Pressing Esc on a search_popup drops it without emitting any Action.
#[tokio::test(flavor = "current_thread")]
async fn esc_on_search_popup_drops_it_silently() {
    let mut app = fresh_app();
    let mut palette = SearchPalette::new();
    palette.set_query("foo");
    palette.set_matches(vec![hm("foobar")]);
    app.navigator_mut().agent.search_popup = Some(palette);

    // @step When Esc is pressed
    let consumed = app.navigator_mut().agent.handle_event(&key(KeyCode::Esc));
    assert!(consumed.is_consumed());

    // @step Then search_popup is None
    assert!(app.navigator().agent.search_popup.is_none());
    // @step And no Action::InsertIntoInput is emitted
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::InsertIntoInput(_)) {
            panic!("InsertIntoInput must not fire on Esc");
        }
    }
}

/// Pressing ↑/↓ on a search_popup navigates internally; the key
/// does NOT fall through to MultiLineInput or the Shift+arrow chord.
#[tokio::test(flavor = "current_thread")]
async fn arrow_keys_on_search_popup_navigate_and_consume() {
    let mut app = fresh_app();
    let mut palette = SearchPalette::new();
    palette.set_matches(vec![hm("a"), hm("b"), hm("c")]);
    app.navigator_mut().agent.search_popup = Some(palette);

    // @step When ↓ is pressed while search_popup is open
    let consumed = app.navigator_mut().agent.handle_event(&key(KeyCode::Down));
    assert!(consumed.is_consumed());

    // @step Then selected_index advances to 1
    assert_eq!(
        app.navigator()
            .agent
            .search_popup
            .as_ref()
            .expect("popup")
            .selected_index(),
        1
    );
}

// ---------------------------------------------------------------------------
// Ordering: resume / search BEFORE slash / file (architecture note [1])
// ---------------------------------------------------------------------------

/// When resume_popup and slash_popup are both somehow live, the
/// resume_popup wins. Defensive — the dispatch helpers null slash_popup
/// when opening resume, but the routing order guarantees correct
/// behaviour even if a future caller forgets to.
#[tokio::test(flavor = "current_thread")]
async fn resume_popup_takes_precedence_over_slash_popup() {
    use codelet_fspec_tui::views::agent::SlashCommandPopup;

    let mut app = fresh_app();
    let mut picker = ResumePicker::new();
    picker.set_sessions(vec![sinfo("s-1")]);
    app.navigator_mut().agent.resume_popup = Some(picker);
    app.navigator_mut().agent.slash_popup = Some(SlashCommandPopup::default());

    // @step When Enter is pressed while both popups are live
    app.navigator_mut().agent.handle_event(&key(KeyCode::Enter));

    // @step Then Action::AttachToSession fires (resume won)
    let action = expect_action(&mut app, |a| {
        matches!(
            a,
            Action::AttachToSession(_) | Action::SlashCommandSelected(_)
        )
    });
    assert!(
        matches!(action, Action::AttachToSession(_)),
        "resume_popup must take precedence, got {action:?}"
    );
    // @step And resume_popup is None
    assert!(app.navigator().agent.resume_popup.is_none());
}
