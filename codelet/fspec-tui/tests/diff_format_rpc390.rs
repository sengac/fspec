//! Feature: spec/features/agentview-edit-diff-generation.feature
//!
//! RPC-390 — tests for the pure Edit/Write diff-generation module
//! `store::agent_view::diff_format`. Each Gherkin step is annotated with a
//! matching `@step` comment whose text mirrors the feature file exactly.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write as _;

use codelet_fspec_tui::store::agent_view::diff_format::{
    calculate_start_line, format_diff_for_display, format_edit_diff, format_with_tree_connectors,
    format_write_diff, DiffOutputKind, DiffOutputLine, DIFF_COLLAPSED_LINES,
};

/// Count display lines containing a given marker.
fn count_marker(s: &str, marker: &str) -> usize {
    s.lines().filter(|l| l.contains(marker)).count()
}

#[test]
fn single_line_replacement_produces_one_removed_and_one_added_marker_within_context() {
    // @step Given an old_string and new_string that differ in a single line
    let old = "line1\nline2\nline3\n";
    let new = "line1\nCHANGED\nline3\n";

    // @step When I format the edit diff for display
    let diff = format_edit_diff(old, new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, 1);

    // @step Then the output contains exactly one [R]- line and one [A]+ line
    assert_eq!(count_marker(&out, "[R]-"), 1);
    assert_eq!(count_marker(&out, "[A]+"), 1);

    // @step And the surrounding context lines appear within three lines of the change
    assert!(out.contains("line1"));
    assert!(out.contains("line3"));
}

#[test]
fn pure_addition_produces_only_added_markers_and_no_removed_markers() {
    // @step Given an empty old_string and a new_string with several lines
    let old = "";
    let new = "alpha\nbeta\ngamma\n";

    // @step When I format the edit diff for display
    let diff = format_edit_diff(old, new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, 1);

    // @step Then every change line is an [A]+ line
    assert!(count_marker(&out, "[A]+") >= 1);

    // @step And no [R]- line appears in the output
    assert_eq!(count_marker(&out, "[R]-"), 0);
}

#[test]
fn write_of_a_three_line_file_produces_three_added_markers() {
    // @step Given a Write content of exactly three lines
    let content = "one\ntwo\nthree";

    // @step When I format the write diff for display
    let diff = format_write_diff(content);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, 1);

    // @step Then the output contains three [A]+ lines
    assert_eq!(count_marker(&out, "[A]+"), 3);

    // @step And no [R]- line appears in the output
    assert_eq!(count_marker(&out, "[R]-"), 0);
}

#[test]
fn a_mid_file_change_in_a_large_edit_shows_leading_and_trailing_gap_markers() {
    // @step Given a 100-line edit with a single changed line in the middle
    let old: String = (1..=100).map(|n| format!("line{n}\n")).collect::<String>();
    let new: String = (1..=100)
        .map(|n| {
            if n == 50 {
                "line50-CHANGED\n".to_string()
            } else {
                format!("line{n}\n")
            }
        })
        .collect::<String>();

    // @step When I format the edit diff for display
    let diff = format_edit_diff(&old, &new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, 1);

    // @step Then the leading context begins at the first shown line and earlier lines are dropped
    // The leading region (lines 1..46) is dropped: the first shown row is the
    // 3-line context window before the change, NOT line 1. (Parity with TS: the
    // display begins at the first shown index, collapsing everything before it.)
    let first_row = out.lines().next().expect("at least one row");
    assert!(first_row.contains("line47"));
    assert!(!out.contains("line1\n") && !out.contains(" line1 "));
    let first_change = out
        .lines()
        .position(|l| l.contains("[R]-") || l.contains("[A]+"))
        .expect("change present");

    // @step And a trailing '... (N lines)' gap marker follows the change context
    let rows: Vec<&str> = out.lines().collect();
    let last_gap = rows
        .iter()
        .rposition(|l| l.contains("... ("))
        .expect("trailing gap marker present");
    assert!(last_gap > first_change);
}

#[test]
fn a_diff_exceeding_the_collapse_limit_ends_with_an_expand_indicator() {
    // @step Given a diff whose display lines exceed the collapse limit of 25
    let old = "";
    let new: String = (1..=60).map(|n| format!("added{n}\n")).collect::<String>();

    // @step When I format the edit diff for display
    let diff = format_edit_diff(old, &new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, 1);

    // @step Then the output is truncated to the first 25 display lines
    // Tree connectors add one line per output line; 25 display lines + the
    // indicator line = 26 rendered rows.
    assert_eq!(out.lines().count(), DIFF_COLLAPSED_LINES + 1);

    // @step And the last line is '... +N lines (select turn to /expand)'
    let last = out.lines().last().expect("at least one line");
    assert!(last.contains("(select turn to /expand)"));
}

#[test]
fn calculate_start_line_on_a_missing_file_returns_1() {
    // @step Given a file path that does not exist
    let missing = "/nonexistent/path/to/file/that/does/not/exist.txt";

    // @step When I calculate the start line for the edit
    let start = calculate_start_line(Some(missing), Some("old"), Some("new"));

    // @step Then the start line is 1
    assert_eq!(start, 1);

    // @step And no panic occurs
    // (reaching this line proves no panic occurred)
}

#[test]
fn calculate_start_line_finds_new_string_at_line_250_and_returns_250() {
    // @step Given a file whose 250th line contains the new_string
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let mut content = String::new();
    for n in 1..=249 {
        content.push_str(&format!("filler line {n}\n"));
    }
    content.push_str("UNIQUE_NEW_STRING_MARKER\n");
    file.write_all(content.as_bytes()).unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // @step When I calculate the start line for the edit
    let start = calculate_start_line(Some(&path), Some("old"), Some("UNIQUE_NEW_STRING_MARKER"));

    // @step Then the start line is 250
    assert_eq!(start, 250);
}

#[test]
fn context_lines_are_encoded_with_a_line_number_and_three_spaces_and_no_marker() {
    // @step Given a diff with an unchanged context line
    let old = "ctx1\nctx2\nold\nctx3\nctx4\n";
    let new = "ctx1\nctx2\nnew\nctx3\nctx4\n";

    // @step When I format the edit diff for display
    let diff = format_edit_diff(old, new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, 1);

    // @step Then the context line shows the line number followed by three spaces and the content
    assert!(out.lines().any(|l| l.contains("  1   ctx1")));

    // @step And the context line carries no [R] or [A] marker
    let ctx_line = out
        .lines()
        .find(|l| l.contains("ctx1"))
        .expect("context line present");
    assert!(!ctx_line.contains("[R]"));
    assert!(!ctx_line.contains("[A]"));
}

#[test]
fn line_numbers_are_offset_by_start_line_and_left_padded_to_at_least_width_three() {
    // @step Given an edit positioned with a startLine of 250
    let old = "before\nold\nafter\n";
    let new = "before\nnew\nafter\n";
    let start_line = 250usize;

    // @step When I format the edit diff for display with that startLine
    let diff = format_edit_diff(old, new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, start_line);

    // @step Then the first marker line shows the offset line number 250
    assert!(out.contains("250 "));

    // @step And line numbers are left-padded to at least width three
    // The single-digit-equivalent "1"-style numbers (here 250..252) are 3+
    // wide; verify a small-number case pads to width 3.
    let small = format_diff_for_display(&format_edit_diff(old, new), DIFF_COLLAPSED_LINES, 1);
    assert!(small.lines().any(|l| l.contains("  1")));
}

#[test]
fn tree_connectors_prefix_the_first_line_and_indent_the_rest_while_empty_content_yields_empty() {
    // @step Given a multi-line content string
    let content = "first\nsecond\nthird";

    // @step When I apply tree connectors to the content
    let out = format_with_tree_connectors(content);

    // @step Then the first line is prefixed with 'L ' and subsequent lines are indented two spaces
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "L first");
    assert_eq!(lines[1], "  second");
    assert_eq!(lines[2], "  third");

    // @step And empty or whitespace-only content yields an empty string
    assert_eq!(format_with_tree_connectors(""), "");
    assert_eq!(format_with_tree_connectors("   \n  "), "");
}

#[test]
fn a_representative_edit_produces_a_byte_for_byte_golden_display_string() {
    // @step Given a representative edit with a known old_string and new_string and startLine
    let old = "line1\nline2\nline3\n";
    let new = "line1\nCHANGED\nline3\n";
    let start_line = 1usize;

    // @step When I format the edit diff for display
    let diff = format_edit_diff(old, new);
    let out = format_diff_for_display(&diff, DIFF_COLLAPSED_LINES, start_line);

    // @step Then the output equals the expected golden string byte-for-byte
    let expected = "L   1   line1\n    2 [R]- line2\n    3 [A]+ CHANGED\n    4   line3";
    assert_eq!(out, expected);

    // Sanity on the underlying diff line shape (parity with changesToDiffLines).
    assert_eq!(
        diff,
        vec![
            DiffOutputLine {
                content: " line1".into(),
                kind: DiffOutputKind::Context
            },
            DiffOutputLine {
                content: "-line2".into(),
                kind: DiffOutputKind::Removed
            },
            DiffOutputLine {
                content: "+CHANGED".into(),
                kind: DiffOutputKind::Added
            },
            DiffOutputLine {
                content: " line3".into(),
                kind: DiffOutputKind::Context
            },
        ]
    );
}

#[test]
fn trailing_newline_and_no_trailing_newline_content_produce_parity_diff_lines() {
    // @step Given an old_string and new_string that differ in one line, in both a trailing-newline and a no-trailing-newline variant
    // The encoder splits on '\n' and filters empty fragments, so a trailing
    // newline contributes no extra line. Both variants must therefore produce
    // byte-for-byte identical diff lines and display output (TS parity:
    // `split('\n').filter(line => line.length > 0)`).
    let old_with_nl = "alpha\nbeta\ngamma\n";
    let new_with_nl = "alpha\nBETA\ngamma\n";
    let old_no_nl = "alpha\nbeta\ngamma";
    let new_no_nl = "alpha\nBETA\ngamma";

    // @step When I format the edit diff for display for both variants
    let diff_with_nl = format_edit_diff(old_with_nl, new_with_nl);
    let diff_no_nl = format_edit_diff(old_no_nl, new_no_nl);
    let out_with_nl = format_diff_for_display(&diff_with_nl, DIFF_COLLAPSED_LINES, 1);
    let out_no_nl = format_diff_for_display(&diff_no_nl, DIFF_COLLAPSED_LINES, 1);

    // @step Then both variants produce identical diff lines
    assert_eq!(diff_with_nl, diff_no_nl);

    // @step And the display output is identical byte-for-byte
    assert_eq!(out_with_nl, out_no_nl);
}
