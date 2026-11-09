//! COPY-004 unit tests for `scrollback_copy.rs` (moved out by TUI-130 to keep
//! that file under the 300-line ceiling).
//!
//! Feature: spec/features/scrollback-selected-text-reconstruction.feature
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::views::agent::RenderedChunk;
use ratatui::text::{Line, Span};

fn chunk(seq: u64, body: &str) -> RenderedChunk {
    RenderedChunk {
        row_ends: Vec::new(),
        seq,
        lines: vec![Line::from(Span::raw(body.to_string()))],
        source: None,
    }
}

#[test]
fn reconstruct_a_single_row_partial_selection() {
    // @step Given a scrollback whose visual row reads "Hello world"
    let mut list = ScrollbackList::new();
    list.push(chunk(0, "Hello world"));

    // @step When I reconstruct the selection of that row from column 0 to column 5
    let text = list.selected_text(&[RowSpan {
        row: 0,
        start_col: 0,
        end_col: 5,
    }]);

    // @step Then the reconstructed text is "Hello"
    assert_eq!(text, "Hello");
}

#[test]
fn reconstruct_a_multi_row_selection_joined_by_newline() {
    // @step Given a scrollback with consecutive visual rows "foo" and "bar"
    let mut list = ScrollbackList::new();
    list.push(chunk(0, "foo"));
    list.push(chunk(1, "bar"));

    // @step When I reconstruct the full-width selection spanning both rows
    let text = list.selected_text(&[
        RowSpan {
            row: 0,
            start_col: 0,
            end_col: 3,
        },
        RowSpan {
            row: 1,
            start_col: 0,
            end_col: 3,
        },
    ]);

    // @step Then the reconstructed text is "foo\nbar"
    assert_eq!(text, "foo\nbar");
}

#[test]
fn the_scrollbar_gutter_glyph_is_excluded_from_copied_text() {
    // @step Given a scrollback row rendered as "answer text  │" where the │ sits in the reserved scrollbar gutter
    // The visual row carries the gutter glyph after the content; the
    // content width is 11 chars ("answer text"), the │ sits beyond it.
    let mut list = ScrollbackList::new();
    list.push(chunk(0, "answer text│"));

    // @step When I reconstruct the full-width selection of that row clamped to the content width
    let text = list.selected_text(&[RowSpan {
        row: 0,
        start_col: 0,
        end_col: 11,
    }]);

    // @step Then the reconstructed text is "answer text"
    assert_eq!(text, "answer text");
    assert!(!text.contains('│'));
}

#[test]
fn scroll_offset_maps_viewport_rows_to_visual_rows() {
    // @step Given a scrollback with scroll offset 10
    // Build 13 single-row chunks (visual rows 0..=12); row 12 has
    // known text. Set offset to 10 so viewport row 2 -> visual row 12.
    let mut list = ScrollbackList::new();
    for i in 0..13u64 {
        list.push(chunk(i, &format!("visual{i}")));
    }
    list.set_offset_for_test(10);

    // @step When I reconstruct the selection of viewport row 2
    let text = list.selected_text(&[RowSpan {
        row: 2,
        start_col: 0,
        end_col: 20,
    }]);

    // @step Then the text comes from scrollback visual row 12
    assert_eq!(text, "visual12");
}

#[test]
fn end_column_beyond_content_length_is_clamped_without_padding() {
    // @step Given a scrollback whose visual row reads "hi"
    let mut list = ScrollbackList::new();
    list.push(chunk(0, "hi"));

    // @step When I reconstruct the selection of that row from column 0 to column 20
    let text = list.selected_text(&[RowSpan {
        row: 0,
        start_col: 0,
        end_col: 20,
    }]);

    // @step Then the reconstructed text is "hi" with no trailing spaces
    assert_eq!(text, "hi");
}

#[test]
fn multi_byte_glyphs_are_sliced_on_character_boundaries() {
    // @step Given a scrollback whose visual row reads "a😀b"
    let mut list = ScrollbackList::new();
    list.push(chunk(0, "a😀b"));

    // @step When I reconstruct the selection of that row from column 0 to column 2
    let text = list.selected_text(&[RowSpan {
        row: 0,
        start_col: 0,
        end_col: 2,
    }]);

    // @step Then the reconstructed text is "a😀" and the emoji bytes are not split
    assert_eq!(text, "a😀");
}

#[test]
fn an_empty_selection_reconstructs_an_empty_string() {
    // @step Given a scrollback with a collapsed (empty) selection region
    let mut list = ScrollbackList::new();
    list.push(chunk(0, "Hello world"));
    let region: &[RowSpan] = &[];

    // @step When I reconstruct the selected text
    let text = list.selected_text(region);

    // @step Then the reconstructed text is the empty string
    assert_eq!(text, "");
}
