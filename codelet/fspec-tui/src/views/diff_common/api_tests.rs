//! RPC-363 — public-API tests for the shared diff-viewer module.
//!
//! Feature: spec/features/shared-diff-view-components.feature
//!
//! Pins the observable behavior of the lifted shared helpers
//! (`diff_line` colors, `file_row` cursor + truncation, and the
//! `render_pane_scrollbar` gutter wrapper that delegates to
//! `list_scrollbar`) so both ChangedFilesView and CheckpointsView
//! can rely on identical rendering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::ChangedFile;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

use crate::views::diff_common::{
    diff_line, file_row, pane_header, render_pane_scrollbar, render_vertical_divider,
};

#[test]
fn shared_diff_line_colors_added_removed_and_hunk() {
    // @step Given the shared diff-viewer module exposes diff_line as a public helper
    // (imported above from crate::views::diff_common)

    // @step When I build diff lines for a +added line, a -removed line, and an @@ hunk header
    let added = diff_line("+added");
    let removed = diff_line("-removed");
    let hunk = diff_line("@@ -1,2 +1,3 @@");

    // @step Then the added line is green, the removed line is red, and the hunk header is dim cyan
    assert_eq!(added.spans[0].style.fg, Some(Color::Green));
    assert_eq!(removed.spans[0].style.fg, Some(Color::Red));
    assert_eq!(hunk.spans[0].style.fg, Some(Color::Cyan));
    assert!(hunk.spans[0].style.add_modifier.contains(Modifier::DIM));
}

#[test]
fn shared_file_row_shows_cursor_and_truncates_long_path() {
    // @step Given the shared diff-viewer module exposes file_row, status_color and truncate_path as public helpers
    let file = ChangedFile {
        path: "very/long/path/to/some/deeply/nested/file.rs".to_string(),
        change_type: "M".to_string(),
        staged: false,
    };

    // @step When I build a file_row for a selected file with a long path in a narrow pane
    let row = file_row(&file, true, 20);

    // @step Then the row begins with a > cursor and the path is truncated with an ellipsis
    assert_eq!(row.spans[0].content.as_ref(), "> ");
    let path_span = row.spans.last().expect("row has a path span");
    assert!(path_span.content.ends_with('…'));
}

#[test]
fn shared_pane_scrollbar_paints_thumb_in_gutter() {
    // @step Given the shared diff-viewer module exposes a pane-scrollbar helper that delegates to list_scrollbar
    let content = Rect { x: 0, y: 0, width: 10, height: 4 };
    let list_width = 9u16;
    let mut buf = Buffer::empty(Rect { x: 0, y: 0, width: 10, height: 4 });

    // @step When I render a pane-scrollbar gutter for a list that overflows its pane
    render_pane_scrollbar(content, &mut buf, list_width, 0, 4, 20);

    // @step Then a proportional scrollbar thumb is painted in the gutter column
    let gutter_x = content.x + list_width;
    let mut symbols = String::new();
    for y in content.y..content.y + content.height {
        symbols.push_str(buf[(gutter_x, y)].symbol());
    }
    assert!(symbols.contains('■'), "expected a thumb glyph in gutter, got {symbols:?}");
}

#[test]
fn pane_header_on_tiny_areas_does_not_overflow_or_panic() {
    // @step Given the shared pane_header helper and a terminal too short to fit the heading, underline and content bands
    // A 1-row-high area cannot fit heading(1) + underline(1) + content, and a
    // 0-row-high area has no room at all. Both must be handled without panic.
    let mut buf = Buffer::empty(Rect { x: 0, y: 0, width: 20, height: 1 });

    // @step When pane_header is rendered into a 1-row-high area
    let one_row = pane_header(Rect { x: 0, y: 0, width: 20, height: 1 }, &mut buf, "Files", true);

    // @step Then it returns a content Rect with height 0 so list rows cannot overdraw the heading
    assert_eq!(one_row.height, 0, "1-row pane leaves no content rows");

    // @step When pane_header is rendered into a 0-row-high area
    let zero_row = pane_header(Rect { x: 0, y: 0, width: 20, height: 0 }, &mut buf, "Files", false);

    // @step Then it returns a content Rect with height 0 and paints nothing
    assert_eq!(zero_row.height, 0, "0-row pane leaves no content rows");
}

#[test]
fn render_vertical_divider_is_a_noop_for_zero_size_gutters() {
    // @step Given the shared render_vertical_divider helper and a buffer with no divider painted yet
    let mut buf = Buffer::empty(Rect { x: 0, y: 0, width: 10, height: 4 });

    // @step When render_vertical_divider is called with a zero-height gutter
    render_vertical_divider(Rect { x: 0, y: 0, width: 1, height: 0 }, &mut buf);

    // @step And render_vertical_divider is called with a zero-width gutter
    render_vertical_divider(Rect { x: 0, y: 0, width: 0, height: 4 }, &mut buf);

    // @step Then no divider glyph is written and the buffer is unchanged
    let mut painted = false;
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if buf[(x, y)].symbol() == "│" {
                painted = true;
            }
        }
    }
    assert!(!painted, "zero-size gutter must paint no '│' divider glyph");
}
