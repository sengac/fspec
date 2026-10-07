//! MENU-007 — the agent view's 2-zone bar shows a single 'Board View'
//! item instead of the board-only Actions/Help menus.
//!
//! Feature: spec/features/agent-view-menu-bar-board-view-item.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: App + MockBackend (the `bug195_agent_view_chip_click.rs`
//! pattern) — full-App render into a TestBackend at 120x24; the agent
//! bar's geometry is located by row scan (AGENT_BAR_ROW = 1: header
//! row 0, bar row 1).
//!
//! Observation points:
//! - the bar row text (the 'Board View' item, the dim `│` separator,
//!   the chips; NO 'Actions' / 'Help' text);
//! - the item highlight: `buf[(x, y)].bg == Color::Cyan` (the same
//!   inverse style a selected Zone A item uses);
//! - view flips: `app.active_view()`;
//! - ring state: `app.navigator().agent.menu_focus()`;
//! - session switch: `app.current_session()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{App, FspecBackend, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
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
/// glyphs (`│`, `●`), so convert to the CHAR (cell) index the buffer
/// addresses by.
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

/// The x of the 'Board View' Zone A item on the agent bar row.
fn board_view_x(buf: &Buffer) -> u16 {
    find_x(buf, AGENT_BAR_ROW, "Board View")
        .expect("the 'Board View' item must paint on the bar row")
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

/// The x of a Zone C button label ("New Agent" / "Close Agent") on the
/// agent bar row (MENU-009).
fn zone_c_button_x(buf: &Buffer, label: &str) -> u16 {
    find_x(buf, AGENT_BAR_ROW, label).unwrap_or_else(|| {
        panic!(
            "the Zone C button '{label}' must paint on the agent bar row:\n{}",
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

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The agent bar paints a single 'Board View' Zone A item instead of Actions and Help
#[tokio::test]
async fn scenario_the_agent_bar_paints_a_single_board_view_item_instead_of_actions_and_help() {
    // @step Given the agent pane has 2 open sessions (one running, one idle) and an empty input
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enter_agent_view(&mut app, "s-1").await;
    app.agent_view_store_mut().set_session_status(
        SessionId::new("s-1".to_string()),
        codelet_rpc_types::SessionStatus::Running,
    );
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);

    // @step When the agent pane renders
    let row = row_text(&buf, AGENT_BAR_ROW);

    // @step Then the bar row shows the 'Board View' item and the dim separator and the two session chips
    assert!(
        row.contains("Board View"),
        "the bar row must show the 'Board View' item: {row:?}"
    );
    assert!(
        row.contains('│'),
        "the bar row must show the zone separator: {row:?}"
    );
    assert!(row.contains("#1"), "the bar row must show chip #1: {row:?}");
    assert!(row.contains("#2"), "the bar row must show chip #2: {row:?}");

    // @step And the bar row does not show the 'Kanban' item (MENU-008:
    // the board-only 'Actions' item was renamed to 'Kanban')
    assert!(
        !row.contains("Kanban"),
        "the agent bar must NOT paint the board-only 'Kanban' item: {row:?}"
    );

    // @step And the bar row does not show the 'Help' item
    assert!(
        !row.contains("Help"),
        "the agent bar must NOT paint the board-only 'Help' item: {row:?}"
    );
}

/// Scenario: The 'Board View' item paints the selected-item highlight when the ring focuses it
#[tokio::test]
async fn scenario_the_board_view_item_paints_the_selected_item_highlight_when_the_ring_focuses_it()
{
    // @step Given the agent pane has 1 open session and an empty input
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the bar geometry (chip count)

    // @step When I press bare Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);

    // @step Then the 'Board View' item paints inverse-video (bg Cyan fg Black bold) with no dropdown open
    let x = board_view_x(&buf);
    let cell = &buf[(x, AGENT_BAR_ROW)];
    assert_eq!(
        cell.bg,
        Color::Cyan,
        "'Board View' must paint inverse-video bg"
    );
    assert_eq!(
        cell.fg,
        Color::Black,
        "'Board View' must paint inverse-video fg"
    );
    assert!(
        cell.modifier.contains(ratatui::style::Modifier::BOLD),
        "'Board View' must paint bold"
    );
    assert!(
        app.navigator().agent.open_menu().is_none(),
        "no dropdown may open for the 'Board View' item"
    );
}

/// Scenario: Enter on 'Board View' returns to the Board view
#[tokio::test]
async fn scenario_enter_on_board_view_returns_to_the_board_view() {
    // @step Given the App is in the Agent view with 2 open sessions and the bar is focused on the 'Board View' item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the bar geometry (chip count)
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE)); // bare Left → Item(0) = 'Board View'
    drain_pending(&mut app).await;

    // @step When I press Enter once
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the active view is the Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Enter on 'Board View' must return to the Board view"
    );

    // @step And the agent bar highlight clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "the bar highlight must clear after activating 'Board View'"
    );
}

/// Scenario: Clicking 'Board View' returns to the Board view
#[tokio::test]
async fn scenario_clicking_board_view_returns_to_the_board_view() {
    // @step Given the App is in the Agent view with 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enter_agent_view(&mut app, "s-1").await;
    let buf = render_app(&mut app);
    let x = board_view_x(&buf);

    // @step When I click the 'Board View' item in the bar
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the active view is the Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "clicking 'Board View' must return to the Board view"
    );

    // @step And no dropdown panel paints over the pane
    let buf = render_app(&mut app);
    assert!(
        app.navigator().agent.open_menu().is_none(),
        "clicking 'Board View' must not open a dropdown"
    );
    let _ = buf;
}

/// Scenario: The agent ring wraps from 'Board View' through the chips and back
#[tokio::test]
async fn scenario_the_agent_ring_wraps_from_board_view_through_the_chips_and_back() {
    // @step Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the bar geometry (chip count)
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE)); // Item(0) = 'Board View'
    drain_pending(&mut app).await;

    // @step When I press Right five times
    // (MENU-009: 2 chips + 2 Zone C buttons = 4 stops after the item.)
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "1st Right must land on chip #1 (no second Zone A item)"
    );
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "2nd Right must land on chip #2"
    );
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "3rd Right must land on the 'New Agent' button (MENU-009)"
    );
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "4th Right must land on the 'Close Agent' button (MENU-009)"
    );
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the focus lands on chip #1 then chip #2 then 'New Agent' then 'Close Agent' and finally back on 'Board View'
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::Item(0)),
        "5th Right must wrap back to 'Board View'"
    );
}

/// Scenario: Left from 'Board View' wraps to the last Zone C button
#[tokio::test]
async fn scenario_left_from_board_view_wraps_to_the_last_chip() {
    // @step Given the agent pane has 3 open sessions and the menu bar is focused on the 'Board View' item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the bar geometry (chip count + Zone C rects)
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE)); // Item(0) = 'Board View'
    drain_pending(&mut app).await;

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);

    // @step Then the 'Close Agent' button paints inverse-video (MENU-009:
    // the left wrap lands on the LAST Zone C button, not the last chip)
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneC(1)),
        "Left from 'Board View' must wrap to the last Zone C button"
    );
    let x = zone_c_button_x(&buf, "Close Agent");
    assert_eq!(
        buf[(x, AGENT_BAR_ROW)].bg,
        Color::Cyan,
        "'Close Agent' must paint inverse-video"
    );
}

/// Scenario: Chip activation in the agent view is unchanged
#[tokio::test]
async fn scenario_chip_activation_in_the_agent_view_is_unchanged() {
    // @step Given the App is in the Agent view with 3 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    enter_agent_view(&mut app, "s-1").await;
    let buf = render_app(&mut app);

    // @step When I click chip #2 in the menu bar
    let x = chip_x(&buf, 2);
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the agent pane shows session s-2
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2".to_string())),
        "clicking chip #2 must switch to s-2"
    );

    // @step And the active view is still the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "chip activation must stay in the Agent view"
    );
}

/// Scenario: The agent bar with no open sessions shows only 'Board View'
#[tokio::test]
async fn scenario_the_agent_bar_with_no_open_sessions_shows_only_board_view() {
    // @step Given the agent pane has no open sessions
    // (view-level harness — the App cannot enter the Agent view with an
    // empty open-session list; it opens the CreateSessionDialog instead)
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut view = codelet_fspec_tui::AgentView::new(tx);
    let mut store = codelet_fspec_tui::AgentViewStore::default();

    // @step When the agent pane renders
    let mut term = Terminal::new(TestBackend::new(100, 24)).expect("Terminal::new");
    term.draw(|frame| view.render_with_store(frame.area(), frame.buffer_mut(), &mut store))
        .expect("draw");
    let buf = term.backend().buffer().clone();

    // @step Then the bar row shows only the 'Board View' item with no separator and no chips
    let row = row_text(&buf, AGENT_BAR_ROW);
    assert!(
        row.contains("Board View"),
        "the bar must still show 'Board View': {row:?}"
    );
    assert!(
        !row.contains('│'),
        "no separator may paint without chips: {row:?}"
    );
    assert!(
        !row.contains("#1"),
        "no chips may paint without sessions: {row:?}"
    );
}

/// Scenario: Board and mux bars keep their Actions and Help items
#[tokio::test]
async fn scenario_board_and_mux_bars_keep_their_actions_and_help_items() {
    // @step Given the App is in the Board view
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I render the board header row
    let buf = render_app(&mut app);
    let bar_row = (0..buf.area.height)
        .find(|&y| {
            let row = row_text(&buf, y);
            row.contains("Kanban") && row.contains("Help")
        })
        .expect("the board bar row must render");

    // @step Then the board menu bar still shows the 'Kanban' item and the 'Help' item (MENU-008)
    assert!(
        row_text(&buf, bar_row).contains("Kanban"),
        "the board bar must keep the 'Kanban' item"
    );
    assert!(
        row_text(&buf, bar_row).contains("Help"),
        "the board bar must keep the 'Help' item"
    );
}
