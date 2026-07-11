//! Feature: spec/features/turn-content-modal-text-selection-copy.feature
//!
//! COPY-008 — wire selection + copy into the turn-content (message
//! selection) modal end-to-end. Each test opens the modal on a seeded
//! `ChunkSource` turn, drives real `Event::Mouse` events over the modal
//! BODY rect (derived from `turn_modal_geometry` / `fixed_dialog_rect`),
//! pumps the emitted `Action`s back through `App::dispatch`, and asserts
//! the observable side effects:
//!   - the injected OSC 52 clipboard writer's bytes (COPY-001), and
//!   - the modal selection state exposed by the new
//!     `AgentView.turn_modal_selection` seam (COPY-008).
//!
//! These tests target the INTENDED public surface and are EXPECTED to
//! fail to COMPILE until COPY-008 lands — the correct red state for ACDD.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{KeyCode, MouseButton, MouseEventKind};

mod common;

#[path = "common/turn_content_modal_copy008_helpers.rs"]
mod helpers;
use helpers::{
    clip_bytes, drain_app, key, modal_body_rect, modal_offset, modal_selection_active, modal_seq,
    mouse, open_modal_app, osc52, render_app,
};

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging across modal body lines copies their text and keeps
//           the highlight
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_across_modal_body_lines_copies_their_text_and_keeps_the_highlight() {
    // @step Given an open turn-content modal showing a message with mouse capture enabled
    let body = "MLINE0\nMLINE1\nMLINE2\nMLINE3";
    let (mut app, clip) = open_modal_app(body, 80, 24);
    assert_eq!(modal_seq(&app), Some(0), "modal must be open on the turn");
    let r = modal_body_rect(80, 24, body);

    // @step When I drag across three body lines and release
    // Down at the start of MLINE0, drag to the far content edge of MLINE2,
    // release — a linewise three-row selection.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        r.x + r.width - 1,
        r.y + 2,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Up(MouseButton::Left),
        r.x + r.width - 1,
        r.y + 2,
    ));
    drain_app(&mut app);

    // @step Then those lines' text without the scrollbar gutter is written to the clipboard
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52("MLINE0\nMLINE1\nMLINE2"),
        "drag over three body lines must copy all three gutter-free lines"
    );
    assert!(
        !bytes.windows(3).any(|w| w == [0xe2, 0x94, 0x82]),
        "the scrollbar glyph │ must NOT appear in the copied text"
    );

    // @step And the modal selection stays highlighted
    assert!(
        modal_selection_active(&app),
        "the modal selection must persist after commit (rule [4])"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Selection tracks the visible rows after scrolling the modal
// ─────────────────────────────────────────────────────────────────────

#[test]
fn selection_tracks_the_visible_rows_after_scrolling_the_modal() {
    // @step Given an open turn-content modal that I have scrolled down
    // Build a long body so the modal overflows and can be scrolled; each
    // line is uniquely marked so we know which rows are visible.
    let mut body = String::new();
    for i in 0..80 {
        body.push_str(&format!("ROW{i:02}\n"));
    }
    body.push_str("ROWLAST");
    let (mut app, clip) = open_modal_app(&body, 80, 24);
    // Scroll the modal down several rows (keyboard Down in SELECT mode
    // routes to TurnModalScrollDown while the modal is open).
    for _ in 0..5 {
        let _ = app.handle_event(&key(KeyCode::Down));
        drain_app(&mut app);
    }
    let off = modal_offset(&app);
    assert!(off > 0, "precondition: the modal must be scrolled down");
    let _ = render_app(&mut app, 80, 24);
    let r = modal_body_rect(80, 24, &body);

    // @step When I drag to select across the currently visible body rows
    // Select the FIRST visible body row (top of the viewport after
    // scrolling) — that is `ROW{off:02}`.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        r.x + r.width - 1,
        r.y,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Up(MouseButton::Left),
        r.x + r.width - 1,
        r.y,
    ));
    drain_app(&mut app);

    // @step Then the highlighted and copied rows are the ones visible after scrolling
    let expected = format!("ROW{off:02}");
    assert_eq!(
        clip_bytes(&clip),
        osc52(&expected),
        "the copied row must be the first row visible after scrolling ({expected})"
    );
    assert!(
        modal_selection_active(&app),
        "the modal selection must be active after the drag"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Copying a wide line abutting the modal scrollbar excludes the
//           scrollbar glyph
// ─────────────────────────────────────────────────────────────────────

#[test]
fn copying_a_wide_line_abutting_the_modal_scrollbar_excludes_the_scrollbar_glyph() {
    // @step Given an open turn-content modal whose body line abuts the scrollbar
    // A first wide line exactly `content_width` chars wide reaches the
    // gutter edge; enough filler lines force the modal to overflow so the
    // scrollbar (and its 1-col gutter) is reserved.
    let mut body = String::new();
    let r0 = modal_body_rect(80, 24, "probe");
    let wide: String = "W".repeat(r0.width as usize);
    body.push_str(&wide);
    body.push('\n');
    for i in 0..80 {
        body.push_str(&format!("filler {i}\n"));
    }
    let (mut app, clip) = open_modal_app(&body, 80, 24);
    // Keep the viewport at the top so the wide line (body row 0) is the
    // first visible row.
    assert_eq!(modal_offset(&app), 0, "precondition: modal at the top");
    let r = modal_body_rect(80, 24, &body);
    let wide_line: String = "W".repeat(r.width as usize);

    // @step When I select that wide line and release
    // Down at the start of the wide line, drag PAST its content edge into
    // the reserved gutter column, release.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        r.x + r.width,
        r.y,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Up(MouseButton::Left),
        r.x + r.width,
        r.y,
    ));
    drain_app(&mut app);

    // @step Then the clipboard text contains the message content but not the scrollbar glyph
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52(&wide_line),
        "the full wide line must be copied, clamped to the gutter-free content width"
    );
    assert!(
        !bytes.windows(3).any(|w| w == [0xe2, 0x94, 0x82]),
        "the scrollbar glyph │ must NOT appear in the copied text"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Wheel scrolling clears an active modal selection and scrolls
//           normally
// ─────────────────────────────────────────────────────────────────────

#[test]
fn wheel_scrolling_clears_an_active_modal_selection_and_scrolls_normally() {
    // @step Given an open turn-content modal with an active text selection
    let mut body = String::new();
    for i in 0..80 {
        body.push_str(&format!("ROW{i:02}\n"));
    }
    body.push_str("ROWLAST");
    let (mut app, _clip) = open_modal_app(&body, 80, 24);
    let r = modal_body_rect(80, 24, &body);
    // Open a live (uncommitted) selection via Down + Drag.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        r.x + 3,
        r.y + 1,
    ));
    drain_app(&mut app);
    assert!(
        modal_selection_active(&app),
        "precondition: modal selection is active"
    );
    let before = modal_offset(&app);

    // @step When I scroll the mouse wheel over the modal
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollDown, r.x + 2, r.y + 2));
    drain_app(&mut app);

    // @step Then the modal selection is cleared
    assert!(
        !modal_selection_active(&app),
        "wheel scrolling must clear the modal selection (rule [6])"
    );

    // @step And the modal scrolls normally
    let after = modal_offset(&app);
    assert!(
        after > before,
        "wheel ScrollDown must scroll the modal; before={before} after={after}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: First Esc clears the selection and a second Esc closes the
//           modal
// ─────────────────────────────────────────────────────────────────────

#[test]
fn first_esc_clears_the_selection_and_a_second_esc_closes_the_modal() {
    // @step Given an open turn-content modal with an active text selection
    let body = "MLINE0\nMLINE1\nMLINE2\nMLINE3";
    let (mut app, clip) = open_modal_app(body, 80, 24);
    let r = modal_body_rect(80, 24, body);
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        r.x + 3,
        r.y + 1,
    ));
    drain_app(&mut app);
    assert!(
        modal_selection_active(&app),
        "precondition: modal selection is active"
    );
    assert!(modal_seq(&app).is_some(), "precondition: modal is open");

    // @step When I press Esc
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain_app(&mut app);

    // @step Then the modal highlight clears and the modal stays open
    assert!(
        !modal_selection_active(&app),
        "the first Esc must clear the modal selection (rule [6])"
    );
    assert_eq!(
        modal_seq(&app),
        Some(0),
        "the first Esc must NOT close the modal"
    );

    // @step And nothing is written to the clipboard by the Esc press
    assert!(
        clip_bytes(&clip).is_empty(),
        "the Esc press must not copy anything"
    );

    // @step When I press Esc again
    let _ = app.handle_event(&key(KeyCode::Esc));
    drain_app(&mut app);

    // @step Then the modal closes
    assert_eq!(modal_seq(&app), None, "the second Esc must close the modal");
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A quick click in the modal body does not select or copy
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_in_the_modal_body_does_not_select_or_copy() {
    // @step Given an open turn-content modal showing a message with mouse capture enabled
    let body = "MLINE0\nMLINE1\nMLINE2\nMLINE3";
    let (mut app, clip) = open_modal_app(body, 80, 24);
    let r = modal_body_rect(80, 24, body);

    // @step When I quickly click in the modal body without dragging
    // Down then immediate Up, with NO drag and NO long-press tick between.
    let _ = app.handle_event(&mouse(
        MouseEventKind::Down(MouseButton::Left),
        r.x + 2,
        r.y + 1,
    ));
    drain_app(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Up(MouseButton::Left),
        r.x + 2,
        r.y + 1,
    ));
    drain_app(&mut app);

    // @step Then nothing is selected and nothing is written to the clipboard
    assert!(
        !modal_selection_active(&app),
        "a quick click must not create a modal selection"
    );
    assert!(
        clip_bytes(&clip).is_empty(),
        "a quick click must not write to the clipboard"
    );
}
