//! Feature: spec/features/board-details-strip-text-selection-copy.feature
//!
//! COPY-009 — wire selection + copy into the BoardView work-unit details
//! strip end-to-end. Each test builds a real `BoardView` + `BoardStore`,
//! renders it onto a `TestBackend` (so `last_details_area` is cached),
//! drives real `Event::Mouse` Down/Drag/Up over the cached strip inner
//! rect, drains the emitted `Action`s, and asserts the observable side
//! effects:
//!   - the injected OSC 52 clipboard writer's bytes (COPY-001), and
//!   - the strip selection state exposed by the new
//!     `BoardView.details_selection` seam (COPY-009).
//!
//! Expected on-screen text is derived by reading the rendered buffer rows
//! inside the strip inner rect and trimming trailing spaces (mirrors the
//! COPY-008 buffer-read helper) so the assertions track wrap/truncation
//! exactly and are border-free.
//!
//! These tests target the INTENDED public surface and are EXPECTED to fail
//! to COMPILE until COPY-009 lands (`BoardView` lacks
//! `set_clipboard_writer_for_test` / `details_selection` / the strip
//! selection wiring) — the correct red state for ACDD.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind,
};

mod common;

#[path = "common/board_details_strip_copy009_helpers.rs"]
mod helpers;
use helpers::{
    board_with_clipboard, buffer_row_text, clip_bytes, details_rect, mouse, osc52, render, wu,
    strip_selection_active,
};

use codelet_fspec_tui::Action;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Drain the BoardView's action receiver, dispatching each queued Action
/// back through the store so selection-clearing reducers run.
fn drain(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>) -> Vec<Action> {
    let mut out = Vec::new();
    while let Ok(action) = rx.try_recv() {
        out.push(action);
    }
    out
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging across the id and title row copies its visible text
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_across_the_id_and_title_row_copies_its_visible_text() {
    // @step Given a board with a work unit selected and its details strip visible
    let units = vec![wu("RPC-014", "Board grid", "backlog", None)];
    let (view, store, mut rx, clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    // The id:title row is the first strip row (y == r.y). Read its exact
    // border-free on-screen text from the rendered buffer.
    let buf = render(&view, &store, 120, 30);
    let expected = buffer_row_text(&buf, r.x, r.y, r.width);
    assert!(
        expected.starts_with("RPC-014: Board grid"),
        "precondition: id:title row text, got `{expected}`"
    );

    // @step When I drag across the id and title row of the details strip and release
    let _ = view.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y), &store);
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Drag(MouseButton::Left), r.x + r.width - 1, r.y),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Up(MouseButton::Left), r.x + r.width - 1, r.y),
        &store,
    );
    let _ = drain(&mut rx);

    // @step Then the visible id and title text is written to the clipboard
    assert_eq!(
        clip_bytes(&clip),
        osc52(&expected),
        "dragging the id:title row must copy its exact visible text"
    );

    // @step And the strip selection stays highlighted
    assert!(
        strip_selection_active(&view),
        "the strip selection must persist after commit (rule [5])"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Dragging across the wrapped description rows copies them as
//           shown  (also asserts Down OUTSIDE the strip keeps existing
//           SetFocusedColumn/SelectIndexInFocused behavior)
// ─────────────────────────────────────────────────────────────────────

#[test]
fn dragging_across_the_wrapped_description_rows_copies_them_as_shown() {
    // @step Given a board with a work unit selected and its details strip visible
    let long = "The board grid renders seven canonical kanban columns with \
        box-drawing separators and a five row details strip plus focused \
        column highlighting and per column viewport scrolling everywhere";
    let units = vec![wu("RPC-014", "Grid", "backlog", Some(long))];
    let (view, store, mut rx, clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    // Description occupies strip rows 1 and 2 (y == r.y+1, r.y+2).
    let buf = render(&view, &store, 120, 30);
    let line1 = buffer_row_text(&buf, r.x, r.y + 1, r.width);
    let line2 = buffer_row_text(&buf, r.x, r.y + 2, r.width);
    let expected = format!("{line1}\n{line2}");
    assert!(
        !line2.is_empty(),
        "precondition: description must wrap to a second row, got `{line2}`"
    );

    // Down OUTSIDE the strip still yields the existing board behavior:
    // a content-row click emits SetFocusedColumn + SelectIndexInFocused.
    // BACKLOG col 0 content band starts below the strip (y >= 14).
    let _ = view.handle_event(
        &mouse(MouseEventKind::Down(MouseButton::Left), 5, 15),
        &store,
    );
    let outside = drain(&mut rx);
    assert!(
        outside
            .iter()
            .any(|a| matches!(a, Action::SetFocusedColumn(_))),
        "a Down outside the strip must still emit SetFocusedColumn (rule [6]), got {outside:?}"
    );
    assert!(
        outside
            .iter()
            .any(|a| matches!(a, Action::SelectIndexInFocused(_))),
        "a Down outside the strip must still emit SelectIndexInFocused (rule [6]), got {outside:?}"
    );
    assert!(
        !strip_selection_active(&view),
        "a Down outside the strip must not begin a strip selection"
    );

    // @step When I drag across both wrapped description rows and release
    let _ = view.handle_event(
        &mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y + 1),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(
            MouseEventKind::Drag(MouseButton::Left),
            r.x + r.width - 1,
            r.y + 2,
        ),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(
            MouseEventKind::Up(MouseButton::Left),
            r.x + r.width - 1,
            r.y + 2,
        ),
        &store,
    );
    let _ = drain(&mut rx);

    // @step Then the two visible description lines are written to the clipboard exactly as shown
    assert_eq!(
        clip_bytes(&clip),
        osc52(&expected),
        "dragging the two description rows must copy both visible lines"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Copying a full-width description line excludes the side
//           border glyph
// ─────────────────────────────────────────────────────────────────────

#[test]
fn copying_a_full_width_description_line_excludes_the_side_border_glyph() {
    // @step Given a board with a work unit whose description fills the strip width
    // A long single-word-free description that fills the wrap width (116
    // cols at width 120) so line 1 reaches the strip content edge.
    let filler = "wide ".repeat(60);
    let units = vec![wu("RPC-014", "Grid", "backlog", Some(filler.trim()))];
    let (view, store, mut rx, clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    // The full-width description line is strip row 1 (y == r.y+1).
    let buf = render(&view, &store, 120, 30);
    let expected = buffer_row_text(&buf, r.x, r.y + 1, r.width);
    assert!(
        !expected.is_empty(),
        "precondition: full-width description row must have text"
    );

    // @step When I drag a full-width description line to the strip edge and release
    // Down at the row start, drag PAST the content edge onto the side
    // border column, release.
    let _ = view.handle_event(
        &mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y + 1),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(
            MouseEventKind::Drag(MouseButton::Left),
            r.x + r.width,
            r.y + 1,
        ),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(
            MouseEventKind::Up(MouseButton::Left),
            r.x + r.width,
            r.y + 1,
        ),
        &store,
    );
    let _ = drain(&mut rx);

    // @step Then the clipboard text excludes the side border glyph
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52(&expected),
        "the full-width line must be copied clamped to the border-free content width"
    );
    assert!(
        !bytes.windows(3).any(|w| w == [0xe2, 0x94, 0x82]),
        "the side border glyph │ must NOT appear in the copied text"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Changing the selected work unit clears an active strip
//           selection
// ─────────────────────────────────────────────────────────────────────

#[test]
fn changing_the_selected_work_unit_clears_an_active_strip_selection() {
    // @step Given a board with an active details-strip selection
    let units = vec![
        wu("RPC-014", "Board grid", "backlog", None),
        wu("RPC-015", "Header", "backlog", None),
    ];
    let (view, mut store, mut rx, _clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    // Open a live (committed) strip selection via Down + Drag + Up.
    let _ = view.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y), &store);
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Drag(MouseButton::Left), r.x + 6, r.y),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Up(MouseButton::Left), r.x + 6, r.y),
        &store,
    );
    let _ = drain(&mut rx);
    assert!(
        strip_selection_active(&view),
        "precondition: strip selection is active"
    );

    // @step When I scroll a column or click another card
    // Select the second card in the backlog column (changes the strip
    // content). Feed the store change back through the view so it can
    // observe the selected-unit change.
    store.set_selected_index_for("backlog", 1);
    let _ = view.handle_event(&key(KeyCode::Down), &store);
    let _ = drain(&mut rx);
    let _ = render(&view, &store, 120, 30);

    // @step Then the strip selection is cleared
    assert!(
        !strip_selection_active(&view),
        "changing the selected work unit must clear the strip selection (rule [7])"
    );

    // @step And the board behaves normally
    // A subsequent content-row click still emits the usual actions.
    let _ = view.handle_event(
        &mouse(MouseEventKind::Down(MouseButton::Left), 5, 15),
        &store,
    );
    let after = drain(&mut rx);
    assert!(
        after
            .iter()
            .any(|a| matches!(a, Action::SetFocusedColumn(_))),
        "the board must keep dispatching normally, got {after:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: Esc clears an active strip selection without copying
// ─────────────────────────────────────────────────────────────────────

#[test]
fn esc_clears_an_active_strip_selection_without_copying() {
    // @step Given a board with an active details-strip selection
    let units = vec![wu("RPC-014", "Board grid", "backlog", None)];
    let (view, store, mut rx, clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);
    let _ = view.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), r.x, r.y), &store);
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Drag(MouseButton::Left), r.x + 6, r.y),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Up(MouseButton::Left), r.x + 6, r.y),
        &store,
    );
    let _ = drain(&mut rx);
    assert!(
        strip_selection_active(&view),
        "precondition: strip selection is active"
    );
    // Snapshot the clipboard bytes BEFORE the Esc press.
    let before = clip_bytes(&clip);

    // @step When I press Esc
    let _ = view.handle_event(&key(KeyCode::Esc), &store);
    let _ = drain(&mut rx);

    // @step Then the strip highlight clears
    assert!(
        !strip_selection_active(&view),
        "Esc must clear the strip selection (rule [7])"
    );

    // @step And nothing is written to the clipboard by the Esc press
    assert_eq!(
        clip_bytes(&clip),
        before,
        "the Esc press must not copy anything new"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Scenario: A quick click in the details strip does not select or copy
// ─────────────────────────────────────────────────────────────────────

#[test]
fn a_quick_click_in_the_details_strip_does_not_select_or_copy() {
    // @step Given a board with a work unit selected and its details strip visible
    let units = vec![wu("RPC-014", "Board grid", "backlog", None)];
    let (view, store, mut rx, clip) = board_with_clipboard(units, 120, 30);
    let r = details_rect(120, 30);

    // @step When I quickly click inside the details strip
    // Down then immediate Up with NO drag in between.
    let _ = view.handle_event(
        &mouse(MouseEventKind::Down(MouseButton::Left), r.x + 4, r.y),
        &store,
    );
    let _ = drain(&mut rx);
    let _ = view.handle_event(
        &mouse(MouseEventKind::Up(MouseButton::Left), r.x + 4, r.y),
        &store,
    );
    let _ = drain(&mut rx);

    // @step Then nothing is selected and nothing is written to the clipboard
    assert!(
        !strip_selection_active(&view),
        "a quick click must not create a strip selection"
    );
    assert!(
        clip_bytes(&clip).is_empty(),
        "a quick click must not write to the clipboard"
    );
}
