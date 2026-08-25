//! BOARD-022 / BUG-160 — `build_rows` helper for WorkUnitSearchDialog.
//!
//! Feature: spec/features/board-search-dialog-result-snippet.feature
//!
//! Extracted from `work_unit_search_dialog.rs` so the parent file stays
//! under the 300-LoC source-shape budget. Pure function; no state.
//! Mirrors `views/agent/file_search_popup_rows.rs`: windows the matches
//! into the visible rows, adds `↑`/`↓` overflow indicators, and emits a
//! single non-selectable empty-state row when there are no matches.
//! BUG-160: each match row is built via the shared
//! `label_description_row` primitive (marker + id + dimmed snippet) and
//! the snippet is width-bounded with `truncate_to` so a long
//! title/description cannot widen the fixed frame (BUG-159).

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::dialog_theme::DialogRow;
use super::dialog_theme_rows::label_description_row;
use super::work_unit_search_filter::SearchMatch;

/// BUG-160: truncate `s` to `max_chars` columns, appending a trailing
/// `…` when it overflows (the shared end-ellipsis contract — same shape
/// as `views/board/details_strip.rs::truncate_to` and
/// `views/diff_common::truncate_path`). `max_chars == 0` yields an empty
/// string.
pub fn truncate_to(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if s.width() <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = s.chars().take(keep).collect();
    out.push('…');
    out
}

/// Window `matches[scroll_offset..scroll_offset+visible_rows]` and
/// convert each visible row into a `DialogRow` showing the work-unit id
/// plus the BUG-160 dimmed snippet (via `label_description_row`).
/// Inserts `↑`/`↓` indicators when the list overflows the window.
/// Empty-state literals (`(no work units match "<query>")` /
/// `(board is empty)`) are emitted as a single non-selectable row.
pub(super) fn build_rows(
    matches: &[SearchMatch],
    filter: &str,
    selected_index: usize,
    scroll_offset: usize,
    visible_rows: usize,
    snippet_budget: usize,
) -> Vec<DialogRow> {
    if matches.is_empty() {
        let label = if filter.is_empty() {
            "(board is empty)".to_string()
        } else {
            format!("(no work units match \"{filter}\")")
        };
        return vec![DialogRow {
            spans: vec![Span::raw("  ".to_string()), Span::raw(label)],
            selectable: false,
            selected: false,
        }];
    }
    let vr = visible_rows.max(1);
    let total = matches.len();
    let so = scroll_offset;
    let up_arrow = so > 0;
    let down_arrow = so + vr < total;
    let end = (so + vr).min(total);
    let mut out: Vec<DialogRow> = Vec::with_capacity(vr);
    for (rel, abs_i) in (so..end).enumerate() {
        let is_first_visible = rel == 0;
        let is_last_visible = rel + 1 == end - so;
        if up_arrow && is_first_visible {
            out.push(DialogRow {
                spans: vec![Span::styled(
                    "↑".to_string(),
                    Style::default().add_modifier(Modifier::DIM),
                )],
                selectable: false,
                selected: false,
            });
            continue;
        }
        if down_arrow && is_last_visible {
            out.push(DialogRow {
                spans: vec![Span::styled(
                    "↓".to_string(),
                    Style::default().add_modifier(Modifier::DIM),
                )],
                selectable: false,
                selected: false,
            });
            continue;
        }
        let m = &matches[abs_i];
        let is_sel = abs_i == selected_index;
        // BUG-160: reuse the canonical "marker + label - dimmed
        // description" row; the snippet is width-bounded so it cannot
        // widen the fixed frame (BUG-159).
        let snippet = truncate_to(&m.snippet, snippet_budget);
        out.push(label_description_row(&m.id, &snippet, is_sel));
    }
    out
}
