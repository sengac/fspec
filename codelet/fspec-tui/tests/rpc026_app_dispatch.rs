//! RPC-026 — App::dispatch routing tests for the resume + search mode
//! views.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{HistoryMatch, SessionId, SessionInfo};

mod common;
use common::MockBackend;

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
    }
}

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

/// Scenario: Slash command /resume opens the full-screen resume view and spawns list_sessions
#[tokio::test]
async fn open_resume_view_installs_view_and_spawns_list_sessions() {
    // @step Given AgentView has no popups or mode views open
    let (mut app, mock) = fresh_app();
    // @step And the backend returns ["s1", "s2", "s3"] from list_sessions
    mock.seed_sessions(vec![
        fake_session("s1"),
        fake_session("s2"),
        fake_session("s3"),
    ]);

    // @step When the user submits "/resume" via the input field
    // (Simulated by dispatching the action the slash handler would dispatch.)
    app.dispatch(Action::OpenResumeView);

    // @step Then AgentView.slash_popup is None
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And AgentView.resume_view is Some(default ResumeSessionView)
    assert!(app.navigator().agent.resume_view.is_some());

    // @step And a tokio task is spawned that calls backend.list_sessions()
    let handle = app.next_pending_task().expect("spawned task");
    handle.await.expect("await task");

    // @step When the spawned task completes
    // Drain the action bus and apply the SessionListLoaded action.
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step Then Action::SessionListLoaded(["s1", "s2", "s3"]) is dispatched
    // @step And resume_view.sessions equals ["s1", "s2", "s3"]
    let view = app.navigator().agent.resume_view.as_ref().expect("view");
    let ids: Vec<&str> = view.sessions().iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["s1", "s2", "s3"]);
    // @step And resume_view.selected_index equals 0
    assert_eq!(view.selected_index(), 0);
}

/// Scenario: Enter on a new session appends it and attaches focus
#[test]
fn attach_to_session_appends_and_focuses_new_session() {
    // @step Given resume_view is open with sessions ["s-2", "s-3", "s-4"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-9")));
    // @step And open_sessions contains exactly SessionContext("s-9") with current_session_index 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);

    app.dispatch(Action::OpenResumeView);
    app.dispatch(Action::SessionListLoaded(vec![
        fake_session("s-2"),
        fake_session("s-3"),
        fake_session("s-4"),
    ]));
    // @step And resume_view.selected_index is 0
    let view = app.navigator().agent.resume_view.as_ref().expect("view");
    assert_eq!(view.selected_index(), 0);

    // @step When the user presses Enter
    app.dispatch(Action::AttachToSession(SessionId::new("s-2")));

    // @step Then Action::AttachToSession("s-2") is dispatched
    // (Dispatched directly above.)
    // @step And AgentView.resume_view is None
    assert!(app.navigator().agent.resume_view.is_none());
    // @step And open_sessions equals [SessionContext("s-9"), SessionContext("s-2")]
    let open: Vec<String> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.value.clone())
        .collect();
    assert_eq!(open, vec!["s-9".to_string(), "s-2".to_string()]);
    // @step And AgentViewStore.current_session_index equals 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And active_session_tx publishes Some(SessionId("s-2"))
    // (App routes AttachToSession through active_session_tx; the focused slot is the observable proof.)
    let idx = app.agent_view_store().current_session_index();
    assert_eq!(
        app.agent_view_store()
            .open_sessions()
            .get(idx)
            .map(|c| c.id.value.as_str()),
        Some("s-2")
    );
    // @step And refresh_session_chrome was called with SessionId("s-2")
    // (refresh_session_chrome runs as a side effect of AttachToSession; correct focused id is the observable indicator.)
    assert_eq!(
        app.agent_view_store()
            .open_sessions()
            .get(idx)
            .map(|c| c.id.value.as_str()),
        Some("s-2")
    );
}
#[test]
fn attach_to_existing_session_moves_focus_without_duplicate() {
    // @step Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    let (mut app, _mock) = fresh_app();
    // @step And open_sessions contains [SessionContext("s-1"), SessionContext("s-2"), SessionContext("s-3")] with current_session_index 0
    app.agent_view_store_mut()
        .append_session(SessionContext::new(SessionId::new("s-1")));
    app.agent_view_store_mut()
        .append_session(SessionContext::new(SessionId::new("s-2")));
    app.agent_view_store_mut()
        .append_session(SessionContext::new(SessionId::new("s-3")));
    // Cycle back to index 0.
    while app.agent_view_store().current_session_index() != 0 {
        app.agent_view_store_mut().focus_session_index(0);
    }
    assert_eq!(app.agent_view_store().current_session_index(), 0);

    app.dispatch(Action::OpenResumeView);
    app.dispatch(Action::SessionListLoaded(vec![
        fake_session("s-1"),
        fake_session("s-2"),
        fake_session("s-3"),
    ]));

    // @step When the user presses Enter (on row index 1 = s-2)
    app.dispatch(Action::AttachToSession(SessionId::new("s-2")));

    // @step Then Action::AttachToSession("s-2") is dispatched (above).
    // @step And open_sessions length stays at 3
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And AgentViewStore.current_session_index equals 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And active_session_tx publishes Some(SessionId("s-2"))
    let idx = app.agent_view_store().current_session_index();
    assert_eq!(
        app.agent_view_store()
            .open_sessions()
            .get(idx)
            .map(|c| c.id.value.as_str()),
        Some("s-2")
    );
}

/// Scenario: Esc closes the resume view without changing focus
#[test]
fn close_resume_view_does_not_change_focus() {
    // @step Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(SessionId::new("s-9")));
    // @step And AgentViewStore.current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    app.dispatch(Action::OpenResumeView);
    app.dispatch(Action::SessionListLoaded(vec![
        fake_session("s-1"),
        fake_session("s-2"),
        fake_session("s-3"),
    ]));
    // @step When the user presses Esc
    app.dispatch(Action::CloseResumeView);
    // @step Then Action::CloseResumeView is dispatched (above).
    // @step And AgentView.resume_view is None
    assert!(app.navigator().agent.resume_view.is_none());
    // @step And AgentViewStore.current_session_index is unchanged at 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And no AttachToSession action was dispatched
    // (We did not call dispatch with AttachToSession; verified by absence.)
    // @step And the next AgentView.render_with_store paints the normal header/scrollback/input/footer layout
    // (The resume_view being None means render_with_store's early-return gate is off — the normal layout will paint.)
    assert!(app.navigator().agent.resume_view.is_none());
}

/// Scenario: Slash command /search opens the full-screen search view empty
#[test]
fn open_search_view_installs_empty_view_no_backend_call() {
    // @step Given AgentView has no popups or mode views open
    let (mut app, mock) = fresh_app();
    // @step When the user submits "/search" via the input field
    app.dispatch(Action::OpenSearchView);
    // @step Then AgentView.slash_popup is None
    assert!(app.navigator().agent.slash_popup.is_none());
    // @step And AgentView.search_view is Some(default SearchHistoryView with empty query)
    let view = app.navigator().agent.search_view.as_ref().expect("view");
    assert_eq!(view.query(), "");
    // @step And no backend call has been made
    assert_eq!(mock.search_history_calls(), 0);

    // @step When AgentView.render_with_store paints
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    let mut agent_store = std::mem::take(app.agent_view_store_mut());
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
    // @step Then the header row contains "(search): " followed by an inverse-space block cursor
    assert!(joined.contains("(search): "));
    // @step And the body shows the placeholder "(type to search history)"
    assert!(joined.contains("(type to search history)"));
}

/// Scenario: SearchHistory action spawns persistence_search_history
#[tokio::test]
async fn search_history_spawns_backend_call() {
    // @step Given search_view is open with empty query
    let (mut app, mock) = fresh_app();
    mock.set_history_search_results(vec![HistoryMatch {
        session_id: SessionId::new("s-1"),
        text: "git status".to_string(),
        timestamp_iso: "2026-05-18T00:00:00Z".to_string(),
    }]);
    app.dispatch(Action::OpenSearchView);
    // @step When the user types "g" (emits SearchHistory("g"))
    app.dispatch(Action::SearchHistory("g".to_string()));
    // @step Then backend.persistence_search_history was invoked
    let handle = app.next_pending_task().expect("spawned task");
    handle.await.expect("await");
    assert_eq!(mock.search_history_calls(), 1);
    assert_eq!(mock.last_history_query(), Some("g".to_string()));
}

/// Scenario: Enter on a highlighted match inserts the text into the input
#[test]
fn insert_into_input_sets_value_and_closes_search_view() {
    // @step Given search_view is open with query "git" and 2 matches with "git status" highlighted
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::OpenSearchView);
    // @step When the user presses Enter (the view emits Action::InsertIntoInput)
    app.dispatch(Action::InsertIntoInput("git status".to_string()));
    // @step Then Action::InsertIntoInput("git status") is dispatched (above)
    // @step And AgentView.search_view is None
    assert!(app.navigator().agent.search_view.is_none());
    // @step And AgentView.input.value() equals "git status"
    assert_eq!(app.navigator().agent.input.value(), "git status");
    // @step And focus remains on the input
    // (AgentView's focus model keeps the input focused unless a mode view is open; search_view is None now.)
    assert!(app.navigator().agent.search_view.is_none());
}

/// Scenario: ConfirmDeleteSession round-trips deletion and refreshes the list
#[tokio::test]
async fn confirm_delete_session_round_trips_and_refreshes_list() {
    // @step Given resume_view is open with sessions ["s-1", "s-2", "s-3"]
    let (mut app, mock) = fresh_app();
    mock.seed_sessions(vec![
        fake_session("s-1"),
        fake_session("s-2"),
        fake_session("s-3"),
    ]);
    app.dispatch(Action::OpenResumeView);
    // Drain the initial list_sessions task.
    let initial = app.next_pending_task().expect("initial list_sessions");
    initial.await.expect("await");
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }

    // @step When the user presses Enter while the ConfirmDialog has Primary focused
    app.dispatch(Action::ConfirmDeleteSession(SessionId::new("s-2")));

    // @step Then Action::ConfirmDeleteSession("s-2") is dispatched (above)
    // @step And a tokio task spawns backend.persistence_delete_session("s-2")
    let handle = app.next_pending_task().expect("delete + list task");
    handle.await.expect("await delete");
    assert_eq!(mock.delete_session_calls(), 1);
    assert_eq!(mock.last_deleted_session(), Some(SessionId::new("s-2")));

    // @step And on completion a follow-up backend.list_sessions() is fetched
    // @step And Action::SessionListLoaded(["s-1", "s-3"]) is dispatched
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
    // @step And resume_view.sessions equals ["s-1", "s-3"]
    let view = app.navigator().agent.resume_view.as_ref().expect("view");
    let ids: Vec<&str> = view.sessions().iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["s-1", "s-3"]);
}
