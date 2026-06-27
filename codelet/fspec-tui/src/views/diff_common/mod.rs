//! RPC-363 — shared diff-viewer module.
//!
//! Feature: spec/features/shared-diff-view-components.feature
//!
//! Lifts the colored diff-line rendering, file-row formatting, and the
//! pane-scrollbar gutter wrapper out of `views/changed_files/`-private
//! scope so both `ChangedFilesView` and the new `CheckpointsView`
//! (RPC-364) reuse identical rendering without duplication. Pure
//! refactor — byte-identical behavior, helpers keep their signatures.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

mod diff_render;
mod row;

#[cfg(test)]
#[path = "api_tests.rs"]
mod api_tests;

pub use diff_render::{classify, diff_line, DiffLineKind};
pub use row::{file_row, status_color, truncate_path};

/// Paint the proportional scrollbar in the reserved 1-col gutter to the
/// right of a pane's content, reusing the shared list-scrollbar helper.
pub fn render_pane_scrollbar(
    content: Rect,
    buf: &mut Buffer,
    list_width: u16,
    scroll: usize,
    visible: usize,
    total: usize,
) {
    crate::components::list_scrollbar::render_list_scrollbar(
        Rect {
            x: content.x + list_width,
            y: content.y,
            width: 1,
            height: content.height,
        },
        buf,
        scroll,
        visible,
        total,
    );
}
