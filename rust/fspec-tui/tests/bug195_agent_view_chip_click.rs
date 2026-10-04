//! BUG-195 — chip click in the Agent view switches to that session.
//!
//! Feature: spec/features/bug195-agent-view-chip-click-switches-session.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: App + MockBackend (the `menu002_board_surface.rs` pattern) —
//! `fresh_app` + `drain_pending` + full-App render into a TestBackend at
//! 120x24 (the agent bar's geometry is located by row scan, not
//! hardcoded, so the agent surface needs no board column seeding).
//!
//! Observation points:
//! - session switch: `app.active_view()` + `app.current_session()` /
//!   `app.agent_view_store().current_session_index()`;
//! - draft round-trip: `SessionContext.input_draft` (the RPC-024
//!   snapshot `App::switch_to_session_index` restores);
//! - bar highlight: `app.navigator().agent.menu_focus()` (cleared after
//!   a chip activation).
//!
//! Geometry (120x24, single Agent view):
//!   y0  SessionHeader
//!   y1  the 2-zone menu bar (the 'Board View' item per MENU-007, dim
//!       `│` separator, chips `#1` `#2` `#3` — x positions located by scan)
//!   y2+ role/scrollback/footer/input

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{App, FspecBackend, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

const AGENT_BAR_ROW: u16 = 1;

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Render the full App (navigator + compositor) into a 120x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The x of the first occurrence of `needle` on `y` (None when absent).
/// `String::find` returns a BYTE index — the row contains multi-byte
/// glyphs (`│`), so convert to the CHAR (cell) index the buffer
/// addresses by.
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

/// The x of chip `n` (1-based) on the agent bar row.
fn chip_x(buf: &Buffer, n: usize) -> u16 {
    find_x(buf, AGENT_BAR_ROW, &format!("#{n}")).unwrap_or_else(|| {
        panic!(
            "chip #{n} must paint on the agent bar row:\n{}",
            row_text(buf, AGENT_BAR_ROW)
        )
    })
}

/// Seed `n` open sessions (s-1..s-n, Idle by default) and focus s-1.
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    // `append_session` focuses the LAST appended session — reset to s-1.
    app.agent_view_store_mut().focus_session_index(0);
    drain_pending(app).await;
}

/// Flip the App into the Agent view on the given open session (the
/// RPC-097 jump-into-existing-session path).
async fn enter_agent_view(app: &mut App, session: &str) {
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new(session.to_string()),
    )));
    drain_pending(app).await;
}

/// Arrange (board): the 2-zone bar's ring is focused on chip `n`
/// (1-based) — focus the last column, then walk Right (items + chips).
/// MENU-008: the board bar has FOUR Zone A items (Kanban, Tools,
/// Settings, Help), so chip #n is `4 + n` Rights from the last column.
fn focus_board_chip(app: &mut App, n: usize) {
    app.board_store_mut().set_focused_column("blocked");
    // last column → 4 items → chips: `4 + n` Rights (menu002 parity:
    // chip #2 = 6 Rights from the last column).
    for _ in 0..(n + 4) {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_events(app);
}

fn drain_events(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Arrange (agent): the 2-zone bar's ring is focused on chip `n`
/// (1-based) — bare Left enters the bar at Item(0), then walk Right.
/// MENU-007: the agent bar has a SINGLE Zone A item ('Board View'), so
/// chip #n is `n` Rights from Item(0).
fn focus_agent_chip(app: &mut App, n: usize) {
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    // Item(0) → chip #1 → … → chip #n: `n` Rights.
    for _ in 0..n {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_events(app);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Clicking a chip in the Agent view switches to that session
#[tokio::test]
async fn scenario_clicking_a_chip_in_the_agent_view_switches_to_that_session() {
    // @step Given the App is in the Agent view with 3 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;

    // @step And session s-1 is the focused session
    enter_agent_view(&mut app, "s-1").await;
    assert_eq!(
        app.agent_view_store().current_session_index(),
        0,
        "s-1 must be the focused open-session slot"
    );
    let buf = render_app(&mut app);
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the App must be in the Agent view"
    );

    // @step When I click chip #2 in the menu bar
    let x = chip_x(&buf, 2);
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the agent pane shows session s-2
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2".to_string())),
        "clicking chip #2 in the Agent view must switch to s-2"
    );
    assert_eq!(
        app.agent_view_store().current_session_index(),
        1,
        "the focused open-session slot must move to s-2's slot"
    );
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "the bar highlight must clear after chip activation"
    );
}

/// Scenario: A chip click in the Agent view keeps the draft round-trip
#[tokio::test]
async fn scenario_a_chip_click_in_the_agent_view_keeps_the_draft_round_trip() {
    // @step Given the App is in the Agent view with 3 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;

    // @step And session s-1 is focused with a live input draft of "hi"
    enter_agent_view(&mut app, "s-1").await;
    app.handle_event(&key(KeyCode::Char('h'), KeyModifiers::NONE));
    app.handle_event(&key(KeyCode::Char('i'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.input.value(),
        "hi",
        "the live input must carry the draft before the click"
    );

    // @step When I click chip #3 in the menu bar
    let buf = render_app(&mut app);
    let x = chip_x(&buf, 3);
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the agent pane shows session s-3
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-3".to_string())),
        "clicking chip #3 in the Agent view must switch to s-3"
    );

    // @step And session s-1's saved draft is "hi"
    let draft = app
        .agent_view_store()
        .session_context_for(&SessionId::new("s-1".to_string()))
        .map(|c| c.input_draft.clone())
        .unwrap_or_default();
    assert_eq!(
        draft, "hi",
        "the outgoing session's draft must be snapshotted into its input_draft (RPC-024 round-trip)"
    );
}

/// Scenario: Enter on a chip in the Agent view switches to that session
#[tokio::test]
async fn scenario_enter_on_a_chip_in_the_agent_view_switches_to_that_session() {
    // @step Given the App is in the Agent view with 3 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;

    // @step And session s-1 is the focused session
    enter_agent_view(&mut app, "s-1").await;

    // @step And the menu bar ring is focused on chip #2
    render_app(&mut app); // cache the agent bar's geometry (chip count)
    focus_agent_chip(&mut app, 2);
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the agent ring must be focused on chip #2"
    );

    // @step When I press Enter once
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the agent pane shows session s-2
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2".to_string())),
        "Enter on chip #2 in the Agent view must switch to s-2 (keyboard path)"
    );
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "the bar highlight must clear after chip activation"
    );
}

/// Scenario: The board chip activation still flips to the Agent view
#[tokio::test]
async fn scenario_the_board_chip_activation_still_flips_to_the_agent_view() {
    // @step Given the App is in the Board view with 3 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the App must start in the Board view"
    );

    // @step And the menu bar ring is focused on chip #2
    focus_board_chip(&mut app, 2);
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the board ring must be focused on chip #2"
    );

    // @step When I press Enter once
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the active view is the Agent view showing session s-2
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Enter on a chip from the Board view must flip to the Agent view (MENU-002, no regression)"
    );
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2".to_string())),
        "the flipped view must host s-2"
    );
}
