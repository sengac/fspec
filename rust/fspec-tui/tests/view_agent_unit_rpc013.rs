//! RPC-013 — AgentView footer rendering tests + migrated inline tests
//! from RPC-012's `views/agent.rs` (extracted here so `agent.rs` stays
//! under the 300 LoC ceiling after adding the footer row).
//!
//! Feature: spec/features/rpc013-agent-footer.feature

use codelet_fspec_tui::{Action, AgentView};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn fresh() -> (AgentView, tokio::sync::mpsc::UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    // BUG-552: the composer belongs to a session, as App::sync_composer_draft sets it.
    let mut view = AgentView::new(tx);
    view.set_composer_session(Some(codelet_rpc_types::SessionId::new("s-1")));
    (view, rx)
}

fn render_agent(
    width: u16,
    height: u16,
    session: Option<SessionId>,
) -> Result<String, codelet_test_helpers::TestError> {
    let (mut view, _rx) = fresh();
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    if let Some(sid) = session {
        store.append_session(codelet_fspec_tui::SessionContext::new(sid));
    }
    let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &mut store);
    })
    .or_fail("draw")?;
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    Ok(joined)
}

fn render_agent_rows(
    width: u16,
    height: u16,
    session: Option<SessionId>,
) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let (mut view, _rx) = fresh();
    let mut store = codelet_fspec_tui::AgentViewStore::default();
    if let Some(sid) = session {
        store.append_session(codelet_fspec_tui::SessionContext::new(sid));
    }
    let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), &mut store);
    })
    .or_fail("draw")?;
    let buf = term.backend().buffer().clone();
    let mut rows: Vec<String> = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Scenario: AgentView no longer paints the placeholder footer hints (superseded by RPC-029)
///
/// RPC-029 removed the left-side footer hints (the canonical TS
/// SessionFooter has an EMPTY left side). The
/// `PLACEHOLDER_FOOTER_HINTS` constant is preserved in `views/agent.rs`
/// purely for the RPC-013 source-shape invariant — it is no longer
/// painted into the buffer.
#[test]
fn agent_view_renders_placeholder_footer_string() -> TestResult {
    // @step Given an App with Navigator.active_view = ViewMode::Agent
    // @step And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    let session = Some(SessionId::new("s-1"));
    // @step When the App renders against a 120x24 TestBackend
    let joined = render_agent(120, 24, session)?;
    // @step Then the rendered buffer does NOT contain the substring "Enter=send"
    assert!(
        !joined.contains("Enter=send"),
        "RPC-029 removed footer hints; got:\n{joined}"
    );
    // @step And the rendered buffer does NOT contain the substring "Ctrl+C=interrupt"
    assert!(
        !joined.contains("Ctrl+C=interrupt"),
        "RPC-029 removed footer hints; got:\n{joined}"
    );
    // @step And the rendered buffer does NOT contain the substring "ESC=back" (RPC-029: footer left side is empty)
    assert!(
        !joined.contains("ESC=back"),
        "RPC-029 left the footer's left side empty; got:\n{joined}"
    );
    Ok(())
}

/// Scenario: AgentView footer omits the legacy generic hint and the BoardView hint
#[test]
fn agent_view_footer_omits_legacy_generic_hint_and_board_view_hint() -> TestResult {
    // @step Given an App with Navigator.active_view = ViewMode::Agent
    // @step And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    let session = Some(SessionId::new("s-1"));
    // @step When the App renders against a 120x24 TestBackend
    let joined = render_agent(120, 24, session)?;
    // @step Then the rendered buffer does NOT contain the substring "? help"
    assert!(
        !joined.contains("? help"),
        "legacy '? help' still present in:\n{joined}"
    );
    // @step And the rendered buffer does NOT contain the substring "switch pane"
    assert!(
        !joined.contains("switch pane"),
        "legacy 'switch pane' still present in:\n{joined}"
    );
    // @step And the rendered buffer does NOT contain the substring "Columns ◆ ↑↓ Work Units"
    assert!(
        !joined.contains("Columns ◆ ↑↓ Work Units"),
        "BoardView footer leaked into AgentView in:\n{joined}"
    );
    Ok(())
}

/// RPC-029: footer now sits ABOVE the input row (was on the bottom).
#[test]
fn agent_view_footer_lives_on_the_bottom_row() -> TestResult {
    let session = Some(SessionId::new("s-1"));
    let rows = render_agent_rows(120, 24, session)?;
    assert_eq!(rows.len(), 24);
    // RPC-029: input is the bottom row, footer is above it.
    assert!(
        rows[23].contains("> "),
        "input prompt should sit on the bottom row, got: {:?}",
        rows[23]
    );
    // The row just above the input is the footer. With no workspace
    // set in this test it paints an empty (dark-grey) row, so we just
    // assert the bottom row is the input, not a footer hint.
    assert!(
        !rows[23].contains("Enter=send"),
        "row 23 must not contain old footer hint: {:?}",
        rows[23]
    );
    Ok(())
}

// ── Migrated RPC-012 inline AgentView tests (extracted from
//     rust/fspec-tui/src/views/agent.rs so that file stays < 300 LoC
//     after the RPC-013 footer row addition). The Feature header for
//     these is the RPC-012 feature; the @step comments are preserved.

/// Scenario: ESC from AgentView emits Action::AgentEscPressed
/// (RPC-051 cascade — App::dispatch routes onward to either
/// `backend.interrupt` or `Action::BackToBoard`).
#[test]
fn esc_emits_back_to_board() -> TestResult {
    let (mut view, mut rx) = fresh();
    let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    let result = view.handle_event(&event);
    assert!(matches!(
        result,
        codelet_fspec_tui::EventResult::Consumed(None)
    ));
    let action = rx.try_recv().or_fail("Action::AgentEscPressed on bus")?;
    assert!(matches!(action, Action::AgentEscPressed));
    Ok(())
}

/// Migrated from RPC-012 inline tests: Enter on non-empty input emits
/// Action::InputSubmitted with the input value.
///
/// RPC-019 update: the old test poked the now-removed
/// `tui_input::Input::with_value` / `.value()` surface directly. After
/// the MultiLineInput swap, we drive the AgentView through real
/// keystrokes and assert via the Action bus + `MultiLineInput::value`.
#[test]
fn enter_on_non_empty_input_emits_input_submitted() -> TestResult {
    let (mut view, mut rx) = fresh();
    view.input.set_value("hi");
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let _ = view.handle_event(&event);
    let action = rx.try_recv().or_fail("Action::InputSubmitted on bus")?;
    match action {
        Action::InputSubmitted(session, s) => {
            assert_eq!(session, codelet_rpc_types::SessionId::new("s-1"));
            assert_eq!(s, "hi");
        }
        other => return Err(format!("expected InputSubmitted, got {other:?}").into()),
    }
    assert!(view.input.is_empty());
    Ok(())
}

/// Migrated from RPC-012 inline tests: title shows session id.
///
/// RPC-029: the scrollback Block + " Agent — <sid> " title were
/// removed to match the canonical TS layout. The visible "AgentView"
/// anchor is now the input placeholder hint (or the SessionHeader's
/// fallback "Agent" model placeholder when no model is loaded).
#[test]
fn render_with_store_paints_agent_title_with_session_id() -> TestResult {
    let joined = render_agent(80, 24, Some(SessionId::new("s-1")))?;
    assert!(joined.contains("Agent"));
    // RPC-029: session id is no longer painted anywhere on the chrome.
    assert!(
        !joined.contains(" Agent — s-1"),
        "old title should not appear"
    );
    Ok(())
}
