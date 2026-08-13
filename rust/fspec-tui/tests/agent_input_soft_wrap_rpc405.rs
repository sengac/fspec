// Feature: spec/features/agent-input-soft-wrap-auto-grow.feature
//! RPC-405 — MultiLineInput soft-wrap + auto-grow acceptance tests.
//!
//! One #[test] per scenario; assertions are buffer-content based
//! (user-observable) through the TestBackend render harness so the
//! file compiles today and fails on BEHAVIOR (red phase).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{AgentView, AgentViewStore};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

/// Build an AgentView + store pair (zz_repro pattern).
fn harness() -> (AgentView, AgentViewStore) {
    let (tx, _rx) = unbounded_channel();
    (AgentView::new(tx), AgentViewStore::default())
}

/// Render one frame on a w x h TestBackend; return every terminal row
/// as a String plus the input area Rect recorded by the view.
fn render_rows(
    w: u16,
    h: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> (Vec<String>, Rect) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), store))
        .unwrap();
    let buf = term.backend().buffer().clone();
    let rows: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect();
    let input_area = view.last_input_area.expect("input area recorded by render");
    (rows, input_area)
}

/// Body text of an input row: strip the 1-col pad + 2-col "> " prompt
/// region (body starts at x = rect.x + 1 + 2 = 3 on a full-width area).
fn body_of(row: &str) -> String {
    row.chars()
        .skip(3)
        .collect::<String>()
        .trim_end()
        .to_string()
}

const LONG_LINE: &str =
    "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Typing past the right edge grows the input to two rows with
//           head and tail visible
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn typing_past_the_right_edge_grows_the_input_to_two_rows_with_head_and_tail_visible() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains the single 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    assert_eq!(LONG_LINE.chars().count(), 85 - 2, "sanity: 83 chars");
    view.input.set_value(LONG_LINE);
    assert_eq!(view.input.line_count(), 1, "single logical line");

    // @step When the agent view renders a frame
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 2 terminal rows tall
    assert_eq!(
        input.height,
        2,
        "input area must grow to 2 rows for an 83-char line at 56-col body width; rows:\n{}",
        rows.join("\n")
    );

    // @step And the first input row shows text starting with "word01"
    let first = body_of(&rows[input.y as usize]);
    assert!(
        first.starts_with("word01"),
        "first input row must start with word01 (head visible), got {first:?}"
    );

    // @step And the second input row shows text ending with "word12"
    let second = body_of(&rows[input.y as usize + 1]);
    assert!(
        second.ends_with("word12"),
        "second input row must end with word12 (tail visible), got {second:?}"
    );

    // @step And no input row shows more than 56 columns of buffer text
    for y in input.y..input.y + input.height {
        let body = body_of(&rows[y as usize]);
        assert!(
            body.chars().count() <= 56,
            "input row {y} shows more than 56 cols of buffer text: {body:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Recalled multi-line history entry renders fully and shrinks
//           the scrollback
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn recalled_multi_line_history_entry_renders_fully_and_shrinks_the_scrollback() {
    // @step Given the agent view is rendered on a 60x12 terminal with an empty input
    let (mut view, mut store) = harness();
    let (_rows, input_before) = render_rows(60, 12, &mut store, &mut view);
    assert_eq!(input_before.height, 1, "empty input must be 1 row tall");

    // @step And the scrollback area is 9 terminal rows tall
    // Layout: header row 0, scrollback rows 1..=9, footer row 10, input row 11.
    // With a 1-row input at the bottom (y=11), the footer sits at y=10 and
    // the scrollback spans rows 1..=9 → 9 rows.
    assert_eq!(input_before.y, 11, "1-row input sits on the last row");
    let scrollback_before = input_before.y - 1 - 1; // rows between header and footer
    assert_eq!(scrollback_before, 9, "scrollback must be 9 rows tall");

    // @step When the user recalls a history entry consisting of the three lines "alpha", "bravo" and "charlie"
    // dispatch_history_recall calls set_value — simulate it directly.
    view.input.set_value("alpha\nbravo\ncharlie");
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 3 terminal rows tall
    assert_eq!(
        input.height,
        3,
        "input must grow to 3 rows for a 3-line recall; rows:\n{}",
        rows.join("\n")
    );

    // @step And the rows "alpha", "bravo" and "charlie" are each visible on their own input row
    let bodies: Vec<String> = (input.y..input.y + input.height)
        .map(|y| body_of(&rows[y as usize]))
        .collect();
    assert_eq!(
        bodies,
        vec!["alpha", "bravo", "charlie"],
        "each recalled line must occupy its own input row"
    );

    // @step And the scrollback area is 7 terminal rows tall
    // header row 0, scrollback rows 1..=7, footer row 8, input rows 9..=11.
    assert_eq!(input.y, 9, "3-row input starts at row 9");
    let scrollback_after = input.y - 1 - 1;
    assert_eq!(scrollback_after, 7, "scrollback must shrink to 7 rows");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Buffer wrapping past the cap shows the last six visual rows
//           when the cursor is at the end
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn buffer_wrapping_past_the_cap_shows_the_last_six_visual_rows_when_the_cursor_is_at_the_end() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains nine logical lines "row01" through "row09"
    let nine: String = (1..=9)
        .map(|i| format!("row{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.input.set_value(&nine);
    assert_eq!(view.input.line_count(), 9, "nine logical lines");

    // @step And the cursor is at the end of "row09"
    // set_value parks the cursor at the end of the FIRST line; walk it
    // down to the last line (test-side precondition, not an assertion
    // of production cursor placement).
    for _ in 0..8 {
        let _ = view.input.handle_key(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        );
    }
    let _ = view.input.handle_key(
        crossterm::event::KeyCode::End,
        crossterm::event::KeyModifiers::NONE,
    );
    assert_eq!(view.input.cursor(), (8, 5), "cursor at end of row09");

    // @step When the agent view renders a frame
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 6 terminal rows tall
    assert_eq!(
        input.height,
        6,
        "input must be capped at 6 rows; rows:\n{}",
        rows.join("\n")
    );

    // @step And the rows "row04" through "row09" are visible in the input area
    let bodies: Vec<String> = (input.y..input.y + input.height)
        .map(|y| body_of(&rows[y as usize]))
        .collect();
    assert_eq!(
        bodies,
        vec!["row04", "row05", "row06", "row07", "row08", "row09"],
        "cursor-follow window must show the LAST six lines"
    );

    // @step And the row "row01" is not visible
    assert!(
        !rows.join("\n").contains("row01"),
        "row01 must be scrolled out of view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Moving the cursor to the top scrolls the six-row window up
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn moving_the_cursor_to_the_top_scrolls_the_six_row_window_up() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains nine logical lines "row01" through "row09"
    let nine: String = (1..=9)
        .map(|i| format!("row{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.input.set_value(&nine);

    // @step And the cursor is at the end of "row09"
    // set_value parks the cursor at the end of the FIRST line; walk it
    // down to the last line (test-side precondition).
    for _ in 0..8 {
        let _ = view.input.handle_key(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        );
    }
    let _ = view.input.handle_key(
        crossterm::event::KeyCode::End,
        crossterm::event::KeyModifiers::NONE,
    );
    assert_eq!(view.input.cursor(), (8, 5), "cursor at end of row09");
    let (_rows, _input) = render_rows(60, 12, &mut store, &mut view);

    // @step When the user presses the Up arrow key 8 times
    for _ in 0..8 {
        let _ = view.input.handle_key(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        );
    }
    assert_eq!(view.input.cursor().0, 0, "cursor must reach the first line");
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 6 terminal rows tall
    assert_eq!(
        input.height,
        6,
        "input must stay capped at 6 rows; rows:\n{}",
        rows.join("\n")
    );

    // @step And the rows "row01" through "row06" are visible in the input area
    let bodies: Vec<String> = (input.y..input.y + input.height)
        .map(|y| body_of(&rows[y as usize]))
        .collect();
    assert_eq!(
        bodies,
        vec!["row01", "row02", "row03", "row04", "row05", "row06"],
        "window must scroll up to keep the cursor row visible"
    );

    // @step And the row "row09" is not visible
    assert!(
        !rows.join("\n").contains("row09"),
        "row09 must be scrolled out of view"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Deleting text shrinks the input back down and the scrollback
//           regains the row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn deleting_text_shrinks_the_input_back_down_and_the_scrollback_regains_the_row() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains the single 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    view.input.set_value(LONG_LINE);

    // @step And the input area is 2 terminal rows tall
    let (_rows, input_before) = render_rows(60, 12, &mut store, &mut view);
    assert_eq!(
        input_before.height, 2,
        "precondition: wrapped 83-char line must occupy 2 rows"
    );

    // @step When the user deletes characters until the buffer is the 55-character line "word01 word02 word03 word04 word05 word06 word07 word08"
    for _ in 0..28 {
        let _ = view.input.handle_key(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE,
        );
    }
    assert_eq!(
        view.input.value(),
        "word01 word02 word03 word04 word05 word06 word07 word08",
        "buffer must be the 55-char line after 28 backspaces"
    );
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 1 terminal row tall
    assert_eq!(
        input.height,
        1,
        "input must shrink back to 1 row; rows:\n{}",
        rows.join("\n")
    );

    // @step And the scrollback area is 9 terminal rows tall
    assert_eq!(input.y, 11, "1-row input sits on the last row");
    let scrollback = input.y - 1 - 1;
    assert_eq!(scrollback, 9, "scrollback must regain the row (9 rows)");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Empty buffer shows the one-row placeholder
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn empty_buffer_shows_the_one_row_placeholder() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer is empty
    assert!(view.input.is_empty(), "buffer must start empty");

    // @step When the agent view renders a frame
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 1 terminal row tall
    assert_eq!(input.height, 1, "empty input must be exactly 1 row tall");

    // @step And the input row shows the placeholder hint starting with "Type a message..."
    let body = body_of(&rows[input.y as usize]);
    assert!(
        body.starts_with("Type a message..."),
        "placeholder hint must be visible on the input row, got {body:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Wide CJK characters wrap without being split
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn wide_cjk_characters_wrap_without_being_split() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains a single line of 30 "漢" characters with a display width of 60 columns
    let cjk = "漢".repeat(30);
    view.input.set_value(&cjk);
    assert_eq!(view.input.line_count(), 1, "single logical line");

    // @step When the agent view renders a frame
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 2 terminal rows tall
    assert_eq!(
        input.height,
        2,
        "30 wide chars (60 cols) at 56-col body width must wrap to 2 rows; rows:\n{}",
        rows.join("\n")
    );

    // @step And the first input row shows exactly 28 "漢" characters filling 56 columns
    let first_count = rows[input.y as usize].matches('漢').count();
    assert_eq!(
        first_count, 28,
        "first input row must hold exactly 28 wide chars (56 cols)"
    );

    // @step And the second input row shows the remaining 2 "漢" characters
    let second_count = rows[input.y as usize + 1].matches('漢').count();
    assert_eq!(
        second_count, 2,
        "second input row must hold the remaining 2 wide chars"
    );

    // @step And no "漢" character is split across two rows
    let total: usize = (input.y..input.y + input.height)
        .map(|y| rows[y as usize].matches('漢').count())
        .sum();
    assert_eq!(
        total, 30,
        "all 30 wide chars must be painted whole (none split/dropped)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Emoji wraps at the correct display-width boundary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn emoji_wraps_at_the_correct_display_width_boundary() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains a single line of 55 "a" characters followed by "😀" with a display width of 57 columns
    let line = format!("{}😀", "a".repeat(55));
    view.input.set_value(&line);
    assert_eq!(view.input.line_count(), 1, "single logical line");

    // @step When the agent view renders a frame
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 2 terminal rows tall
    assert_eq!(
        input.height,
        2,
        "57 display cols at 56-col body width must wrap to 2 rows; rows:\n{}",
        rows.join("\n")
    );

    // @step And the first input row shows exactly the 55 "a" characters
    let first = body_of(&rows[input.y as usize]);
    assert_eq!(
        first,
        "a".repeat(55),
        "first input row must show exactly the 55 a's (emoji pushed to next row)"
    );

    // @step And the second input row starts with "😀"
    let second = body_of(&rows[input.y as usize + 1]);
    assert!(
        second.starts_with('😀'),
        "second input row must start with the emoji, got {second:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Pressing Enter mid-wrap submits the entire buffer unchanged
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn pressing_enter_mid_wrap_submits_the_entire_buffer_unchanged() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains the single 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    view.input.set_value(LONG_LINE);
    let (_rows, _input) = render_rows(60, 12, &mut store, &mut view);

    // @step And the cursor has been moved 40 characters to the left so it sits on the first visual row
    for _ in 0..40 {
        let _ = view.input.handle_key(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::NONE,
        );
    }
    assert_eq!(
        view.input.cursor(),
        (0, 83 - 40),
        "cursor must sit 40 chars left of the end"
    );

    // @step When the user presses Enter
    let outcome = view.input.handle_key(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    );

    // @step Then the submitted value is the full 83-character line "word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12"
    let value = match outcome {
        codelet_fspec_tui::views::agent::InputEventOutcome::Submitted(v) => v,
        other => panic!("plain Enter must submit the buffer; got {other:?}"),
    };
    assert_eq!(value, LONG_LINE, "submitted value must be the full line");

    // @step And the submitted value contains no newline characters
    assert!(
        !value.contains('\n'),
        "wrapping is visual only — no newline may leak into the submitted value"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Input height derives from the exact renderer body width
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn input_height_derives_from_the_exact_renderer_body_width() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input body width is 56 columns after 1 column of padding on each side and the 2-column "> " prompt
    // 60 - 2*1 (pad) - 2 ("> ") = 56 — verified below against the rendered rows.

    // @step And the input buffer contains a single line of 57 "x" characters
    view.input.set_value(&"x".repeat(57));

    // @step When the agent view renders a frame
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    // @step Then the input area is 2 terminal rows tall
    assert_eq!(
        input.height,
        2,
        "57 chars at exactly-56-col body width must wrap to 2 rows; rows:\n{}",
        rows.join("\n")
    );

    // @step And the first input row shows exactly 56 "x" characters
    let first_count = rows[input.y as usize].matches('x').count();
    assert_eq!(
        first_count, 56,
        "first input row must hold exactly 56 x's (renderer body width)"
    );

    // @step And the second input row shows exactly 1 "x" character
    let second_count = rows[input.y as usize + 1].matches('x').count();
    assert_eq!(
        second_count, 1,
        "second input row must hold exactly the 1 overflow x"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// SUPPLEMENTARY: a single WRAPPED logical line exceeding the 6-row cap
// must show a cursor-following 6-row window (last 6 visual rows when the
// cursor is at End). Red today: no wrap-aware viewport exists. (No @step
// comments: augments scenarios 3/4 for the wrapped-line case.)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn supplementary_wrapped_single_line_past_cap_shows_last_six_visual_rows() {
    let (mut view, mut store) = harness();

    // 8 segments of exactly 56 chars each ("SEGn " + padding), then a
    // short tail — 9 visual rows at a 56-col body width.
    let mut line = String::new();
    for i in 1..=8 {
        let marker = format!("SEG{i} ");
        let mut seg = marker.clone();
        seg.push_str(&"-".repeat(56 - marker.chars().count()));
        line.push_str(&seg);
    }
    line.push_str("TAIL");
    view.input.set_value(&line); // cursor lands at End

    let (rows, input) = render_rows(60, 12, &mut store, &mut view);

    assert_eq!(
        input.height,
        6,
        "9 visual rows must clamp to the 6-row cap; rows:\n{}",
        rows.join("\n")
    );
    let last_body = body_of(&rows[(input.y + input.height - 1) as usize]);
    assert!(
        last_body.contains("TAIL"),
        "cursor-follow: the last visible row must contain the tail, got {last_body:?}"
    );
    assert!(
        !rows.join("\n").contains("SEG1"),
        "SEG1 (first visual row) must be scrolled out of the 6-row window"
    );
}
