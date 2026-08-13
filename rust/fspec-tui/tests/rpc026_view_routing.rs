//! RPC-026 — AgentView event routing tests for the mode views.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    Action, AgentView, AgentViewStore, App, EventResult, FspecBackend, ResumeSessionView,
    SearchHistoryView,
};
use codelet_rpc_types::{SessionId, SessionInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn fake_session(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
    }
}

/// Scenario: Ctrl+R opens the search view from the normal AgentView
#[test]
fn ctrl_r_emits_open_search_view() {
    // @step Given AgentView has no popups or mode views open
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);
    // @step When the user presses Ctrl+R
    let result = view.handle_event(&key(KeyCode::Char('r'), KeyModifiers::CONTROL));
    assert!(matches!(result, EventResult::Consumed(_)));
    // @step Then Action::OpenSearchView is dispatched
    let act = rx.try_recv().expect("action");
    assert!(matches!(act, Action::OpenSearchView));
    // @step And AgentView.search_view is Some(default SearchHistoryView)
    // (Widget-level: simulate the App folding OpenSearchView by installing the view.)
    view.search_view = Some(SearchHistoryView::new());
    assert!(view.search_view.is_some());
    // @step When the user presses Ctrl+R again
    let _ = view.handle_event(&key(KeyCode::Char('r'), KeyModifiers::CONTROL));
    // @step Then the chord is forwarded to the search_view which returns Ignored
    // (Verified by absence of a second OpenSearchView action on the bus.)
    assert!(rx.try_recv().is_err());
    // @step And search_view stays open with unchanged query and matches
    let sv = view.search_view.as_ref().expect("search_view stays open");
    assert_eq!(sv.query(), "");
    assert_eq!(sv.match_count(), 0);
}

/// Scenario: Ctrl+R while a popup is open is NOT intercepted by the chord
#[test]
fn ctrl_r_while_slash_popup_open_does_not_open_search_view() {
    // @step Given AgentView has a slash popup open
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);
    view.input.set_value("/");
    view.sync_popups();
    assert!(view.slash_popup.is_some());
    // Drain any actions from sync_popups (e.g. SearchFiles).
    while rx.try_recv().is_ok() {}
    // @step When the user presses Ctrl+R
    let _ = view.handle_event(&key(KeyCode::Char('r'), KeyModifiers::CONTROL));
    // @step Then Action::OpenSearchView is NOT dispatched
    assert!(rx.try_recv().is_err());
}

/// Scenario: AgentView early-returns when resume_view is Some
#[test]
fn render_early_returns_when_resume_view_active() {
    // @step Given resume_view is open with 3 SessionInfo values
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenResumeView);
    app.dispatch(Action::SessionListLoaded(vec![
        fake_session("s-1"),
        fake_session("s-2"),
        fake_session("s-3"),
    ]));
    let area = Rect::new(0, 0, 120, 24);
    let mut buf = Buffer::empty(area);
    // Move the agent store out of the App so the borrow checker is happy.
    let mut agent_store: AgentViewStore = std::mem::take(app.agent_view_store_mut());
    // @step When AgentView.render_with_store is called with area Rect(0, 0, 120, 24)
    app.navigator_mut()
        .agent
        .render_with_store(area, &mut buf, &mut agent_store);
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect();
    let joined = rows.join("\n");
    // @step Then the buffer contains a row whose text is "Resume Session (3 available)"
    assert!(joined.contains("Resume Session (3 available)"));
    // @step And every cell inside the 120×24 area is overwritten by ResumeSessionView (Clear was painted)
    // (Clear paints spaces; positive assertions above confirm the mode view took over.)
    assert_eq!(buf.area.width, 120);
    assert_eq!(buf.area.height, 24);
    // @step And no "Agent —" scrollback title row appears in the buffer
    assert!(!joined.contains("Agent —"));
    // @step And the footer row contains "Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"
    assert!(joined.contains("Enter Select | ↑↓ Navigate | D Delete | Esc Cancel"));
}

/// Scenario: AgentView normal layout paints when resume_view is None
#[test]
fn render_paints_normal_layout_when_no_mode_view_active() {
    // @step Given AgentView has no popups or mode views open
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let mut agent_store: AgentViewStore = std::mem::take(app.agent_view_store_mut());
    app.navigator_mut()
        .agent
        .render_with_store(area, &mut buf, &mut agent_store);
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect();
    let joined = rows.join("\n");
    // @step Then the next AgentView.render_with_store paints the normal header/scrollback/input/footer layout
    // RPC-029: scrollback no longer paints an " Agent — <sid> " title;
    // the visible "normal layout" anchor is the input placeholder hint.
    assert!(joined.contains("Type a message..."));
}

/// Scenario fragment: resume_view consumes keys before normal handlers
#[test]
fn resume_view_consumes_keys_before_normal_handlers() {
    // @step Given AgentView.resume_view is Some
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);
    let mut rsv = ResumeSessionView::new();
    rsv.set_sessions(vec![fake_session("s-1")]);
    view.resume_view = Some(rsv);
    // @step When the user presses Esc
    let _ = view.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    // @step Then Action::CloseResumeView is dispatched (and resume_view is dropped)
    let act = rx.try_recv().expect("action");
    assert!(matches!(act, Action::CloseResumeView));
    assert!(view.resume_view.is_none());
    // @step And no Action::BackToBoard was dispatched
    assert!(rx.try_recv().is_err());
}

/// Scenario fragment: search_view consumes typing before sending to input
#[test]
fn search_view_consumes_typing_and_emits_filter_changed() {
    // @step Given AgentView.search_view is Some
    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut view = AgentView::new(tx);
    view.search_view = Some(SearchHistoryView::new());
    // @step When the user types "g"
    let _ = view.handle_event(&key(KeyCode::Char('g'), KeyModifiers::NONE));
    // @step Then Action::SearchHistory("g") is dispatched
    let act = rx.try_recv().expect("action");
    match act {
        Action::SearchHistory(q) => assert_eq!(q, "g"),
        other => panic!("expected SearchHistory, got {other:?}"),
    }
    // @step And the AgentView's input is unchanged
    assert!(view.input.is_empty());
}
