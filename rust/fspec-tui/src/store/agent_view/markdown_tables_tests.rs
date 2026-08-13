#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

// Feature: spec/features/markdown-table-box-drawing-rendering-in-rust-chat-view.feature

use super::*;

/// Display width = char count (the visual-width proxy used by the port).
fn display_width(line: &str) -> usize {
    line.chars().count()
}

const BOX_CHARS: [char; 11] = ['┌', '┬', '┐', '│', '├', '┼', '┤', '└', '┴', '┘', '─'];

fn has_box_chars(s: &str) -> bool {
    s.chars().any(|c| BOX_CHARS.contains(&c))
}

// Scenario: Simple two-column table renders as an aligned box-drawing grid
#[test]
fn simple_two_column_table_renders_as_box_drawing_grid() {
    // @step Given an AI response containing the markdown table "| col1 | col2 |\n|---|---|\n| a | bb |"
    let input = "| col1 | col2 |\n|---|---|\n| a | bb |";

    // @step When the response is finalized and formatted for the chat view
    let out = format_markdown_tables(input);
    let lines: Vec<&str> = out.lines().collect();

    // @step Then the output contains a top border line starting with "┌" and ending with "┐"
    let top = lines
        .iter()
        .find(|l| l.starts_with('┌'))
        .expect("a top border line should be present");
    assert!(
        top.starts_with('┌') && top.ends_with('┐'),
        "top border must start with ┌ and end with ┐, got {top:?}"
    );

    // @step And the output contains a header separator line starting with "├" and ending with "┤"
    let sep = lines
        .iter()
        .find(|l| l.starts_with('├'))
        .expect("a header separator line should be present");
    assert!(
        sep.starts_with('├') && sep.ends_with('┤'),
        "header separator must start with ├ and end with ┤, got {sep:?}"
    );

    // @step And the output contains a bottom border line starting with "└" and ending with "┘"
    let bottom = lines
        .iter()
        .find(|l| l.starts_with('└'))
        .expect("a bottom border line should be present");
    assert!(
        bottom.starts_with('└') && bottom.ends_with('┘'),
        "bottom border must start with └ and end with ┘, got {bottom:?}"
    );

    // @step And every box-drawing border row has the same display width
    let border_rows: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with('┌') || l.starts_with('├') || l.starts_with('└'))
        .collect();
    assert!(border_rows.len() >= 3, "expected three border rows");
    let first = display_width(border_rows[0]);
    for row in &border_rows {
        assert_eq!(
            display_width(row),
            first,
            "all border rows must share the same display width; row {row:?}"
        );
    }
}

// Scenario: Colon separators set per-column left, center, and right alignment
#[test]
fn colon_separators_set_per_column_alignment() {
    // @step Given an AI response containing the markdown table "| a | b | c |\n|:---|:---:|---:|\n| x | y | z |"
    let input = "| aaaa | bbbb | cccc |\n|:---|:---:|---:|\n| x | y | z |";

    // @step When the response is finalized and formatted for the chat view
    let out = format_markdown_tables(input);
    // Find the data row (the one containing x, y, z but no border corners).
    let data_row = out
        .lines()
        .find(|l| l.contains('x') && l.contains('y') && l.contains('z'))
        .expect("data row should be present");
    // Split into cells by the vertical bar; drop the empty leading/trailing pieces.
    let cells: Vec<&str> = data_row
        .split('│')
        .filter(|c| !c.trim().is_empty())
        .collect();
    assert_eq!(
        cells.len(),
        3,
        "expected three rendered cells, got {cells:?}"
    );

    // @step Then column 1 cells are left-aligned within their padded width
    // Left-align: content immediately after the leading single space, trailing padding spaces.
    let c1 = cells[0];
    assert!(
        c1.starts_with(" x") && c1.ends_with(' '),
        "column 1 should be left-aligned (leading content, trailing pad): {c1:?}"
    );

    // @step And column 2 cells are center-aligned within their padded width
    // Center: padding on both sides of 'y' within the cell (excluding the framing spaces).
    let c2_inner = cells[1].trim_start_matches(' ').trim_end_matches(' ');
    // Recover the padded field between the framing single spaces.
    let c2_field = &cells[1][1..cells[1].len() - 1];
    let leading = c2_field.len() - c2_field.trim_start_matches(' ').len();
    let trailing = c2_field.len() - c2_field.trim_end_matches(' ').len();
    assert!(
        !c2_inner.is_empty() && leading > 0 && trailing > 0,
        "column 2 should be center-aligned (pad both sides): field={c2_field:?}"
    );

    // @step And column 3 cells are right-aligned within their padded width
    let c3 = cells[2];
    assert!(
        c3.ends_with("z ") && c3.starts_with("  "),
        "column 3 should be right-aligned (leading pad, content at end): {c3:?}"
    );
}

// Scenario: Data row with fewer cells than the header keeps the grid rectangular
#[test]
fn short_data_row_keeps_grid_rectangular() {
    // @step Given an AI response containing the markdown table "| h1 | h2 |\n|---|---|\n| a |"
    let input = "| h1 | h2 |\n|---|---|\n| a |";

    // @step When the response is finalized and formatted for the chat view
    let out = format_markdown_tables(input);
    let lines: Vec<&str> = out.lines().collect();

    // @step Then the missing second cell is rendered as a blank padded cell
    let data_row = lines
        .iter()
        .find(|l| l.starts_with('│') && l.contains('a'))
        .expect("data row should be present");
    let cells: Vec<&str> = data_row.split('│').collect();
    // Leading and trailing splits are empty framing; the two real cells are the middle.
    assert_eq!(
        cells.len(),
        4,
        "expected two cells framed by │ (4 split parts), got {cells:?}"
    );
    let second_cell = cells[2];
    assert!(
        second_cell.trim().is_empty() && !second_cell.is_empty(),
        "second cell must be blank but padded: {second_cell:?}"
    );

    // @step And every rendered data row has the same display width as the header row
    let header_row = lines
        .iter()
        .find(|l| l.starts_with('│') && l.contains("h1"))
        .expect("header row should be present");
    let data_rows: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with('│') && !l.contains("h1"))
        .collect();
    assert!(!data_rows.is_empty(), "expected at least one data row");
    for row in &data_rows {
        assert_eq!(
            display_width(row),
            display_width(header_row),
            "data row must match header display width; header={header_row:?} row={row:?}"
        );
    }
}

// Scenario: Non-table prose passes through unchanged
#[test]
fn non_table_prose_passes_through_unchanged() {
    // @step Given an AI response containing the text "hello world\nnot a table"
    let input = "hello world\nnot a table";

    // @step When the response is finalized and formatted for the chat view
    let out = format_markdown_tables(input);

    // @step Then the output equals the input byte-for-byte
    assert_eq!(out, input, "non-table prose must be unchanged");

    // @step And no box-drawing characters are added
    assert!(
        !has_box_chars(&out),
        "no box-drawing characters should be added"
    );
}

// Scenario: Rendered table grid survives the scrollback wrap path with padding preserved
#[test]
fn rendered_grid_survives_scrollback_wrap_with_padding_preserved() {
    use crate::views::agent::text_wrap::wrap_to_width;

    // @step Given a rendered grid row "│ Name  │ Role     │ Location  │" that fits within the viewport width
    let row = "│ Name  │ Role     │ Location  │";
    assert!(row.chars().count() <= 200, "row must fit within width 200");

    // @step When the chat view wraps the row to the viewport width
    let wrapped = wrap_to_width(row, 200);

    // @step Then the wrapped row equals the input with every internal column padding space preserved
    assert_eq!(
        wrapped,
        vec![row.to_string()],
        "fitting grid row must pass through verbatim"
    );

    // @step Then no column padding is collapsed to a single space
    assert!(
        wrapped[0].contains("Name  ") && wrapped[0].contains("Role     "),
        "internal column padding must not collapse to a single space: {:?}",
        wrapped[0]
    );
}

// Scenario: A table embedded in prose is rendered in place with surrounding lines kept
#[test]
fn table_embedded_in_prose_renders_in_place() {
    // @step Given an AI response containing the text "Here:\n| a | b |\n|---|---|\n| 1 | 2 |\nDone."
    let input = "Here:\n| a | b |\n|---|---|\n| 1 | 2 |\nDone.";

    // @step When the response is finalized and formatted for the chat view
    let out = format_markdown_tables(input);
    let lines: Vec<&str> = out.lines().collect();

    // @step Then the line "Here:" is preserved before the grid
    let here_idx = lines
        .iter()
        .position(|l| *l == "Here:")
        .expect("'Here:' line should be preserved");
    let top_idx = lines
        .iter()
        .position(|l| l.starts_with('┌'))
        .expect("top border should be present");
    assert!(here_idx < top_idx, "'Here:' must come before the grid");

    // @step And the line "Done." is preserved after the grid
    let done_idx = lines
        .iter()
        .position(|l| *l == "Done.")
        .expect("'Done.' line should be preserved");
    let bottom_idx = lines
        .iter()
        .position(|l| l.starts_with('└'))
        .expect("bottom border should be present");
    assert!(done_idx > bottom_idx, "'Done.' must come after the grid");

    // @step And the output contains a box-drawing top border line
    assert!(
        lines.iter().any(|l| l.starts_with('┌') && l.ends_with('┐')),
        "output should contain a box-drawing top border line"
    );
}

// Scenario: A pipe block with no separator row is left unchanged
#[test]
fn pipe_block_without_separator_left_unchanged() {
    // @step Given an AI response containing the text "| a | b |\n| c | d |"
    let input = "| a | b |\n| c | d |";

    // @step When the response is finalized and formatted for the chat view
    let out = format_markdown_tables(input);

    // @step Then the output equals the input byte-for-byte
    assert_eq!(
        out, input,
        "pipe block without a separator must be unchanged"
    );

    // @step And no box-drawing characters are added
    assert!(
        !has_box_chars(&out),
        "no box-drawing characters should be added"
    );
}
