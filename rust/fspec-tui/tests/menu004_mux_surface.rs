//! MENU-004 — Mux surface: single top-of-mux menu bar, per-pane suppression,
//! spinner draw tick.
//!
//! Feature: spec/features/mux-surface-single-top-of-mux-menu-bar-per-pane-suppression-spinner-draw-tick.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: App + MockBackend (the `menu002_board_surface.rs` pattern) —
//! `fresh_app` + `drain_pending` + full-App render into a TestBackend at
//! 240x24 (wide enough for the board pane to paint its 7-column grid).
//!
//! Observation points:
//! - mux bar state: `app.navigator().mux.menu_focus()` + `.open_menu()`
//! - the live bar: inverse-video (bg Cyan) cells on row 0 (the top row)
//! - pane rects: `app.navigator().mux.pane_rects()` (y == 1 when bar present)
//! - chip activation: `app.agent_view_store().current_session()`
//! - tick gate: `app.navigator().is_menu_bar_animating()`
//!
//! Geometry (240x24, mux active, bar present):
//!   y0    mux menu bar (2-zone: Kanban Tools Settings Help │ Board #1 #2 …)
//!   y1-22 panes body (22 rows)
//!   y23   mux footer
//!
//! Bar content geometry (inside the 240x24 buffer, ASCII cells):
//!   content starts at x=1 (1-cell bar padding):
//!   "Kanban" x1-6, "Tools" x8-12, "Settings" x14-21, "Help" x23-26
//!   (MENU-008: four Zone A items), dim `│` separator at x29,
//!   Zone B from x31: "Board" x31-35 (5 chars), 2-cell gap,
//!   "#1" x38-41, 2-cell gap, "#2" x44-47, … (use `find_x` for
//!   robustness — do NOT hardcode).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::MenuFocus, App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind,
    ViewMode,
};
use codelet_rpc_types::{SessionId, SessionStatus, WorkUnitInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
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

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn click(col: u16, row: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), col, row)
}

/// Render the full App into a 240x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    render_app_sized(app, 240, 24)
}

/// Render the full App into a `w`×`h` buffer.
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

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Find the x position (CELL offset) of `needle` in row `y` (returns
/// `None` if not found). `row.find` yields a BYTE offset; the wide
/// glyphs on the bar row (the dim `│` separator, the braille/glyph
/// chip marks) are 3 bytes each but 1 cell, so the byte offset is
/// mapped back to a cell index (the needle itself is always ASCII, so
/// the match starts on a char boundary).
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_off = row.find(needle)?;
    let mut byte = 0usize;
    for (cell, ch) in row.chars().enumerate() {
        if byte == byte_off {
            return Some(cell as u16);
        }
        byte += ch.len_utf8();
    }
    None
}

/// The mux bar row (always row 0 when the bar is painted).
fn bar_row(_buf: &Buffer) -> u16 {
    0
}

/// Enable mux with the given pane list (horizontal, equal splits, focus
/// pane 0). BUG-182 R7: entering mux triggers the initial load of every
/// rendered lazy pane (Files / Checkpoints) — the App's
/// `mux_load_lazy_panes` does exactly this for the `/mux` subcommand
/// and MuxConfigDialog paths; dispatch the equivalent open action here
/// so the test-side entry mirrors the production entry.
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let n = panes.len();
    let config = MuxConfig {
        orientation: MuxOrientation::Horizontal,
        splits: vec![50; n.saturating_sub(1)],
        panes: panes.clone(),
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
    // BUG-182 R7 parity: initial loads for the rendered lazy panes —
    // IN PLACE (no whole-view flip, the `/mux` apply path's
    // `mux_load_lazy_panes` semantics). The `OpenChangedFilesView` /
    // `OpenCheckpointsView` bus actions must NOT be dispatched here:
    // their `apply_action` arms flip `active_view` out of Mux to the
    // single lazy view (the mux entry guards against exactly that).
    // Resetting the owned view + dispatching the *Loaded actions drives
    // the same load tracker without the flip.
    if panes.contains(&MuxPaneKind::ChangedFiles) {
        app.navigator_mut().changed_files = codelet_fspec_tui::views::ChangedFilesView::new();
        app.dispatch(codelet_fspec_tui::Action::ChangedFilesLoaded(Vec::new()));
    }
    if panes.contains(&MuxPaneKind::Checkpoints) {
        app.navigator_mut().checkpoints = codelet_fspec_tui::views::CheckpointsView::new();
        app.dispatch(codelet_fspec_tui::Action::CheckpointsLoaded(Vec::new()));
    }
}

/// Focus a specific mux pane index.
fn focus_pane(app: &mut App, idx: usize) {
    app.navigator_mut().mux.set_focus(idx);
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

/// Set a session's status.
fn set_status(app: &mut App, sid: &str, status: SessionStatus) {
    app.dispatch(codelet_fspec_tui::Action::SessionStatusChanged(
        SessionId::new(sid),
        status,
    ));
}

/// Seed a small board so the board pane has content.
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

/// Arrange: the board pane is focused with the cursor on the last column
/// (blocked) — the ring's edge just before the mux bar.
fn focus_last_column(app: &mut App) {
    app.board_store_mut().set_focused_column("blocked");
}

/// Poll until a lazy view's (Files / Checkpoints) load tracker is idle.
/// BUG-182 R7: mux entry triggers the initial load of the rendered lazy
/// panes; BUG-183 R2: Esc only closes the pane once the load has
/// flushed (a reflex Esc mid-load stays Ignored).
async fn wait_for_view_idle(app: &mut App, changed_files: bool) {
    for _ in 0..100 {
        drain_pending(app).await;
        let idle = if changed_files {
            !app.navigator().changed_files.load.is_loading()
        } else {
            !app.navigator().checkpoints.load.is_loading()
        };
        if idle {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("timeout: the lazy view did not finish loading");
}

/// Arrange: the mux bar is focused on the first item (Kanban) via the
/// board ring entry (Right off the last column).
async fn focus_bar_first_item(app: &mut App) {
    focus_last_column(app);
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(app).await;
}

/// Arrange: open the `item_index`-th menu item's dropdown with the
/// cursor on `cursor_row` (0 = Kanban, 1 = Tools, 2 = Settings,
/// 3 = Help — MENU-008 four Zone A items).
async fn open_category_dropdown_at(app: &mut App, item_index: usize, cursor_row: usize) {
    focus_bar_first_item(app).await;
    for _ in 1..=item_index {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(app).await;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(app).await;
    for _ in 0..cursor_row {
        app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    }
    drain_pending(app).await;
}

/// Arrange: open the Kanban dropdown (item 0) with the cursor on
/// `cursor_row`.
async fn open_kanban_dropdown_at(app: &mut App, cursor_row: usize) {
    open_category_dropdown_at(app, 0, cursor_row).await;
}

/// Arrange: open the Help dropdown (item 3) with the cursor on
/// `cursor_row`.
async fn open_help_dropdown_at(app: &mut App, cursor_row: usize) {
    open_category_dropdown_at(app, 3, cursor_row).await;
}

/// The y of the dropdown row whose text contains `needle`.
fn dropdown_row_y(buf: &Buffer, needle: &str) -> u16 {
    let bar = bar_row(buf);
    (bar + 2..buf.area.height)
        .find(|&y| row_text(buf, y).contains(needle))
        .unwrap_or_else(|| panic!("dropdown row '{needle}' not found below row {bar}"))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The mux top row paints the 2-zone menu bar full width
#[tokio::test]
async fn scenario_the_mux_top_row_paints_the_2_zone_menu_bar_full_width() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions (one Running, one Idle)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    set_status(&mut app, "s-1", SessionStatus::Running);
    drain_pending(&mut app).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let row = row_text(&buf, y);

    // @step Then the top row shows the menu items (Kanban Tools Settings Help) a separator the pane label 'Board' and the two session chips (#1 with the running glyph #2 with the idle glyph)
    assert!(
        row.contains("Kanban") && row.contains("Help"),
        "the menu items must paint on row {y}:\n{row}"
    );
    assert!(
        row.contains("│"),
        "the dim separator must separate the zones:\n{row}"
    );
    assert!(
        row.contains("Board"),
        "the Board pane label must paint on the bar row:\n{row}"
    );
    assert!(row.contains("#1"), "chip #1 must paint (Running): \n{row}");
    assert!(row.contains("#2"), "chip #2 must paint (Idle):\n{row}");
    // The Running chip's glyph is magenta; verify the cell AFTER the
    // "#1 " prefix carries the magenta fg (the braille dot).
    let chip1_x = find_x(&buf, y, "#1").expect("chip #1 must be on the bar row");
    // The chip cell is "#1 " (prefix) + WU id (when painted) + glyph:
    // scan forward for the cell the chip prefix is followed by.
    let glyph_x = chip1_x + 3;
    assert_eq!(
        buf[(glyph_x, y)].fg,
        Color::Magenta,
        "the Running chip's glyph must be magenta (chip #1 at x {chip1_x})"
    );

    // @step And the panes paint within the body below the bar row (each pane body starts one row lower than before the bar existed)
    let pane_rects = app.navigator().mux.pane_rects();
    assert!(
        !pane_rects.is_empty(),
        "pane rects must be cached after render"
    );
    for rect in pane_rects {
        assert_eq!(
            rect.y, 1,
            "each pane body must start at row 1 (below the bar row)"
        );
    }

    // @step And the mux footer still occupies the bottom row
    let footer_y = buf.area.height - 1;
    assert!(
        row_text(&buf, footer_y).contains("MUX"),
        "the mux footer must still occupy the bottom row (y {footer_y})"
    );
}

/// Scenario: The focused pane label is active and the chips use the global session list
#[tokio::test]
async fn scenario_the_focused_pane_label_is_active_and_the_chips_use_the_global_session_list() {
    // @step Given the mux grid is [Board | Agent | Agent] with 3 open sessions (the focused pane is the Board)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    focus_pane(&mut app, 0); // Board pane focused

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let row = row_text(&buf, y);

    // @step Then the top row shows 'Board' active and the chips '#1' '#2' '#3' in global open-session order (the window is irrelevant — chips are global)
    assert!(
        row.contains("Board"),
        "the Board pane label must paint:\n{row}"
    );
    // Active pane label is fg Cyan + bold.
    let board_x = find_x(&buf, y, "Board").expect("Board label must be on the bar row");
    assert_eq!(
        buf[(board_x, y)].fg,
        Color::Cyan,
        "the focused pane label must be Cyan (active)"
    );
    assert!(
        row.contains("#1") && row.contains("#2") && row.contains("#3"),
        "all three global chips must paint:\n{row}"
    );

    // @step And Cleared sessions produce no chip
    // Seed a 4th session and mark it Cleared.
    app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
        "s-4",
    )));
    set_status(&mut app, "s-4", SessionStatus::Cleared);
    drain_pending(&mut app).await;
    let buf2 = render_app(&mut app);
    let row2 = row_text(&buf2, bar_row(&buf2));
    assert!(
        !row2.contains("#4"),
        "a Cleared session must NOT produce a chip:\n{row2}"
    );
}

/// Scenario: The bar omits the separator and chips when no sessions are open
#[tokio::test]
async fn scenario_the_bar_omits_the_separator_and_chips_when_no_sessions_are_open() {
    // @step Given the mux grid is [Board | Files | Ckpts] with no open sessions
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
    );

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let row = row_text(&buf, y);

    // @step Then the top row shows 'Kanban Tools Settings Help' followed by the pane labels 'Board Files Ckpts' (focused pane bold)
    assert!(
        row.contains("Kanban") && row.contains("Help"),
        "the menu items must paint:\n{row}"
    );
    assert!(
        row.contains("Board") && row.contains("Files") && row.contains("Ckpts"),
        "all three pane view labels must paint:\n{row}"
    );

    // @step And no chips and no extra separator run appear
    assert!(
        !row.contains("#1"),
        "no chips must paint without sessions:\n{row}"
    );
    // The bar's own `│` separator (zone divider) is present, but there
    // should be no chip separator. The box-border `│` characters from
    // the panes are NOT on row 0 (the bar row spans the full width with
    // its own #333333 bg). Count `│` on the bar row: exactly 1 (the zone
    // separator between Items and Zone B pane labels).
    let sep_count = row.matches('│').count();
    assert_eq!(
        sep_count, 1,
        "exactly one zone separator on the bar row (no extra separator run):\n{row}"
    );
}

/// Scenario: The board pane's header row 3 is blank in mux
#[tokio::test]
async fn scenario_the_board_pane_header_row_3_is_blank_in_mux() {
    // @step Given the mux grid is [Board | Agent] with 1 open session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);

    // @step When the mux view renders
    let buf = render_app(&mut app);

    // @step Then the board pane's 4-row header strip still renders (logo checkpoint status divider) but its row 3 is blank (no per-pane menu bar)
    // The board pane starts at y=1 (body row 0). Its header strip is 4 rows
    // (y1..y4). Row 3 of the header (y=1+3=4) must NOT contain "Kanban".
    let board_bar_row = 4u16; // pane y=1 + header row 3 (0-indexed)
    let board_row = row_text(&buf, board_bar_row);
    assert!(
        !board_row.contains("Kanban"),
        "the board pane's header row 3 must be blank (no per-pane bar):\n{board_row}"
    );

    // @step And exactly ONE menu bar is on screen (the mux top row)
    let text = buf_text(&buf);
    let kanban_count = text.matches("Kanban").count();
    assert_eq!(
        kanban_count, 1,
        "exactly one 'Kanban' on screen (the mux top row):\n{text}"
    );
}

/// Scenario: The agent panes paint no per-pane menu bar in mux
#[tokio::test]
async fn scenario_the_agent_panes_paint_no_per_pane_menu_bar_in_mux() {
    // @step Given the mux grid is [Agent | Agent] with 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);

    // @step When the mux view renders
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let row = row_text(&buf, y);

    // @step Then each agent pane paints its 5-row chrome (header role scrollback footer input) with no menu bar row
    // The agent panes' rows do NOT contain the Zone A items (the per-pane
    // bar is suppressed).
    // Check rows within the agent pane areas (y >= 1).
    for pane_y in 1..buf.area.height.saturating_sub(1) {
        let pane_row = row_text(&buf, pane_y);
        assert!(
            !pane_row.contains("Kanban Tools"),
            "no agent pane may paint its own menu bar (row {pane_y}):\n{pane_row}"
        );
    }

    // @step And the only menu bar on screen is the mux top row
    assert!(
        row.contains("Kanban"),
        "the mux top row must paint the bar:\n{row}"
    );
    let text = buf_text(&buf);
    let kanban_count = text.matches("Kanban Tools").count();
    assert_eq!(
        kanban_count, 1,
        "exactly one 'Kanban Tools' Zone A run on screen (the mux top row)"
    );
}

/// Scenario: The board column ring continues into the mux bar
#[tokio::test]
async fn scenario_the_board_column_ring_continues_into_the_mux_bar() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused with the cursor on the last column
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    focus_last_column(&mut app);

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the 'Kanban' menu item in the top bar (leaving the columns)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "Right off the last column must land on the first mux bar item (Kanban)"
    );

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on 'Tools'
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(1)),
        "Right from Kanban must land on Tools"
    );

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on 'Settings'
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(2)),
        "Right from Tools must land on Settings"
    );

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on 'Help'
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(3)),
        "Right from Settings must land on Help"
    );

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the 'Board' pane label
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Right from Help must land on the Board pane label (first Zone B cell)"
    );

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on chip #1
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "Right from the Board label must land on chip #1"
    );
}

/// Scenario: The mux bar ring wraps in both directions
#[tokio::test]
async fn scenario_the_mux_bar_ring_wraps_in_both_directions() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the Board pane is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    focus_last_column(&mut app);

    // @step When I walk the ring Right until it wraps past the last chip
    // Enter the bar (Right off last column), then walk 6 more Rights
    // (Kanban→Tools→Settings→Help→Board→#1→#2) to reach the last chip,
    // then one more Right wraps past the last chip back to Kanban.
    for _ in 0..8 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;

    // @step Then the highlight returns to the 'Kanban' menu item
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "Right past the last chip must wrap back to Kanban"
    );

    // @step When I press Left from 'Kanban'
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight wraps to the last chip
    // Zone B for [Board | Agent | Agent] with 2 sessions:
    //   [View("Board"), Chip(0), Chip(1)] → last cell = ZoneB(2)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "Left from Kanban must wrap to the last chip"
    );
}

/// Scenario: The agent pane's empty-input Left enters the mux bar
#[tokio::test]
async fn scenario_the_agent_pane_empty_input_left_enters_the_mux_bar() {
    // @step Given the mux grid is [Agent | Agent] with 2 open sessions and the Agent pane is focused with an empty input
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);
    focus_pane(&mut app, 0); // first Agent pane

    // @step When I press bare Left
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight lands on the 'Kanban' menu item in the top bar
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "bare Left on an empty agent input must enter the mux bar at Kanban"
    );
}

/// Scenario: A character key while the bar is focused dismisses it and types into the focused pane input
#[tokio::test]
async fn scenario_a_character_key_while_the_bar_is_focused_dismisses_it_and_types_into_the_focused_pane_input(
) {
    // @step Given the mux grid is [Agent | Agent] with 2 open sessions the Agent pane is focused and the bar is focused (dropdown closed)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);
    focus_pane(&mut app, 0);
    // Enter the bar via bare Left on empty input.
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the bar must be focused before the character key"
    );

    // @step When I type 'x'
    app.handle_event(&key(KeyCode::Char('x'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the bar highlight clears AND 'x' is typed into that pane's input on the same keystroke
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar focus must clear after the character key"
    );
    assert_eq!(
        app.navigator().agent.input.value(),
        "x",
        "the character must be typed into the focused pane's input"
    );
}

/// Scenario: Up and Down from a closed bar drop focus back into the focused pane input
#[tokio::test]
async fn scenario_up_and_down_from_a_closed_bar_drop_focus_back_into_the_focused_pane_input() {
    // @step Given the mux grid is [Agent | Agent] with 2 open sessions the Agent pane is focused and the bar is focused (dropdown closed) with a draft 'abc' in its input
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);
    // BUG-163 lockstep: the focused agent pane hosts the store's CURRENT
    // session (pane 1 → s-2, the last seeded). Focusing pane 0 instead
    // would leave the live composer on s-1, and the first event's
    // `sync_mux_focus_to_session` would round-trip the draft away from
    // s-1's empty slot — the pane's window session must be the current
    // one for the Given to hold.
    focus_pane(&mut app, 1);
    // The Given state (bar focused AND a draft present) is reached via
    // the MOUSE entry path (the keyboard entry requires an empty input,
    // and a character key would dismiss the bar on the same keystroke).
    app.navigator_mut().agent.input.set_value("abc");
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);
    let kanban_x = find_x(&buf, 0, "Kanban").expect("Kanban must be on the bar row");
    // The item click focuses AND toggles its dropdown open (R-MOUSE) —
    // close it with Esc (the item STAYS focused) to reach the Given:
    // bar focused, dropdown closed.
    app.handle_event(&click(kanban_x, 0));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the bar must be focused (via mouse click on Kanban, dropdown Esc-closed)"
    );
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "the dropdown must be closed (the Given: bar focused, dropdown closed)"
    );
    assert_eq!(
        app.navigator().agent.input.value(),
        "abc",
        "the input draft must be 'abc'"
    );

    // @step When I press Up
    app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the bar highlight clears and the pane input keeps its draft 'abc' (cursor active)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "Up from a closed bar must clear the bar focus"
    );
    assert_eq!(
        app.navigator().agent.input.value(),
        "abc",
        "the input draft must be preserved"
    );
}

/// Scenario: The Files pane does not feed the mux bar
#[tokio::test]
async fn scenario_the_files_pane_does_not_feed_the_mux_bar() {
    // @step Given the mux grid is [Board | Files] and the Files pane is focused
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::ChangedFiles],
    );
    focus_pane(&mut app, 1); // Files pane

    // @step When I press Left or Right
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the file list scrolls as before and the highlight never moves onto the top menu bar
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "Files pane Left/Right must NOT enter the mux bar"
    );

    // BUG-182 R7 / BUG-183 R2: the Files pane's initial load is spawned
    // on mux entry and Esc only closes once it has flushed.
    wait_for_view_idle(&mut app, true).await;

    // @step When I press Esc
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Files pane closes (the bar is untouched and stays reachable by mouse)
    // BUG-183: Esc on the focused Files pane closes it (removes it from
    // the live grid). The bar remains (it's on the mux layout, not the pane).
    let panes = app.navigator().mux.effective_panes();
    assert!(
        !panes.contains(&MuxPaneKind::ChangedFiles),
        "the Files pane must close on Esc"
    );
}

/// Scenario: Enter on a menu item opens its dropdown at row 0
#[tokio::test]
async fn scenario_enter_on_a_menu_item_opens_its_dropdown_at_row_0() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions the Board pane is focused and the 'Kanban' item has the bar highlight
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    focus_bar_first_item(&mut app).await;

    // @step When I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dropdown opens under 'Kanban' with the cursor on row 0 ('. New Agent') painting over the panes below
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 0)),
        "the Kanban dropdown must be open at row 0"
    );
    let buf = render_app(&mut app);
    let row0_y = bar_row(&buf) + 2; // panel border + first entry row
    assert!(
        row_text(&buf, row0_y).contains("New Agent"),
        "the panel's first row must be 'New Agent':\n{}",
        row_text(&buf, row0_y)
    );
}

/// Scenario: Up and Down move the dropdown cursor wrapping at the ends
#[tokio::test]
async fn scenario_up_and_down_move_the_dropdown_cursor_wrapping_at_the_ends() {
    // @step Given the mux 'Kanban' dropdown is open with the cursor on row 0
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_kanban_dropdown_at(&mut app, 0).await;
    assert_eq!(app.navigator().mux.open_menu(), Some((0, 0)));

    // @step When I press Down
    app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the cursor moves to row 1 ('/ Search')
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 1)),
        "Down must move the cursor to row 1 (Search)"
    );

    // @step When I press Down past the last row
    // Kanban has 4 entries (rows 0..3). From row 1, 3 more Downs wrap.
    for _ in 0..3 {
        app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;

    // @step Then the cursor wraps to row 0
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 0)),
        "Down past the last row must wrap to row 0"
    );
}

/// Scenario: While a dropdown is open Left and Right walk the ring and re-anchor
#[tokio::test]
async fn scenario_while_a_dropdown_is_open_left_and_right_walk_the_ring_and_reanchor() {
    // @step Given the mux 'Kanban' dropdown is open (cursor row 0) and 'Kanban' has the bar highlight
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_kanban_dropdown_at(&mut app, 0).await;

    // @step When I press Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the highlight moves to 'Tools' and the open dropdown re-anchors under 'Tools' at row 0
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(1)),
        "Right must walk to Tools"
    );
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((1, 0)),
        "the dropdown must re-anchor under Tools at row 0"
    );
}

/// Scenario: Enter on an open dropdown executes the highlighted row and closes the dropdown
#[tokio::test]
async fn scenario_enter_on_an_open_dropdown_executes_the_highlighted_row_and_closes_the_dropdown() {
    // @step Given the mux 'Tools' dropdown is open with the cursor on the 'C Checkpoints' row
    // Tools entries: 0=Changed Files, 1=Checkpoints.
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_category_dropdown_at(&mut app, 1, 1).await;
    assert_eq!(app.navigator().mux.open_menu(), Some((1, 1)));

    // @step When I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the checkpoints action is dispatched (the existing App routing runs unchanged) and the dropdown closes
    // In mux mode, executing 'C Checkpoints' opens the checkpoints view.
    // The existing App routing for OpenCheckpointsView in mux mode
    // focuses the checkpoints pane (or opens it if not present).
    // The dropdown must close regardless.
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "the dropdown must close after execute"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "the bar focus must clear after execute"
    );
}

/// Scenario: Esc closes the open dropdown and keeps the item focused
#[tokio::test]
async fn scenario_esc_closes_the_open_dropdown_and_keeps_the_item_focused() {
    // @step Given the mux 'Kanban' dropdown is open and 'Kanban' has the bar highlight
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_kanban_dropdown_at(&mut app, 0).await;
    assert_eq!(app.navigator().mux.open_menu(), Some((0, 0)));

    // @step When I press Esc
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dropdown closes and 'Kanban' stays highlighted
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "the dropdown must close"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must stay focused"
    );
}

/// Scenario: While a dropdown is open all other keys are swallowed
#[tokio::test]
async fn scenario_while_a_dropdown_is_open_all_other_keys_are_swallowed() {
    // @step Given the mux 'Kanban' dropdown is open
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_kanban_dropdown_at(&mut app, 3).await;

    // @step When I press 'a' (not a menu entry shortcut)
    app.handle_event(&key(KeyCode::Char('a'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then nothing is typed and nothing is opened (the key is swallowed — true-modal, BUG-161 parity)
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 3)),
        "the dropdown must stay open (key swallowed)"
    );
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "nothing must be typed into the input"
    );
    // The agent pane's input must not have received the 'a'.
    let buf = render_app(&mut app);
    assert!(
        row_text(&buf, bar_row(&buf)).contains("Kanban"),
        "the bar must still be present"
    );
}

/// Scenario: Enter on a chip focuses that session without flipping out of mux
#[tokio::test]
async fn scenario_enter_on_a_chip_focuses_that_session_without_flipping_out_of_mux() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and the chip '#2' has the bar highlight
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    // Navigate to chip #2: Kanban → Tools → Settings → Help → Board → #1 → #2
    // (6 Rights from Kanban)
    focus_bar_first_item(&mut app).await;
    for _ in 0..6 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "the bar must be focused on chip #2 (ZoneB(2))"
    );

    // @step When I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then session #2 is focused in the agent store and the view stays in Mux (no single-view flip)
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the view must stay in Mux (no flip to Agent)"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&SessionId::new("s-2")),
        "session #2 must be focused in the agent store"
    );
}

/// Scenario: Enter on a pane view label focuses that pane
#[tokio::test]
async fn scenario_enter_on_a_pane_view_label_focuses_that_pane() {
    // @step Given the mux grid is [Board | Files | Ckpts] and the 'Files' pane label has the bar highlight
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
    );
    // No sessions → Zone B = [View("Board"), View("Files"), View("Ckpts")]
    // Navigate: Kanban → Tools → Settings → Help → Board → Files
    // (5 Rights from Kanban)
    focus_bar_first_item(&mut app).await;
    for _ in 0..5 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "the bar must be focused on the Files pane label (ZoneB(1))"
    );

    // @step When I press Enter
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Files pane is focused (set_focus)
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the Files pane (index 1) must be focused"
    );
}

/// Scenario: Clicking a menu item focuses it and opens its dropdown
#[tokio::test]
async fn scenario_clicking_a_menu_item_focuses_it_and_opens_its_dropdown() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    let buf = render_app(&mut app);
    let help_x = find_x(&buf, 0, "Help").expect("Help must be on the bar row");

    // @step When I click the 'Help' menu item in the top bar
    app.handle_event(&click(help_x, 0));
    drain_pending(&mut app).await;

    // @step Then 'Help' is focused and its dropdown is open
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(3)),
        "the Help item must be focused (MENU-008: 4th Zone A item)"
    );
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((3, 0)),
        "the Help dropdown must be open at row 0"
    );
}

/// Scenario: Clicking the already open item closes the dropdown
#[tokio::test]
async fn scenario_clicking_the_already_open_item_closes_the_dropdown() {
    // @step Given the mux 'Help' dropdown is open (Help item focused)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_help_dropdown_at(&mut app, 0).await;
    assert_eq!(app.navigator().mux.open_menu(), Some((3, 0)));
    let buf = render_app(&mut app);
    let help_x = find_x(&buf, 0, "Help").expect("Help must be on the bar row");

    // @step When I click the 'Help' item again
    app.handle_event(&click(help_x, 0));
    drain_pending(&mut app).await;

    // @step Then the dropdown closes
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "the dropdown must close on re-click"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(3)),
        "the Help item must stay focused"
    );
}

/// Scenario: Clicking a chip activates that session
#[tokio::test]
async fn scenario_clicking_a_chip_activates_that_session() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    let buf = render_app(&mut app);
    let chip2_x = find_x(&buf, 0, "#2").expect("chip #2 must be on the bar row");

    // @step When I click chip '#2' in the top bar
    app.handle_event(&click(chip2_x, 0));
    drain_pending(&mut app).await;

    // @step Then session #2 is focused and the view stays in Mux
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the view must stay in Mux"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&SessionId::new("s-2")),
        "session #2 must be focused"
    );
}

/// Scenario: Clicking a pane label focuses that pane
#[tokio::test]
async fn scenario_clicking_a_pane_label_focuses_that_pane() {
    // @step Given the mux grid is [Board | Files | Ckpts]
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
    );
    let buf = render_app(&mut app);
    let ckpts_x = find_x(&buf, 0, "Ckpts").expect("Ckpts label must be on the bar row");

    // @step When I click the 'Ckpts' pane label in the top bar
    app.handle_event(&click(ckpts_x, 0));
    drain_pending(&mut app).await;

    // @step Then the Checkpoints pane is focused
    assert_eq!(
        app.navigator().mux.focus(),
        2,
        "the Checkpoints pane (index 2) must be focused"
    );
}

/// Scenario: Clicking outside the dropdown closes it while the click still lands on the pane
#[tokio::test]
async fn scenario_clicking_outside_the_dropdown_closes_it_while_the_click_still_lands_on_the_pane()
{
    // @step Given the mux 'Kanban' dropdown is open
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![
            MuxPaneKind::Board,
            MuxPaneKind::ChangedFiles,
            MuxPaneKind::Checkpoints,
        ],
    );
    focus_bar_first_item(&mut app).await;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(app.navigator().mux.open_menu(), Some((0, 0)));
    // The opened dropdown must be on screen (the next frame paints the
    // panel and caches its rect) — the outside-click arm hit-tests
    // against that cached geometry.
    let _buf = render_app(&mut app);
    // The Files pane is pane index 1. At 240x24 with 3 panes the equal
    // split percent list is [50, 50]: pane 0 takes 50% (x0..118), pane 1
    // takes 50% (x120..237), the last pane absorbs the 1-cell remainder
    // (BUG-166 scale math) — click well inside the Files pane.
    let files_click_x = 150u16;
    let files_click_y = 10u16;

    // @step When I click the Files pane (outside the dropdown panel and off the bar items)
    app.handle_event(&click(files_click_x, files_click_y));
    drain_pending(&mut app).await;

    // @step Then the dropdown closes AND the Files pane is focused (the click lands on the same event)
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "the dropdown must close on the outside click"
    );
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the Files pane (index 1) must be focused"
    );
}

/// Scenario: Wheel left and right over the bar walk the ring
#[tokio::test]
async fn scenario_wheel_left_and_right_over_the_bar_walk_the_ring() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and a bar item is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    focus_bar_first_item(&mut app).await;
    assert_eq!(app.navigator().mux.menu_focus(), Some(MenuFocus::Item(0)));

    // @step When I wheel ScrollRight over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollRight, 50, 0));
    drain_pending(&mut app).await;

    // @step Then the ring focus advances one stop (like the Right key)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(1)),
        "wheel right must advance the ring to Tools"
    );

    // @step When I wheel ScrollLeft over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollLeft, 50, 0));
    drain_pending(&mut app).await;

    // @step Then the ring focus retreats one stop (like the Left key)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "wheel left must retreat the ring to Kanban"
    );
}

/// Scenario: Wheel up and down over an open dropdown move the cursor
#[tokio::test]
async fn scenario_wheel_up_and_down_over_an_open_dropdown_move_the_cursor() {
    // @step Given the mux 'Kanban' dropdown is open (cursor row 0)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    open_kanban_dropdown_at(&mut app, 0).await;
    let buf = render_app(&mut app);
    let search_y = dropdown_row_y(&buf, "Search");

    // @step When I wheel ScrollDown over the open dropdown panel
    app.handle_event(&mouse(MouseEventKind::ScrollDown, 30, search_y));
    drain_pending(&mut app).await;

    // @step Then the cursor moves to row 1
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 1)),
        "wheel down must move the cursor to row 1"
    );

    // @step When I wheel ScrollUp over the open dropdown panel
    let buf2 = render_app(&mut app);
    let search_y2 = dropdown_row_y(&buf2, "Search");
    app.handle_event(&mouse(MouseEventKind::ScrollUp, 30, search_y2));
    drain_pending(&mut app).await;

    // @step Then the cursor returns to row 0
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 0)),
        "wheel up must return the cursor to row 0"
    );
}

/// Scenario: Wheel over the bar with no bar focus and no open dropdown is ignored
#[tokio::test]
async fn scenario_wheel_over_the_bar_with_no_bar_focus_and_no_open_dropdown_is_ignored() {
    // @step Given the mux grid is [Board | Agent | Agent] with 2 open sessions and no bar focus and no dropdown open
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    assert_eq!(app.navigator().mux.menu_focus(), None);
    assert_eq!(app.navigator().mux.open_menu(), None);

    // @step When I wheel ScrollLeft or ScrollRight over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollLeft, 50, 0));
    drain_pending(&mut app).await;
    app.handle_event(&mouse(MouseEventKind::ScrollRight, 50, 0));
    drain_pending(&mut app).await;

    // @step Then nothing happens (the event is ignored — the bar is inert)
    assert_eq!(
        app.navigator().mux.menu_focus(),
        None,
        "wheel with no bar focus must be ignored"
    );
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "no dropdown must open"
    );
}

/// Scenario: Shift+Left and Shift+Right cycle the focused pane without touching the bar
#[tokio::test]
async fn scenario_shift_left_and_shift_right_cycle_the_focused_pane_without_touching_the_bar() {
    // @step Given the mux grid is [Board | Agent | Agent] and the Board pane is focused with the 'Kanban' item highlighted in the top bar
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    focus_bar_first_item(&mut app).await;
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must be highlighted"
    );
    assert_eq!(app.navigator().mux.focus(), 0);

    // @step When I press Shift+Right
    app.handle_event(&key(KeyCode::Right, KeyModifiers::SHIFT));
    drain_pending(&mut app).await;

    // @step Then the Agent pane is focused (pane focus moved) and the 'Kanban' bar highlight is unchanged (Shift+arrows never enter or move the bar)
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "Shift+Right must move pane focus to the Agent pane"
    );
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the bar highlight must stay on Kanban (Shift+arrows don't touch the bar)"
    );
}

/// Scenario: The draw tick stays open while a session is Running and a bar is painted
#[tokio::test]
async fn scenario_the_draw_tick_stays_open_while_a_session_is_running_and_a_bar_is_painted() {
    // @step Given the mux grid is [Agent | Agent] with one session Running
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    set_status(&mut app, "s-1", SessionStatus::Running);
    drain_pending(&mut app).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);
    // Force a render so the bar paints and the geometry is cached.
    let _buf = render_app(&mut app);

    // @step When the run loop evaluates the draw-tick gate
    let animating = app.navigator().is_menu_bar_animating();

    // @step Then the 'menu bar animating' operand is true (active surface paints a bar AND a session is Running/Compacting)
    assert!(
        animating,
        "is_menu_bar_animating must be true: mux view active + Running session"
    );

    // @step And the 16ms tick keeps redrawing so the Running chip's braille frame advances
    // (The run loop ORs this into tick_should_draw — verified by the
    // app/mod.rs tick_should_draw unit tests + this gate being true.)
}

/// Scenario: The draw tick closes when every session is idle
#[tokio::test]
async fn scenario_the_draw_tick_closes_when_every_session_is_idle() {
    // @step Given the mux grid is [Agent | Agent] with all sessions Idle
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    // All sessions are Idle by default (no status set).
    drain_pending(&mut app).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Agent, MuxPaneKind::Agent]);
    let _buf = render_app(&mut app);

    // @step When the run loop evaluates the draw-tick gate
    let animating = app.navigator().is_menu_bar_animating();

    // @step Then the 'menu bar animating' operand is false and the chip glyph sits still (no forced redraw)
    assert!(
        !animating,
        "is_menu_bar_animating must be false: all sessions idle"
    );
}

/// Scenario: A 3-row terminal does not paint the bar and keeps the legacy layout
#[tokio::test]
async fn scenario_a_3_row_terminal_does_not_paint_the_bar_and_keeps_the_legacy_layout() {
    // @step Given the mux grid is [Board | Agent] on a 3-row terminal (too short for bar + body + footer)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);

    // @step When the mux view renders
    let buf = render_app_sized(&mut app, 240, 3);

    // @step Then the top row does not paint a menu bar
    let row0 = row_text(&buf, 0);
    assert!(
        !row0.contains("Kanban"),
        "the 3-row terminal must NOT paint the menu bar on row 0:\n{row0}"
    );

    // @step And the panes render with the pre-existing layout (old body height, same panes) with no crash or regression
    // The legacy layout uses the full 3 rows for body + footer (no bar row).
    // The pane rects should span the full area height (no top-row reservation).
    let pane_rects = app.navigator().mux.pane_rects();
    assert!(!pane_rects.is_empty(), "pane rects must exist");
    for rect in pane_rects {
        assert_eq!(
            rect.y, 0,
            "with no bar, panes must start at row 0 (legacy layout)"
        );
    }
    // The footer is on the last row (y=2).
    let footer_y = buf.area.height - 1;
    assert!(
        row_text(&buf, footer_y).contains("MUX"),
        "the mux footer must still be on the bottom row"
    );
}
