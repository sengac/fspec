//! MENU-006 — the active session's chip keeps the selected-item blue background.
//!
//! Feature: spec/features/active-session-chip-paints-the-selected-item-blue-background.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! MENU-010 superseded two of the original four scenarios: the BOARD
//! scenario now pins the NO-highlight rule (the board never carries the
//! current-session chip highlight), and the MUX scenario moved to the
//! MENU-010 feature (`menu010_chip_highlight_scoped.rs` — the mux marks
//! only the FOCUSED Agent pane's session chip).
//!
//! Harnesses:
//! - App + MockBackend (the `bug195_agent_view_chip_click.rs` pattern)
//!   for the board / agent SURFACE scenarios — full-App render into a
//!   TestBackend at 120x24, chip x positions located by row scan.
//! - `paint_menu_bar` directly for the pure-painter scenarios (no ring
//!   focus; WU id whole-cell inverse), mirroring the `menu001` pattern.
//!
//! Observation points:
//! - the active-chip highlight: `buf[(x, y)].bg == Color::Cyan` (the
//!   same inverse style a selected Zone A item uses — `inverse_style`);
//! - inactivity: the bar row's `MENU_BAR_BG` (#333333) elsewhere;
//! - ring state: `app.board_store().menu_focus()` /
//!   `app.navigator().agent.menu_focus()`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::{paint_menu_bar, MenuChip, MenuSnapshot, ZoneBCell};
use codelet_fspec_tui::store::AgentViewStore;
use codelet_fspec_tui::views::multiplex::{menu_snapshot, MultiplexLayout, MuxPaneKind};
use codelet_fspec_tui::{App, FspecBackend, SessionContext, Theme, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
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

/// The x of chip `n` (1-based) on row `y`.
fn chip_x(buf: &Buffer, y: u16, n: usize) -> u16 {
    find_x(buf, y, &format!("#{n}"))
        .unwrap_or_else(|| panic!("chip #{n} must paint on row {y}:\n{}", row_text(buf, y)))
}

/// Seed `n` open sessions (s-1..s-n, Idle by default).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
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

/// A pure-painter chip (Idle glyph; `active` marks the current session).
fn chip(n: usize, wu: Option<&'static str>, active: bool) -> MenuChip {
    MenuChip {
        index: (n, 3),
        glyph: "●".to_string(),
        glyph_style: Default::default(),
        wu_id: wu.map(str::to_string),
        active,
    }
}

/// The pure-painter snapshot: `chips` in display order, chips-only Zone B.
fn snap(
    chips: Vec<MenuChip>,
    focus: Option<codelet_fspec_tui::components::menu_bar::MenuFocus>,
) -> MenuSnapshot {
    let zone_b = chips
        .iter()
        .enumerate()
        .map(|(i, _)| ZoneBCell::Chip(i))
        .collect();
    MenuSnapshot {
        zone_b,
        focus,
        chips,
        ..Default::default()
    }
}

/// Render a snapshot into a `w`-column row and return the buffer.
fn paint_row(
    snap: &MenuSnapshot,
    w: u16,
) -> (Buffer, codelet_fspec_tui::components::menu_bar::MenuLayout) {
    let area = Rect::new(0, 0, w, 1);
    let mut buf = Buffer::empty(area);
    let layout = paint_menu_bar(area, &mut buf, snap, &Theme::default())
        .expect("layout for a non-zero area");
    (buf, layout)
}

/// The y of the 2-zone bar row — the single row carrying a Zone A item
/// and the chip separator: the board/mux rows carry "Kanban" +
/// "Help" (MENU-008), while the agent view's row carries the single
/// "Board View" item (MENU-007).
fn bar_row(buf: &Buffer) -> u16 {
    (0..buf.area.height)
        .find(|&y| {
            let row = row_text(buf, y);
            let board_or_mux = row.contains("Kanban") && row.contains("Help") && row.contains('│');
            let agent = row.contains("Board View") && row.contains('│');
            board_or_mux || agent
        })
        .unwrap_or_else(|| panic!("the 2-zone bar row must render:\n{}", buf_text(buf)))
}

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// True when the cell carries the inverse-video highlight (bg Cyan).
fn is_inverse(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].bg == Color::Cyan
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The board view does not carry the current session's chip highlight (superseded by MENU-010)
#[tokio::test]
async fn scenario_the_board_view_does_not_carry_the_current_sessions_chip_highlight_superseded_by_menu_010(
) {
    // @step Given the board has 3 open sessions and session s-2 is the focused session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    app.agent_view_store_mut().focus_session_index(1);
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store().current_session_index(),
        1,
        "s-2 must be the focused open-session slot"
    );
    assert_eq!(
        app.board_store().menu_focus(),
        None,
        "no ring focus — the column owns the input"
    );

    // @step When the bar renders with the ring focus cleared
    let buf = render_app(&mut app);
    let y = bar_row(&buf);

    // @step Then no chip's cells are styled bg Cyan fg Black (the board never carries the current-session highlight — MENU-010)
    let x2 = chip_x(&buf, y, 2);
    for (x, label) in [(x2, "#2"), (x2 + 2, "#2"), (x2 + 3, "#2")] {
        assert_ne!(
            buf[(x, y)].bg,
            Color::Cyan,
            "chip cell {label}@{x} must NOT be highlighted (MENU-010 R2)"
        );
    }

    // @step And chips #1 and #3's cells keep the #333333 background
    let x1 = chip_x(&buf, y, 1);
    let x3 = chip_x(&buf, y, 3);
    for (x, label) in [
        (x1, "#1"),
        (x1 + 2, "#1"),
        (x1 + 3, "#1"),
        (x3, "#3"),
        (x3 + 3, "#3"),
    ] {
        assert_ne!(
            buf[(x, y)].bg,
            Color::Cyan,
            "chip cell {label}@{x} must NOT be highlighted"
        );
    }
}

/// Scenario: The active session's chip keeps its blue background in the Agent view after the ring focus clears
#[tokio::test]
async fn scenario_the_active_session_chip_keeps_its_blue_background_in_the_agent_view_after_the_ring_focus_clears(
) {
    // @step Given the App is in the Agent view with 3 open sessions and session s-2 is the focused session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    // `append_session` focuses the LAST appended session — pin s-2.
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
        "s-2 must be the focused open-session slot"
    );

    // @step When the menu bar ring focus is cleared
    // Enter the bar (bare Left on the empty input → Item(0)), then press
    // Up — R10 stage 2 drops the ring focus back into the input.
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(
        app.navigator().agent.menu_focus(),
        Some(codelet_fspec_tui::components::menu_bar::MenuFocus::Item(0)),
        "the bare Left must enter the bar"
    );
    app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(
        app.navigator().agent.menu_focus(),
        None,
        "Up must clear the bar ring focus"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);

    // @step Then chip #2's cells in the agent bar are styled bg Cyan fg Black
    let x2 = chip_x(&buf, y, 2);
    assert!(
        is_inverse(&buf, x2, y) && is_inverse(&buf, x2 + 3, y),
        "chip #2 must stay highlighted after the ring cleared:\n{}",
        row_text(&buf, y)
    );

    // @step And chip #1's and chip #3's cells keep the #333333 background
    let x1 = chip_x(&buf, y, 1);
    let x3 = chip_x(&buf, y, 3);
    for (x, label) in [(x1, "#1"), (x1 + 3, "#1"), (x3, "#3"), (x3 + 3, "#3")] {
        assert_ne!(
            buf[(x, y)].bg,
            Color::Cyan,
            "chip cell {label}@{x} must NOT be highlighted"
        );
    }
}

/// Scenario: A ring-focused active chip with a work-unit id paints the whole cell inverse
#[test]
fn scenario_a_ring_focused_active_chip_with_a_work_unit_id_paints_the_whole_cell_inverse() {
    // @step Given a MenuSnapshot with 3 open sessions and session s-2 is the focused session and chip #2 is bound to work unit "MENU-006" and the menu bar ring is focused on chip #2
    let snap = snap(
        vec![
            chip(1, None, false),
            chip(2, Some("MENU-006"), true),
            chip(3, None, false),
        ],
        Some(codelet_fspec_tui::components::menu_bar::MenuFocus::ZoneB(1)),
    );

    // @step When the bar is rendered into a 120-column row
    let (buf, layout) = paint_row(&snap, 120);
    assert_eq!(layout.level, 0, "no truncation at 120 cols");
    let rect = &layout.cell_rects[1];

    // @step Then chip #2's whole cell (prefix, work-unit id and glyph) is styled bg Cyan fg Black
    for cx in rect.x..rect.x + rect.width {
        assert!(
            is_inverse(&buf, cx, 0),
            "cell {cx} of the active chip must be inverse"
        );
        assert_eq!(
            buf[(cx, 0)].fg,
            Color::Black,
            "cell {cx} of the active chip must read black (no magenta/dim on top)"
        );
    }
}

/// Scenario: In mux mode the focused Agent pane's session chip paints the blue background while view labels keep their styling
#[test]
fn scenario_in_mux_mode_the_focused_agent_panes_session_chip_paints_the_blue_background_while_view_labels_keep_their_styling(
) {
    // @step Given a MenuSnapshot in mux mode with panes Board and Agent, 2 open sessions, and the Agent pane is focused showing session s-1
    let mut layout = MultiplexLayout::new();
    layout.set_pane_list(vec![MuxPaneKind::Board, MuxPaneKind::Agent], None);
    layout.set_focus(1);
    let mut store = AgentViewStore::default();
    store.open_sessions_mut().push(session_ctx("s-1"));
    store.open_sessions_mut().push(session_ctx("s-2"));
    // The agent window is synced onto the live session list the same
    // way the mux render path does (render.rs `sync_window`).
    let session_ids: Vec<SessionId> = store.open_sessions().iter().map(|c| c.id.clone()).collect();
    layout.sync_window(&session_ids);
    assert_eq!(
        layout.focused_session_id(),
        Some(SessionId::new("s-1")),
        "the focused Agent pane must show session s-1"
    );
    let snap = menu_snapshot::build_snapshot(&layout, &store, None, None, 0);
    assert!(
        snap.chips.iter().any(|c| c.active && c.index == (1, 2)),
        "the mux builder must flag the FOCUSED Agent pane's session chip \
         (s-1): {:?}",
        snap.chips
            .iter()
            .map(|c| (c.index, c.active))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        snap.chips.iter().filter(|c| c.active).count(),
        1,
        "exactly one chip (s-1) must carry the highlight — s-2 (the \
         store's current session) must NOT leak into the mux bar (MENU-010)"
    );

    // @step When the bar is rendered into a 120-column row
    let (buf, painted) = paint_row(&snap, 120);

    // @step Then chip #1's cells are styled bg Cyan fg Black
    let chip_cells: Vec<usize> = snap
        .zone_b
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, ZoneBCell::Chip(_)))
        .map(|(i, _)| i)
        .collect();
    let chip1 = painted
        .cell_rects
        .get(chip_cells[0])
        .expect("chip #1 cell rect");
    for cx in chip1.x..chip1.x + chip1.width {
        assert!(
            is_inverse(&buf, cx, 0),
            "cell {cx} of chip #1 must be inverse"
        );
    }

    // @step And chip #2's cells keep the #333333 background
    let chip2 = painted
        .cell_rects
        .get(chip_cells[1])
        .expect("chip #2 cell rect");
    for cx in chip2.x..chip2.x + chip2.width {
        assert_ne!(
            buf[(cx, 0)].bg,
            Color::Cyan,
            "chip #2 (the store's current session) cell {cx} must stay \
             unhighlighted"
        );
    }
    // the view label never carries the inverse highlight (no focus either)
    let board_label = snap
        .zone_b
        .iter()
        .position(|c| matches!(c, ZoneBCell::View { label: "Board", .. }))
        .expect("Board view label");
    let board_rect = &painted.cell_rects[board_label];
    for cx in board_rect.x..board_rect.x + board_rect.width {
        assert_ne!(
            buf[(cx, 0)].bg,
            Color::Cyan,
            "the Board view label cell {cx} must stay unhighlighted"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Test-only seams
// ─────────────────────────────────────────────────────────────────────────

fn session_ctx(id: &str) -> SessionContext {
    SessionContext::new(SessionId::new(id.to_string()))
}
