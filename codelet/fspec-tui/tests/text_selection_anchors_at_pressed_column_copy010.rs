//! Feature: spec/features/text-selection-anchors-at-pressed-column.feature
//!
//! COPY-010 — a drag selection must begin at the exact cell where the
//! mouse was pressed (its real row AND column), not at column 0. Today
//! the four view-level `Begin` handlers pin the anchor to column 0, so a
//! mid-line drag wrongly copies the whole line from its start. These
//! tests press at a MID-LINE column and assert a partial-from-that-column
//! selection across all four surfaces, plus a long-press regression guard
//! (whole line still copied) and a zero-width-drag guard (nothing
//! copied). The mid-line assertions are EXPECTED to be RED against the
//! current code.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{MouseButton, MouseEventKind};

mod common;

#[path = "common/text_selection_anchor_copy010_helpers.rs"]
mod sb;
use sb::{
    app_with_clipboard, clip_bytes, drain, highlight_spans, mouse, osc52, render_app,
    seed_line, selection_active,
};

#[path = "common/turn_content_modal_copy008_helpers.rs"]
mod modal;

#[path = "common/board_details_strip_copy009_helpers.rs"]
mod board;

use codelet_fspec_tui::Action;

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging from a mid-line column in the scrollback copies from
//           that column
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_from_a_mid_line_column_in_the_scrollback_copies_from_that_column() {
    // @step Given a scrollback whose visible row reads "Hello world" with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    let _ = seed_line(&mut app, "Hello world");
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16; // header row is 0; scrollback band starts at y=1.

    // @step When I press the left mouse button at column 6 of that row and drag to the end of the row and release
    // "Hello world": 'w' of "world" is at char index 6. Press at local
    // column 6 (rect x is 0), drag past the last content column, release.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 6, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 11, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 11, rect_y));
    drain(&mut app);

    // @step Then the clipboard receives "world"
    assert_eq!(
        clip_bytes(&clip),
        osc52("world"),
        "a drag from column 6 must copy from that column, not the line start"
    );

    // @step And the copied text does not start at the beginning of the line
    let bytes = clip_bytes(&clip);
    assert_ne!(
        bytes,
        osc52("Hello world"),
        "the copied text must NOT be the whole line from column 0"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging from a mid-word column in the input composer copies
//           from that column
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_from_a_mid_word_column_in_the_input_composer_copies_from_that_column() {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    use codelet_fspec_tui::views::agent::multiline_input::MultiLineInput;
    use crossterm::event::{KeyModifiers, MouseEvent};
    use ratatui::layout::Rect;

    // Body-relative origin x = area.x + INPUT_PAD_X(1) + PROMPT_WIDTH(2).
    const BODY_X: u16 = 3;
    fn cmouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }
    fn osc52_for(text: &str) -> Vec<u8> {
        let mut out = b"\x1b]52;c;".to_vec();
        out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
        out.push(0x07);
        out
    }

    // @step Given a composer whose visible row reads "the quick brown fox"
    let mut input = MultiLineInput::new();
    input.set_value("the quick brown fox");
    // Wide enough that "the quick brown fox" stays on one visual row.
    let area = Rect::new(0, 0, 60, 6);

    // @step When I press the left mouse button at the start of "brown" and drag to the end of "fox" and release
    // "the quick brown fox": 'b' of "brown" is at char index 10; the row
    // ends at char index 19. Press at body col 10, drag to body col 19.
    let _ = input.handle_mouse(cmouse(MouseEventKind::Down(MouseButton::Left), BODY_X + 10, 0), area);
    let _ = input.handle_mouse(cmouse(MouseEventKind::Drag(MouseButton::Left), BODY_X + 19, 0), area);
    let text = input
        .handle_mouse(cmouse(MouseEventKind::Up(MouseButton::Left), BODY_X + 19, 0), area)
        .expect("Commit must return the selected text");

    // @step Then the clipboard receives "brown fox"
    assert_eq!(
        text, "brown fox",
        "a drag from the 'b' of brown must copy from that column"
    );
    assert_eq!(
        osc52_for(&text),
        osc52_for("brown fox"),
        "the injected OSC 52 writer receives the mid-word bytes"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging from a mid-line column in the turn-content modal
//           copies from that column
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_from_a_mid_line_column_in_the_turn_content_modal_copies_from_that_column() {
    use modal::{clip_bytes as mclip, drain_app, modal_body_rect, open_modal_app, osc52 as mosc52};

    // @step Given an open turn-content modal showing a body line with known text and mouse capture enabled
    let body = "PREFIX_HEADER SUFFIX_TAIL";
    let (mut app, clip) = open_modal_app(body, 80, 24);
    let r = modal_body_rect(80, 24, body);

    // @step When I press the left mouse button at a mid-line column of the body and drag to the end of the line and release
    // 'S' of "SUFFIX_TAIL" is at char index 14; the line is 25 chars long.
    // Press at body col 14, drag past the content edge, release.
    let _ = app.handle_event(&modal::mouse(
        MouseEventKind::Down(MouseButton::Left),
        r.x + 14,
        r.y,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&modal::mouse(
        MouseEventKind::Drag(MouseButton::Left),
        r.x + r.width - 1,
        r.y,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&modal::mouse(
        MouseEventKind::Up(MouseButton::Left),
        r.x + r.width - 1,
        r.y,
    ));
    drain_app(&mut app);

    // @step Then the copied text starts at the pressed column and not at the start of the line
    let bytes = mclip(&clip);
    assert_eq!(
        bytes,
        mosc52("SUFFIX_TAIL"),
        "a drag from column 14 must copy from that column"
    );
    assert_ne!(
        bytes,
        mosc52(body),
        "the copied text must NOT be the whole line from column 0"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging from a mid-title column in the board details strip
//           copies from that column
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_from_a_mid_title_column_in_the_board_details_strip_copies_from_that_column() {
    use board::{
        board_with_clipboard, buffer_row_text, clip_bytes as bclip, details_rect, osc52 as bosc52,
        render, wu,
    };

    // @step Given a board with a work unit selected and its details strip visible
    let units = vec![wu("RPC-014", "Board grid", "backlog", None)];
    let (view, store, mut rx, clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    let buf = render(&view, &store, 120, 30);
    let full = buffer_row_text(&buf, r.x, r.y, r.width);
    assert!(
        full.starts_with("RPC-014: Board grid"),
        "precondition: id:title row text, got `{full}`"
    );
    // Press at the 'B' of "Board" — char index 9 in "RPC-014: Board grid".
    let press_col = 9u16;
    let expected: String = full.chars().skip(press_col as usize).collect();
    fn bdrain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>) {
        while rx.try_recv().is_ok() {}
    }

    // @step When I press the left mouse button at a mid-title column of the id and title row and drag to the end of the title and release
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Down(MouseButton::Left), r.x + press_col, r.y),
        &store,
    );
    bdrain(&mut rx);
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Drag(MouseButton::Left), r.x + r.width - 1, r.y),
        &store,
    );
    bdrain(&mut rx);
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Up(MouseButton::Left), r.x + r.width - 1, r.y),
        &store,
    );
    bdrain(&mut rx);

    // @step Then the copied text starts at the pressed column and not at the start of the line
    let bytes = bclip(&clip);
    assert_eq!(
        bytes,
        bosc52(&expected),
        "a drag from the mid-title column must copy from that column"
    );
    assert_ne!(
        bytes,
        bosc52(&full),
        "the copied text must NOT be the whole title from column 0"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A long-press with no drag still selects and copies the whole
//           line  (REGRESSION GUARD)
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_long_press_with_no_drag_still_selects_and_copies_the_whole_line() {
    // @step Given a scrollback whose visible row reads "whole line text" with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    let _ = seed_line(&mut app, "whole line text");
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;

    // @step When I press and hold the left mouse button on that row for about half a second without moving and release
    // Press mid-line at column 5 (NOT column 0) so this also proves that
    // a stationary long-press still selects the WHOLE line regardless of
    // the pressed column.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 5, rect_y));
    drain(&mut app);
    std::thread::sleep(std::time::Duration::from_millis(450));
    app.poll_selection_tick_for_test();
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 5, rect_y));
    drain(&mut app);

    // @step Then the whole line text is written to the clipboard
    assert_eq!(
        clip_bytes(&clip),
        osc52("whole line text"),
        "a stationary long-press must still copy the whole line"
    );

    // @step And the selection stays highlighted
    assert!(
        selection_active(&app),
        "the long-press selection must persist after commit"
    );
    render_app(&mut app, 80, 40);
    assert!(
        highlight_spans(&app) > 0,
        "the highlight overlay must still be painted after copy"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A zero-width drag copies nothing
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_zero_width_drag_copies_nothing() {
    // @step Given a scrollback whose visible row reads "Hello world" with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    let _ = seed_line(&mut app, "Hello world");
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;

    // @step When I press and release the left mouse button on the same cell without moving
    // Down then immediate Up on the SAME cell, with NO drag and NO
    // long-press tick between — a zero-width gesture.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 6, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 6, rect_y));
    drain(&mut app);

    // @step Then nothing is written to the clipboard
    assert!(
        clip_bytes(&clip).is_empty(),
        "a zero-width drag must not write to the clipboard"
    );
}
