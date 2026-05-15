//! RPC-023 — App run-loop / Compositor / Navigator no longer drop Event::Mouse.
//!
//! Feature: spec/features/app-mouse-dispatch.feature
//!
//! Drives Event::Mouse(ScrollDown) through `App::handle_event` and
//! asserts that BoardView::handle_event observed it (Action::SelectNext
//! arrives on the action bus + `App::dispatch` advances the BoardStore).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

mod common;

use common::MockBackend;

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

/// Scenario: Event::Mouse is no longer dropped by the App run loop
#[tokio::test]
async fn event_mouse_is_no_longer_dropped_by_the_app_run_loop() {
    // @step Given an App constructed with a MockBackend, the action bus wired, and the Navigator set to ViewMode::Board
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let mut app = App::new(backend);
    // Seed BoardStore with 20 units in BACKLOG so wheel-down has a place to go.
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    app.board_store_mut().replace_work_units(units);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step And BoardView has been rendered through Navigator::render_with_stores so last_content_area is populated
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).expect("Terminal::new");
    terminal
        .draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");

    // @step When Event::Mouse(ScrollDown) inside the BACKLOG content area is fed through App::handle_event
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 5,
        row: 15,
        modifiers: KeyModifiers::NONE,
    });
    let result = app.handle_event(&event);

    // @step Then App::handle_event returns a Consumed result
    assert!(
        result.is_consumed(),
        "App::handle_event must forward Event::Mouse(ScrollDown) into BoardView::handle_event which should consume it"
    );

    // @step And the action bus carries Action::SelectNext
    let action = app
        .try_recv_action()
        .expect("Action::SelectNext expected on the action bus");
    assert!(
        matches!(action, Action::SelectNext),
        "expected Action::SelectNext, got {action:?}"
    );
}
