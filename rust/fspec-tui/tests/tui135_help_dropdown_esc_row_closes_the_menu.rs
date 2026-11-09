//! TUI-135: the Help dropdown's Esc row reads 'Esc Close', and Enter or a
//! click on it closes the menu; Exit stays reachable from Esc on the board
//! and from the 'u' Menu bar dialog.
//!
//! Feature: spec/features/help-dropdown-esc-row-closes-the-menu.feature
//!
//! Harness: App + MockBackend (the `bug196_menu_dropdown_row_click.rs`
//! pattern): keys and clicks go through `App::handle_event`, the queued
//! actions are drained, and the full App is rendered into a TestBackend
//! (board 120x24, mux 240x24). Deterministic: no clock, no network.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

use codelet_fspec_tui::components::menu_bar::items::HELP;
use codelet_fspec_tui::{App, FspecBackend, MuxConfig, MuxLayout, MuxPaneKind, ViewMode};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

const BOARD_EXIT_DIALOG_ID: &str = "board-exit-confirmation-dialog";
const MENU_BAR_HELP_DIALOG_ID: &str = "board-actions-dialog";
/// The Help item's index in the Zone A ring (Kanban, Tools, Settings, Help).
const HELP_ITEM: usize = 3;
/// The Close row's index in the Help dropdown.
const CLOSE_ROW: usize = 1;

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    App::new(backend)
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn render_app(app: &mut App, w: u16, h: u16) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(w, h)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The board's 2-zone menu bar row (3 rows below the `Checkpoints:` row).
fn bar_row(buf: &Buffer) -> Result<u16, codelet_test_helpers::TestError> {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .or_fail("the checkpoint status row must render")?;
    Ok(status_y + 3)
}

/// The y of the first row below `from` whose text contains `needle`.
fn row_y_below(
    buf: &Buffer,
    from: u16,
    needle: &str,
) -> Result<u16, codelet_test_helpers::TestError> {
    Ok((from..buf.area.height)
        .find(|&y| row_text(buf, y).contains(needle))
        .or_fail(&format!("no row containing '{needle}' below row {from}"))?)
}

/// Walk from the last kanban column onto the bar, Right to the Help item,
/// open its dropdown with Enter and move the cursor down to `cursor_row`.
/// Works for the board bar and the mux bar (same ring entry).
async fn open_help_dropdown_at(
    app: &mut App,
    cursor_row: usize,
) -> Result<(), codelet_test_helpers::TestError> {
    app.board_store_mut().set_focused_column("blocked");
    // Drain after every step: the mux bar walks on dispatched actions.
    for _ in 0..=HELP_ITEM {
        app.handle_event(&key(KeyCode::Right));
        drain_pending(app).await?;
    }
    app.handle_event(&key(KeyCode::Enter));
    drain_pending(app).await?;
    for _ in 0..cursor_row {
        app.handle_event(&key(KeyCode::Down));
    }
    drain_pending(app).await?;
    Ok(())
}

/// Enable mux with a Board and an Agent pane, focus on the Board pane.
fn enable_mux(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    let layout = MuxLayout::columns(&[MuxPaneKind::Board, MuxPaneKind::Agent])
        .or_fail("a Board|Agent layout")?;
    let config = MuxConfig {
        layout,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
    Ok(())
}

#[tokio::test]
async fn the_help_dropdowns_esc_row_reads_esc_close() -> TestResult {
    // @step Given the board with the Help dropdown open
    let mut app = fresh_app();
    open_help_dropdown_at(&mut app, 0).await?;
    assert_eq!(app.board_store().open_menu(), Some((HELP_ITEM, 0)));

    // @step When the dropdown is painted
    let buf = render_app(&mut app, 120, 24)?;
    let bar = bar_row(&buf)?;

    // @step Then its rows read "? Help" and "Esc Close"
    let help_y = row_y_below(&buf, bar + 2, "? ")?;
    assert!(
        row_text(&buf, help_y).contains("Help"),
        "{}",
        row_text(&buf, help_y)
    );
    let close_y = row_y_below(&buf, bar + 2, "Esc ")?;
    assert_eq!(close_y, help_y + 1, "the Esc row is the second row");
    assert!(
        row_text(&buf, close_y).contains("Close"),
        "the Esc row must read 'Esc Close': {}",
        row_text(&buf, close_y)
    );
    assert_eq!((HELP[1].key, HELP[1].label), ("Esc", "Close"));

    // @step And the Esc row's description reads "Close this menu"
    assert_eq!(HELP[1].description, "Close this menu");

    // @step And no dropdown row reads "Esc Exit"
    assert!(
        (bar + 1..buf.area.height).all(|y| !row_text(&buf, y).contains("Esc Exit")),
        "no dropdown row may read 'Esc Exit' (the board footer's 'ESC Exit' is not a row)"
    );
    Ok(())
}

#[tokio::test]
async fn enter_on_the_close_row_closes_the_menu_and_opens_nothing() -> TestResult {
    // @step Given the board with the Help dropdown open on the Close row
    let mut app = fresh_app();
    open_help_dropdown_at(&mut app, CLOSE_ROW).await?;
    assert_eq!(app.board_store().open_menu(), Some((HELP_ITEM, CLOSE_ROW)));
    let layers_before = app.compositor().len();

    // @step When the user presses Enter
    app.handle_event(&key(KeyCode::Enter));
    drain_pending(&mut app).await?;

    // @step Then the dropdown is closed
    assert_eq!(app.board_store().open_menu(), None);

    // @step And the bar item is de-selected
    assert!(app.board_store().menu_focus().is_none());

    // @step And no dialog opens
    assert_eq!(
        app.compositor().len(),
        layers_before,
        "no layer may open; layers={:?}",
        app.compositor().layer_ids()
    );
    assert!(!app.compositor().contains(BOARD_EXIT_DIALOG_ID));
    assert!(!app.should_quit());
    Ok(())
}

#[tokio::test]
async fn clicking_the_close_row_closes_the_menu_and_opens_nothing() -> TestResult {
    // @step Given the board with the Help dropdown open on the Help row
    let mut app = fresh_app();
    open_help_dropdown_at(&mut app, 0).await?;
    let buf = render_app(&mut app, 120, 24)?;
    let bar = bar_row(&buf)?;
    let close_y = row_y_below(&buf, bar + 2, "Close")?;
    // The Help panel anchors under the bracketed "[ Help ]" item at x49
    // (MENU-011); x50 is inside the panel.
    let click_x = 50u16;

    // @step When the user left-clicks the Close row
    app.handle_event(&click(click_x, close_y));
    drain_pending(&mut app).await?;

    // @step Then the dropdown is closed
    assert_eq!(app.board_store().open_menu(), None);

    // @step And the bar item is de-selected
    assert!(app.board_store().menu_focus().is_none());

    // @step And the exit confirmation does not open
    assert!(
        !app.compositor().contains(BOARD_EXIT_DIALOG_ID),
        "layers={:?}",
        app.compositor().layer_ids()
    );
    Ok(())
}

#[tokio::test]
async fn enter_on_the_mux_bars_close_row_closes_the_mux_menu() -> TestResult {
    // @step Given the mux view with its bar's Help dropdown open on the Close row
    let mut app = fresh_app();
    enable_mux(&mut app)?;
    open_help_dropdown_at(&mut app, CLOSE_ROW).await?;
    assert_eq!(
        app.navigator().mux.open_menu(),
        Some((HELP_ITEM, CLOSE_ROW))
    );
    let buf = render_app(&mut app, 240, 24)?;
    assert!(
        row_text(&buf, row_y_below(&buf, 2, "Esc ")?).contains("Close"),
        "the mux Help dropdown's Esc row must read 'Esc Close'"
    );
    let layers_before = app.compositor().len();

    // @step When the user presses Enter
    app.handle_event(&key(KeyCode::Enter));
    drain_pending(&mut app).await?;

    // @step Then the mux dropdown is closed
    assert_eq!(app.navigator().mux.open_menu(), None);

    // @step And the mux bar item is de-selected
    assert!(app.navigator().mux.menu_focus().is_none());

    // @step And no dialog opens
    assert_eq!(
        app.compositor().len(),
        layers_before,
        "layers={:?}",
        app.compositor().layer_ids()
    );
    assert_eq!(app.navigator().active_view, ViewMode::Mux);
    Ok(())
}

#[tokio::test]
async fn esc_on_the_board_with_nothing_open_still_opens_the_exit_confirmation() -> TestResult {
    // @step Given the board with no menu or dialog open
    let mut app = fresh_app();
    assert!(app.compositor().is_empty());
    assert_eq!(app.board_store().open_menu(), None);

    // @step When the user presses Esc
    app.handle_event(&key(KeyCode::Esc));
    drain_pending(&mut app).await?;

    // @step Then the exit confirmation opens
    assert!(
        app.compositor().contains(BOARD_EXIT_DIALOG_ID),
        "layers={:?}",
        app.compositor().layer_ids()
    );
    Ok(())
}

#[tokio::test]
async fn the_u_dialog_keeps_its_esc_exit_row() -> TestResult {
    // @step Given the u Menu bar dialog open on its last row
    let mut app = fresh_app();
    app.handle_event(&key(KeyCode::Char('u')));
    drain_pending(&mut app).await?;
    assert!(app.compositor().contains(MENU_BAR_HELP_DIALOG_ID));
    app.handle_event(&key(KeyCode::Up));
    let buf = render_app(&mut app, 120, 30)?;
    let exit_y = row_y_below(&buf, 0, "Esc Exit")?;
    let exit_row = row_text(&buf, exit_y);

    // @step When the user presses Enter
    app.handle_event(&key(KeyCode::Enter));
    drain_pending(&mut app).await?;

    // @step Then the exit confirmation opens
    assert!(
        app.compositor().contains(BOARD_EXIT_DIALOG_ID),
        "Enter on the last row must open the exit confirmation; layers={:?}",
        app.compositor().layer_ids()
    );
    assert!(!app.compositor().contains(MENU_BAR_HELP_DIALOG_ID));

    // @step And the row read "Esc Exit" with the description "Confirm exiting fspec"
    assert!(
        exit_row.contains("Confirm exiting fspec"),
        "the u dialog's last row: {exit_row}"
    );
    Ok(())
}
