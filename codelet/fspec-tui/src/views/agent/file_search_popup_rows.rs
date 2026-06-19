//! RPC-028 — `build_rows` helper for FileSearchPopup.
//!
//! Extracted from `file_search_popup.rs` so the parent file stays
//! under the 300-LoC source-shape budget enforced by
//! `tests/source_shape_rpc019.rs`. Pure function; no state.

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::components::dialog_theme::{DialogRow, MARKER_SELECTED, MARKER_UNSELECTED};

/// Window `matches[scroll_offset..scroll_offset+visible_rows]` and
/// convert each visible row into a `DialogRow`. Inserts `↑`/`↓`
/// indicators on the top/bottom visible rows when the list overflows
/// the window. Empty-state literals (`(type to search files)` /
/// `(no files match "<filter>")`) are emitted as a single
/// non-selectable row.
pub(super) fn build_rows(
    matches: &[String],
    filter: &str,
    selected_index: usize,
    scroll_offset: usize,
    visible_rows: usize,
) -> Vec<DialogRow> {
    if matches.is_empty() {
        let label = if filter.is_empty() {
            "(type to search files)".to_string()
        } else {
            format!("(no files match \"{filter}\")")
        };
        return vec![DialogRow {
            spans: vec![Span::raw(MARKER_UNSELECTED.to_string()), Span::raw(label)],
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
        let path = &matches[abs_i];
        let is_sel = abs_i == selected_index;
        let marker = if is_sel {
            MARKER_SELECTED
        } else {
            MARKER_UNSELECTED
        };
        out.push(DialogRow {
            spans: vec![Span::raw(marker.to_string()), Span::raw(path.clone())],
            selectable: true,
            selected: is_sel,
        });
    }
    out
}
