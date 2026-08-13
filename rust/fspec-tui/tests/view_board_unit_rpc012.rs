//! RPC-012 — Inline-equivalent unit tests for BoardView extracted to a
//! separate integration file so `views/board.rs` stays < 300 LoC per
//! the file-size invariant.
//!
//! Feature: spec/features/rpc012-board-agent-navigation.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, EventResult, Theme};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

#[test]
fn enter_emits_enter_work_unit_for_selected_unit() {
    let (view, mut rx) = fresh();
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("AUTH-002", "implementing")]);
    store.set_focused_column("implementing");
    store.set_selected_index_for("implementing", 0);

    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = view.handle_event(&event, &store);
    assert!(matches!(result, EventResult::Consumed(None)));
    let action = rx.try_recv().expect("Action::EnterWorkUnit on bus");
    match action {
        Action::EnterWorkUnit(id) => assert_eq!(id, "AUTH-002"),
        other => panic!("expected EnterWorkUnit, got {other:?}"),
    }
}

#[test]
fn shift_right_with_attached_session_emits_open_agent_view_some() {
    let (view, mut rx) = fresh();
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    store.attach_session("AUTH-001", SessionId::new("s-1"));

    let event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    let _ = view.handle_event(&event, &store);
    let action = rx.try_recv().expect("Action::OpenAgentView on bus");
    match action {
        Action::OpenAgentView(Some(id)) => assert_eq!(id, SessionId::new("s-1")),
        other => panic!("expected OpenAgentView(Some(s-1)), got {other:?}"),
    }
}

#[test]
fn shift_right_with_no_attached_session_emits_open_agent_view_none() {
    let (view, mut rx) = fresh();
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("AUTH-001", "backlog")]);
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);

    let event = Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT));
    let _ = view.handle_event(&event, &store);
    let action = rx.try_recv().expect("Action::OpenAgentView on bus");
    match action {
        Action::OpenAgentView(None) => {}
        other => panic!("expected OpenAgentView(None), got {other:?}"),
    }
}

#[test]
fn render_with_store_paints_seven_column_headers() {
    let (view, _rx) = fresh();
    let mut store = BoardStore::default();
    store.replace_work_units(vec![wu("AUTH-001", "backlog")]);

    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();

    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    for header in [
        "BACKLOG",
        "SPECIFYING",
        "TESTING",
        "IMPLEMENTING",
        "VALIDATING",
        "DONE",
        "BLOCKED",
    ] {
        assert!(
            joined.contains(header),
            "expected header {header} in:\n{joined}"
        );
    }
    assert!(joined.contains("AUTH-001"));
}
