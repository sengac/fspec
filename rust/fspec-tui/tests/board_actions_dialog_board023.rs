//! BOARD-023 — Board actions popup dialog triggered by the 'u' key.
//!
//! Feature: spec/features/board-actions-popup-dialog-triggered-by-u-key.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert: exactly one Given setup, one When act, then the
//! Then + And assertions — no repeated When/Then sequences).
//!
//! Harness: App + MockBackend (the `mux009_board_m_key.rs` pattern) —
//! `fresh_app` + `drain_pending` + full-App render into a TestBackend.
//! The dialog's stable compositor id is `"board-actions-dialog"`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{App, FspecBackend, ViewMode};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;

mod common;
use common::MockBackend;

/// The stable compositor id for the actions dialog.
const BOARD_ACTIONS_DIALOG_ID: &str = "board-actions-dialog";

/// The 8 dialog body rows in canonical order (R3).
const DIALOG_ROWS: [&str; 8] = [
    "C Checkpoints",
    "F Changed Files",
    "D FOUNDATION.md",
    ". New Agent",
    "/ Search",
    "M Mux",
    "? Help",
    "Esc Exit",
];

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

/// Arrange: open the actions dialog with the 'u' trigger key.
async fn open_actions_dialog(app: &mut App) {
    app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(app).await;
}

/// Arrange: open the dialog and move the cursor `downs` rows down.
async fn open_actions_dialog_at(app: &mut App, downs: usize) {
    open_actions_dialog(app).await;
    for _ in 0..downs {
        app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    }
}

fn make_unit(id: &str, status: &str, work_type: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: work_type.to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// Render the full App (navigator + compositor) into a 120x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

/// Render the full App into a `width x height` buffer (the full-list
/// help test needs the whole list to fit without scrolling).
fn render_app_at(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn buf_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row_text(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
}

/// The y of the first buffer row containing `needle` (None when absent).
fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    (0..buf.area.height).find(|&y| row_text(buf, y).contains(needle))
}

/// True iff any cell on row `y` carries the given background colour —
/// the dialog theme's selected-row inverse highlight uses
/// `bg = accent` (Yellow for this dialog) across the full inner row.
fn row_has_bg(buf: &Buffer, y: u16, bg: Color) -> bool {
    (0..buf.area.width).any(|x| buf[(x, y)].style().bg == Some(bg))
}

/// True iff any cell in the buffer carries the given foreground colour
/// (the dialog border paints in the accent colour).
fn buffer_has_fg(buf: &Buffer, fg: Color) -> bool {
    (0..buf.area.width).any(|x| (0..buf.area.height).any(|y| buf[(x, y)].style().fg == Some(fg)))
}

/// The y-range `(title row, footer row)` of the actions dialog in a
/// rendered buffer.
///
/// BOARD-023 R10: the board HEADER now paints the short `u Actions`
/// hint — a whole-buffer search for "Actions" matches the HEADER first.
/// The dialog title is therefore located as the LAST row containing
/// "Actions" (the dialog sits below the header; no body row contains
/// "Actions" as a label — R3 has no trigger row).
fn dialog_title_footer(buf: &Buffer) -> (u16, u16) {
    let footer = find_row(buf, "↑↓ Navigate").expect("dialog footer row");
    let title = (0..footer)
        .rev()
        .find(|&y| row_text(buf, y).contains("Actions"))
        .expect("dialog title row");
    assert!(
        title < footer,
        "title row {title} must precede footer row {footer}"
    );
    (title, footer)
}

/// First buffer row in the dialog's own body rows (`title+2..footer-1`)
/// that contains `needle` (panics when absent).
fn find_dialog_row(buf: &Buffer, needle: &str, title: u16, footer: u16) -> u16 {
    ((title + 2)..(footer - 1))
        .find(|&y| row_text(buf, y).contains(needle))
        .unwrap_or_else(|| panic!("row '{needle}' not found inside the dialog body"))
}

/// Seed a session so `/mux on` produces the Board | Agent grid.
async fn seed_session(app: &mut App, id: &str) {
    app.dispatch(codelet_fspec_tui::Action::SessionCreated(
        codelet_rpc_types::SessionId::new(id),
    ));
    drain_pending(app).await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 1: u opens the actions dialog from the single Board view
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_u_opens_the_actions_dialog_from_the_single_board_view() {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.mux_state().config().enabled, "mux must be disabled");

    // @step When I press the 'u' key
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is open overlaying the board with a Yellow accent border and the title 'Actions'
    assert!(
        app.compositor().contains(BOARD_ACTIONS_DIALOG_ID),
        "the actions dialog must be open on the compositor; layers={:?}",
        app.compositor().layer_ids()
    );
    let buf = render_app(&mut app);
    let text = buf_text(&buf);
    assert!(
        text.contains("Actions"),
        "the dialog must show its 'Actions' title; got:\n{text}"
    );
    assert!(
        buffer_has_fg(&buf, Color::Yellow),
        "the dialog border must paint in the Yellow accent colour"
    );

    // @step And the dialog body lists 8 rows in order: C Checkpoints, F Changed Files, D FOUNDATION.md, . New Agent, / Search, M Mux, ? Help, Esc Exit — each with a one-line description
    let (title, footer) = dialog_title_footer(&buf);
    let mut prev = 0u16;
    for label in &DIALOG_ROWS {
        let pos = find_dialog_row(&buf, label, title, footer);
        assert!(
            pos > prev,
            "row order must be top-to-bottom: '{label}' at {pos} not after {prev}"
        );
        prev = pos;
    }
    let dialog_rows: String = (title..=footer)
        .map(|y| row_text(&buf, y))
        .collect::<Vec<_>>()
        .join("\n");
    for desc in [
        "checkpoints view",
        "changed-files view",
        "browser",
        "agent",
        "Search work units",
        "mux layout",
        "help",
        "exit",
    ] {
        assert!(
            dialog_rows.contains(desc),
            "rows must carry one-line descriptions (missing '{desc}'); got:\n{dialog_rows}"
        );
    }

    // @step And the first row ('C Checkpoints') is highlighted and the board view is still visible underneath
    let y0 = find_dialog_row(&buf, "C Checkpoints", title, footer);
    assert!(
        row_has_bg(&buf, y0, Color::Yellow),
        "the first row ('C Checkpoints') must carry the inverse (Yellow bg) highlight"
    );
    assert!(
        text.contains("Checkpoints: None"),
        "the board header must still be visible underneath the dialog"
    );
    assert_eq!(app.active_view(), ViewMode::Board);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 2: U opens the actions dialog (case-insensitive trigger)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_uppercase_u_opens_the_actions_dialog_case_insensitive_trigger() {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.mux_state().config().enabled, "mux must be disabled");

    // @step When I press the 'U' key (uppercase)
    let _ = app.handle_event(&key(KeyCode::Char('U'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is open overlaying the board with the title 'Actions'
    assert!(
        app.compositor().contains(BOARD_ACTIONS_DIALOG_ID),
        "'U' (uppercase) must open the actions dialog; layers={:?}",
        app.compositor().layer_ids()
    );
    let buf = render_app(&mut app);
    let (title, footer) = dialog_title_footer(&buf);
    assert!(
        row_text(&buf, title).contains("Actions"),
        "the dialog title row must read 'Actions'; got: `{}`",
        row_text(&buf, title)
    );
    assert!(footer > title, "the footer must sit below the title");

    // @step And the first row ('C Checkpoints') is highlighted
    let y0 = find_dialog_row(&buf, "C Checkpoints", title, footer);
    assert!(
        row_has_bg(&buf, y0, Color::Yellow),
        "the first row ('C Checkpoints') must carry the inverse (Yellow bg) highlight"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 3: Ctrl+U does not open the actions dialog
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_ctrl_u_does_not_open_the_actions_dialog() {
    // @step Given I am in the single Board view with mux disabled
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I press Ctrl+U (Ctrl-chorded uppercase U)
    let _ = app.handle_event(&key(KeyCode::Char('U'), KeyModifiers::CONTROL));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is not open
    assert!(
        !app.compositor().contains(BOARD_ACTIONS_DIALOG_ID),
        "Ctrl+U must NOT open the actions dialog; layers={:?}",
        app.compositor().layer_ids()
    );

    // @step And the key falls through to the App-level handling
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must remain Board after Ctrl+U (no dialog, no crash)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 4: The actions dialog body lists all board shortcuts with descriptions
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_actions_dialog_body_lists_all_board_shortcuts_with_descriptions() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I open the actions dialog with 'u'
    open_actions_dialog(&mut app).await;
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step Then the dialog shows 8 rows in order: "C Checkpoints", "F Changed Files", "D FOUNDATION.md", ". New Agent", "/ Search", "M Mux", "? Help", "Esc Exit"
    let buf = render_app(&mut app);
    let (title, footer) = dialog_title_footer(&buf);
    let mut prev = 0u16;
    for label in &DIALOG_ROWS {
        let pos = find_dialog_row(&buf, label, title, footer);
        assert!(pos > prev, "row order broken at '{label}'");
        prev = pos;
    }

    // @step And each row shows its key, a label, and a one-line description
    for pair in [
        ("C Checkpoints", "- Open the checkpoints view"),
        ("F Changed Files", "- Open the changed-files view"),
        ("D FOUNDATION.md", "- Open FOUNDATION.md in the browser"),
        (". New Agent", "- Open/start an agent for the focused unit"),
        ("/ Search", "- Search work units"),
        ("M Mux", "- Open the mux layout config"),
        ("? Help", "- Show the full board keybinding help"),
        ("Esc Exit", "- Confirm exiting fspec"),
    ] {
        let y = find_dialog_row(&buf, pair.0, title, footer);
        let row = row_text(&buf, y);
        assert!(
            row.contains(pair.1),
            "row '{}' must show its description; got: `{row}`",
            pair.0
        );
    }

    // @step And the footer reads "↑↓ Navigate │ Enter Execute │ Esc Close"
    let text = buf_text(&buf);
    assert!(
        text.contains("↑↓ Navigate │ Enter Execute │ Esc Close"),
        "the dialog footer must advertise the bindings; got:\n{text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 5: The actions dialog has no trigger row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_actions_dialog_has_no_trigger_row() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I open the actions dialog with 'u'
    open_actions_dialog(&mut app).await;
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step Then the dialog shows exactly 8 rows
    let buf = render_app(&mut app);
    let (title, footer) = dialog_title_footer(&buf);
    // Spacious layout: title, gap, 8 body rows, gap, footer — so the
    // footer sits exactly 11 rows below the title (10 rows between).
    assert_eq!(
        footer - title,
        11,
        "the dialog body must contain exactly 8 rows (title+gap+8+gap+footer); got {} rows between",
        footer - title - 2
    );
    for label in &DIALOG_ROWS {
        find_dialog_row(&buf, label, title, footer);
    }

    // @step And no row in the dialog body advertises the 'u' trigger key
    let body: String = ((title + 2)..(footer - 1))
        .map(|y| row_text(&buf, y))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !body.contains("u Actions") && !body.contains(", Keys"),
        "no body row may advertise the trigger key; body:\n{body}"
    );

    // @step And the dialog body does not contain the text "Actions" as a row label other than the title
    assert!(
        !body.contains("Actions"),
        "the word 'Actions' must only appear in the dialog title; body:\n{body}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 6: Down moves the selection to the next row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_down_moves_the_selection_to_the_next_row() {
    // @step Given the actions dialog is open with "C Checkpoints" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog(&mut app).await;
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));
    let buf = render_app(&mut app);
    let (title, _footer) = dialog_title_footer(&buf);
    let y0 = find_dialog_row(&buf, "C Checkpoints", title, _footer);
    assert!(
        row_has_bg(&buf, y0, Color::Yellow),
        "'C Checkpoints' starts highlighted"
    );

    // @step When I press the Down arrow
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));

    // @step Then "F Changed Files" is highlighted
    let buf = render_app(&mut app);
    let y1 = find_dialog_row(&buf, "F Changed Files", title, _footer);
    assert!(
        row_has_bg(&buf, y1, Color::Yellow),
        "'F Changed Files' must be highlighted after Down"
    );
    assert!(
        !row_has_bg(&buf, y0, Color::Yellow),
        "'C Checkpoints' must no longer be highlighted"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 7: Up moves the selection to the previous row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_up_moves_the_selection_to_the_previous_row() {
    // @step Given the actions dialog is open with "F Changed Files" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog_at(&mut app, 1).await;
    let buf = render_app(&mut app);
    let (title, _footer) = dialog_title_footer(&buf);
    let y1 = find_dialog_row(&buf, "F Changed Files", title, _footer);
    assert!(
        row_has_bg(&buf, y1, Color::Yellow),
        "'F Changed Files' starts highlighted"
    );

    // @step When I press the Up arrow
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));

    // @step Then "C Checkpoints" is highlighted
    let buf = render_app(&mut app);
    let y0 = find_dialog_row(&buf, "C Checkpoints", title, _footer);
    assert!(
        row_has_bg(&buf, y0, Color::Yellow),
        "'C Checkpoints' must be highlighted after Up"
    );
    assert!(
        !row_has_bg(&buf, y1, Color::Yellow),
        "'F Changed Files' must no longer be highlighted"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 8: Up from the first row wraps to the last row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_up_from_the_first_row_wraps_to_the_last_row() {
    // @step Given the actions dialog is open with "C Checkpoints" (the first row) highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog(&mut app).await;

    // @step When I press the Up arrow
    let _ = app.handle_event(&key(KeyCode::Up, KeyModifiers::NONE));

    // @step Then "Esc Exit" (the last row) is highlighted
    let buf = render_app(&mut app);
    let (title, footer) = dialog_title_footer(&buf);
    assert!(
        row_has_bg(
            &buf,
            find_dialog_row(&buf, "Esc Exit", title, footer),
            Color::Yellow
        ),
        "Up from the first row must wrap to the last row ('Esc Exit')"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 9: Down from the last row wraps to the first row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_down_from_the_last_row_wraps_to_the_first_row() {
    // @step Given the actions dialog is open with "Esc Exit" (the last row) highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog_at(&mut app, 7).await;

    // @step When I press the Down arrow
    let _ = app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));

    // @step Then "C Checkpoints" (the first row) is highlighted
    let buf = render_app(&mut app);
    let (title, footer) = dialog_title_footer(&buf);
    assert!(
        row_has_bg(
            &buf,
            find_dialog_row(&buf, "C Checkpoints", title, footer),
            Color::Yellow
        ),
        "Down from the last row must wrap to the first row ('C Checkpoints')"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 10: Enter executes the highlighted action and closes the dialog
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_enter_executes_the_highlighted_action_and_closes_the_dialog() {
    // @step Given the actions dialog is open with "F Changed Files" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog_at(&mut app, 1).await;

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is closed
    assert!(
        !app.compositor().contains(BOARD_ACTIONS_DIALOG_ID),
        "the actions dialog must close on Enter"
    );

    // @step And the Changed Files view opens — the same outcome as pressing 'f' directly on the board
    assert_eq!(
        app.active_view(),
        ViewMode::ChangedFiles,
        "Enter on the 'F Changed Files' row must open the Changed Files view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 11: Enter on the Mux row opens the Mux config dialog
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_enter_on_the_mux_row_opens_the_mux_config_dialog() {
    // @step Given the actions dialog is open with "M Mux" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog_at(&mut app, 5).await;

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is closed
    assert!(!app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step And the Mux config dialog is open seeded from the live mux config — the same dialog bare /mux opens
    assert!(
        app.compositor().contains("mux-config-dialog"),
        "Enter on the 'M Mux' row must open the Mux config dialog; layers={:?}",
        app.compositor().layer_ids()
    );
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must stay Board"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 12: Enter on the Help row opens the board help dialog
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_enter_on_the_help_row_opens_the_board_help_dialog() {
    // @step Given the actions dialog is open with "? Help" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog_at(&mut app, 6).await;

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is closed
    assert!(!app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step And the board Help dialog (the one '?' opens) is open on top
    assert!(
        app.compositor().contains("help-dialog"),
        "Enter on the '? Help' row must open the board HelpDialog; layers={:?}",
        app.compositor().layer_ids()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 13: Enter on the Exit row opens the exit confirmation
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_enter_on_the_exit_row_opens_the_exit_confirmation() {
    // @step Given the actions dialog is open with "Esc Exit" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog_at(&mut app, 7).await;

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is closed
    assert!(!app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step And the "Exit fspec?" confirmation dialog is open with Exit pre-selected
    assert!(
        app.compositor().contains("board-exit-confirmation-dialog"),
        "Enter on the 'Esc Exit' row must open the exit confirmation; layers={:?}",
        app.compositor().layer_ids()
    );
    assert!(
        !app.should_quit(),
        "merely opening the confirmation must not quit"
    );

    // @step And pressing Esc inside the confirmation cancels it and I stay on the board
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert!(
        !app.compositor().contains("board-exit-confirmation-dialog"),
        "Esc inside the confirmation must cancel it"
    );
    assert_eq!(app.active_view(), ViewMode::Board);
    assert!(!app.should_quit());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 14: Esc closes the dialog without executing the highlighted row
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_esc_closes_the_dialog_without_executing_the_highlighted_row() {
    // @step Given the actions dialog is open with "C Checkpoints" highlighted
    let (mut app, _mock) = fresh_app();
    open_actions_dialog(&mut app).await;
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step When I press Esc
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is closed
    assert!(!app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step And the checkpoints view is NOT open
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Esc must NOT execute the highlighted 'C Checkpoints' row"
    );

    // @step And the exit confirmation is NOT open
    assert!(
        !app.compositor().contains("board-exit-confirmation-dialog"),
        "Esc must NEVER trigger the exit confirmation"
    );

    // @step And I am still on the board
    assert!(!app.should_quit());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 15: Keys behind the dialog never leak to the board
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_keys_behind_the_dialog_never_leak_to_the_board() {
    // @step Given the actions dialog is open
    let (mut app, _mock) = fresh_app();
    open_actions_dialog(&mut app).await;
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step And a work unit below the current selection is available in the focused column
    app.board_store_mut().replace_work_units(vec![
        make_unit("AUTH-001", "backlog", "story"),
        make_unit("AUTH-002", "backlog", "story"),
    ]);
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When I press 'j' then 'f'
    let _ = app.handle_event(&key(KeyCode::Char('j'), KeyModifiers::NONE));
    let _ = app.handle_event(&key(KeyCode::Char('f'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the board selection has NOT moved
    assert_eq!(
        app.board_store().selected_index_for("backlog"),
        0,
        "'j' must not leak through the modal to the board selection"
    );

    // @step And the Changed Files view has NOT opened
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "'f' must not leak through the modal to the board"
    );

    // @step And the actions dialog is still open
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 16: Pressing u again while open is a no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_pressing_u_again_while_open_is_a_no_op() {
    // @step Given the actions dialog is open
    let (mut app, _mock) = fresh_app();
    open_actions_dialog(&mut app).await;
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));

    // @step When I press 'u' again
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog remains open exactly once (no stacked second layer)
    let count = app
        .compositor()
        .layer_ids()
        .iter()
        .filter(|id| id.as_str() == BOARD_ACTIONS_DIALOG_ID)
        .count();
    assert_eq!(
        count,
        1,
        "a second 'u' must not stack a second dialog layer; layers={:?}",
        app.compositor().layer_ids()
    );
    assert!(app.compositor().contains(BOARD_ACTIONS_DIALOG_ID));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 17: The board header chord row is replaced by a short Actions hint
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_board_header_chord_row_is_replaced_by_a_short_actions_hint() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I look at the board header's keybinding hint row
    let buf = render_app(&mut app);
    // The checkpoint-status row (header row 0) sits 3 rows above the
    // keybinding hint row (header row 3).
    let status_y =
        find_row(&buf, "Checkpoints:").expect("the checkpoint status row must still render");
    let hint_y = status_y + 3;
    let hint = row_text(&buf, hint_y);

    // @step Then the row reads "u Actions" as a single plain foreground span
    assert!(
        hint.contains("u Actions"),
        "the header hint row must read 'u Actions'; got: `{hint}`"
    );

    // @step And the old chord "C Checkpoints ◆ F Changed Files ◆ D FOUNDATION.md ◆ . New Agent ◆ / Search ◆ M Mux" is no longer rendered in the header
    let text = buf_text(&buf);
    assert!(
        !text.contains("C Checkpoints ◆"),
        "the old six-action chord must no longer render in the header"
    );
    assert!(
        !hint.contains('◆'),
        "the hint row must be a single short hint, got: `{hint}`"
    );

    // @step And the checkpoint status row above it is unchanged
    assert!(row_text(&buf, status_y).contains("Checkpoints: None"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 18: u opens the actions dialog from the focused Board pane in mux mode
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_u_opens_the_actions_dialog_from_the_focused_board_pane_in_mux_mode() {
    // @step Given I am in mux mode with the Board | Agent grid and the Board pane focused
    let (mut app, _mock) = fresh_app();
    seed_session(&mut app, "s-1").await;
    app.dispatch(codelet_fspec_tui::Action::InputSubmitted(
        "/mux on".to_string(),
    ));
    drain_pending(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux, "mux mode must be active");
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the Board pane must be focused"
    );

    // @step When I press the 'u' key
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the actions dialog is open overlaying the whole grid
    assert!(
        app.compositor().contains(BOARD_ACTIONS_DIALOG_ID),
        "'u' must open the actions dialog from the focused Board pane"
    );

    // @step And the view remains Mux while the dialog is open
    assert_eq!(app.active_view(), ViewMode::Mux);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario 19: The board help dialog advertises the u keybinding
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_the_board_help_dialog_advertises_the_u_keybinding() {
    // @step Given I am in the single Board view
    let (mut app, _mock) = fresh_app();
    assert_eq!(app.active_view(), ViewMode::Board);

    // @step When I open the board help dialog with '?'
    let _ = app.handle_event(&key(KeyCode::Char('?'), KeyModifiers::NONE));
    drain_pending(&mut app).await;
    assert!(app.compositor().contains("help-dialog"));

    // @step Then the help lists a row advertising 'u' opens the actions dialog (key column 'u', label 'Actions')
    // 200x60 fits the full board help list without scrolling (the
    // rpc397 full-content size).
    let buf = render_app_at(&mut app, 200, 60);
    let text = buf_text(&buf);
    // The key column is 14 wide (see "m             Mux layout").
    assert!(
        text.contains("u             Actions"),
        "the board help must list the 'u Actions' row; got:\n{text}"
    );
    assert!(
        !text.contains("Keybindings dialog"),
        "the old comma-era help row must be gone; got:\n{text}"
    );
}
