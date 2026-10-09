//! BUG-196 — menu dropdown: row click must execute the entry; click away
//! must close + de-select the bar.
//!
//! Feature: spec/features/menu-dropdown-row-click-must-execute-the-entry-click-away-must-close-de-select-the-bar.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: App + MockBackend (the `menu002_board_surface.rs` /
//! `menu004_mux_surface.rs` pattern) — `fresh_app` + `drain_pending` +
//! full-App render into a TestBackend (board at 120x24, mux at 240x24).
//!
//! Geometry (120x24, single Board view):
//!   y1-4  4-row header (row 0 `Checkpoints:…` at y1, the 2-zone menu
//!         bar at y4 — `bar_row(buf)`), "[ Kanban ]" item x15,
//!         "[ Help ]" x49 (MENU-011: bracketed Zone A items).
//!   The open dropdown's panel anchors at (item_x, bar+1): the Kanban
//!   panel is x15..56 (width 42), y5..y10 (6 rows: 4 entries + 2
//!   border rows) — entry rows y6..y9, top border row y5, bottom
//!   border row y10.
//!
//! Geometry (240x24, mux active, bar present):
//!   y0    mux menu bar (MENU-011: the Zone A items paint BRACKETED):
//!         "[ Kanban ]" x1-10, "[ Tools ]" x12-20, "[ Settings ]"
//!         x22-33, "[ Help ]" x35-42 (1-cell bar padding — content x1+).
//!   The open dropdown's panel anchors one row below the bar: the
//!   Kanban panel is x1..42, y1..y6 (entry rows y2..y5, top border y1);
//!   the Settings panel (2 entries, min width 24) is x22..45, y1..y4
//!   (entry rows y2..y3).
//!
//! Observation points:
//! - row execution: `app.active_view()` + `app.current_session()` +
//!   `app.compositor()` dialog ids (the executed MenuAction re-dispatches
//!   on the bus — exactly what Enter on the row does).
//! - close + de-select: `app.board_store().menu_focus()` /
//!   `app.navigator().mux.menu_focus()` + `.open_menu()`.
//! - the click still lands: the board's card selection / the mux's
//!   pane focus.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{
    components::menu_bar::MenuFocus, App, FspecBackend, MuxConfig, MuxOrientation, MuxPaneKind,
    ViewMode,
};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

const BOARD_EXIT_DIALOG_ID: &str = "board-exit-confirmation-dialog";
const MUX_CONFIG_DIALOG_ID: &str = "mux-config-dialog";
const CREATE_SESSION_DIALOG_ID: &str = "create-session-dialog";

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers (menu002/menu004 pattern)
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

/// Render the full App into a `w`×`h` buffer.
fn render_app(app: &mut App, w: u16, h: u16) -> Buffer {
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

/// The 2-zone menu bar's row (3 rows below the `Checkpoints:` row).
fn bar_row(buf: &Buffer) -> u16 {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .expect("the checkpoint status row must render");
    status_y + 3
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

/// Seed a small board (AUTH-001 backlog, AUTH-002 testing, AUTH-003
/// blocked) so columns have selectable cards.
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

fn focus_column(app: &mut App, name: &str) {
    app.board_store_mut().set_focused_column(name);
}

/// Arrange: the 2-zone bar is focused on the FIRST menu item (dropdown
/// closed): focus the last column and step Right once.
async fn focus_first_item(app: &mut App) {
    focus_column(app, "blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(app).await;
}

/// Arrange: open the `item_index`-th menu item's dropdown with the
/// cursor on `cursor_row` (0 = Kanban, 1 = Tools, 2 = Settings,
/// 3 = Help — MENU-008 four Zone A items).
async fn open_category_dropdown_at(app: &mut App, item_index: usize, cursor_row: usize) {
    focus_first_item(app).await;
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

/// The y of the dropdown row whose text contains `needle` (the panel
/// anchors at bar + 1; the first entry row sits at bar + 2).
fn dropdown_row_y(buf: &Buffer, needle: &str) -> u16 {
    let bar = bar_row(buf);
    (bar + 2..buf.area.height)
        .find(|&y| row_text(buf, y).contains(needle))
        .unwrap_or_else(|| panic!("dropdown row '{needle}' not found below row {bar}"))
}

/// The mux bar's row (always row 0 when the bar is painted —
/// `menu004_mux_surface.rs` parity: a constant, not a search).
const MUX_BAR_ROW: u16 = 0;

/// The y of the mux dropdown row whose text contains `needle` (the
/// panel anchors at row 1; the first entry row sits at row 2).
fn mux_dropdown_row_y(buf: &Buffer, needle: &str) -> u16 {
    (MUX_BAR_ROW + 2..buf.area.height)
        .find(|&y| row_text(buf, y).contains(needle))
        .unwrap_or_else(|| panic!("mux dropdown row '{needle}' not found below row {MUX_BAR_ROW}"))
}

/// Enable mux with the given pane list (horizontal, equal splits, focus
/// pane 0) — mirrors `menu004_mux_surface.rs::enable_mux` including the
/// BUG-182 R7 initial loads for the rendered lazy panes.
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
    if panes.contains(&MuxPaneKind::ChangedFiles) {
        app.navigator_mut().changed_files = codelet_fspec_tui::views::ChangedFilesView::new();
        app.dispatch(codelet_fspec_tui::Action::ChangedFilesLoaded(Vec::new()));
    }
    if panes.contains(&MuxPaneKind::Checkpoints) {
        app.navigator_mut().checkpoints = codelet_fspec_tui::views::CheckpointsView::new();
        app.dispatch(codelet_fspec_tui::Action::CheckpointsLoaded(Vec::new()));
    }
}

/// Arrange (mux): the board pane is focused with the cursor on the last
/// column (blocked) — the ring's edge just before the mux bar.
fn focus_last_column(app: &mut App) {
    app.board_store_mut().set_focused_column("blocked");
}

/// Arrange (mux): the mux bar is focused on the first item (Kanban) via
/// the board ring entry (Right off the last column).
async fn focus_bar_first_item(app: &mut App) {
    focus_last_column(app);
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(app).await;
}

/// Arrange (mux): open the `item_index`-th menu item's dropdown with
/// the cursor on `cursor_row`.
async fn mux_open_category_dropdown_at(app: &mut App, item_index: usize, cursor_row: usize) {
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

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Clicking a dropdown entry row executes that entry
#[tokio::test]
async fn scenario_clicking_a_dropdown_entry_row_executes_that_entry() {
    // @step Given the board has 1 open session and the Kanban dropdown is open with the cursor on row 0 (New Agent)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    open_kanban_dropdown_at(&mut app, 0).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the Kanban dropdown must be open at row 0"
    );
    // The panel must have painted (the click arm hit-tests against the
    // cached panel rect from this frame).
    let buf = render_app(&mut app, 120, 24);
    let new_agent_y = dropdown_row_y(&buf, "New Agent");
    assert!(
        row_text(&buf, new_agent_y).contains("New Agent"),
        "the 'New Agent' entry row must be painted:\n{}",
        row_text(&buf, new_agent_y)
    );
    // Click INSIDE the panel on that row's inner area (x = panel.x + 1).
    let click_x = 16u16;

    // @step When I click the 'New Agent' row in the open dropdown
    app.handle_event(&click(click_x, new_agent_y));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the board (BUG-203: the
    // 'New Agent' row ALWAYS starts a new agent — the pre-BUG-203 R8
    // substitution, which flipped to the Agent view on session s-1, is gone)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "a click on the 'New Agent' row must mount the CreateSessionDialog (BUG-199 parity)"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the user stays on the board until the dialog is confirmed (RPC-097 reopen #1)"
    );

    // @step And the dropdown closes and the bar highlight clears
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "the dropdown must close after the click executed the row"
    );
    assert!(
        app.board_store().menu_focus().is_none(),
        "the bar highlight must clear after the click executed the row"
    );
}

/// Scenario: Clicking a non-cursor dropdown row executes the clicked row
#[tokio::test]
async fn scenario_clicking_a_non_cursor_dropdown_row_executes_the_clicked_row() {
    // @step Given the board has 2 open sessions and the Kanban dropdown is open with the cursor on row 3 (Attachments)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    open_kanban_dropdown_at(&mut app, 3).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 3)),
        "the Kanban dropdown must be open at row 3"
    );
    let buf = render_app(&mut app, 120, 24);
    // The cursor (Attachments) is the inverse row; the 'New Agent' row
    // (row 0) is NOT the cursor.
    let new_agent_y = dropdown_row_y(&buf, "New Agent");
    let attach_y = dropdown_row_y(&buf, "Attachments");
    assert_ne!(
        new_agent_y, attach_y,
        "'New Agent' (row 0) and the cursor row (Attachments, row 3) are distinct"
    );

    // @step When I click the 'New Agent' row (row 0) in the open dropdown
    app.handle_event(&click(16, new_agent_y));
    drain_pending(&mut app).await;

    // @step Then the CreateSessionDialog mounts over the board (BUG-203: a
    // click on a NON-cursor row executes THAT row — and the 'New Agent' row
    // now ALWAYS mounts the dialog; the pre-BUG-203 R8 substitution, which
    // flipped to the Agent view on the session current before the click,
    // is gone)
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "a click on a NON-cursor row must execute THAT row (GUI parity: click selects + confirms)"
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the user stays on the board until the dialog is confirmed"
    );

    // @step And the dropdown closes and the bar highlight clears
    assert_eq!(app.board_store().open_menu(), None);
    assert!(app.board_store().menu_focus().is_none());
}

/// Scenario: Clicking the Help dropdown Exit row opens the exit confirmation
#[tokio::test]
async fn scenario_clicking_the_help_dropdown_exit_row_opens_the_exit_confirmation() {
    // @step Given the Help dropdown is open with the cursor on row 0 (Help)
    let (mut app, _mock) = fresh_app();
    open_help_dropdown_at(&mut app, 0).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((3, 0)),
        "the Help dropdown must be open at row 0"
    );
    let buf = render_app(&mut app, 120, 24);
    let exit_y = dropdown_row_y(&buf, "Exit");
    // Click inside the Help panel's inner area (the Help item sits at
    // x49 — MENU-011 moved it right to the bracketed "[ Help ]" — so
    // the panel starts at x49; click x50).
    let click_x = 50u16;

    // @step When I click the 'Exit' row in the open dropdown
    app.handle_event(&click(click_x, exit_y));
    drain_pending(&mut app).await;

    // @step Then the exit confirmation dialog opens
    assert!(
        app.compositor().contains(BOARD_EXIT_DIALOG_ID),
        "the 'Esc Exit' row must execute OpenBoardExitConfirmation"
    );

    // @step And the dropdown closes and the bar highlight clears
    assert_eq!(app.board_store().open_menu(), None);
    assert!(app.board_store().menu_focus().is_none());
}

/// Scenario: A click on the dropdown border rows does nothing
#[tokio::test]
async fn scenario_a_click_on_the_dropdown_border_rows_does_nothing() {
    // @step Given the Kanban dropdown is open with the cursor on row 0
    let (mut app, _mock) = fresh_app();
    open_kanban_dropdown_at(&mut app, 0).await;
    let buf = render_app(&mut app, 120, 24);
    // The panel anchors at (x15, bar+1) with a 1-cell border on all
    // sides: the top border row is bar+1 (the panel's first row).
    let bar = bar_row(&buf);
    let top_border_y = bar + 1;
    assert!(
        !row_text(&buf, top_border_y).contains("New Agent"),
        "the top border row must not carry an entry row:\n{}",
        row_text(&buf, top_border_y)
    );

    // @step When I click the panel's top border row
    app.handle_event(&click(16, top_border_y));
    drain_pending(&mut app).await;

    // @step Then the dropdown stays open with the cursor on row 0
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "a click on a border row must NOT close the dropdown or execute anything"
    );
    let buf = render_app(&mut app, 120, 24);
    let new_agent_y = dropdown_row_y(&buf, "New Agent");
    assert!(
        row_text(&buf, new_agent_y).contains("New Agent"),
        "the panel must still be painted after a border-row click"
    );
}

/// Scenario: Clicking away closes the dropdown and de-selects the bar
#[tokio::test]
async fn scenario_clicking_away_closes_the_dropdown_and_de_selects_the_bar() {
    // @step Given the Kanban dropdown is open and the board is focused on a kanban column
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    open_kanban_dropdown_at(&mut app, 0).await;
    assert_eq!(app.board_store().open_menu(), Some((0, 0)));
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must be focused before the click"
    );
    // The panel must have painted (the outside-click arm hit-tests
    // against the cached panel rect).
    let _buf = render_app(&mut app, 120, 24);
    // The card in another column: BLOCKED is the last column — its
    // content rect starts at x = 118 - 16 (column width) + 1 (menu002
    // parity: card click at (103, 14)).
    let card_x = 103u16;
    let card_y = 14u16;

    // @step When I click a work-unit card in another column
    app.handle_event(&click(card_x, card_y));
    drain_pending(&mut app).await;

    // @step Then the dropdown closes and that card becomes selected in its column
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "the dropdown must close on the outside click"
    );
    assert_eq!(app.board_store().focused_column(), "blocked");
    assert_eq!(
        app.board_store().selected_index_for("blocked"),
        0,
        "the click must STILL land: the clicked card becomes selected"
    );

    // @step And the bar highlight clears (no menu item is focused)
    assert!(
        app.board_store().menu_focus().is_none(),
        "clicking away must DE-SELECT the parent menu item (BUG-196: the bar was keeping \
         Item(0) focused, swallowing the surface's key bindings)"
    );
}

/// Scenario: The board key bindings are live again after a click away
#[tokio::test]
async fn scenario_the_board_key_bindings_are_live_again_after_a_click_away() {
    // @step Given the Kanban dropdown is open and the board has no open sessions
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    open_kanban_dropdown_at(&mut app, 0).await;
    assert_eq!(app.board_store().open_menu(), Some((0, 0)));
    let _buf = render_app(&mut app, 120, 24);

    // @step When I click a work-unit card in another column and then press the '.' key
    // Click away (closes + de-selects — the '.' arm must no longer be
    // swallowed by the bar), then press '.' (the new-agent binding).
    app.handle_event(&click(103, 14));
    drain_pending(&mut app).await;
    assert!(app.board_store().menu_focus().is_none());
    app.handle_event(&key(KeyCode::Char('.'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the create-session dialog opens (the '.' binding ran and was not swallowed by the bar)
    // No sessions are open, so the '.' arm's OpenAgentView(None) falls
    // through to the RPC-097 create-session dialog.
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the '.' binding must be live again after the click away (it was swallowed \
         while the Kanban item kept the bar focus)"
    );
}

/// Scenario: Clicking a mux dropdown row executes that entry
#[tokio::test]
async fn scenario_clicking_a_mux_dropdown_row_executes_that_entry() {
    // @step Given the mux 'Settings' dropdown is open with the cursor on row 0 (Mux)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    enable_mux(
        &mut app,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    mux_open_category_dropdown_at(&mut app, 2, 0).await;
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((2, 0)),
        "the Settings dropdown must be open at row 0"
    );
    // The panel must have painted (the click arm hit-tests against the
    // cached panel rect from this frame).
    let buf = render_app(&mut app, 240, 24);
    // The Settings item sits at x22 (MENU-011: the bracketed Zone A
    // items — "[ Kanban ]" x1-10, "[ Tools ]" x12-20, "[ Settings ]"
    // x22-33 — so the panel starts at x22, min width 24 → x22..45):
    // click the inner area (x23) of the 'Mux' entry row.
    let mux_row_y = mux_dropdown_row_y(&buf, "Mux");
    assert!(
        row_text(&buf, mux_row_y).contains("Mux"),
        "the 'Mux' entry row must be painted:\n{}",
        row_text(&buf, mux_row_y)
    );
    let click_x = 23u16;

    // @step When I click the 'Mux' row in the open dropdown
    app.handle_event(&click(click_x, mux_row_y));
    drain_pending(&mut app).await;

    // @step Then the mux config dialog opens
    assert!(
        app.compositor().contains(MUX_CONFIG_DIALOG_ID),
        "the 'M Mux' row must execute OpenMuxConfigDialog"
    );

    // @step And the dropdown closes and the mux bar highlight clears
    assert_eq!(
        app.navigator().mux.open_menu(),
        None,
        "the dropdown must close after the click executed the row"
    );
    assert!(
        app.navigator().mux.menu_focus().is_none(),
        "the mux bar highlight must clear after the click executed the row"
    );
}

/// Scenario: Clicking away in mux closes the dropdown and de-selects the bar
#[tokio::test]
async fn scenario_clicking_away_in_mux_closes_the_dropdown_and_de_selects_the_bar() {
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
    assert_eq!(
        app.navigator().mux.menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must be focused before the click"
    );
    // The opened dropdown must be on screen (the next frame paints the
    // panel and caches its rect) — the outside-click arm hit-tests
    // against that cached geometry.
    let _buf = render_app(&mut app, 240, 24);
    // The Files pane is pane index 1 (pane 0 x0..118, pane 1 x120..237):
    // click well inside the Files pane (menu004 parity: (150, 10)).
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
        "the click must STILL land: the Files pane (index 1) is focused"
    );

    // @step And the mux bar highlight clears (the parent item is de-selected)
    assert!(
        app.navigator().mux.menu_focus().is_none(),
        "clicking away must DE-SELECT the parent item (BUG-196: the bar was keeping \
         Item(0) focused)"
    );
}

/// Scenario: A mux click on the dropdown border rows does nothing
#[tokio::test]
async fn scenario_a_mux_click_on_the_dropdown_border_rows_does_nothing() {
    // @step Given the mux 'Kanban' dropdown is open with the cursor on row 0
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
    let buf = render_app(&mut app, 240, 24);
    // The Kanban panel anchors at (x1, y1) with a 1-cell border on all
    // sides: the top border row is y1 (the panel's first row), inside
    // the Board pane's body.
    assert!(
        row_text(&buf, 2).contains("New Agent"),
        "the panel's first entry row (y2) must be 'New Agent':\n{}",
        row_text(&buf, 2)
    );
    let focus_before = app.navigator().mux.focus();

    // @step When I click the panel's top border row
    app.handle_event(&click(2, 1));
    drain_pending(&mut app).await;

    // @step Then the dropdown stays open with the cursor on row 0
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((0, 0)),
        "a click on a border row must NOT close the dropdown or execute anything"
    );
    assert_eq!(
        app.navigator().mux.focus(),
        focus_before,
        "a border-row click must NOT land on the pane beneath it (the panel owns its rect)"
    );
}

/// Scenario: Esc still closes the dropdown and keeps the item focused
#[tokio::test]
async fn scenario_esc_still_closes_the_dropdown_and_keeps_the_item_focused() {
    // @step Given the Kanban dropdown is open
    let (mut app, _mock) = fresh_app();
    open_kanban_dropdown_at(&mut app, 0).await;
    assert_eq!(app.board_store().open_menu(), Some((0, 0)));

    // @step When I press Esc once
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dropdown is closed and the Kanban item stays focused
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "Esc must close the dropdown"
    );
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "R4: the keyboard close is a 'stay on the bar' gesture — the item \
         must stay focused (only the click-away path de-selects)"
    );
    let buf = render_app(&mut app, 120, 24);
    // MENU-009: the bar row now ALWAYS paints the right-aligned 'New
    // Agent' Zone C button, so 'New Agent' appearing on the bar row is
    // expected — the panel-closed check must target the dropdown's
    // first entry row (bar + 2), not the whole buffer.
    assert!(
        !row_text(&buf, bar_row(&buf) + 2).contains("New Agent"),
        "the panel must be closed (its first entry row must not paint \
         'New Agent'):\n{}",
        buf_text(&buf)
    );
}
