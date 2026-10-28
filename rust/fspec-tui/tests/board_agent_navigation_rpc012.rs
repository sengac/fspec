//! RPC-012 — App-level integration tests for the BoardStore +
//! AgentViewStore + Navigator base refactor.
//!
//! Feature: spec/features/rpc012-board-agent-navigation.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, ViewMode};
use codelet_rpc_types::{SessionId, StreamChunk, WorkUnitInfo};

use crate::common::MockBackend;

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

/// Scenario: AgentViewStore is empty after App::bootstrap with no pre-created session
#[tokio::test]
async fn agent_view_store_is_empty_after_bootstrap_navigator() {
    // @step Given an App constructed against a MockBackend with no scripted session
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "implementing"),
    ]);
    let mut app = App::new(mock.clone());
    // @step When the developer drives App::bootstrap to completion
    app.bootstrap().await.expect("bootstrap");
    // @step Then MockBackend.list_work_units_calls equals 1
    assert_eq!(mock.list_work_units_calls(), 1);
    // @step And MockBackend.create_session_calls equals 0
    assert_eq!(mock.create_session_calls(), 0);
    // @step And AgentViewStore.current_session returns None
    assert!(app.agent_view_store().current_session().is_none());
    // @step And AgentViewStore.show_create_session_dialog returns false
    assert!(!app.agent_view_store().show_create_session_dialog());
    // @step And AgentViewStore.current_work_unit_id returns None
    assert!(app.agent_view_store().current_work_unit_id().is_none());
}

#[path = "board_agent_navigation/creation.rs"]
mod creation;

/// Scenario: Shift+Right on an attached work unit sets the navigation target
#[tokio::test]
async fn open_agent_view_with_session_sets_navigation_target() {
    // @step Given an App with bootstrap complete and BoardStore seeded with [AUTH-001 backlog]
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut app = App::new(mock.clone());
    app.bootstrap().await.expect("bootstrap");
    // @step And BoardStore.attach_session("AUTH-001", SessionId::new("s-1")) has been called
    app.board_store_mut()
        .attach_session("AUTH-001", SessionId::new("s-1"));
    // @step When the App dispatches Action::OpenAgentView(Some(SessionId::new("s-1")))
    app.dispatch(Action::OpenAgentView(Some(SessionId::new("s-1"))));
    // @step Then AgentViewStore.navigation_target_session equals Some(SessionId::new("s-1"))
    assert_eq!(
        app.agent_view_store().navigation_target_session(),
        Some(&SessionId::new("s-1"))
    );
    // @step And the Navigator's active_view equals ViewMode::Agent
    assert_eq!(app.active_view(), ViewMode::Agent);
}

/// Scenario: ESC from AgentView returns to BoardView preserving focus and selection
#[tokio::test]
async fn back_to_board_preserves_focus_and_selection() {
    // @step Given an App with Navigator.active_view = ViewMode::Agent
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-002", "implementing")]);
    let mut app = App::new(mock.clone());
    app.bootstrap().await.expect("bootstrap");
    app.board_store_mut().set_focused_column("implementing");
    app.board_store_mut()
        .set_selected_index_for("implementing", 0);
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And BoardStore.focused_column() returns "implementing"
    assert_eq!(app.board_store().focused_column(), "implementing");
    // @step And BoardStore.selected_index_for("implementing") returns 0
    assert_eq!(app.board_store().selected_index_for("implementing"), 0);
    // @step When the App dispatches Action::BackToBoard
    app.dispatch(Action::BackToBoard);
    // @step Then the Navigator's active_view equals ViewMode::Board
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And BoardStore.focused_column() still returns "implementing"
    assert_eq!(app.board_store().focused_column(), "implementing");
    // @step And BoardStore.selected_index_for("implementing") still returns 0
    assert_eq!(app.board_store().selected_index_for("implementing"), 0);
}

/// Scenario: Navigator renders BoardView as the first landing view
#[tokio::test]
async fn navigator_renders_board_view_as_first_landing_view() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut app = App::new(mock.clone());
    // @step Given an App with bootstrap complete and Navigator.active_view defaulting to ViewMode::Board
    app.bootstrap().await.expect("bootstrap");
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step When the App renders against an 80x24 TestBackend
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    // Move the stores out of the App so we can lend a `&` to board and a `&mut`
    // to agent without violating Rust's borrow rules.
    let mut agent_snapshot = std::mem::take(app.agent_view_store_mut());
    let board_snapshot: codelet_fspec_tui::BoardStore = std::mem::take(app.board_store_mut());
    // Use Navigator render directly so we don't paint the old RootView on top.
    assert_eq!(board_snapshot.column_units("backlog").len(), 1);
    // Render via mutable navigator borrow + mutable agent store borrow.
    let mut nav = codelet_fspec_tui::Navigator::new(
        Arc::new(codelet_fspec_tui::Theme::default()),
        app.action_tx_clone(),
    );
    nav.render_with_stores(
        ratatui::layout::Rect::new(0, 0, 120, 24),
        term.current_buffer_mut(),
        &board_snapshot,
        &mut agent_snapshot,
    );
    let buf = term.current_buffer_mut().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    // @step Then the rendered buffer contains the seven column headers BACKLOG SPECIFYING TESTING IMPLEMENTING VALIDATING DONE BLOCKED
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
    // @step And the rendered buffer does NOT contain the AgentView "Agent" block title
    // (We assert the body line, not "Agent" as substring — "Agent" might appear
    // in other UI strings; here we check the unique " Agent " block-title pattern.)
    assert!(!joined.contains("Agent — "));
}

/// Scenario: Chunks subscriber forwards background and focused text through transcript routing
#[tokio::test]
async fn chunks_subscriber_filter_follows_current_session_via_watch_channel(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_fspec_tui::app::transcript_sync::TranscriptAction;

    let mock = Arc::new(MockBackend::new());
    mock.seed_work_units(vec![wu("AUTH-001", "backlog")]);
    let mut app = App::new(mock.clone());
    app.bootstrap().await?;
    // @step Given an App with bootstrap complete and AgentViewStore.current_session = Some(SessionId::new("s-1"))
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&SessionId::new("s-1"))
    );
    // @step When the App dispatches Action::SessionCreated(SessionId::new("s-2"))
    app.dispatch(Action::SessionCreated(SessionId::new("s-2")));
    // @step Then AgentViewStore.current_session equals Some(SessionId::new("s-2"))
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&SessionId::new("s-2"))
    );
    // @step And the App publishes Some(SessionId::new("s-2")) onto the chunks watch channel
    mock.push_chunk(
        SessionId::new("s-1"),
        StreamChunk::text("background".to_string()),
    );
    mock.push_chunk(
        SessionId::new("s-2"),
        StreamChunk::text("foreground".to_string()),
    );
    let mut saw_s2 = false;
    let mut saw_s1 = false;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !saw_s1 || !saw_s2 {
            while let Some(action) = app.try_recv_action() {
                match action {
                    Action::Transcript(TranscriptAction::Live(session, chunk))
                        if session == SessionId::new("s-1") =>
                    {
                        // @step And text "background" for session "s-1" reaches the action bus as Action::Transcript(TranscriptAction::Live)
                        assert!(matches!(&chunk, StreamChunk::Text { text, correlation_id: None, observed_correlation_ids: None } if text == "background"), "unexpected background payload: {chunk:?}");
                        saw_s1 = true;
                    }
                    Action::Transcript(TranscriptAction::Live(session, chunk))
                        if session == SessionId::new("s-2") =>
                    {
                        // @step And text "foreground" for session "s-2" reaches the action bus as Action::Transcript(TranscriptAction::Live)
                        assert!(matches!(&chunk, StreamChunk::Text { text, correlation_id: None, observed_correlation_ids: None } if text == "foreground"), "unexpected focused payload: {chunk:?}");
                        saw_s2 = true;
                    }
                    _ => {}
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await?;
    assert!(
        saw_s2,
        "expected Transcript(Live) for s-2 (focused) on the bus"
    );
    assert!(
        saw_s1,
        "expected Transcript(Live) for s-1 (background) on the bus per RPC-045"
    );
    Ok(())
}

/// Scenario: Navigator renders AgentView when active_view is Agent
#[tokio::test]
async fn navigator_renders_agent_view_when_active_view_is_agent() {
    use codelet_fspec_tui::Navigator;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let mock = Arc::new(MockBackend::new());
    let mut app = App::new(mock);
    // @step Given an App with Navigator.active_view = ViewMode::Agent
    app.navigator_mut().active_view = ViewMode::Agent;
    // @step And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    app.agent_view_store_mut()
        .append_session(codelet_fspec_tui::SessionContext::new(SessionId::new(
            "s-1",
        )));
    // @step When the App renders against an 80x24 TestBackend
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    let mut nav = Navigator::new(
        std::sync::Arc::new(codelet_fspec_tui::Theme::default()),
        app.action_tx_clone(),
    );
    nav.active_view = ViewMode::Agent;
    let mut agent_snapshot = std::mem::take(app.agent_view_store_mut());
    let board_snapshot: codelet_fspec_tui::BoardStore = std::mem::take(app.board_store_mut());
    nav.render_with_stores(
        ratatui::layout::Rect::new(0, 0, 120, 24),
        term.current_buffer_mut(),
        &board_snapshot,
        &mut agent_snapshot,
    );
    let buf = term.current_buffer_mut().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    // @step Then the rendered buffer contains the AgentView "Agent" block title
    // RPC-029: AgentView no longer paints an "Agent — <sid>" title;
    // the visible AgentView anchor is the input placeholder hint.
    assert!(
        joined.contains("Type a message..."),
        "AgentView placeholder hint must be visible"
    );
    assert!(
        joined.contains("Agent"),
        "header should render 'Agent' placeholder when no model"
    );
    // @step And the rendered buffer does NOT contain the BACKLOG SPECIFYING column headers
    assert!(!joined.contains("BACKLOG"));
}
