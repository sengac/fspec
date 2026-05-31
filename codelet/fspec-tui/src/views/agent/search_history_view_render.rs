//! RPC-064 — Render helpers for [`super::SearchHistoryView`].
//!
//! Feature: spec/features/search-history-debounce-and-polish.feature
//!
//! Extracted from `search_history_view.rs` to keep that file under
//! the 300-LoC source-shape ceiling pinned by
//! `tests/rpc026_source_shape.rs`. The view delegates its painting
//! pipeline (title / body / footer) to these free functions which
//! receive an immutable borrow of the view's state.
//!
//! RPC-064 additions on top of the RPC-026 baseline:
//!   * `highlight_query` — splits a result row's text into BOLD vs
//!     plain spans for each case-insensitive occurrence of the live
//!     filter query, preserving the original casing.
//!   * `render_body` — uses `highlight_query` so the visible matches
//!     visually answer the question "WHY is this row relevant?".

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::search_history_view::SearchHistoryView;

/// Paint the search palette's 1-row title (with editable filter and
/// inverse-cursor block).
pub(super) fn render_title(view: &SearchHistoryView, area: Rect, buf: &mut Buffer) {
    let cursor_style = Style::default().add_modifier(Modifier::REVERSED);
    let spans = vec![
        Span::raw("(search): "),
        Span::raw(view.query().to_string()),
        Span::styled(" ", cursor_style),
    ];
    Paragraph::new(Line::from(spans)).render(area, buf);
}

/// Paint the 1-row footer with keybinding hints.
pub(super) fn render_footer(_view: &SearchHistoryView, area: Rect, buf: &mut Buffer) {
    Paragraph::new("Enter Select | ↑↓ Navigate | Esc Cancel").render(area, buf);
}

/// Paint the body (either the placeholder OR the scrollable list of
/// matched history entries with the live query highlighted).
pub(super) fn render_body(view: &SearchHistoryView, area: Rect, buf: &mut Buffer) {
    if view.matches().is_empty() {
        let placeholder = if view.query().is_empty() {
            "(type to search history)".to_string()
        } else {
            format!("(no history matches \"{}\")", view.query())
        };
        let mid_y = area.y.saturating_add(area.height / 2);
        let row = Rect { x: area.x, y: mid_y, width: area.width, height: 1 };
        Paragraph::new(placeholder)
            .alignment(Alignment::Center)
            .render(row, buf);
        return;
    }
    let visible_rows = area.height as usize;
    if visible_rows == 0 {
        return;
    }
    let end = (view.scroll_offset() + visible_rows).min(view.matches().len());
    for (row_idx, m) in view.matches()[view.scroll_offset()..end].iter().enumerate() {
        let global_idx = view.scroll_offset() + row_idx;
        let selected = global_idx == view.selected_index();
        let marker = if selected { "▸" } else { " " };
        let row_style = if selected {
            // RPC-064: drop BOLD from the selected row's base style so
            // the per-substring query highlight (which adds BOLD on
            // top) is visually distinguishable from non-highlighted
            // text. REVERSED alone is enough to mark the selected row.
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        // Prefix (" ▸ " / "   ") is rendered with the row's base style
        // (REVERSED inversion on the selected row, plain on others) but
        // never with the query-highlight BOLD by itself.
        let mut spans: Vec<Span<'static>> =
            vec![Span::styled(format!(" {marker} "), row_style)];
        spans.extend(highlight_query(&m.text, view.query(), row_style));

        let y = area.y + row_idx as u16;
        let row_area = Rect { x: area.x, y, width: area.width, height: 1 };
        Paragraph::new(Line::from(spans)).render(row_area, buf);
    }
}

/// RPC-064: split `text` into a `Vec<Span>` where every case-insensitive
/// occurrence of `query` is wrapped in a BOLD span (added on top of
/// `base_style`) and the surrounding text uses `base_style` plain.
///
/// Empty `query` returns a single span over the full text — there is
/// nothing to highlight in that case.
///
/// Returned spans use `'static` lifetimes because each span owns its
/// String (a clone of the matched slice).
pub fn highlight_query(text: &str, query: &str, base_style: Style) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    if lower_query.is_empty() {
        return vec![Span::styled(text.to_string(), base_style)];
    }
    let qlen = lower_query.len();
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() {
        match lower_text[cursor..].find(&lower_query) {
            Some(rel) => {
                let absolute = cursor + rel;
                if absolute > cursor {
                    out.push(Span::styled(text[cursor..absolute].to_string(), base_style));
                }
                let bold_end = absolute + qlen;
                let bold_style = base_style.add_modifier(Modifier::BOLD);
                out.push(Span::styled(text[absolute..bold_end].to_string(), bold_style));
                cursor = bold_end;
            }
            None => {
                out.push(Span::styled(text[cursor..].to_string(), base_style));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn highlight_query_empty_query_returns_single_plain_span() {
        let spans = highlight_query("hello world", "", Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello world");
        assert!(!spans[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn highlight_query_single_occurrence_splits_into_three_spans() {
        let spans = highlight_query("git status now", "git", Style::default());
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "git");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, " status now");
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn highlight_query_case_insensitive_preserves_original_casing() {
        let spans = highlight_query("GIT add then git push", "git", Style::default());
        // Expect: BOLD("GIT") + plain(" add then ") + BOLD("git") + plain(" push")
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].content, "GIT");
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, " add then ");
        assert!(!spans[1].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[2].content, "git");
        assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[3].content, " push");
        assert!(!spans[3].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn highlight_query_no_match_returns_single_plain_span() {
        let spans = highlight_query("foo bar", "xyz", Style::default());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "foo bar");
        assert!(!spans[0].style.add_modifier.contains(Modifier::BOLD));
    }
}
