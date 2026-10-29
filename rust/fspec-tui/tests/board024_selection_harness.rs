//! Regression tests for the BOARD-024 rendered-selection test harness.
//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

#[path = "board024/selection.rs"]
mod selection;

fn highlight(buffer: &mut Buffer, column: u16, row: u16, text: &str) {
    buffer.set_string(
        column,
        row,
        text,
        Style::default().fg(Color::Black).bg(Color::Yellow),
    );
}

#[test]
fn soft_wrapping_can_replace_a_path_space_without_erasing_internal_spaces() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 7, 4, "spec/attachments/A/a");
    highlight(&mut buffer, 7, 5, "日本語 #%.md");
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a 日本語 #%.md"),
        Some((7, 4))
    );
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a 日本語#%.md"),
        None
    );
}

#[test]
fn duplicate_basenames_require_the_complete_selected_path() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 6, 4, "spec/attachments/A/a 日本語 #%.md");
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/B/a 日本語 #%.md"),
        None
    );
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a 日本語 #%.md"),
        Some((6, 4))
    );
}

#[test]
fn selected_coordinates_belong_to_the_matched_path_not_highlighted_chrome() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 1, 1, "Unrelated selected chrome");
    highlight(&mut buffer, 7, 5, "▸ spec/attachments/B/a.md");
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/B/a.md"),
        Some((9, 5))
    );
}

#[test]
fn wrapped_unicode_paths_retain_exact_prefix_and_matched_coordinates() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 1, 1, "Unrelated selected chrome");
    highlight(&mut buffer, 7, 4, "spec/attachments/A/");
    highlight(&mut buffer, 7, 5, "a 日本語 #%.md");
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a 日本語 #%.md"),
        Some((7, 4))
    );
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/B/a 日本語 #%.md"),
        None
    );
}

#[test]
fn separated_highlights_cannot_be_concatenated_into_a_target() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 4, 1, "spec/attachments/A/");
    highlight(&mut buffer, 4, 4, "a.md");
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a.md"),
        None
    );
}

#[test]
fn internal_whitespace_and_literal_percent_paths_are_not_normalized_away() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 4, 3, "spec/attachments/A/a b%20.md");
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/ab%20.md"),
        None
    );
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a b .md"),
        None
    );
    assert_eq!(
        selection::selected_position(&buffer, "spec/attachments/A/a b%20.md"),
        Some((4, 3))
    );
}

#[test]
fn filename_whitespace_does_not_end_the_selected_attachment_path() {
    for stored_path in [
        "spec/attachments/A/a.md extra",
        "prefix spec/attachments/A/a.md",
    ] {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
        highlight(&mut buffer, 4, 3, stored_path);
        assert_eq!(
            selection::selected_position(&buffer, "spec/attachments/A/a.md"),
            None,
            "the requested attachment is not the highlighted full stored path"
        );
        assert_eq!(
            selection::selected_position(&buffer, stored_path),
            Some((4, 3))
        );
    }
}

#[test]
fn generic_link_ids_can_have_a_separate_display_title() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 8));
    highlight(&mut buffer, 7, 4, "▸ AUTH-002 Linked display title");
    assert_eq!(
        selection::selected_position(&buffer, "AUTH-002"),
        Some((9, 4))
    );
}
