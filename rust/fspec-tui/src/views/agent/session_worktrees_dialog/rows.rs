//! TUI-131 — the worktree list of `SessionWorktreesDialog`: each worktree
//! is a `[dirty] <session id>` row with its full path wrapped on the rows
//! under it, and the list shows the window of rows that fits, keeping the
//! chosen worktree in view.
//!
//! Feature: spec/features/long-text-wraps-instead-of-being-cut-off.feature

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use codelet_rpc_types::SessionWorktreeInfo;

use crate::components::dialog_theme::DialogRow;
use crate::components::dialog_theme_rows::wrapped_rows;

/// Cells a worktree's path rows are indented by under its session row.
const PATH_INDENT: &str = "  ";

/// The rows of each worktree wrapped to `width` columns: its session row,
/// then its path. Every row of the `chosen` worktree is highlighted.
pub(super) fn worktree_entries(
    worktrees: &[SessionWorktreeInfo],
    chosen: usize,
    width: usize,
) -> Vec<Vec<DialogRow>> {
    if worktrees.is_empty() {
        return vec![wrapped_rows(
            "(no session worktrees)",
            Style::default(),
            width,
        )];
    }
    let path_width = width.saturating_sub(PATH_INDENT.len()).max(1);
    let dim = Style::default().add_modifier(Modifier::DIM);
    worktrees
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let marker = if w.dirty { "[dirty]" } else { "       " };
            let header = format!("{marker} {}", w.session_id.value);
            let mut rows = wrapped_rows(&header, Style::default(), width);
            for row in &mut rows {
                row.selectable = true;
            }
            rows.extend(
                wrapped_rows(&w.worktree_path, dim, path_width)
                    .into_iter()
                    .map(|mut row| {
                        row.spans.insert(0, Span::raw(PATH_INDENT));
                        row
                    }),
            );
            for row in &mut rows {
                row.selected = i == chosen;
            }
            rows
        })
        .collect()
}

/// The list rows to paint: at most `visible` rows from an offset near
/// `offset` that shows the whole `chosen` worktree (its top when it is
/// taller than the window). Returns the rows and the offset used.
pub(super) fn list_window(
    entries: Vec<Vec<DialogRow>>,
    chosen: usize,
    offset: usize,
    visible: usize,
) -> (Vec<DialogRow>, usize) {
    let total: usize = entries.iter().map(Vec::len).sum();
    let start: usize = entries.iter().take(chosen).map(Vec::len).sum();
    let end = start + entries.get(chosen).map_or(0, Vec::len);
    let mut offset = offset.min(total.saturating_sub(visible));
    if start < offset {
        offset = start;
    }
    if end > offset + visible {
        offset = end.saturating_sub(visible).min(start);
    }
    let rows = entries
        .into_iter()
        .flatten()
        .skip(offset)
        .take(visible)
        .collect();
    (rows, offset)
}
