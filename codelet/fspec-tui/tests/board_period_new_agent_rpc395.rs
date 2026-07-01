//! RPC-395 — Board '.' (period) key starts a new agent.
//!
//! Feature: spec/features/board-key-starts-new-agent.feature
//!
//! Drives `BoardView::handle_event` with a modifier-free `.` key press and
//! asserts it emits `Action::OpenAgentView(...)` mirroring the Shift+Right
//! handler, plus verifies the header hint row now reads ". New Agent".

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, Theme};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn fresh() -> (BoardView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn make_unit(id: &str, status: &str, work_type: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: work_type.to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn render(width: u16, height: u16, store: &BoardStore) -> Buffer {
    let (view, _rx) = fresh();
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

fn join_buffer(buf: &Buffer) -> String {
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

/// Scenario: Pressing '.' with a selected work unit opens the AgentView for its session
#[test]
fn pressing_period_with_a_selected_work_unit_opens_the_agent_view_for_its_session() {
    // @step Given a BoardStore containing AUTH-001 in backlog with the focused column "backlog" and selected index 0
    let mut store = BoardStore::default();
    store.replace_work_units(vec![make_unit("AUTH-001", "backlog", "story")]);
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    let (view, mut rx) = fresh();
    // @step When the user presses the key '.'
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::empty())),
        &store,
    );
    // @step Then BoardView emits an Action::OpenAgentView for the selected work unit's session
    let mut actions: Vec<Action> = Vec::new();
    while let Ok(a) = rx.try_recv() {
        actions.push(a);
    }
    assert_eq!(
        actions.len(),
        1,
        "expected exactly one action, got {actions:?}"
    );
    assert!(
        matches!(actions[0], Action::OpenAgentView(_)),
        "expected Action::OpenAgentView, got {:?}",
        actions[0]
    );
}

/// Scenario: Pressing '.' with no work unit selected still opens the AgentView with no attached session
#[test]
fn pressing_period_with_no_work_unit_selected_still_opens_the_agent_view_with_no_attached_session()
{
    // @step Given an empty BoardStore with no work units
    let store = BoardStore::default();
    let (view, mut rx) = fresh();
    // @step When the user presses the key '.'
    let _ = view.handle_event(
        &Event::Key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::empty())),
        &store,
    );
    // @step Then BoardView emits an Action::OpenAgentView with no attached session
    let mut actions: Vec<Action> = Vec::new();
    while let Ok(a) = rx.try_recv() {
        actions.push(a);
    }
    assert_eq!(
        actions.len(),
        1,
        "expected exactly one action, got {actions:?}"
    );
    assert!(
        matches!(actions[0], Action::OpenAgentView(None)),
        "expected Action::OpenAgentView(None), got {:?}",
        actions[0]
    );
}

/// Scenario: The board header hint row displays '. New Agent'
#[test]
fn the_board_header_hint_row_displays_period_new_agent() {
    // @step Given a BoardStore with any selection state
    let store = BoardStore::default();
    // @step When the App renders BoardView against a 120x24 TestBackend
    let buf = render(120, 24, &store);
    let joined = join_buffer(&buf);
    // @step Then the rendered buffer contains the substring ". New Agent"
    assert!(
        joined.contains(". New Agent"),
        "missing '. New Agent':\n{joined}"
    );
    // @step And the rendered buffer does not contain the substring "/ New Agent"
    assert!(
        !joined.contains("/ New Agent"),
        "unexpected '/ New Agent' still present:\n{joined}"
    );
}
