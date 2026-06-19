//! RPC-014 — Pure-function grid helper tests.
//!
//! Feature: spec/features/rpc014-grid-helpers.feature
//!
//! Exercises `codelet/fspec-tui/src/views/board/grid.rs`:
//!   - calculate_column_widths
//!   - column_width_at
//!   - build_border_row
//!
//! Pure functions — no `Buffer` / `TestBackend` involved.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::board::grid::{
    build_border_row, calculate_column_widths, column_width_at, SeparatorType,
};

/// Scenario: 120-wide terminal divides into seven equal 16-wide columns
#[test]
fn one_hundred_twenty_wide_terminal_divides_into_seven_equal_sixteen_wide_columns() {
    // @step Given terminal_width = 120
    let width: u16 = 120;
    // @step When the test calls calculate_column_widths(120)
    let widths = calculate_column_widths(width);
    // @step Then the returned base_width equals 16
    assert_eq!(widths.base_width, 16, "base_width must be 16 at width=120");
    // @step And the returned remainder equals 0
    assert_eq!(widths.remainder, 0, "remainder must be 0 at width=120");
    // @step And column_width_at(idx, ...) returns 16 for every idx in 0..7
    for idx in 0..7 {
        assert_eq!(
            column_width_at(idx, widths),
            16,
            "column {idx} must be 16 wide at width=120"
        );
    }
    // @step And the seven column widths plus the six separator chars sum to 118 (= 120 - 2 outer borders)
    let total_col_width: u16 = (0..7).map(|i| column_width_at(i, widths)).sum();
    assert_eq!(
        total_col_width + 6,
        118,
        "col widths + 6 separators must equal inner width 118"
    );
}

/// Scenario: 125-wide terminal spreads a 5-wide remainder across the leading columns
#[test]
fn one_hundred_twenty_five_wide_terminal_spreads_remainder_across_leading_columns() {
    // @step Given terminal_width = 125
    let width: u16 = 125;
    // @step When the test calls calculate_column_widths(125)
    let widths = calculate_column_widths(width);
    // @step Then the returned base_width equals 16
    assert_eq!(widths.base_width, 16, "base_width must be 16 at width=125");
    // @step And the returned remainder equals 5
    assert_eq!(widths.remainder, 5, "remainder must be 5 at width=125");
    // @step And column_width_at(0, ...) through column_width_at(4, ...) each return 17
    for idx in 0..5 {
        assert_eq!(
            column_width_at(idx, widths),
            17,
            "leading column {idx} must be 17 wide"
        );
    }
    // @step And column_width_at(5, ...) and column_width_at(6, ...) each return 16
    for idx in 5..7 {
        assert_eq!(
            column_width_at(idx, widths),
            16,
            "trailing column {idx} must be 16 wide"
        );
    }
    // @step And the seven column widths plus the six separator chars sum to 123 (= 125 - 2 outer borders)
    let total_col_width: u16 = (0..7).map(|i| column_width_at(i, widths)).sum();
    assert_eq!(
        total_col_width + 6,
        123,
        "col widths + 6 separators must equal inner width 123"
    );
}

/// Scenario: Narrow terminals clamp to the 8-column minimum and report zero remainder
#[test]
fn narrow_terminals_clamp_to_eight_minimum_and_report_zero_remainder() {
    // @step Given terminal_width = 60
    let width: u16 = 60;
    // @step When the test calls calculate_column_widths(60)
    let widths = calculate_column_widths(width);
    // @step Then the returned base_width is at least 8
    assert!(
        widths.base_width >= 8,
        "base_width must be >= 8 (min clamp) at width=60, got {}",
        widths.base_width
    );
    // @step And the returned remainder equals 0
    assert_eq!(
        widths.remainder, 0,
        "remainder must be 0 when base_width was clamped"
    );
}

/// Scenario: build_border_row produces the four canonical separator strings for a 120-wide layout
#[test]
fn build_border_row_produces_four_canonical_separator_strings_for_120_wide_layout() {
    // @step Given the widths produced by calculate_column_widths(120)
    let widths = calculate_column_widths(120);

    // @step When the test calls build_border_row(widths, "├", "┤", Plain)
    let plain = build_border_row(widths, "├", "┤", SeparatorType::Plain);
    // @step Then the returned string starts with "├" and ends with "┤" and has length 120
    assert!(
        plain.starts_with('├'),
        "plain must start with ├, got `{plain}`"
    );
    assert!(plain.ends_with('┤'), "plain must end with ┤, got `{plain}`");
    assert_eq!(plain.chars().count(), 120, "plain must be 120 chars wide");

    // @step When the test calls build_border_row(widths, "├", "┤", Top)
    let top = build_border_row(widths, "├", "┤", SeparatorType::Top);
    // @step Then the returned string starts with "├" and ends with "┤" and contains exactly six "┬" glyphs
    assert!(top.starts_with('├'), "top must start with ├");
    assert!(top.ends_with('┤'), "top must end with ┤");
    assert_eq!(top.matches('┬').count(), 6, "top must contain exactly 6 ┬");

    // @step When the test calls build_border_row(widths, "├", "┤", Cross)
    let cross = build_border_row(widths, "├", "┤", SeparatorType::Cross);
    // @step Then the returned string starts with "├" and ends with "┤" and contains exactly six "┼" glyphs
    assert!(cross.starts_with('├'), "cross must start with ├");
    assert!(cross.ends_with('┤'), "cross must end with ┤");
    assert_eq!(
        cross.matches('┼').count(),
        6,
        "cross must contain exactly 6 ┼"
    );

    // @step When the test calls build_border_row(widths, "├", "┤", Bottom)
    let bottom = build_border_row(widths, "├", "┤", SeparatorType::Bottom);
    // @step Then the returned string starts with "├" and ends with "┤" and contains exactly six "┴" glyphs
    assert!(bottom.starts_with('├'), "bottom must start with ├");
    assert!(bottom.ends_with('┤'), "bottom must end with ┤");
    assert_eq!(
        bottom.matches('┴').count(),
        6,
        "bottom must contain exactly 6 ┴"
    );
}
