//! MENU-002 — Board surface: the 2-zone bar + column→menu→chip continuous ring.
//!
//! Feature: spec/features/board-surface-header-row-replaced-by-the-2-zone-bar-column-menu-chip-continuous-ring-keys-wheel-mouse.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: App + MockBackend (the `board_actions_dialog_board023.rs`
//! pattern) — `fresh_app` + `drain_pending` + full-App render into a
//! TestBackend at 120x24.
//!
//! Observation points:
//! - ring focus: `app.board_store().menu_focus()` (the new BoardStore
//!   API: `Option<MenuFocus>`) + `focused_column_index()`.
//! - the live bar: inverse-video (bg Cyan) cells on the header's row 3
//!   (`bar_row(buf)` = the row 3 below the `Checkpoints:` row).
//! - the anchored dropdown: its rounded panel + inverse cursor row.
//! - chip activation / execution: `app.active_view()` +
//!   `app.current_session()` / `app.board_store()` state.
//!
//! Geometry (120x24, single Board view):
//!   y0    top border
//!   y1-4  4-row header (row 0 `Checkpoints:…` at y1, `─` divider at y3,
//!         the 2-zone menu bar at y4)
//!   y5    plain separator
//!   y6-10 details strip (5 rows)
//!   y11   ├┬┤ separator
//!   y12   column headers (BACKLOG … BLOCKED)
//!   y13   ├┼┤ separator
//!   y14+  content rows
//!   y22   footer hint row, y23 bottom border
//!
//! Bar content geometry (inside the 120x24 buffer, ASCII cells):
//!   logo block x2..13, right column x14..118 (1-cell bar padding →
//!   content x15..): MENU-011 paints the Zone A items BRACKETED —
//!   "[ Kanban ]" x15-24 (item x = 15), "[ Tools ]" x26-34,
//!   "[ Settings ]" x36-47, "[ Help ]" x49-56 (MENU-008: four Zone A
//!   items), dim `│` separator at x59, bracketed chips from x61 (6-cell
//!   cells, 2-cell gaps: `[ #1 ● ]` x61-66, `[ #2 ● ]` x69-74,
//!   `[ #3 ● ]` x77-82).
//!   The Kanban dropdown panel anchors under the item: x15, y5.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::{App, FspecBackend, ViewMode};
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

/// The Kanban item's Zone A rect x (header right column, bar row).
const KANBAN_X: u16 = 15;
/// The Tools item's Zone A rect x.
/// MENU-011: the Zone A items paint bracketed (`[ Kanban ]` etc.), so the
/// item x positions moved right (`[ Kanban ]` = 10 cells wide).
const TOOLS_X: u16 = 26;
/// The Help item's Zone A rect x.
const HELP_X: u16 = 49;
/// The first chip's cell x.
/// MENU-011 R2: the bracketed cells widen the row — chip #1 x61-66,
/// chip #2 x69-74, chip #3 x77-82 (2-cell gaps between cells).
const CHIP1_X: u16 = 61;

const BOARD_EXIT_DIALOG_ID: &str = "board-exit-confirmation-dialog";

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

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The header's row 3 — the 2-zone menu bar's row (3 rows below the
/// checkpoint-status row, whose leading text is `Checkpoints:`).
fn bar_row(buf: &Buffer) -> u16 {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .expect("the checkpoint status row must render");
    status_y + 3
}

/// True when the cell carries the inverse-video highlight (bg Cyan).
fn is_inverse(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].bg == Color::Cyan
}

/// The cell x of the first occurrence of `needle` on row `y`
/// (the row's multibyte-safe char index).
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
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

/// Seed `n` open sessions (s-1..s-n, Idle by default).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await;
}

/// Seed a small board so the focused column has a selectable card
/// (AUTH-001 in backlog, one in testing, one in blocked).
async fn seed_units(app: &mut App) {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog"),
        make_unit("AUTH-002", "testing"),
        make_unit("AUTH-003", "blocked"),
    ]));
    drain_pending(app).await;
}

/// Focus the named column via the existing action bus.
fn focus_column(app: &mut App, name: &str) {
    app.board_store_mut().set_focused_column(name);
}

/// Arrange: the 2-zone bar is focused on the FIRST menu item (the
/// dropdown closed): focus the last column and step Right once.
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
/// anchors at bar_row + 1; rows sit just below the bar row).
fn dropdown_row_y(buf: &Buffer, needle: &str) -> u16 {
    let bar = bar_row(buf);
    // Start at bar + 2: the panel's border row (bar + 1) and the bar's
    // OWN row (which can contain the item label, e.g. "Help") never
    // carry entry rows — the first entry row sits at bar + 2.
    (bar + 2..buf.area.height)
        .find(|&y| row_text(buf, y).contains(needle))
        .unwrap_or_else(|| panic!("dropdown row '{needle}' not found below row {bar}"))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Left from the first column focuses the New Agent button
#[tokio::test]
async fn scenario_left_from_the_first_column_focuses_the_last_chip() {
    // @step Given the board has 3 open sessions and the first column (backlog) is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    assert_eq!(
        app.board_store().menu_focus(),
        None,
        "a column focus is not a bar focus"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the New Agent button paints inverse-video
    // (MENU-009 R3: the right-aligned Zone C button is now the ring's
    // LAST stop — Left from the first column lands on it, not the
    // last chip; Right from it lands on the last chip.)
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Left from the first column lands on the New Agent button (the ring's last stop)"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    // The 'New Agent' button's rightmost cell (its last char, 't')
    // paints inverse-video.
    let na_x = find_x(&buf, y, "New Agent").expect("New Agent on the bar row");
    // MENU-011: the button paints bracketed — "[ New Agent ]" (find_x
    // lands on the 'N'; the last 't' sits 10 cells past the '[').
    let na_last = na_x + 10; // "[ New Agent".len() - 1
    assert!(
        is_inverse(&buf, na_last, y),
        "the New Agent button's last cell must paint inverse-video on row {y}:\n{}",
        row_text(&buf, y)
    );
    // And the first column stays the focused column (its header is the
    // grid focus the ring returns to).
    assert_eq!(app.board_store().focused_column_index(), 0);
}

/// Scenario: Left from the New Agent button focuses the last chip
#[tokio::test]
async fn scenario_left_from_the_new_agent_button_focuses_the_last_chip() {
    // @step Given the board has 3 open sessions and the ring is focused on the New Agent button
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Left from the first column must land on the New Agent button"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then chip #3 paints inverse-video (the button's Left lands on the last chip — the pre-MENU-009 last stop)
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(2)),
        "Left from the New Agent button must land on the last chip (chip #3)"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    // Chips sit 8 cells wide (MENU-011 R2: the bracketed `[ #n ● ]` =
    // "#n " prefix + glyph + the "[ " / " ]" brackets) with 2-cell gaps
    // after the `│` separator: #1 x61-68, #2 x71-78, #3 x81-88 — so
    // chip #3 starts at x81 (CHIP1_X + 2*10).
    assert!(
        is_inverse(&buf, CHIP1_X + 20, y),
        "chip #3 cell must paint inverse-video on row {y}:\n{}",
        row_text(&buf, y)
    );
}

/// Scenario: Enter on an open dropdown executes the highlighted row
#[tokio::test]
async fn scenario_enter_on_an_open_dropdown_executes_the_highlighted_row() {
    // @step Given the Tools dropdown is open with the cursor on row 1 (Checkpoints)
    let (mut app, _mock) = fresh_app();
    open_category_dropdown_at(&mut app, 1, 1).await;
    let buf = render_app(&mut app);
    let cp_y = dropdown_row_y(&buf, "Checkpoints");
    assert!(
        is_inverse(&buf, TOOLS_X + 1, cp_y),
        "the cursor row must be inverse before Enter:\n{}",
        row_text(&buf, cp_y)
    );

    // @step When I press Enter once
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Checkpoints view opens and the dropdown closes
    assert_eq!(
        app.active_view(),
        ViewMode::Checkpoints,
        "Enter on the Tools dropdown's Checkpoints row must open the Checkpoints view"
    );
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        !text.contains("New Agent"),
        "the dropdown panel must be closed after execute:\n{text}"
    );
    // @step And the bar highlight clears (BUG-196 R2: execute is the 'leave the bar' gesture)
    // BUG-196 R2: execute is the 'leave the bar' gesture — the bar's
    // ring focus clears (GUI parity with the click-execute path, and
    // with the mux layout's `menu_execute_item` clearing its focus).
    assert!(
        app.board_store().menu_focus().is_none(),
        "the bar highlight must clear after executing the row (BUG-196 R2)"
    );
}

/// Scenario: Enter on a chip opens that session agent view
#[tokio::test]
async fn scenario_enter_on_a_chip_opens_that_session_agent_view() {
    // @step Given the board has 3 open sessions and the menu bar is focused on chip #2
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "blocked");
    for _ in 0..6 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(1)),
        "Right x6 from the last column must land on chip #2 (4 items then chips)"
    );

    // @step When I press Enter once
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the active view flips to session #2's agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Enter on a chip must flip to the Agent view"
    );
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-2")),
        "the flipped view must host session #2"
    );
}

/// Scenario: Up and Down from the bar drop focus back into the focused column
#[tokio::test]
async fn scenario_up_and_down_from_the_bar_drop_focus_back_into_the_focused_column() {
    // @step Given the board has 3 open sessions and the menu bar is focused on chip #1
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Left x4 from the first column must land on chip #1 (New Agent + items in between — MENU-009)"
    );
    let before = app.board_store().selected_index_for("backlog");

    // @step When I press Up once
    app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the focused column keeps its previous card selection and the bar highlight clears
    assert!(
        app.board_store().menu_focus().is_none(),
        "Up from the bar must clear the bar focus"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        0,
        "the focused column must be re-focused"
    );
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        before,
        "the card selection must be unchanged"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    // MENU-010: the board never carries the current session's chip
    // highlight — with the ring cleared, NO cell on the bar row may
    // paint inverse-video (pre-MENU-010 the focused session's chip,
    // #3 here, kept its active highlight).
    for x in 0..buf.area.width {
        assert!(
            !is_inverse(&buf, x, y),
            "no inverse cell may remain on the bar row {y} (MENU-010: \
             the board never highlights the current session's chip):\n{}",
            row_text(&buf, y)
        );
    }
}

/// Scenario: Enter on a menu item opens its dropdown at row 0
#[tokio::test]
async fn scenario_enter_on_a_menu_item_opens_its_dropdown_at_row_0() {
    // @step Given the board is focused on the Kanban menu item with its dropdown closed
    let (mut app, _mock) = fresh_app();
    focus_first_item(&mut app).await;
    let buf = render_app(&mut app);
    assert!(
        is_inverse(&buf, KANBAN_X, bar_row(&buf)),
        "the Kanban item must be focused before Enter"
    );

    // @step When I press Enter once
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Kanban dropdown panel paints under the item with its cursor on row 0 (New Agent)
    let buf = render_app(&mut app);
    let bar = bar_row(&buf);
    let row0 = bar + 2;
    assert!(
        row_text(&buf, row0).contains("New Agent"),
        "the panel's first row (y {row0}) must be 'New Agent':\n{}",
        row_text(&buf, row0)
    );
    assert!(
        is_inverse(&buf, KANBAN_X + 1, row0),
        "the cursor row (row 0) must be inverse-video"
    );
    let attach_y = dropdown_row_y(&buf, "Attachments");
    assert!(
        !is_inverse(&buf, KANBAN_X + 1, attach_y),
        "only row 0 is the cursor row"
    );
}

/// Scenario: The ring walks items then chips then the New Agent button then wraps to the first column
#[tokio::test]
async fn scenario_the_ring_walks_items_then_chips_then_wraps_to_the_first_column() {
    // @step Given the board has 1 open session and the last column (blocked) is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "blocked");
    let _ = render_app(&mut app);

    // @step When I press Right seven times
    // (MENU-009: the ring is now 7 columns + 4 items + 1 chip + 1 Zone C
    // button = 13 stops — 7 Rights from the last column walk through
    // Kanban, Tools, Settings, Help, chip #1, the New Agent button,
    // and wrap back to the first column.)
    for _ in 0..7 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;

    // @step Then the focus lands on Kanban then Tools then Settings then Help then chip #1 then the 'New Agent' button and finally back on the first column (backlog) and no cell on the bar row paints inverse-video (the ring wrapped off the bar and the board carries no current-session chip highlight — MENU-010)
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert!(
        app.board_store().menu_focus().is_none(),
        "after 7 Right presses the focus must have wrapped back to a column"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        0,
        "the ring must wrap back to the first column"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    // MENU-010: the board never carries the current session's chip
    // highlight — after the ring wrapped off the bar onto the
    // first column, NO cell on the bar row may paint inverse-video
    // (pre-MENU-010 the focused session's chip, #1 here, kept its
    // active highlight).
    for x in 0..buf.area.width {
        assert!(
            !is_inverse(&buf, x, y),
            "no inverse cell may remain on the bar row {y} (MENU-010: \
             the board never highlights the current session's chip):\n{}",
            row_text(&buf, y)
        );
    }
    // The intermediate stops are pinned: 5 Right presses must sit ON
    // chip #1 (Kanban → Tools → Settings → Help → chip #1 → back to
    // columns is the 6th stop).
    let (mut app2, _mock2) = fresh_app();
    seed_sessions(&mut app2, 1).await;
    seed_units(&mut app2).await;
    focus_column(&mut app2, "blocked");
    for _ in 0..5 {
        app2.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app2).await;
    assert_eq!(
        app2.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "5 Right presses must land on chip #1 (4 items → chip #1)"
    );
}

/// Scenario: The board header row 3 paints the 2-zone menu bar instead of the u Actions chord
#[tokio::test]
async fn scenario_the_board_header_row_3_paints_the_2_zone_menu_bar() {
    // @step Given the board has 2 open sessions (one running, one idle)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    app.dispatch(codelet_fspec_tui::Action::SessionStatusChanged(
        SessionId::new("s-1"),
        SessionStatus::Running,
    ));
    drain_pending(&mut app).await;
    seed_units(&mut app).await;

    // @step When the board view renders
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let row = row_text(&buf, y);

    // @step Then row 3 of the header right column shows the menu items (Kanban Tools Settings Help) a separator and the two session chips and the string 'u Actions' appears nowhere on the board
    assert!(
        row.contains("Kanban")
            && row.contains("Tools")
            && row.contains("Settings")
            && row.contains("Help"),
        "the four MENU-008 menu items must paint on row {y}:\n{row}"
    );
    assert!(
        row.contains("│"),
        "the dim separator must separate the zones:\n{row}"
    );
    assert!(
        row.contains("#1") && row.contains("#2"),
        "both session chips must paint:\n{row}"
    );
    assert!(
        !buf_text(&buf).contains("u Actions"),
        "the old 'u Actions' chord must no longer render anywhere on the board"
    );
}

/// Scenario: Right from the last column focuses the first menu item
#[tokio::test]
async fn scenario_right_from_the_last_column_focuses_the_first_menu_item() {
    // @step Given the board has no open sessions and the last column (blocked) is focused
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_column(&mut app, "blocked");
    let _ = render_app(&mut app);

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Kanban item paints inverse-video with its dropdown closed
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "Right on the last column must land on the first menu item"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    assert!(is_inverse(&buf, KANBAN_X, y), "Kanban must be inverse");
    assert!(
        !row_text(&buf, y + 2).contains("New Agent"),
        "the dropdown must stay closed"
    );
}

/// Scenario: The board footer hint describes the ring
#[tokio::test]
async fn scenario_the_board_footer_hint_describes_the_ring() {
    // @step Given the board has 3 open sessions
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;

    // @step When the board view renders
    let buf = render_app(&mut app);

    // @step Then the footer hint row reads that Left Right cycle columns then menu items then chips
    let footer_y = (0..buf.area.height)
        .find(|&y| row_text(&buf, y).contains("Columns"))
        .expect("the footer hint row must render");
    let footer = row_text(&buf, footer_y);
    assert!(
        footer.contains("menu items") && footer.contains("chips"),
        "the footer hint must describe the full ring:\n{footer}"
    );
}

/// Scenario: The header bar omits the separator and chips when no sessions are open
#[tokio::test]
async fn scenario_the_header_bar_omits_the_separator_and_chips_when_no_sessions_are_open() {
    // @step Given the board has no open sessions
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;

    // @step When the board view renders
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let row = row_text(&buf, y);

    // @step Then the bar row shows only the menu items (Kanban Tools Settings Help) with no separator and no chips
    assert!(
        row.contains("Kanban") && row.contains("Help"),
        "the menu items must still paint:\n{row}"
    );
    // The board box paints `│` side-borders at x0/x119 on EVERY row, so the
    // bar row always carries exactly 2 `│`. When a session is open the bar
    // adds a 3rd (the dim zone separator); with no chips it must stay at 2.
    assert_eq!(
        row.matches('│').count(),
        2,
        "no bar separator without chips (only the 2 box borders):\n{row}"
    );
    assert!(!row.contains("#1"), "no chips without sessions:\n{row}");
}

/// Scenario: Esc closes the open dropdown and keeps the item focused
#[tokio::test]
async fn scenario_esc_closes_the_open_dropdown_and_keeps_the_item_focused() {
    // @step Given the Kanban dropdown is open and the cursor is on row 3
    let (mut app, _mock) = fresh_app();
    open_kanban_dropdown_at(&mut app, 3).await;

    // @step When I press Esc once
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dropdown is closed and the Kanban item stays focused
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "the item must stay focused"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    assert!(is_inverse(&buf, KANBAN_X, y), "the item must stay inverse");
    assert!(
        !row_text(&buf, y + 2).contains("New Agent"),
        "the panel must be closed"
    );
    assert!(
        !app.compositor().contains(BOARD_EXIT_DIALOG_ID),
        "Esc with the dropdown open must NOT run the exit cascade"
    );
}

/// Scenario: Clicking a menu item focuses it and opens its dropdown
#[tokio::test]
async fn scenario_clicking_a_menu_item_focuses_it_and_opens_its_dropdown() {
    // @step Given the board is focused on a kanban column
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    let _ = render_app(&mut app);

    // @step When I click the Help menu item in the bar
    let y = bar_row(&render_app(&mut app));
    app.handle_event(&click(HELP_X, y));
    drain_pending(&mut app).await;

    // @step Then the Help item paints inverse-video and its dropdown opens with the cursor on row 0
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(3)),
        "the click must focus the Help item (MENU-008: 4th Zone A item)"
    );
    let buf = render_app(&mut app);
    let row = bar_row(&buf);
    assert!(is_inverse(&buf, HELP_X, row), "Help must be inverse");
    let row0 = row + 2;
    assert!(
        row_text(&buf, row0).contains("Help"),
        "the Help dropdown's row 0 must be the '?' Help row:\n{}",
        row_text(&buf, row0)
    );
    assert!(
        is_inverse(&buf, HELP_X + 1, row0),
        "the cursor must be on row 0"
    );
}

/// Scenario: With no open sessions the ring wraps from the last item to the first column
#[tokio::test]
async fn scenario_with_no_open_sessions_the_ring_wraps_from_the_last_item_to_the_first_column() {
    // @step Given the board has no open sessions and the menu bar is focused on the last menu item (Help)
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_first_item(&mut app).await;
    for _ in 0..3 {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(&mut app).await;
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(3)),
        "with no chips, Right x3 from Kanban must land on Help"
    );

    // @step When I press Right twice
    // (MENU-009: with no chips the ring is 7 columns + 4 items + 1 Zone C
    // button — Right from Help lands on the 'New Agent' button; a second
    // Right wraps back to the first column.)
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneC(0)),
        "Right from Help must land on the New Agent button (the last ring stop)"
    );
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    let na_x = find_x(&buf, y, "New Agent").expect("New Agent on the bar row");
    assert!(
        is_inverse(&buf, na_x + 10, y), // MENU-011: bracketed "[ New Agent ]"
        "the New Agent button must paint inverse-video on row {y}:\n{}",
        row_text(&buf, y)
    );
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the first column (backlog) is focused again and the bar highlight clears
    assert!(
        app.board_store().menu_focus().is_none(),
        "the second Right must wrap back to a column"
    );
    assert_eq!(app.board_store().focused_column_index(), 0);
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    for x in 0..buf.area.width {
        assert!(!is_inverse(&buf, x, y), "the bar highlight must be cleared");
    }
}

/// Scenario: While a dropdown is open all other keys are swallowed and Up Down move the cursor
#[tokio::test]
async fn scenario_while_a_dropdown_is_open_all_other_keys_are_swallowed() {
    // @step Given the Kanban dropdown is open with the cursor on row 3 (Attachments)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    open_kanban_dropdown_at(&mut app, 3).await;
    let buf = render_app(&mut app);
    let attach_y = dropdown_row_y(&buf, "Attachments");
    assert!(
        is_inverse(&buf, KANBAN_X + 1, attach_y),
        "the cursor must start on row 3 (Attachments)"
    );

    // @step When I press the bare C key
    app.handle_event(&key(KeyCode::Char('c'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the checkpoints view does not open and the cursor stays on row 3
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the bare 'c' shortcut must be swallowed while the dropdown is open"
    );
    let buf = render_app(&mut app);
    assert!(
        is_inverse(&buf, KANBAN_X + 1, attach_y),
        "the cursor must stay on row 3"
    );
    // And Up/Down drive the dropdown cursor (not the board selection).
    app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);
    let foundation_y = dropdown_row_y(&buf, "FOUNDATION.md");
    assert!(
        is_inverse(&buf, KANBAN_X + 1, foundation_y),
        "Up must move the cursor to row 2 (FOUNDATION.md)"
    );
    app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    let buf = render_app(&mut app);
    let attach_y2 = dropdown_row_y(&buf, "Attachments");
    assert!(
        is_inverse(&buf, KANBAN_X + 1, attach_y2),
        "Down must move the cursor back to row 3 (Attachments)"
    );
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        0,
        "the board selection must not move"
    );
}

/// Scenario: Wheel left and right over the bar walk the ring
#[tokio::test]
async fn scenario_wheel_left_and_right_over_the_bar_walk_the_ring() {
    // @step Given the board has 3 open sessions and the last column (blocked) is focused
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 3).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "blocked");
    let bar = bar_row(&render_app(&mut app));

    // @step When I scroll the wheel right once over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollRight, 20, bar));
    drain_pending(&mut app).await;

    // @step Then the Actions item is focused
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "wheel right over the bar must enter the ring at the first item"
    );
    // @step When I scroll the wheel left once over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollLeft, 20, bar));
    drain_pending(&mut app).await;

    // @step Then the last column (blocked) is focused again
    assert!(
        app.board_store().menu_focus().is_none(),
        "wheel left from the first item must return to the columns"
    );
    assert_eq!(app.board_store().focused_column_index(), 6);
}

/// Scenario: Esc with no dropdown open is unchanged
#[tokio::test]
async fn scenario_esc_with_no_dropdown_open_is_unchanged() {
    // @step Given no menu dropdown is open and the menu bar is not focused
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    let _ = render_app(&mut app);
    assert!(app.board_store().menu_focus().is_none());

    // @step When I press Esc once
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the app-level exit cascade runs exactly as before the menu bar existed
    assert!(
        app.compositor().contains(BOARD_EXIT_DIALOG_ID),
        "the app-level exit cascade must open the exit confirmation"
    );
    assert_eq!(app.active_view(), ViewMode::Board);
}

/// Scenario: Clicking a chip activates that session
#[tokio::test]
async fn scenario_clicking_a_chip_activates_that_session() {
    // @step Given the board has 2 open sessions and is focused on a kanban column
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 2).await;
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    let y = bar_row(&render_app(&mut app));

    // @step When I click chip #1 in the bar
    app.handle_event(&click(CHIP1_X, y));
    drain_pending(&mut app).await;

    // @step Then the active view flips to session #1's agent view
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert_eq!(app.current_session(), Some(SessionId::new("s-1")));
}

/// Scenario: Clicking the already open item closes the dropdown
#[tokio::test]
async fn scenario_clicking_the_already_open_item_closes_the_dropdown() {
    // @step Given the Kanban dropdown is open and the Kanban item is focused
    let (mut app, _mock) = fresh_app();
    open_kanban_dropdown_at(&mut app, 1).await;
    let buf = render_app(&mut app);
    let y = bar_row(&buf);
    assert!(
        row_text(&buf, y + 2).contains("New Agent"),
        "the panel must be open before the click"
    );

    // @step When I click the Kanban menu item again
    app.handle_event(&click(KANBAN_X, y));
    drain_pending(&mut app).await;

    // @step Then the dropdown closes and the Kanban item stays focused
    use codelet_fspec_tui::components::menu_bar::MenuFocus;
    assert_eq!(app.board_store().menu_focus(), Some(MenuFocus::Item(0)));
    let buf = render_app(&mut app);
    let row = bar_row(&buf);
    assert!(
        is_inverse(&buf, KANBAN_X, row),
        "the item must stay focused"
    );
    assert!(
        !row_text(&buf, row + 2).contains("New Agent"),
        "the panel must be closed"
    );
}

/// Scenario: Wheel up and down over an open dropdown move the cursor
#[tokio::test]
async fn scenario_wheel_up_and_down_over_an_open_dropdown_move_the_cursor() {
    // @step Given the Help dropdown is open with the cursor on row 1 (Exit)
    let (mut app, _mock) = fresh_app();
    open_help_dropdown_at(&mut app, 1).await;
    let buf = render_app(&mut app);
    let exit_y = dropdown_row_y(&buf, "Exit");
    assert!(is_inverse(&buf, HELP_X + 1, exit_y));

    // @step When I scroll the wheel down once over the dropdown
    // (the Help panel anchors at x37 (MENU-008); the wheel column must
    // sit INSIDE the panel, so x = HELP_X + 3)
    app.handle_event(&mouse(MouseEventKind::ScrollDown, HELP_X + 3, exit_y));
    drain_pending(&mut app).await;

    // @step Then the cursor wraps back to row 0 (Help)
    let buf = render_app(&mut app);
    let help_y = dropdown_row_y(&buf, "Help");
    assert!(
        is_inverse(&buf, HELP_X + 1, help_y),
        "wheel down from the last row must wrap to row 0 (Help)"
    );
}

/// Scenario: Clicking outside the dropdown closes it while the click still works
#[tokio::test]
async fn scenario_clicking_outside_the_dropdown_closes_it_while_the_click_still_works() {
    // @step Given the Help dropdown is open and the board is focused on a kanban column with a card selected
    let (mut app, _mock) = fresh_app();
    seed_units(&mut app).await;
    focus_column(&mut app, "backlog");
    open_help_dropdown_at(&mut app, 0).await;
    let buf = render_app(&mut app);
    assert!(
        row_text(&buf, bar_row(&buf) + 2).contains("Help"),
        "the Help panel must be open before the click"
    );
    // The card in another column: BLOCKED is the last column — its
    // content rect starts at x = 118 - 16 (column width) + 1.
    let card_x = 103u16;
    let card_y = 14u16; // first content row (y14+)

    // @step When I click a work-unit card in another column
    app.handle_event(&click(card_x, card_y));
    drain_pending(&mut app).await;

    // @step Then the dropdown closes and that card becomes selected in its column
    assert!(
        !row_text(&render_app(&mut app), bar_row(&buf) + 2).contains("Help"),
        "the dropdown must close on the outside click"
    );
    assert_eq!(app.board_store().focused_column(), "blocked");
    assert_eq!(
        app.board_store().selected_index_for("blocked"),
        0,
        "the clicked card must become selected in its column"
    );
    // BUG-196 R3: the outside click is the 'leave the bar' gesture —
    // the bar's ring focus clears (de-selects the Help item), so the
    // surface's key bindings are live again.
    assert!(
        app.board_store().menu_focus().is_none(),
        "clicking away must de-select the parent menu item (BUG-196)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// BUG-198 — the open dropdown must follow the Left/Right ring walk
// (feature: spec/features/menu-dropdown-cycles-with-the-board-ring-walk.feature)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: While a dropdown is open Left and Right walk the ring and re-anchor
#[tokio::test]
async fn scenario_while_a_dropdown_is_open_left_and_right_walk_the_ring_and_reanchor() {
    // @step Given the Kanban dropdown is open with the cursor on row 1 (Search)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    open_kanban_dropdown_at(&mut app, 1).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 1)),
        "the Kanban dropdown must be open with the cursor on row 1"
    );
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must hold the ring focus"
    );

    // @step When I press Right once and then Left once
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dropdown re-anchors under the focused item at row 0 each time and the panel paints under the new item
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "Left must walk back onto the Kanban item"
    );
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the re-anchored dropdown must sit at row 0 of Kanban"
    );
    let buf = render_app(&mut app);
    let search_y = dropdown_row_y(&buf, "New Agent");
    assert!(
        row_text(&buf, search_y).contains("New Agent"),
        "the Kanban panel must be painted under the re-anchored item:\n{}",
        row_text(&buf, search_y)
    );
}

/// Scenario: While a dropdown is open Left walks the ring back onto the previous item
#[tokio::test]
async fn scenario_left_with_an_open_dropdown_moves_the_panel_back_onto_the_previous_item() {
    // @step Given the Kanban dropdown is open and the ring has walked onto the Tools item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    open_kanban_dropdown_at(&mut app, 2).await;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((1, 0)),
        "Right must have re-anchored the open panel under Tools at row 0"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the Tools item loses the focus and the dropdown re-anchors under the Kanban item at row 0
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "Left must walk back onto the Kanban item"
    );
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the open panel must follow the walk back under Kanban at row 0"
    );
    let buf = render_app(&mut app);
    let new_agent_y = dropdown_row_y(&buf, "New Agent");
    assert!(
        is_inverse(&buf, KANBAN_X + 1, new_agent_y),
        "the Kanban panel must be painted with the cursor re-armed at row 0"
    );
}

/// Scenario: While a dropdown is open Left from the first item walks into the columns and closes it
#[tokio::test]
async fn scenario_left_from_the_first_item_with_an_open_dropdown_closes_the_panel() {
    // @step Given the Kanban dropdown is open with the ring on the first item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    open_kanban_dropdown_at(&mut app, 0).await;
    let buf = render_app(&mut app);
    assert!(
        row_text(&buf, bar_row(&buf) + 2).contains("New Agent"),
        "the Kanban panel must be open before the walk"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the focused column is re-focused, the bar highlight clears and the dropdown closes
    assert!(
        app.board_store().menu_focus().is_none(),
        "a column landing must drop the bar focus"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "Left from the first item must land on the last column (blocked)"
    );
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "walking off the items must close the open dropdown"
    );
    let buf = render_app(&mut app);
    assert!(
        !row_text(&buf, bar_row(&buf) + 2).contains("New Agent"),
        "the panel must no longer be painted"
    );
}

/// Scenario: While a dropdown is open Right from the last item walks onto a chip and closes it
#[tokio::test]
async fn scenario_right_from_the_last_item_with_an_open_dropdown_closes_the_panel() {
    // @step Given the Help dropdown is open and the board has 1 open session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    open_help_dropdown_at(&mut app, 0).await;
    assert_eq!(
        app.board_store().open_menu(),
        Some((3, 0)),
        "the Help dropdown must be open at row 0"
    );

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the first chip has the focus and the dropdown closes
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Right from the last item must land on chip #1"
    );
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "a Zone B landing must close the open dropdown"
    );
}

/// Scenario: Wheel left and right over the bar with an open dropdown re-anchors the panel
#[tokio::test]
async fn scenario_wheel_over_the_bar_with_an_open_dropdown_reanchors_the_panel() {
    // @step Given the Kanban dropdown is open and the board has 1 open session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    seed_units(&mut app).await;
    open_kanban_dropdown_at(&mut app, 1).await;
    assert_eq!(app.board_store().open_menu(), Some((0, 1)));
    let bar = bar_row(&render_app(&mut app));

    // @step When I scroll the wheel right once over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollRight, 20, bar));
    drain_pending(&mut app).await;

    // @step Then the Tools item has the focus and the open dropdown re-anchors under it at row 0
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(1)),
        "wheel right must walk the ring onto Tools"
    );
    assert_eq!(
        app.board_store().open_menu(),
        Some((1, 0)),
        "the open panel must follow the wheel walk, re-anchored at row 0"
    );
}
