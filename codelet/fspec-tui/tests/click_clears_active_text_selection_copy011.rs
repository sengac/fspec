//! Feature: spec/features/click-clears-active-text-selection.feature
//!
//! COPY-011 — a quick click (left Down then Up, no drag, no long-press)
//! over an ACTIVE text selection must clear the selection and its
//! REVERSED highlight and copy nothing, on every surface. These tests
//! first ESTABLISH an active selection, then quick-click, and assert the
//! selection is gone / highlight span count == 0 / the clipboard is
//! unchanged since the original copy. The clear-on-click scenarios are
//! EXPECTED to be RED against the current code (the recognizer emits no
//! gesture for a quick click, so the prior selection persists). Two
//! guards — a new drag replaces the old selection, and a quick click with
//! no prior selection stays inert — already pass.

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
// Scenario: A quick click clears an active scrollback selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_clears_an_active_scrollback_selection() {
    // @step Given a scrollback with an active highlighted text selection and mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    let _ = seed_line(&mut app, "Hello world");
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;
    // Drag the whole row (col 0 → far edge) then release to commit + keep
    // the highlight, establishing an active selection.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 79, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 79, rect_y));
    drain(&mut app);
    render_app(&mut app, 80, 40);
    assert!(selection_active(&app), "precondition: selection is active");
    assert!(highlight_spans(&app) > 0, "precondition: highlight painted");
    let clip_before = clip_bytes(&clip);
    assert!(!clip_before.is_empty(), "precondition: original copy happened");

    // @step When I quickly click a line without dragging
    // Down then immediate Up on the SAME cell, no drag, no long-press tick.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 4, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 4, rect_y));
    drain(&mut app);

    // @step Then the scrollback selection is cleared and its highlight is removed
    assert!(
        !selection_active(&app),
        "a quick click must clear the active selection (rule [1])"
    );
    render_app(&mut app, 80, 40);
    assert_eq!(
        highlight_spans(&app),
        0,
        "a quick click must remove the highlight overlay"
    );

    // @step And nothing new is written to the clipboard by the click
    assert_eq!(
        clip_bytes(&clip),
        clip_before,
        "the click must not write anything new to the clipboard"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A quick click clears an active input composer selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_clears_an_active_input_composer_selection() {
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

    // @step Given an input composer with an active text selection
    let mut input = MultiLineInput::new();
    input.set_value("the quick brown fox");
    let area = Rect::new(0, 0, 60, 6);
    let _ = input.handle_mouse(cmouse(MouseEventKind::Down(MouseButton::Left), BODY_X, 0), area);
    let _ = input.handle_mouse(cmouse(MouseEventKind::Drag(MouseButton::Left), BODY_X + 8, 0), area);
    let _ = input.handle_mouse(cmouse(MouseEventKind::Up(MouseButton::Left), BODY_X + 8, 0), area);
    assert!(
        input.text_selection_active(),
        "precondition: composer selection is active"
    );

    // @step When I quickly click without dragging
    // Down then immediate Up on the SAME cell, no drag in between.
    let _ = input.handle_mouse(cmouse(MouseEventKind::Down(MouseButton::Left), BODY_X + 2, 0), area);
    let _ = input.handle_mouse(cmouse(MouseEventKind::Up(MouseButton::Left), BODY_X + 2, 0), area);

    // @step Then the composer selection is cleared
    assert!(
        !input.text_selection_active(),
        "a quick click must clear the composer selection (rule [1])"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A quick click clears an active turn-content modal selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_clears_an_active_turn_content_modal_selection() {
    use modal::{
        clip_bytes as mclip, drain_app, modal_body_rect, modal_selection_active, open_modal_app,
    };

    // @step Given an open turn-content modal with an active text selection
    let body = "MLINE0\nMLINE1\nMLINE2\nMLINE3";
    let (mut app, clip) = open_modal_app(body, 80, 24);
    let r = modal_body_rect(80, 24, body);
    let _ = app.handle_event(&modal::mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y));
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
    assert!(
        modal_selection_active(&app),
        "precondition: modal selection is active"
    );
    let clip_before = mclip(&clip);

    // @step When I quickly click in the modal body without dragging
    let _ = app.handle_event(&modal::mouse(
        MouseEventKind::Down(MouseButton::Left),
        r.x + 2,
        r.y + 1,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&modal::mouse(
        MouseEventKind::Up(MouseButton::Left),
        r.x + 2,
        r.y + 1,
    ));
    drain_app(&mut app);

    // @step Then the modal selection is cleared
    assert!(
        !modal_selection_active(&app),
        "a quick click must clear the modal selection (rule [1])"
    );

    // @step And nothing new is written to the clipboard by the click
    assert_eq!(
        mclip(&clip),
        clip_before,
        "the click must not write anything new to the clipboard"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A quick click clears an active board details strip selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_clears_an_active_board_details_strip_selection() {
    use board::{board_with_clipboard, details_rect, strip_selection_active, wu};

    fn bdrain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>) {
        while rx.try_recv().is_ok() {}
    }

    // @step Given a board details strip with an active text selection
    let units = vec![wu("RPC-014", "Board grid", "backlog", None)];
    let (view, store, mut rx, _clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    let _ = view.handle_event(&board::mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y), &store);
    bdrain(&mut rx);
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Drag(MouseButton::Left), r.x + 6, r.y),
        &store,
    );
    bdrain(&mut rx);
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Up(MouseButton::Left), r.x + 6, r.y),
        &store,
    );
    bdrain(&mut rx);
    assert!(
        strip_selection_active(&view),
        "precondition: strip selection is active"
    );

    // @step When I quickly click inside the strip without dragging
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Down(MouseButton::Left), r.x + 4, r.y),
        &store,
    );
    bdrain(&mut rx);
    let _ = view.handle_event(
        &board::mouse(MouseEventKind::Up(MouseButton::Left), r.x + 4, r.y),
        &store,
    );
    bdrain(&mut rx);

    // @step Then the strip selection is cleared
    assert!(
        !strip_selection_active(&view),
        "a quick click must clear the strip selection (rule [1])"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Starting a new drag replaces the old selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn starting_a_new_drag_replaces_the_old_selection() {
    // @step Given a scrollback with an active highlighted text selection and mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    let _ = seed_line(&mut app, "Hello world");
    let _ = seed_line(&mut app, "Second line");
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;
    // Establish an active selection on the first row ("Hello world").
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 79, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 79, rect_y));
    drain(&mut app);
    assert!(selection_active(&app), "precondition: selection is active");
    let clip_before = clip_bytes(&clip);

    // @step When I press and drag to select different text and release
    // A fresh Down → Drag → Up over the SECOND row ("Second line").
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, rect_y + 1));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 79, rect_y + 1));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 79, rect_y + 1));
    drain(&mut app);

    // @step Then the old selection is replaced by the new one
    assert!(
        selection_active(&app),
        "the new drag must leave a fresh active selection"
    );

    // @step And the newly selected text is written to the clipboard
    let appended = clip_bytes(&clip)[clip_before.len()..].to_vec();
    assert_eq!(
        appended,
        osc52("Second line"),
        "the new drag must copy the newly selected text"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A quick click with no active selection stays inert
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_with_no_active_selection_stays_inert() {
    // @step Given a scrollback with no active text selection and mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    let _ = seed_line(&mut app, "Hello world");
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;
    assert!(!selection_active(&app), "precondition: no selection active");

    // @step When I quickly click a line without dragging
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 4, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 4, rect_y));
    drain(&mut app);

    // @step Then nothing is selected
    assert!(
        !selection_active(&app),
        "a quick click with no prior selection stays inert"
    );

    // @step And nothing is written to the clipboard
    assert!(
        clip_bytes(&clip).is_empty(),
        "a quick click must not write to the clipboard"
    );
}
