//! RPC-394 — context-aware Edit diff row builder.
//!
//! Feature: spec/features/edit-diff-surrounding-file-context.feature
//!
//! The existing [`super::diff_format::format_edit_diff`] diffs ONLY the two
//! fragments the model supplied; when `old_string`/`new_string` share no
//! unchanged lines the diff has zero `Equal` rows, so no spatial context is
//! shown. This module reads the POST-EDIT file and injects up to
//! [`super::diff_format::CONTEXT_LINES`] real unchanged file lines BEFORE and
//! AFTER the changed region as `Context` rows, so an Edit always renders in its
//! surrounding file context — a deliberate UX improvement over strict TS
//! parity.
//!
//! ## Line-number scheme (1-based, file-accurate)
//! Let `start = calculate_start_line(...)` be the file line where `new_string`
//! begins in the post-edit file, and `n` the number of `new_string` lines.
//! * **before-context**: file lines `[start - CONTEXT_LINES .. start)` clamped
//!   at line 1; each numbered with its real file line number.
//! * **fragment diff** (the `format_edit_diff` rows): numbered exactly as the
//!   legacy [`super::diff_format::build_diff_rows`] does (`start + idx`), so the
//!   RPC-390/393 golden numbering is preserved.
//! * **after-context**: file lines `[start + n .. start + n + CONTEXT_LINES)`
//!   clamped to EOF; each numbered with its real file line number.
//!
//! ## Fallback (never panics)
//! When `file_path` is `None`, the file cannot be read, `new_string` is empty,
//! or `new_string` is not found in the file, NO context is injected and the
//! result is exactly the legacy fragments-only
//! [`super::diff_format::build_diff_rows`] — mirroring `calculate_start_line`'s
//! graceful `return 1`.
//!
//! ## Known limitation
//! The changed region is located via the FIRST occurrence of `new_string` in
//! the file (`calculate_start_line`). If `new_string` appears earlier than the
//! actual edit, the injected context anchors to the wrong region. This is
//! inherited from RPC-390 / TS-reference parity.

use super::diff_format::{
    build_diff_rows, calculate_start_line, format_edit_diff, format_with_tree_connectors, to_line,
    DiffDisplayRow, CONTEXT_LINES,
};

/// **RPC-394**: serialize already-built [`DiffDisplayRow`]s to the canonical
/// display string (each row via [`to_line`], then tree connectors). Used by the
/// context-aware Edit path which builds rows directly instead of from
/// `DiffOutputLine`s.
pub fn with_tree_connectors(rows: &[DiffDisplayRow]) -> String {
    let joined = rows.iter().map(to_line).collect::<Vec<_>>().join("\n");
    format_with_tree_connectors(&joined)
}

/// Build the typed diff display rows for an Edit, injecting up to
/// [`CONTEXT_LINES`] real unchanged file lines before/after the changed region.
///
/// `file_path` is the POST-EDIT file (the edit has already been applied on
/// disk). `visible_lines` is the collapse window passed through to
/// [`build_diff_rows`]. Falls back to fragments-only on any IO/locate failure
/// (see module docs); never panics, no `unwrap`/`expect`.
pub fn build_edit_diff_rows_with_context(
    old_string: &str,
    new_string: &str,
    file_path: Option<&str>,
    visible_lines: usize,
) -> Vec<DiffDisplayRow> {
    let start_line = calculate_start_line(file_path, Some(old_string), Some(new_string));
    let fragment_lines = format_edit_diff(old_string, new_string);

    // Fallback path: no file / unreadable / new_string empty or not locatable.
    let Some(context) = read_context(file_path, new_string, start_line) else {
        return build_diff_rows(&fragment_lines, visible_lines, start_line);
    };

    let fragment_rows = build_diff_rows(&fragment_lines, visible_lines, start_line);
    merge(context.before, fragment_rows, context.after)
}

/// The before/after unchanged file lines surrounding a change, with their real
/// 1-based file line numbers.
struct ContextWindows {
    before: Vec<DiffDisplayRow>,
    after: Vec<DiffDisplayRow>,
}

/// Read the post-edit file and slice up to [`CONTEXT_LINES`] unchanged lines
/// before `start_line` and after the `new_string` span. Returns `None`
/// (signalling fallback) when the path is missing, unreadable, or `new_string`
/// is empty (a Write / pure addition has no surrounding-context concept here).
fn read_context(
    file_path: Option<&str>,
    new_string: &str,
    start_line: usize,
) -> Option<ContextWindows> {
    if new_string.is_empty() {
        return None;
    }
    let path = file_path?;
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    // `start_line` is 1-based; the changed region in the post-edit file is the
    // `new_string` span [start_line .. start_line + new_count).
    let new_count = new_string.lines().count().max(1);
    let start_idx = start_line.saturating_sub(1); // 0-based index of first changed line
    let after_idx = start_idx + new_count; // 0-based index of first line AFTER the span

    let before_from = start_idx.saturating_sub(CONTEXT_LINES);
    let before = slice_context(&lines, before_from, start_idx);

    let after_to = (after_idx + CONTEXT_LINES).min(lines.len());
    let after = slice_context(&lines, after_idx, after_to);

    Some(ContextWindows { before, after })
}

/// Build `Context` rows for file lines in the 0-based half-open range
/// `[from .. to)`, each numbered with its real 1-based file line number.
fn slice_context(lines: &[&str], from: usize, to: usize) -> Vec<DiffDisplayRow> {
    let to = to.min(lines.len());
    if from >= to {
        return Vec::new();
    }
    lines[from..to]
        .iter()
        .enumerate()
        .map(|(offset, text)| DiffDisplayRow::Context {
            line_no: from + offset + 1,
            text: (*text).to_string(),
        })
        .collect()
}

/// Concatenate `[before-context] + [fragment rows] + [after-context]`.
fn merge(
    before: Vec<DiffDisplayRow>,
    fragments: Vec<DiffDisplayRow>,
    after: Vec<DiffDisplayRow>,
) -> Vec<DiffDisplayRow> {
    let mut rows = Vec::with_capacity(before.len() + fragments.len() + after.len());
    rows.extend(before);
    rows.extend(fragments);
    rows.extend(after);
    rows
}
