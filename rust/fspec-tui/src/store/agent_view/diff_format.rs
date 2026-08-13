//! RPC-390/393 — Edit/Write diff generation + typed display-row building.
//!
//! Feature: spec/features/agentview-edit-diff-generation.feature
//!          spec/features/agentview-edit-diff-structured-rows.feature
//!
//! **RPC-393**: the display layer is a typed [`DiffDisplayRow`] model;
//! [`build_diff_rows`] does windowing/collapse and the single codec
//! ([`super::diff_codec`]) serializes/parses the canonical line stored on
//! `ChunkSource::text` (re-wrapped on resize).

use similar::{ChangeTag, TextDiff};

/// Number of display lines kept before collapsing a diff (TS
/// `DIFF_COLLAPSED_LINES`).
pub const DIFF_COLLAPSED_LINES: usize = 25;

/// Lines of surrounding context shown around each change (TS `CONTEXT_LINES`).
pub(super) const CONTEXT_LINES: usize = 3;

/// Minimum line-number gutter width (right-aligned). Shared with the codec.
pub const GUTTER_MIN_WIDTH: usize = 3;

/// Kind of a single encoded diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOutputKind {
    Context,
    Added,
    Removed,
}

/// **RPC-393**: the typed diff DISPLAY row model + its single codec.
pub use super::diff_codec::{parse_line, to_line, DiffDisplayRow};

/// **RPC-394**: the context-aware Edit-diff builder (injects real surrounding
/// file lines). Lives in a sibling module to keep this file < 300 LoC.
pub use super::diff_context::{build_edit_diff_rows_with_context, with_tree_connectors};

/// A single diff line: `content` carries the TS prefix char (` `/`+`/`-`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffOutputLine {
    pub content: String,
    pub kind: DiffOutputKind,
}

/// Port of `changesToDiffLines`: Myers line diff of `(old, new)` →
/// prefixed `DiffOutputLine`s. Empty fragments dropped (TS parity).
pub fn format_edit_diff(old_string: &str, new_string: &str) -> Vec<DiffOutputLine> {
    let diff = TextDiff::from_lines(old_string, new_string);
    let mut result: Vec<DiffOutputLine> = Vec::new();

    for change in diff.iter_all_changes() {
        let value = change.value();
        let (prefix, kind) = match change.tag() {
            ChangeTag::Equal => (' ', DiffOutputKind::Context),
            ChangeTag::Delete => ('-', DiffOutputKind::Removed),
            ChangeTag::Insert => ('+', DiffOutputKind::Added),
        };
        // Mirror `change.value.split('\n').filter(len > 0)`.
        for line in value.split('\n').filter(|l| !l.is_empty()) {
            result.push(DiffOutputLine {
                content: format!("{prefix}{line}"),
                kind,
            });
        }
    }

    result
}

/// Port of `formatWriteDiff`: every line of `content` becomes an addition
/// (`"+{line}"`). Splits on `\n` without filtering empties (TS parity).
pub fn format_write_diff(content: &str) -> Vec<DiffOutputLine> {
    content
        .split('\n')
        .map(|line| DiffOutputLine {
            content: format!("+{line}"),
            kind: DiffOutputKind::Added,
        })
        .collect()
}

/// Port of `formatWithTreeConnectors`: empty/whitespace-only → `""`; else the
/// first line is prefixed `"L "` and every subsequent line indented two spaces.
pub fn format_with_tree_connectors(content: &str) -> String {
    if content.trim().is_empty() {
        return String::new();
    }
    content
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                format!("L {line}")
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Port of `formatDiffForDisplay`: the canonical display string. **RPC-393**:
/// a THIN wrapper over [`build_diff_rows`] + [`to_line`] (RPC-390 golden).
pub fn format_diff_for_display(
    diff_lines: &[DiffOutputLine],
    visible_lines: usize,
    start_line: usize,
) -> String {
    with_tree_connectors(&build_diff_rows(diff_lines, visible_lines, start_line))
}

/// **RPC-393**: build the typed, context-windowed, collapse-truncated diff
/// DISPLAY rows. Same windowing/collapse as the legacy formatter, but emits
/// [`DiffDisplayRow`]s.
pub fn build_diff_rows(
    diff_lines: &[DiffOutputLine],
    visible_lines: usize,
    start_line: usize,
) -> Vec<DiffDisplayRow> {
    let changed_indices: Vec<usize> = diff_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| matches!(l.kind, DiffOutputKind::Added | DiffOutputKind::Removed))
        .map(|(i, _)| i)
        .collect();

    let max_line_num = start_line + diff_lines.len().saturating_sub(1);
    let gutter_width = gutter_width_for(max_line_num);

    if changed_indices.is_empty() {
        return build_no_change_rows(diff_lines, visible_lines, start_line, gutter_width);
    }
    let sorted_indices = indices_to_show(&changed_indices, diff_lines.len());
    let rows = build_change_rows(diff_lines, &sorted_indices, start_line, gutter_width);

    if rows.len() <= visible_lines {
        return rows;
    }

    let mut visible: Vec<DiffDisplayRow> = rows[..visible_lines].to_vec();
    visible.push(collapse_hint(rows.len() - visible_lines, gutter_width));
    visible
}

/// **RPC-393 (C1 + W6)**: the SINGLE gutter-column width for a diff whose
/// largest line number is `max_line_num`. The line-number gutter AND every
/// elision row's leading indent derive from this so columns line up at any
/// file size (incl. 1000+ line edits).
pub fn gutter_width_for(max_line_num: usize) -> usize {
    max_line_num.to_string().len().max(GUTTER_MIN_WIDTH)
}

/// **RPC-393 (C1 + W6)**: the ONE helper deciding elision leading indent; gap
/// markers AND the collapse hint both use it.
fn elision_indent(gutter_width: usize) -> String {
    " ".repeat(gutter_width)
}

/// The single collapse-hint Elision row, indented via [`elision_indent`] like
/// gap markers (C1).
fn collapse_hint(remaining: usize, gutter_width: usize) -> DiffDisplayRow {
    let indent = elision_indent(gutter_width);
    DiffDisplayRow::Elision {
        text: format!("{indent} ... +{remaining} lines (select turn to /expand)"),
    }
}

/// No-changes branch: the first `visible_lines` diff lines become Context
/// rows; a trailing collapse Elision row is appended when truncated.
fn build_no_change_rows(
    diff_lines: &[DiffOutputLine],
    visible_lines: usize,
    start_line: usize,
    gutter_width: usize,
) -> Vec<DiffDisplayRow> {
    let take = diff_lines.len().min(visible_lines);
    let mut rows: Vec<DiffDisplayRow> = diff_lines[..take]
        .iter()
        .enumerate()
        .map(|(idx, line)| DiffDisplayRow::Context {
            line_no: start_line + idx,
            text: strip_prefix(&line.content).to_string(),
        })
        .collect();
    if diff_lines.len() > visible_lines {
        rows.push(collapse_hint(
            diff_lines.len() - visible_lines,
            gutter_width,
        ));
    }
    rows
}

/// Build the indices to show: each changed index plus up to `CONTEXT_LINES`
/// before/after, clamped to the diff bounds.
fn indices_to_show(changed_indices: &[usize], len: usize) -> Vec<usize> {
    let mut set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for &idx in changed_indices {
        let from = idx.saturating_sub(CONTEXT_LINES);
        let to = (idx + CONTEXT_LINES).min(len.saturating_sub(1));
        for i in from..=to {
            set.insert(i);
        }
    }
    set.into_iter().collect()
}

/// Walk the shown indices, emitting typed rows plus uniform Elision rows for
/// skipped/trailing regions, indented through [`elision_indent`] (C1/W6).
fn build_change_rows(
    diff_lines: &[DiffOutputLine],
    sorted_indices: &[usize],
    start_line: usize,
    gutter_width: usize,
) -> Vec<DiffDisplayRow> {
    let mut rows: Vec<DiffDisplayRow> = Vec::new();
    let pad = elision_indent(gutter_width);
    let mut last_shown: Option<usize> = None;

    for &idx in sorted_indices {
        if let Some(last) = last_shown {
            if idx > last + 1 {
                let skipped = idx - last - 1;
                rows.push(DiffDisplayRow::Elision {
                    text: format!("{pad} ... ({skipped} lines)"),
                });
            }
        }
        let line = &diff_lines[idx];
        let line_no = start_line + idx;
        let text = strip_prefix(&line.content).to_string();
        rows.push(match line.kind {
            DiffOutputKind::Removed => DiffDisplayRow::Removed { line_no, text },
            DiffOutputKind::Added => DiffDisplayRow::Added { line_no, text },
            DiffOutputKind::Context => DiffDisplayRow::Context { line_no, text },
        });
        last_shown = Some(idx);
    }

    if let Some(last) = last_shown {
        if last < diff_lines.len().saturating_sub(1) {
            let remaining = diff_lines.len() - 1 - last;
            rows.push(DiffDisplayRow::Elision {
                text: format!("{pad} ... ({remaining} lines)"),
            });
        }
    }

    rows
}

/// Strip the leading prefix char (` `/`+`/`-`), mirroring TS `content.slice(1)`.
pub(super) fn strip_prefix(content: &str) -> &str {
    let mut chars = content.char_indices();
    chars.next();
    match chars.next() {
        Some((i, _)) => &content[i..],
        None => "",
    }
}

/// Port of `calculateStartLine`: the 1-based line of the edit within the file.
/// Returns 1 when the path is `None`, the file cannot be read, or neither
/// string is found. Never panics on IO error.
pub fn calculate_start_line(
    file_path: Option<&str>,
    old_string: Option<&str>,
    new_string: Option<&str>,
) -> usize {
    let Some(path) = file_path else {
        return 1;
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return 1;
    };

    if let Some(new_s) = new_string {
        if !new_s.is_empty() {
            if let Some(idx) = content.find(new_s) {
                return line_of(&content, idx);
            }
        }
    }
    if let Some(old_s) = old_string {
        if !old_s.is_empty() {
            if let Some(idx) = content.find(old_s) {
                return line_of(&content, idx);
            }
        }
    }
    1
}

/// 1-based line number of byte offset `idx` within `content`.
fn line_of(content: &str, idx: usize) -> usize {
    content[..idx].matches('\n').count() + 1
}
