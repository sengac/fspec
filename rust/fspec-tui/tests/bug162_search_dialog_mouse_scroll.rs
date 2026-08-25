//! BUG-162 — Board search dialog: the match list scrolls with the mouse wheel.
//!
//! Feature: spec/features/board-search-dialog-mouse-wheel-scroll.feature
//!
//! ACDD TESTING phase: these tests assert the BUG-162 behaviour — the
//! WorkUnitSearchDialog handles `Event::Mouse`: ScrollDown/ScrollUp inside
//! the dialog's last-rendered rect move the highlighted match by the
//! WheelVelocity step (1x–5x ramp) and update `scroll_offset` via
//! `ensure_visible` (Consumed); wheel events outside the rect are Ignored
//! so they bubble to the BoardView behind the modal; and when the matches
//! overflow the visible rows a proportional scrollbar gutter
//! (`render_list_scrollbar` + `ScrollbarDrag`) is painted and its rect is
//! cached for hit-testing.
//!
//! RED note: `WorkUnitSearchDialog::handle_event` currently matches only
//! `Event::Key` and returns `Ignored` for `Event::Mouse`, and the
//! `scroll_offset()` / `last_dialog_rect()` / `last_scrollbar_rect()`
//! accessors do not exist yet, so this file is RED until the BUG-162
//! implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{Component, WorkUnitSearchDialog};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

fn wu(id: &str, title: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// A dialog with `count` units (UNIT-001 .. UNIT-0NN).
fn dialog_with(count: usize) -> WorkUnitSearchDialog {
    let units: Vec<WorkUnitInfo> = (1..=count)
        .map(|i| wu(&format!("UNIT-{i:03}"), &format!("Title {i}")))
        .collect();
    WorkUnitSearchDialog::new(units)
}

/// Render the dialog into an 80x24 TestBackend (caches the dialog rect and
/// the visible-rows window) and return the buffer.
fn render_80x24(dialog: &mut WorkUnitSearchDialog) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
    term.draw(|frame| dialog.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn wheel(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// A point inside the dialog's last-rendered rect (center of the rect).
fn inside_point(dialog: &WorkUnitSearchDialog) -> (u16, u16) {
    let rect = dialog
        .last_dialog_rect()
        .expect("last_dialog_rect must be cached after render");
    (rect.x + rect.width / 2, rect.y + rect.height / 2)
}

/// Scenario: Scrolling the mouse wheel down inside the dialog moves the selection down
#[test]
fn scrolling_the_mouse_wheel_down_inside_the_dialog_moves_the_selection_down() {
    // @step Given the work-unit search dialog is open with more matches than visible rows
    let mut dialog = dialog_with(20);
    render_80x24(&mut dialog);
    let (x, y) = inside_point(&dialog);
    let visible = dialog.visible_rows();
    assert!(
        dialog.matches().len() > visible,
        "20 matches must overflow the {visible} visible rows"
    );

    // @step When I scroll the mouse wheel down inside the dialog
    let result = dialog.handle_event(&wheel(MouseEventKind::ScrollDown, x, y));

    // @step Then the dialog consumes the event
    assert!(
        result.is_consumed(),
        "ScrollDown inside the dialog rect must be CONSUMED"
    );

    // @step And the highlighted match moves down by the wheel step
    assert_eq!(
        dialog.selected_index(),
        1,
        "a single fresh notch is a 1x step: selection 0 → 1"
    );

    // @step And the scroll offset follows the highlighted match
    assert_eq!(
        dialog.scroll_offset(),
        0,
        "row 1 is still inside the window starting at offset 0"
    );
}

/// Scenario: Scrolling the mouse wheel up inside the dialog moves the selection up
#[test]
fn scrolling_the_mouse_wheel_up_inside_the_dialog_moves_the_selection_up() {
    // @step Given the work-unit search dialog is open with the selection past the first visible row
    let mut dialog = dialog_with(20);
    render_80x24(&mut dialog);
    // Move the selection down twice with the keyboard so the scroll
    // offset is non-zero, then wheel back up.
    use crossterm::event::{KeyCode, KeyEvent};
    for _ in 0..2 {
        let _ = dialog.handle_event(&Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
    }
    assert_eq!(dialog.selected_index(), 2);
    let (x, y) = inside_point(&dialog);

    // @step When I scroll the mouse wheel up inside the dialog
    let result = dialog.handle_event(&wheel(MouseEventKind::ScrollUp, x, y));

    // @step Then the dialog consumes the event
    assert!(
        result.is_consumed(),
        "ScrollUp inside the dialog rect must be CONSUMED"
    );

    // @step And the highlighted match moves up by the wheel step
    assert_eq!(
        dialog.selected_index(),
        1,
        "a single fresh notch is a 1x step: selection 2 → 1"
    );
}

/// Scenario: Rapid wheel notches ramp the wheel velocity
#[test]
fn rapid_wheel_notches_ramp_the_wheel_velocity() {
    // @step Given the work-unit search dialog is open with a long match list
    let mut dialog = dialog_with(20);
    render_80x24(&mut dialog);
    let (x, y) = inside_point(&dialog);

    // @step When I scroll the mouse wheel down five times in rapid succession
    for _ in 0..5 {
        let result = dialog.handle_event(&wheel(MouseEventKind::ScrollDown, x, y));
        assert!(result.is_consumed(), "every notch must be CONSUMED");
    }

    // @step Then the fifth step moves the selection by five rows
    // WheelVelocity ramps 1,2,3,4,5 within the 150 ms gap (back-to-back
    // notches in a test are microseconds apart), so the total move is
    // 1+2+3+4+5 = 15.
    assert_eq!(
        dialog.selected_index(),
        15,
        "five rapid notches ramp to 5x: total move 1+2+3+4+5 = 15"
    );
}

/// Scenario: A wheel event outside the dialog rect is ignored so it bubbles to the board
#[test]
fn a_wheel_event_outside_the_dialog_rect_is_ignored_so_it_bubbles_to_the_board() {
    // @step Given the work-unit search dialog is open with a match list
    let mut dialog = dialog_with(20);
    render_80x24(&mut dialog);
    let rect = dialog
        .last_dialog_rect()
        .expect("last_dialog_rect must be cached after render");
    // One row above the dialog's top border — outside the rect.
    let outside_row = rect.y.saturating_sub(1);

    // @step When I scroll the mouse wheel down outside the dialog rect
    let result = dialog.handle_event(&wheel(
        MouseEventKind::ScrollDown,
        rect.x + rect.width / 2,
        outside_row,
    ));

    // @step Then the dialog ignores the event
    assert!(
        !result.is_consumed(),
        "a wheel event outside the dialog rect must be Ignored so it bubbles to the board"
    );

    // @step And the selection and scroll offset are unchanged
    assert_eq!(dialog.selected_index(), 0, "selection must stay frozen");
    assert_eq!(dialog.scroll_offset(), 0, "scroll offset must stay frozen");
}

/// Scenario: Repeated wheel-down reaches the last match and keeps it on screen
#[test]
fn repeated_wheel_down_reaches_the_last_match_and_keeps_it_on_screen() {
    // @step Given the work-unit search dialog is open with more matches than visible rows
    // 21 matches: with the WheelVelocity ramp (1,2,3,4,5,5,...) the
    // cumulative move after 6 notches is 1+2+3+4+5+5 = 20, which lands
    // exactly on the last index (20) of a 21-item list — wrap_index
    // never overshoots past it.
    let mut dialog = dialog_with(21);
    render_80x24(&mut dialog);
    let (x, y) = inside_point(&dialog);
    let visible = dialog.visible_rows();
    let last = dialog.matches().len() - 1;

    // @step When I scroll the mouse wheel down until the last match is highlighted
    // wrap_index wraps, so keep scrolling until the selection lands on the
    // last match (at most `total` notches guarantees a full lap).
    for _ in 0..dialog.matches().len() {
        if dialog.selected_index() == last {
            break;
        }
        let _ = dialog.handle_event(&wheel(MouseEventKind::ScrollDown, x, y));
    }

    // @step Then the last match is highlighted
    assert_eq!(
        dialog.selected_index(),
        last,
        "repeated ScrollDown must reach the last match"
    );

    // @step And the scroll offset keeps the last match inside the visible window
    let offset = dialog.scroll_offset();
    assert!(
        offset <= last && last < offset + visible,
        "ensure_visible must keep the last match on screen (offset {offset}, visible {visible})"
    );
}

/// Scenario: A proportional scrollbar gutter is painted when matches overflow the visible rows
#[test]
fn a_proportional_scrollbar_gutter_is_painted_when_matches_overflow_the_visible_rows() {
    // @step Given the work-unit search dialog is open with more matches than visible rows
    let mut dialog = dialog_with(20);
    let buf = render_80x24(&mut dialog);
    let visible = dialog.visible_rows();
    assert!(
        dialog.matches().len() > visible,
        "20 matches must overflow the {visible} visible rows"
    );

    // @step When the dialog is rendered
    let gutter = dialog
        .last_scrollbar_rect()
        .expect("the scrollbar gutter rect must be cached when matches overflow");

    // @step Then a scrollbar gutter rect is cached for hit-testing
    assert_eq!(gutter.width, 1, "the gutter is a single column");
    let rect = dialog
        .last_dialog_rect()
        .expect("last_dialog_rect must be cached after render");
    assert!(
        gutter.x >= rect.x && gutter.x < rect.x + rect.width,
        "the gutter must sit inside the dialog rect"
    );

    // @step And the gutter is painted in the rightmost body column
    // The shared render_list_scrollbar paints '■' (thumb) over '│' (track).
    let mut painted = 0;
    for y in gutter.y..gutter.y + gutter.height {
        let sym = buf[(gutter.x, y)].symbol();
        if sym == "■" || sym == "│" {
            painted += 1;
        }
    }
    assert_eq!(
        painted,
        gutter.height as usize,
        "every gutter row must be painted with the scrollbar glyphs"
    );
}
