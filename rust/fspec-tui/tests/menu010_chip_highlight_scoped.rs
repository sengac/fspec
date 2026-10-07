//! MENU-010 — the chip's selected-item highlight is scoped to the surface.
//!
//! Feature: spec/features/chip-selected-highlight-scoped-to-surface.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! MENU-010 supersedes the MENU-006 board/mux rule: the store's current
//! session's chip carries the selected-item (inverse-video) highlight
//! ONLY on the Agent view; on the mux surface it carries it only when
//! the FOCUSED pane is an Agent pane (that pane's window session —
//! agent-pane parity); on the BOARD view NO chip ever carries it — the
//! ring selector (`MenuFocus::ZoneB`) is the only chip highlight path
//! there. The shared painter's `focused_cell()` is unchanged: it ORs the
//! ring focus with the per-chip `active` flag that the SURFACE's
//! snapshot builder now decides (the board's builder marks no chip).
//!
//! Harness: App + MockBackend (the `menu002_board_surface.rs` /
//! `menu004_mux_surface.rs` pattern) — `fresh_app` + `drain_pending` +
//! full-App render into a TestBackend (board/agent 120x24, mux 240x24),
//! chip x positions located by row scan.
//!
//! Observation points:
//! - the chip highlight: `buf[(x, y)].bg == Color::Cyan` (the shared
//!   painter's inverse style — the same `inverse_style()` a selected
//!   Zone A item uses);
//! - inactivity: the bar row's #333333 background (`MENU_BAR_BG`)
//!   everywhere else;
//! - ring state: `app.board_store().menu_focus()` /
//!   `app.navigator().agent.menu_focus()` /
//!   `app.navigator().mux.menu_focus()`.
//!
//! Geometry (120x24, board view): the 2-zone bar row = 3 rows below the
//! `Checkpoints:` status row; chips paint as 4-cell cells with 2-cell
//! gaps after the dim `│` separator (menu002 parity: `#1 ●` 4 cells).
//! Geometry (120x24, agent view): the bar row is the row carrying
//! "Board View" + the `│` separator. Geometry (240x24, mux): the bar
//! row is row 0 (menu004 parity).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::{App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;

mod common;
use common::MockBackend;

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

/// Render the full App (board / agent surface) into a 120x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    render_app_sized(app, 120, 24)
}

/// Render the full App into a `w`×`h` buffer (the mux scenarios use
/// 240x24, menu004 parity).
fn render_app_sized(app: &mut App, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
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

/// The x of chip `n` (1-based) on row `y`.
fn chip_x(buf: &Buffer, y: u16, n: usize) -> u16 {
    find_x(buf, y, &format!("#{n}"))
        .unwrap_or_else(|| panic!("chip #{n} must paint on row {y}:\n{}", row_text(buf, y)))
}

/// The board bar's row (3 rows below the `Checkpoints:` status row,
/// menu002 parity).
fn board_bar_row(buf: &Buffer) -> u16 {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .expect("the checkpoint status row must render");
    status_y + 3
}

/// The agent bar's row — the single row carrying the "Board View" item
/// and the `│` separator (menu006 parity).
fn agent_bar_row(buf: &Buffer) -> u16 {
    (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Board View") && row_text(buf, y).contains('│'))
        .unwrap_or_else(|| {
            panic!(
                "the 2-zone agent bar row must render:\n{}",
                (0..buf.area.height)
                    .map(|y| row_text(buf, y))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
}

/// Seed `n` open sessions (s-1..s-n, Idle by default). The store's
/// current session ends up being the LAST appended one.
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await;
}

/// Seed a small board (AUTH-001 backlog, AUTH-002 testing, AUTH-003
/// blocked) so the board's columns have selectable cards.
async fn seed_units(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog"),
        make_unit("AUTH-002", "testing"),
        make_unit("AUTH-003", "blocked"),
    ]));
    drain_pending(app).await;
}

fn make_unit(id: &str, status: &str) -> WorkUnitInfo {
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

/// Flip the App into the Agent view on the given open session (the
/// RPC-097 jump-into-existing-session path).
async fn enter_agent_view(app: &mut App, session: &str) {
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new(session.to_string()),
    )));
    drain_pending(app).await;
}

/// Enable mux with the given pane list (horizontal, equal splits, focus
/// pane 0) — mirrors `menu004_mux_surface.rs::enable_mux` (the BUG-182
/// R7 initial loads only apply to the lazy Files/Checkpoints panes).
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let n = panes.len();
    let config = MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50; n.saturating_sub(1)],
        panes,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
}

/// Focus a specific mux pane index (menu004 `focus_pane` parity).
fn focus_pane(app: &mut App, idx: usize) {
    app.navigator_mut().mux.set_focus(idx);
}

/// A chip's painted cell: 4 cells wide (`#n` + gap + glyph) when no
/// work-unit id is bound — assert every cell of the range.
fn chip_cell_range(x: u16) -> std::ops::Range<u16> {
    x..x + 4
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The board view does not highlight the current session's chip after flipping back from the Agent view
#[tokio::test]
async fn scenario_the_board_view_does_not_highlight_the_current_sessions_chip_after_flipping_back_from_the_agent_view(
) {
    // @step Given the board has 3 open sessions and session s-2 is the store's current session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    app.agent_view_store_mut().focus_session_index(1);
    assert_eq!(
        app.agent_view_store().current_session_index(),
        1,
        "s-2 must be the store's current session"
    );

    // @step When I open session s-2's Agent view from the board and then return to the board with the ring focus cleared
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(
        SessionId::new("s-2"),
    )));
    drain_pending(&mut app).await;
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the flip must land in the Agent view"
    );
    app.dispatch(codelet_fspec_tui::Action::BackToBoard);
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(
        app.board_store().menu_focus().is_none(),
        "returning to the board must clear the ring focus"
    );

    // @step Then no cell on the board bar row paints the inverse-video highlight (chip #2's cells keep the #333333 background)
    let buf = render_app(&mut app);
    let y = board_bar_row(&buf);
    let x2 = chip_x(&buf, y, 2);
    for x in 0..buf.area.width {
        assert_ne!(
            buf[(x, y)].bg,
            Color::Cyan,
            "no inverse cell may survive on the board bar row {y} \
             (the current session s-2 is chip #2 at x {x2} — the stale \
              highlight the board rule forbids):\n{}",
            row_text(&buf, y)
        );
    }
}

/// Scenario: The ring selector highlights a chip on the board view
#[tokio::test]
async fn scenario_the_ring_selector_highlights_a_chip_on_the_board_view() {
    // @step Given the board has 3 open sessions and the ring selector (MenuFocus::ZoneB) is on chip #2
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    app.board_store_mut().set_focused_column("blocked");
    let _ = render_app(&mut app);
    // The ring from the last column (pos 6): +4 lands on the last item
    // (Help), +5 on chip #1, +6 on chip #2 (MENU-002 ring parity).
    for _ in 0..6 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "6 Rights from the last column must land on chip #2 (4 items + 2 chips)"
    );

    // @step When the board view renders the 2-zone menu bar
    let buf = render_app(&mut app);
    let y = board_bar_row(&buf);
    let x2 = chip_x(&buf, y, 2);

    // @step Then chip #2's cells are styled bg Cyan fg Black (the ring selector path) and chip #1's and chip #3's cells keep the #333333 background
    for cx in chip_cell_range(x2) {
        assert_eq!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #2 cell {cx} must be inverse (the ring selector path)"
        );
    }
    for n in [1usize, 3] {
        let x = chip_x(&buf, y, n);
        for cx in chip_cell_range(x) {
            assert_ne!(
                buf[(cx, y)].bg,
                Color::Cyan,
                "chip #{n} cell {cx} must keep the #333333 background"
            );
        }
    }
}

/// Scenario: The Agent view highlights the current session's chip without a ring focus
#[tokio::test]
async fn scenario_the_agent_view_highlights_the_current_sessions_chip_without_a_ring_focus() {
    // @step Given the App is in the Agent view with 3 open sessions and session s-2 is the store's current session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    app.agent_view_store_mut().focus_session_index(1);
    enter_agent_view(&mut app, "s-2").await;
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the App must be in the Agent view"
    );
    assert_eq!(
        app.agent_view_store().current_session_index(),
        1,
        "s-2 must be the store's current session"
    );
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "the agent bar must start with no ring focus"
    );

    // @step When the Agent view's menu bar renders with no ring focus
    let buf = render_app(&mut app);
    let y = agent_bar_row(&buf);
    let x2 = chip_x(&buf, y, 2);

    // @step Then chip #2's cells are styled bg Cyan fg Black and chip #1's and chip #3's cells keep the #333333 background
    for cx in chip_cell_range(x2) {
        assert_eq!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #2 cell {cx} must be inverse (the Agent view's \
             current-session highlight — MENU-010 R1)"
        );
    }
    for n in [1usize, 3] {
        let x = chip_x(&buf, y, n);
        for cx in chip_cell_range(x) {
            assert_ne!(
                buf[(cx, y)].bg,
                Color::Cyan,
                "chip #{n} cell {cx} must keep the #333333 background"
            );
        }
    }
}

/// Scenario: The ring selector highlights a chip on the Agent view
#[tokio::test]
async fn scenario_the_ring_selector_highlights_a_chip_on_the_agent_view() {
    // @step Given the App is in the Agent view with 3 open sessions, session s-2 is the store's current session, and the ring selector is on chip #1
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    app.agent_view_store_mut().focus_session_index(1);
    enter_agent_view(&mut app, "s-2").await;
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // Paint a frame first: the agent's ring math reads its chip count
    // from the last focused render (the bug197 parity note) — without a
    // painted frame the ring has no chips yet.
    let _ = render_app(&mut app);
    // Enter the bar (bare Left on the empty input → Item(0), MENU-003
    // R3), then Right onto chip #1 (the single 'Board View' item's ring
    // successor).
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the bare Left must enter the bar at the 'Board View' item"
    );
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Right from the single item must land on chip #1"
    );

    // @step When the Agent view's menu bar renders
    let buf = render_app(&mut app);
    let y = agent_bar_row(&buf);
    let x1 = chip_x(&buf, y, 1);
    let x2 = chip_x(&buf, y, 2);
    let x3 = chip_x(&buf, y, 3);

    // @step Then chip #1's cells are styled bg Cyan fg Black (ring selector) and chip #2's cells are styled bg Cyan fg Black (the current session's chip) while chip #3's cells keep the #333333 background
    for cx in chip_cell_range(x1) {
        assert_eq!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #1 cell {cx} must be inverse (the ring selector)"
        );
    }
    for cx in chip_cell_range(x2) {
        assert_eq!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #2 cell {cx} must be inverse (the current session's chip)"
        );
    }
    for cx in chip_cell_range(x3) {
        assert_ne!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #3 cell {cx} must keep the #333333 background"
        );
    }
}

/// Scenario: Mux: the focused Agent pane's session chip is highlighted
#[tokio::test]
async fn scenario_mux_the_focused_agent_panes_session_chip_is_highlighted() {
    // @step Given the mux surface has panes Board and Agent with 2 open sessions, and the Agent pane is focused showing session s-1
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    focus_pane(&mut app, 1);
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the Agent pane must be focused"
    );
    assert!(
        app.navigator().mux.menu_focus().is_none(),
        "the mux bar must start with no ring focus"
    );
    // The store's current session is s-2 (the last appended) — the
    // scenario's "leak" case: it must NOT drive the mux bar highlight.
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2")),
        "the store's current session must still be s-2 (the leak guard)"
    );
    // The agent window is synced onto the live session list during the
    // first render (render.rs `sync_window`).
    let buf = render_app_sized(&mut app, 240, 24);
    assert_eq!(
        app.navigator().mux.focused_session_id(),
        Some(SessionId::new("s-1")),
        "the focused Agent pane must show session s-1"
    );

    // @step When the mux bar renders with no ring focus
    let y = 0u16; // menu004 parity: the mux bar row is row 0.
    let x1 = chip_x(&buf, y, 1);
    let x2 = chip_x(&buf, y, 2);

    // @step Then chip #1's cells (the focused Agent pane's session) are styled bg Cyan fg Black and chip #2's cells keep the #333333 background
    for cx in chip_cell_range(x1) {
        assert_eq!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #1 cell {cx} must be inverse (agent-pane parity)"
        );
    }
    for cx in chip_cell_range(x2) {
        assert_ne!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #2 (the store's current session) must NOT be \
             highlighted while the Agent pane shows s-1"
        );
    }
}

/// Scenario: Mux: no chip is highlighted while a non-Agent pane is focused
#[tokio::test]
async fn scenario_mux_no_chip_is_highlighted_while_a_non_agent_pane_is_focused() {
    // @step Given the mux surface has panes Board and Agent with 2 open sessions, and the Board pane is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    focus_pane(&mut app, 0);
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused"
    );
    assert!(
        app.navigator().mux.focused_session_id().is_none(),
        "a non-Agent focused pane must yield no focused session"
    );
    assert!(
        app.navigator().mux.menu_focus().is_none(),
        "the mux bar must start with no ring focus"
    );
    // The store's current session is s-2 — the "stale" chip the board
    // rule forbids; it must not leak into the mux bar.
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2")),
        "the store's current session must still be s-2 (the leak guard)"
    );

    // @step When the mux bar renders with no ring focus
    let buf = render_app_sized(&mut app, 240, 24);
    let y = 0u16;
    let x1 = chip_x(&buf, y, 1);
    let x2 = chip_x(&buf, y, 2);

    // @step Then chip #1's and chip #2's cells keep the #333333 background
    for cx in chip_cell_range(x1) {
        assert_ne!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #1 cell {cx} must keep the #333333 background (no Agent \
             pane focused)"
        );
    }
    for cx in chip_cell_range(x2) {
        assert_ne!(
            buf[(cx, y)].bg,
            Color::Cyan,
            "chip #2 (the store's current session) must keep the \
             #333333 background — no stale highlight"
        );
    }
}
