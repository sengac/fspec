// Feature: spec/features/agent-input-cursor-containment.feature
//! RPC-404 — hardware cursor containment acceptance tests.
//!
//! One #[test] per scenario. cursor_position() is queried AFTER a
//! render (render runs sync_viewport) through the same TestBackend
//! harness as agent_input_soft_wrap_rpc405.rs, so assertions are on
//! user-observable behavior and fail RED for behavior reasons today.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::{AgentView, AgentViewStore};
use crossterm::event::{KeyCode, KeyModifiers};
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

/// Press one key on the input widget (plain, no modifiers).
fn press(view: &mut AgentView, code: KeyCode) {
    let _ = view.input.handle_key(code, KeyModifiers::NONE);
}

/// Ten logical lines "line01" through "line10".
fn ten_lines() -> String {
    (1..=10)
        .map(|i| format!("line{i:02}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cursor on the last line of a 10-line buffer sits on the last
//           input-area row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cursor_on_the_last_line_of_a_10_line_buffer_sits_on_the_last_input_area_row() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains ten logical lines "line01" through "line10"
    view.input.set_value(&ten_lines());
    assert_eq!(view.input.line_count(), 10, "ten logical lines");

    // @step And the cursor is at the end of "line10"
    // set_value parks the cursor at Bottom+End (RPC-405); pin it anyway.
    press(&mut view, KeyCode::End);
    assert_eq!(view.input.cursor(), (9, 6), "cursor at end of line10");

    // @step When the agent view renders a frame and cursor_position() is queried
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);
    let (x, y) = view.cursor_position().expect("cursor pos");

    // @step Then the input area is 6 terminal rows tall occupying rows 6 through 11
    assert_eq!(input.height, 6, "input capped at 6 rows");
    assert_eq!(input.y, 6, "input starts at terminal row 6");
    assert_eq!(input.y + input.height - 1, 11, "input ends at row 11");

    // @step And the cursor y coordinate is 11, the last row of the input area
    assert_eq!(
        y,
        11,
        "cursor must sit on the LAST input-area row (y=11); got y={y}; rows:\n{}",
        rows.join("\n")
    );

    // @step And the cursor x coordinate is 9, the body start column 3 plus the 6-character display column
    assert_eq!(x, 9, "x must be body start 3 + display col 6");

    // @step And the cursor is not below the terminal bottom
    assert!(
        y < 12,
        "hardware cursor row {y} escaped below the 12-row terminal"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Scrolling the window up with the cursor on the first line pins
//           the cursor to the first input-area row
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scrolling_the_window_up_with_the_cursor_on_the_first_line_pins_the_cursor_to_the_first_input_area_row(
) {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains ten logical lines "line01" through "line10"
    view.input.set_value(&ten_lines());

    // @step And the cursor is at the end of "line10"
    press(&mut view, KeyCode::End);
    assert_eq!(view.input.cursor(), (9, 6), "cursor at end of line10");
    let (_rows, _input) = render_rows(60, 12, &mut store, &mut view);

    // @step When the user presses the Up arrow key 9 times and cursor_position() is queried after a render
    for _ in 0..9 {
        press(&mut view, KeyCode::Up);
    }
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);
    let (x, y) = view.cursor_position().expect("cursor pos");

    // @step Then the cursor is on the first logical line
    assert_eq!(view.input.cursor().0, 0, "cursor on the first logical line");

    // @step And the cursor y coordinate is 6, the first row of the input area
    assert_eq!(input.y, 6, "input starts at terminal row 6");
    assert_eq!(
        y,
        6,
        "cursor must pin to the FIRST input-area row (y=6); got y={y}; rows:\n{}",
        rows.join("\n")
    );

    // @step And the cursor x coordinate is 9, the body start column 3 plus the 6-character display column
    assert_eq!(x, 9, "x must be body start 3 + display col 6");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cursor mid-way through a line wrapped to three visual rows
//           lands on the matching wrapped row and column
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cursor_mid_way_through_a_line_wrapped_to_three_visual_rows_lands_on_the_matching_wrapped_row_and_column(
) {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains a single 120-character line of "x" characters wrapping to visual rows of 56, 56 and 8 columns
    view.input.set_value(&"x".repeat(120));
    assert_eq!(view.input.line_count(), 1, "single logical line");

    // @step And the cursor has been moved 40 characters to the left from the end so it sits at char index 80
    for _ in 0..40 {
        press(&mut view, KeyCode::Left);
    }
    assert_eq!(view.input.cursor(), (0, 80), "cursor at char index 80");

    // @step When the agent view renders a frame and cursor_position() is queried
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);
    let (x, y) = view.cursor_position().expect("cursor pos");

    // @step Then the input area is 3 terminal rows tall occupying rows 9 through 11
    assert_eq!(input.height, 3, "120 chars at 56-col body width → 3 rows");
    assert_eq!(input.y, 9, "input starts at terminal row 9");
    assert_eq!(input.y + input.height - 1, 11, "input ends at row 11");

    // @step And the cursor y coordinate is 10, the second visual row of the wrapped line
    assert_eq!(
        y,
        10,
        "char 80 sits on the 2nd wrapped visual row (y=10); got y={y}; rows:\n{}",
        rows.join("\n")
    );

    // @step And the cursor x coordinate is 27, the body start column 3 plus display column 24 within the second segment
    assert_eq!(
        x, 27,
        "x must be body start 3 + segment-2 display col 24 (80 - 56); got x={x}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CJK line places the cursor at the display-width column not the
//           char index
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cjk_line_places_the_cursor_at_the_display_width_column_not_the_char_index() {
    // @step Given the agent view is rendered on a 60x12 terminal
    let (mut view, mut store) = harness();

    // @step And the input buffer contains the single line "漢漢漢漢漢"
    view.input.set_value("漢漢漢漢漢");
    assert_eq!(view.input.line_count(), 1, "single logical line");

    // @step And the cursor has been moved 2 characters to the left from the end so it sits after the 3rd ideograph at char index 3
    for _ in 0..2 {
        press(&mut view, KeyCode::Left);
    }
    assert_eq!(view.input.cursor(), (0, 3), "cursor at char index 3");

    // @step When the agent view renders a frame and cursor_position() is queried
    let (rows, input) = render_rows(60, 12, &mut store, &mut view);
    let (x, y) = view.cursor_position().expect("cursor pos");

    // @step Then the cursor x coordinate is 9, the body start column 3 plus display column 6 for three double-width ideographs
    assert_eq!(
        x,
        9,
        "x must be body start 3 + DISPLAY col 6 (3 double-width ideographs); got x={x}; rows:\n{}",
        rows.join("\n")
    );

    // @step And the cursor x coordinate is not 6, which would be the body start column plus the char index 3
    assert_ne!(x, 6, "x must NOT be body start 3 + char index 3");

    // @step And the cursor y coordinate is 11, the single input row
    assert_eq!(input.y, 11, "single-row input sits on the last row");
    assert_eq!(y, 11, "cursor y must be the single input row 11");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Cursor position always lies inside the input area rect across
//           a grid of buffer and cursor cases
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cursor_position_always_lies_inside_the_input_area_rect_across_a_grid_of_buffer_and_cursor_cases()
{
    // @step And a grid of input buffers: an empty buffer, a single short line, a single 120-character wrapped line, and ten logical lines
    let buffers: Vec<(&str, String)> = vec![
        ("empty buffer", String::new()),
        ("single short line", "hello".to_string()),
        ("120-char wrapped line", "x".repeat(120)),
        ("ten logical lines", ten_lines()),
    ];

    // @step And for each buffer the cursor is placed at the line start, the middle and the end via Home, arrow keys and End
    let placements = ["line start", "middle", "end"];

    // @step When the agent view renders a frame and cursor_position() is queried for every grid case
    for (buf_name, content) in &buffers {
        for placement in placements {
            // @step Given the agent view is rendered on a 60x12 terminal
            let (mut view, mut store) = harness();
            view.input.set_value(content);
            let last_len = content.rsplit('\n').next().unwrap_or("").chars().count();
            match placement {
                "line start" => press(&mut view, KeyCode::Home),
                "middle" => {
                    press(&mut view, KeyCode::End);
                    for _ in 0..(last_len / 2) {
                        press(&mut view, KeyCode::Left);
                    }
                }
                _ => press(&mut view, KeyCode::End),
            }
            let (rows, input) = render_rows(60, 12, &mut store, &mut view);
            let (x, y) = view
                .cursor_position()
                .expect("cursor position after render");

            // @step Then every returned cursor position lies inside the input area rect recorded by the render
            assert!(
                x >= input.x && x < input.x + input.width,
                "[{buf_name} / {placement}] x={x} outside input rect {input:?}; rows:\n{}",
                rows.join("\n")
            );
            assert!(
                y >= input.y && y < input.y + input.height,
                "[{buf_name} / {placement}] y={y} outside input rect {input:?}; rows:\n{}",
                rows.join("\n")
            );

            // @step And no returned cursor y coordinate exceeds row 11, the terminal bottom
            assert!(
                y <= 11,
                "[{buf_name} / {placement}] cursor y={y} exceeds terminal bottom row 11"
            );
        }
    }
}
