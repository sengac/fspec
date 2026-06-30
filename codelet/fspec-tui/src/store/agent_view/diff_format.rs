//! RPC-390 — Edit/Write diff generation + `[R]-`/`[A]+` marker encoding.
//!
//! Feature: spec/features/agentview-edit-diff-generation.feature
//!
//! Pure port of the TS diff pipeline:
//!   - `computeLineDiff` / `changesToDiffLines` (src/git/diff-parser.ts)
//!   - `formatEditDiff` / `formatWriteDiff` / `formatDiffForDisplay` /
//!     `formatWithTreeConnectors` / `calculateStartLine`
//!     (src/tui/components/AgentView.tsx:530-817)
//!
//! No rendering / coloring / wire-up here — that is RPC-391. This module only
//! produces the marker-encoded display string byte-for-byte identical to the
//! TS reference for the same inputs.

use similar::{ChangeTag, TextDiff};

/// Number of display lines kept before collapsing a diff (TS
/// `DIFF_COLLAPSED_LINES`, AgentView.tsx:535).
pub const DIFF_COLLAPSED_LINES: usize = 25;

/// Lines of surrounding context shown around each change (TS `CONTEXT_LINES`,
/// AgentView.tsx:703).
const CONTEXT_LINES: usize = 3;

/// Kind of a single encoded diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOutputKind {
    Context,
    Added,
    Removed,
}

/// A single diff line: `content` carries the TS prefix character (` `, `+`,
/// `-`) as its first byte, exactly like the TS `DiffOutputLine.content`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffOutputLine {
    pub content: String,
    pub kind: DiffOutputKind,
}

/// Port of `changesToDiffLines`: turn the Myers line diff of `(old, new)` into
/// prefixed `DiffOutputLine`s. Context → `" {line}"`, removed → `"-{line}"`,
/// added → `"+{line}"`. Empty line fragments are dropped (parity with the TS
/// `split('\n').filter(line => line.length > 0)`).
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
/// (`"+{line}"`). Splits on `\n` WITHOUT filtering empties (parity with the TS
/// `content.split('\n')`).
pub fn format_write_diff(content: &str) -> Vec<DiffOutputLine> {
    content
        .split('\n')
        .map(|line| DiffOutputLine {
            content: format!("+{line}"),
            kind: DiffOutputKind::Added,
        })
        .collect()
}

/// Port of `formatWithTreeConnectors`: empty/whitespace-only → `""`; otherwise
/// the first line is prefixed `"L "` and every subsequent line indented two
/// spaces.
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

/// Port of `formatDiffForDisplay` (AgentView.tsx:670-771): turn the encoded
/// diff lines into the marker-encoded, context-windowed, collapse-truncated
/// display string with tree connectors applied.
pub fn format_diff_for_display(
    diff_lines: &[DiffOutputLine],
    visible_lines: usize,
    start_line: usize,
) -> String {
    let changed_indices: Vec<usize> = diff_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| matches!(l.kind, DiffOutputKind::Added | DiffOutputKind::Removed))
        .map(|(i, _)| i)
        .collect();

    let max_line_num = start_line + diff_lines.len().saturating_sub(1);
    let line_num_width = max_line_num.to_string().len().max(3);

    if changed_indices.is_empty() {
        return format_no_changes(diff_lines, visible_lines, start_line, line_num_width);
    }

    let sorted_indices = indices_to_show(&changed_indices, diff_lines.len());
    let output_lines = build_output_lines(diff_lines, &sorted_indices, start_line, line_num_width);

    if output_lines.len() <= visible_lines {
        return format_with_tree_connectors(&output_lines.join("\n"));
    }

    let mut visible: Vec<String> = output_lines[..visible_lines].to_vec();
    let remaining = output_lines.len() - visible_lines;
    visible.push(format!("... +{remaining} lines (select turn to /expand)"));
    format_with_tree_connectors(&visible.join("\n"))
}

/// No-changes branch: show the first `visible_lines` lines as context, with a
/// trailing collapse indicator when truncated.
fn format_no_changes(
    diff_lines: &[DiffOutputLine],
    visible_lines: usize,
    start_line: usize,
    line_num_width: usize,
) -> String {
    let take = diff_lines.len().min(visible_lines);
    let mut formatted: Vec<String> = diff_lines[..take]
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            let line_num = pad_left(start_line + idx, line_num_width);
            let rest = strip_prefix(&line.content);
            format!("{line_num}   {rest}")
        })
        .collect();
    if diff_lines.len() > visible_lines {
        let remaining = diff_lines.len() - visible_lines;
        formatted.push(format!("... +{remaining} lines (select turn to /expand)"));
    }
    format_with_tree_connectors(&formatted.join("\n"))
}

/// Build the sorted, deduplicated set of indices to show: each changed index
/// plus up to `CONTEXT_LINES` before/after, clamped to the diff bounds.
fn indices_to_show(changed_indices: &[usize], len: usize) -> Vec<usize> {
    let mut set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for &idx in changed_indices {
        set.insert(idx);
        let from = idx.saturating_sub(CONTEXT_LINES);
        for i in from..idx {
            set.insert(i);
        }
        let to = (idx + CONTEXT_LINES).min(len.saturating_sub(1));
        for i in (idx + 1)..=to {
            set.insert(i);
        }
    }
    set.into_iter().collect()
}

/// Walk the shown indices, emitting gap markers for skipped regions and a
/// trailing gap marker if the diff continues past the last shown index.
fn build_output_lines(
    diff_lines: &[DiffOutputLine],
    sorted_indices: &[usize],
    start_line: usize,
    line_num_width: usize,
) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    let pad = " ".repeat(line_num_width);
    let mut last_shown: Option<usize> = None;

    for &idx in sorted_indices {
        if let Some(last) = last_shown {
            if idx > last + 1 {
                let skipped = idx - last - 1;
                output.push(format!("{pad} ... ({skipped} lines)"));
            }
        }
        let line = &diff_lines[idx];
        let line_num = pad_left(start_line + idx, line_num_width);
        let rest = strip_prefix(&line.content);
        let formatted = match line.kind {
            DiffOutputKind::Removed => format!("{line_num} [R]- {rest}"),
            DiffOutputKind::Added => format!("{line_num} [A]+ {rest}"),
            DiffOutputKind::Context => format!("{line_num}   {rest}"),
        };
        output.push(formatted);
        last_shown = Some(idx);
    }

    if let Some(last) = last_shown {
        if last < diff_lines.len().saturating_sub(1) {
            let remaining = diff_lines.len() - 1 - last;
            output.push(format!("{pad} ... ({remaining} lines)"));
        }
    }

    output
}

/// Strip the leading prefix char (` `/`+`/`-`) from an encoded content string,
/// mirroring TS `content.slice(1)`.
fn strip_prefix(content: &str) -> &str {
    let mut chars = content.char_indices();
    match chars.next() {
        Some(_) => match chars.next() {
            Some((i, _)) => &content[i..],
            None => "",
        },
        None => "",
    }
}

/// Left-pad a line number to at least `width` columns with spaces.
fn pad_left(value: usize, width: usize) -> String {
    let s = value.to_string();
    if s.len() >= width {
        s
    } else {
        format!("{}{}", " ".repeat(width - s.len()), s)
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
