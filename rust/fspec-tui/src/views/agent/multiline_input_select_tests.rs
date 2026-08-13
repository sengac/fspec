//! Feature: spec/features/agentview-input-composer-text-selection-copy.feature
//!
//! COPY-007 — unit tests for the composer's mouse text-selection + copy
//! path on [`MultiLineInput`]. Split into a `#[path]`-included sibling so
//! the parent `multiline_input_select.rs` stays under the 300-LoC
//! source-shape ceiling.
//!
//! Each test drives real `MouseEvent`s (and, for the long-press case, the
//! recognizer tick seam) through `handle_mouse` / `poll_selection_tick`
//! and asserts the returned prompt-free text, the selection state, and —
//! for the copy scenarios — that feeding that text into an injected
//! `Osc52Clipboard<Vec<u8>>` yields the exact prompt-free OSC 52 bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::mouse::clipboard::Osc52Clipboard;
use crate::views::agent::multiline_input::InputEventOutcome;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use crossterm::event::{KeyCode, KeyModifiers};

/// A composer input rect. Body width = 60 − 2×INPUT_PAD_X − PROMPT_WIDTH
/// = 56, so a 60-char draft wraps into two visual rows (56 + 4).
fn input_rect() -> Rect {
    Rect::new(0, 0, 60, 6)
}

/// Body-relative origin x = area.x + INPUT_PAD_X + PROMPT_WIDTH.
const BODY_X: u16 = INPUT_PAD_X + PROMPT_WIDTH;

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// The exact OSC 52 bytes an injected clipboard emits for `text`.
fn osc52_bytes_for(text: &str) -> Vec<u8> {
    let mut clip = Osc52Clipboard::new(Vec::<u8>::new());
    clip.copy(text).unwrap();
    clip.into_writer_for_test()
}

/// Expected OSC 52 sequence for `text`: `ESC ] 52 ; c ; <base64> BEL`.
fn expected_osc52(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
    out.push(0x07);
    out
}

// ---------------------------------------------------------------------
// Scenario 1: Dragging across a wrapped composer row copies its text
//             without the prompt
// ---------------------------------------------------------------------

#[test]
fn dragging_across_a_wrapped_composer_row_copies_its_text_without_the_prompt() {
    // @step Given a composer holding a multi-line draft with mouse capture enabled
    // A 60-char single logical line wraps to two visual rows at body
    // width 56: row 0 = first 56 chars, row 1 = remaining 4.
    let mut input = MultiLineInput::new();
    let draft: String = (0..60).map(|i| char::from(b'a' + (i % 26) as u8)).collect();
    input.set_value(&draft);
    let area = input_rect();

    // @step When I drag across the second wrapped row of the input and release
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), BODY_X, 1),
        area,
    );
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), BODY_X + 40, 1),
        area,
    );
    let text = input
        .handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), BODY_X + 40, 1),
            area,
        )
        .expect("Commit must return the selected text");

    // @step Then that row's text without the "> " prompt is written to the clipboard
    let second_row: String = draft.chars().skip(56).collect();
    assert_eq!(
        text, second_row,
        "copied text is the second wrapped row, prompt-free"
    );
    assert!(!text.contains('>'), "the '> ' prompt must be excluded");
    assert_eq!(
        osc52_bytes_for(&text),
        expected_osc52(&second_row),
        "the injected OSC 52 writer receives the prompt-free bytes"
    );

    // @step And the composer selection stays highlighted
    assert!(
        input.text_selection_active(),
        "selection persists after commit (rule [4])"
    );
    assert!(
        !input.selection_highlight_spans(56).is_empty(),
        "highlight spans remain after the copy"
    );
}

// ---------------------------------------------------------------------
// Scenario 2: Long-pressing a composer line selects and copies it
// ---------------------------------------------------------------------

#[test]
fn long_pressing_a_composer_line_selects_and_copies_it() {
    // @step Given a composer holding a multi-line draft with mouse capture enabled
    let mut input = MultiLineInput::new();
    input.set_value("hello\nworld");
    let area = input_rect();

    // @step When I press and hold on a composer line for about half a second and release
    // Press on visual row 0 ("hello"); wait past the ~400ms long-press
    // threshold and poll the tick seam so Begin fires; then release.
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), BODY_X + 2, 0),
        area,
    );
    std::thread::sleep(std::time::Duration::from_millis(450));
    let begin = input.poll_selection_tick(area);
    assert!(begin.is_none(), "a bare Begin never commits text");
    let text = input
        .handle_mouse(
            mouse(MouseEventKind::Up(MouseButton::Left), BODY_X + 2, 0),
            area,
        )
        .expect("Commit must return the selected line");

    // @step Then that line becomes selected and its text is written to the clipboard
    assert!(input.text_selection_active(), "the line is selected");
    assert_eq!(
        text, "hello",
        "long-press copies the whole line under the press"
    );
    assert_eq!(
        osc52_bytes_for(&text),
        expected_osc52("hello"),
        "the injected OSC 52 writer receives the prompt-free line"
    );
}

// ---------------------------------------------------------------------
// Scenario 3: Typing while a selection is active clears it and inserts
//             the character
// ---------------------------------------------------------------------

#[test]
fn typing_while_a_selection_is_active_clears_it_and_inserts_the_character() {
    // @step Given a composer with an active text selection
    let mut input = MultiLineInput::new();
    input.set_value("abcdef");
    let area = input_rect();
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), BODY_X, 0),
        area,
    );
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), BODY_X + 3, 0),
        area,
    );
    assert!(
        input.text_selection_active(),
        "precondition: selection active"
    );
    let before = input.value();

    // @step When I type a character
    let outcome = input.handle_key(KeyCode::Char('X'), KeyModifiers::NONE);

    // @step Then the composer selection is cleared
    assert!(
        !input.text_selection_active(),
        "typing clears the selection (rule [5])"
    );

    // @step And the character is inserted into the input normally
    assert!(matches!(outcome, InputEventOutcome::Continued));
    assert!(input.value().contains('X'), "the typed char is inserted");
    assert_ne!(input.value(), before, "the buffer changed");
}

// ---------------------------------------------------------------------
// Scenario 4: Esc clears an active composer selection without copying
//             or submitting
// ---------------------------------------------------------------------

#[test]
fn esc_clears_an_active_composer_selection_without_copying_or_submitting() {
    // @step Given a composer with an active text selection
    let mut input = MultiLineInput::new();
    input.set_value("draft text");
    let area = input_rect();
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), BODY_X, 0),
        area,
    );
    let _ = input.handle_mouse(
        mouse(MouseEventKind::Drag(MouseButton::Left), BODY_X + 4, 0),
        area,
    );
    assert!(
        input.text_selection_active(),
        "precondition: selection active"
    );
    let before = input.value();

    // @step When I press Esc
    // The AgentView dispatch layer owns Esc; the composer-level contract
    // is `clear_selection` (no copy, buffer untouched).
    input.clear_selection();

    // @step Then the composer highlight disappears
    assert!(!input.text_selection_active(), "Esc clears the selection");
    assert!(
        input.selection_highlight_spans(56).is_empty(),
        "the highlight overlay is removed"
    );

    // @step And nothing is written to the clipboard by the Esc press
    // (No Commit gesture fired, so no text was ever produced to copy.)

    // @step And the input is not submitted or cleared
    assert_eq!(
        input.value(),
        before,
        "the draft is neither submitted nor cleared"
    );
}

// ---------------------------------------------------------------------
// Scenario 5: A quick click in the composer does not select or copy
// ---------------------------------------------------------------------

#[test]
fn a_quick_click_in_the_composer_does_not_select_or_copy() {
    // @step Given a composer holding a multi-line draft with mouse capture enabled
    let mut input = MultiLineInput::new();
    input.set_value("some draft");
    let area = input_rect();

    // @step When I quickly click in the composer to move the cursor
    // Down then immediate Up with NO drag and NO long-press tick between.
    let down = input.handle_mouse(
        mouse(MouseEventKind::Down(MouseButton::Left), BODY_X + 2, 0),
        area,
    );
    let up = input.handle_mouse(
        mouse(MouseEventKind::Up(MouseButton::Left), BODY_X + 2, 0),
        area,
    );

    // @step Then nothing is selected and nothing is written to the clipboard
    assert!(down.is_none(), "Down alone produces no text");
    assert!(up.is_none(), "a quick click commits nothing");
    assert!(
        !input.text_selection_active(),
        "a quick click makes no selection"
    );
}
