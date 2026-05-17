//! RPC-013 — AgentView footer rendering tests + migrated inline tests
//! from RPC-012's `views/agent.rs` (extracted here so `agent.rs` stays
//! under the 300 LoC ceiling after adding the footer row).
//!
//! Feature: spec/features/rpc013-agent-footer.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{Action, AgentView};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn fresh() -> (AgentView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    (AgentView::new(tx), rx)
}

fn render_agent(width: u16, height: u16, session: Option<SessionId>) -> String {
    let (mut view, _rx) = fresh();
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    if let Some(sid) = session {
        store.append_session(codelet_fspec_tui::SessionContext::new(sid));
    }
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &mut store);
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
    joined
}

fn render_agent_rows(width: u16, height: u16, session: Option<SessionId>) -> Vec<String> {
    let (mut view, _rx) = fresh();
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    if let Some(sid) = session {
        store.append_session(codelet_fspec_tui::SessionContext::new(sid));
    }
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &mut store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

/// Scenario: AgentView renders the placeholder footer string for this slice
///
/// The placeholder uses DOUBLE SPACES between hints — that exact form
/// (`Enter=send  Ctrl+C=interrupt  ESC=back`) appears nowhere in the
/// pre-RPC-013 view (the existing input-box title uses comma separators),
/// so the assertion is a discriminating red-phase guarantee.
#[test]
fn agent_view_renders_placeholder_footer_string() {
    // @step Given an App with Navigator.active_view = ViewMode::Agent
    // @step And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    let session = Some(SessionId::new("s-1"));
    // @step When the App renders against a 120x24 TestBackend
    let joined = render_agent(120, 24, session);
    // @step Then the rendered buffer contains the substring "Enter=send"
    // @step And the rendered buffer contains the substring "Ctrl+C=interrupt"
    // @step And the rendered buffer contains the substring "ESC=back"
    // Combined into the single canonical placeholder form:
    let needle = "Enter=send  Ctrl+C=interrupt  ESC=back";
    assert!(
        joined.contains(needle),
        "missing canonical footer placeholder `{needle}` in:\n{joined}"
    );
}

/// Scenario: AgentView footer omits the legacy generic hint and the BoardView hint
#[test]
fn agent_view_footer_omits_legacy_generic_hint_and_board_view_hint() {
    // @step Given an App with Navigator.active_view = ViewMode::Agent
    // @step And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    let session = Some(SessionId::new("s-1"));
    // @step When the App renders against a 120x24 TestBackend
    let joined = render_agent(120, 24, session);
    // @step Then the rendered buffer does NOT contain the substring "? help"
    assert!(!joined.contains("? help"), "legacy '? help' still present in:\n{joined}");
    // @step And the rendered buffer does NOT contain the substring "switch pane"
    assert!(!joined.contains("switch pane"), "legacy 'switch pane' still present in:\n{joined}");
    // @step And the rendered buffer does NOT contain the substring "Columns ◆ ↑↓ Work Units"
    assert!(
        !joined.contains("Columns ◆ ↑↓ Work Units"),
        "BoardView footer leaked into AgentView in:\n{joined}"
    );
}

/// RPC-013: footer occupies the bottom row of the AgentView area.
#[test]
fn agent_view_footer_lives_on_the_bottom_row() {
    let session = Some(SessionId::new("s-1"));
    let rows = render_agent_rows(120, 24, session);
    assert_eq!(rows.len(), 24);
    assert!(
        rows[23].contains("Enter=send  Ctrl+C=interrupt  ESC=back"),
        "expected canonical footer string on row 23, got:\nrow 22: {}\nrow 23: {}",
        rows[22], rows[23]
    );
}

// ── Migrated RPC-012 inline AgentView tests (extracted from
//     codelet/fspec-tui/src/views/agent.rs so that file stays < 300 LoC
//     after the RPC-013 footer row addition). The Feature header for
//     these is the RPC-012 feature; the @step comments are preserved.

/// Scenario: ESC from AgentView returns to BoardView preserving focus and selection
/// (Migrated from RPC-012 inline tests; mirrors `esc_emits_back_to_board`.)
#[test]
fn esc_emits_back_to_board() {
    let (mut view, mut rx) = fresh();
    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let result = view.handle_event(&event);
    assert!(matches!(
        result,
        codelet_fspec_tui::EventResult::Consumed(None)
    ));
    let action = rx.try_recv().expect("Action::BackToBoard on bus");
    assert!(matches!(action, Action::BackToBoard));
}

/// Migrated from RPC-012 inline tests: Enter on non-empty input emits
/// Action::InputSubmitted with the input value.
///
/// RPC-019 update: the old test poked the now-removed
/// `tui_input::Input::with_value` / `.value()` surface directly. After
/// the MultiLineInput swap, we drive the AgentView through real
/// keystrokes and assert via the Action bus + `MultiLineInput::value`.
#[test]
fn enter_on_non_empty_input_emits_input_submitted() {
    let (mut view, mut rx) = fresh();
    view.input.set_value("hi");
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let _ = view.handle_event(&event);
    let action = rx.try_recv().expect("Action::InputSubmitted on bus");
    match action {
        Action::InputSubmitted(s) => assert_eq!(s, "hi"),
        other => panic!("expected InputSubmitted, got {other:?}"),
    }
    assert!(view.input.is_empty());
}

/// Migrated from RPC-012 inline tests: title shows session id.
#[test]
fn render_with_store_paints_agent_title_with_session_id() {
    let joined = render_agent(80, 24, Some(SessionId::new("s-1")));
    assert!(joined.contains("Agent"));
    assert!(joined.contains("s-1"));
}
