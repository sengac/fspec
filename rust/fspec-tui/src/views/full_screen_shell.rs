//! RPC-337 — shared full-screen shell scaffold.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Extracts the `Clear` + 4-constraint (title / separator / body /
//! footer) Layout + optional `ConfirmDialog` overlay scaffold that was
//! hand-rolled three times (ProviderSettingsView, ResumeSessionView,
//! SearchHistoryView — RPC-026/RPC-054). The body sub-rect is handed to
//! a caller-supplied closure so each view keeps ownership of its own
//! state (e.g. ProviderSettingsView captures `body_area.height` into
//! `self.visible_rows` from inside the closure).
//!
//! TUI-131 (spec/features/long-text-wraps-instead-of-being-cut-off.feature):
//! the footer hint wraps onto as many rows as it needs and the body gets
//! the rows that are left ([`scaffold_rects`]).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Clear, Widget};

use crate::views::agent::confirm_dialog::ConfirmDialog;
use crate::views::agent::mode_view_render::{render_footer_hint, render_title_with_count};
use crate::views::agent::text_wrap::wrap_to_width;

/// TUI-131: the scaffold's split of a full-screen area — the title row,
/// the body and the footer rows (a separator row sits between title and
/// body). The footer takes as many rows as its hint wraps onto at the
/// area's width; the body gets the rows that are left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScaffoldRects {
    pub title: Rect,
    pub body: Rect,
    pub footer: Rect,
}

/// TUI-131: split `area` the way the scaffold paints it, with the footer
/// wrapped to the area's width (`text_wrap::wrap_to_width`). Both the
/// render and every page-size or hit-test computation use this, so the
/// body they assume is the body that is painted.
pub(crate) fn scaffold_rects(area: Rect, footer_hint: &str) -> ScaffoldRects {
    let title_rows = area.height.min(1);
    let separator_rows = area.height.saturating_sub(title_rows).min(1);
    let rest = area.height - title_rows - separator_rows;
    let wrapped = wrap_to_width(footer_hint, usize::from(area.width)).len();
    let footer_rows = u16::try_from(wrapped).unwrap_or(u16::MAX).min(rest);
    let body_rows = rest - footer_rows;
    let row = |offset: u16, height: u16| Rect {
        y: area.y + offset,
        height,
        ..area
    };
    ScaffoldRects {
        title: row(0, title_rows),
        body: row(title_rows + separator_rows, body_rows),
        footer: row(title_rows + separator_rows + body_rows, footer_rows),
    }
}

/// TUI-103: the 1-column scrollbar gutter rect the scaffold reserves in
/// the rightmost body column when the content list exceeds
/// `visible_rows` entries. Views with a TUI-101 click-and-drag
/// `ScrollbarDrag` pre-compute this from the same body rect the shell
/// paints ([`scaffold_rects`], TUI-131: below a wrapped footer too) and
/// cache it on render for mouse hit-testing. `None` when no scrollbar
/// shows.
pub(crate) fn mode_view_scrollbar_rect(
    area: Rect,
    footer_hint: &str,
    visible_rows: usize,
    count: usize,
) -> Option<Rect> {
    if count <= visible_rows {
        return None;
    }
    let body = scaffold_rects(area, footer_hint).body;
    let content_width = body.width.saturating_sub(1);
    Some(Rect {
        x: body.x + content_width,
        y: body.y,
        width: 1,
        height: body.height,
    })
}

/// Render the shared full-screen scaffold:
///   1. `Clear.render(area, buf)` — overwrite the underlying view.
///   2. Vertical split [`scaffold_rects`]: title, separator, body, and
///      the footer on the rows its hint wraps onto (TUI-131).
///   3. `render_title_with_count(title_area, title, count, suffix)`.
///   4. `body_fn(body_area, buf)` — caller paints the body.
///   5. `render_footer_hint(footer_area, footer_hint)`.
///   6. If `overlay` is `Some`, paint the `ConfirmDialog` over the FULL
///      area AFTER the body.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_full_screen_scaffold<F>(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    count: usize,
    suffix: &str,
    footer_hint: &str,
    body_fn: F,
    overlay: Option<&ConfirmDialog>,
) where
    F: FnOnce(Rect, &mut Buffer),
{
    render_full_screen_scaffold_with_title(
        area,
        buf,
        |title_area, buf| render_title_with_count(title_area, buf, title, count, suffix),
        footer_hint,
        body_fn,
        overlay,
    );
}

/// Generalized variant of [`render_full_screen_scaffold`] (RPC-339) that
/// hands the 1-row title region to a caller-supplied closure instead of
/// the `{title} ({count} {suffix})` builder. This lets views with a
/// non-count title — e.g. SearchHistoryView's editable query input
/// (`(search): <query>` + inverse cursor) — reuse the same `Clear` +
/// 4-constraint split + optional `ConfirmDialog` overlay scaffold. The
/// count-title [`render_full_screen_scaffold`] and the verbatim
/// [`render_full_screen_scaffold_raw_title`] are both expressible on top
/// of this; `render_full_screen_scaffold` delegates here directly.
pub(crate) fn render_full_screen_scaffold_with_title<T, B>(
    area: Rect,
    buf: &mut Buffer,
    title_fn: T,
    footer_hint: &str,
    body_fn: B,
    overlay: Option<&ConfirmDialog>,
) where
    T: FnOnce(Rect, &mut Buffer),
    B: FnOnce(Rect, &mut Buffer),
{
    Clear.render(area, buf);
    let rects = scaffold_rects(area, footer_hint);
    let (title_area, body_area, footer_area) = (rects.title, rects.body, rects.footer);
    title_fn(title_area, buf);
    body_fn(body_area, buf);
    render_footer_hint(footer_area, buf, footer_hint);
    if let Some(dialog) = overlay {
        dialog.render(area, buf);
    }
}

/// Variant of [`render_full_screen_scaffold`] that paints a pre-formatted
/// title string verbatim (e.g. `"Select Model (3 models)"` or
/// `"Select Model (3 models) (refreshing...)"`) instead of the
/// `{title} ({count} {suffix})` builder. Used by the ModelSelector
/// mode-view whose title already embeds the count + refresh state.
pub(crate) fn render_full_screen_scaffold_raw_title<F>(
    area: Rect,
    buf: &mut Buffer,
    title: &str,
    footer_hint: &str,
    body_fn: F,
    overlay: Option<&ConfirmDialog>,
) where
    F: FnOnce(Rect, &mut Buffer),
{
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    Clear.render(area, buf);
    let rects = scaffold_rects(area, footer_hint);
    let (title_area, body_area, footer_area) = (rects.title, rects.body, rects.footer);
    let style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    Paragraph::new(Line::from(Span::styled(title.to_string(), style))).render(title_area, buf);
    body_fn(body_area, buf);
    render_footer_hint(footer_area, buf, footer_hint);
    if let Some(dialog) = overlay {
        dialog.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::views::agent::confirm_dialog::ConfirmDialog;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::cell::Cell;

    /// Render the scaffold into a `width`x`height` TestBackend and return
    /// the joined rows as a single newline-separated string.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_to_string(
        width: u16,
        height: u16,
        title: &str,
        count: usize,
        suffix: &str,
        footer: &str,
        body_text: &str,
        overlay: Option<&ConfirmDialog>,
    ) -> String {
        let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
        term.draw(|frame| {
            render_full_screen_scaffold(
                frame.area(),
                frame.buffer_mut(),
                title,
                count,
                suffix,
                footer,
                |body_area, buf| {
                    use ratatui::widgets::{Paragraph, Widget};
                    Paragraph::new(body_text).render(body_area, buf);
                },
                overlay,
            );
        })
        .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                joined.push_str(buf[(x, y)].symbol());
            }
            joined.push('\n');
        }
        joined
    }

    #[test]
    fn shell_renders_title_count_body_and_footer_with_no_overlay() {
        // @step Given the Resume Session view has 5 available sessions
        let title = "Resume Session";
        let count = 5usize;

        // @step When the shared shell renders it onto the full area
        let out = render_to_string(
            80,
            24,
            title,
            count,
            "available",
            "Enter Select | Esc Cancel",
            "session-row-body",
            None,
        );

        // @step Then the title row reads "Resume Session (5 available)"
        assert!(out.contains("Resume Session (5 available)"));

        // @step And the body lists the 5 session rows
        assert!(out.contains("session-row-body"));

        // @step And the footer shows the static hint
        assert!(out.contains("Enter Select | Esc Cancel"));

        // @step And no ConfirmDialog overlay is painted
        assert!(!out.contains("Delete"));
    }

    #[test]
    fn shell_paints_optional_confirm_dialog_overlay_over_full_area() {
        // @step Given the Provider Settings view has a delete confirmation active
        let dialog = ConfirmDialog::new(
            "Delete credentials?",
            "Delete credentials for openai?",
            "Delete",
            None,
            "Cancel",
        );

        // @step When the shared shell renders it onto the full area
        let out = render_to_string(
            80,
            24,
            "Provider Settings",
            3,
            "items",
            "Esc back",
            "provider-list-body",
            Some(&dialog),
        );

        // @step Then the list body is painted first
        // (body content is overwritten where the centered dialog sits,
        // but the title chrome remains)
        assert!(out.contains("Provider Settings (3 items)"));

        // @step And the ConfirmDialog overlay is painted over the full area on top of the body
        assert!(out.contains("Delete credentials?"));
    }

    #[test]
    fn shell_skips_overlay_slot_when_none() {
        // @step Given a view with no destructive action pending
        let overlay: Option<&ConfirmDialog> = None;

        // @step When the shared shell renders it onto the full area
        let out = render_to_string(
            80,
            24,
            "Search History",
            7,
            "matches",
            "Enter Select | Esc Cancel",
            "search-match-body",
            overlay,
        );

        // @step Then the body renders normally
        assert!(out.contains("search-match-body"));

        // @step And no overlay is painted over the body
        assert!(!out.contains("Delete"));
    }

    #[test]
    fn shell_reports_body_height_to_body_renderer() {
        // @step Given a terminal area that is 24 rows tall
        let reported = Cell::new(0u16);

        // @step When the shared shell splits the area into title, separator, body and footer
        let mut term = Terminal::new(TestBackend::new(80, 24)).expect("Terminal::new");
        term.draw(|frame| {
            render_full_screen_scaffold(
                frame.area(),
                frame.buffer_mut(),
                "Resume Session",
                5,
                "available",
                "footer",
                |body_area, _buf| {
                    reported.set(body_area.height);
                },
                None,
            );
        })
        .expect("draw");

        // @step Then the body sub-rect height reported to the body renderer is 21
        assert_eq!(reported.get(), 21);
    }

    #[test]
    fn shell_collapses_body_gracefully_on_tiny_area() {
        // @step Given a terminal area that is 3 rows tall or smaller
        let reported = Cell::new(99u16);

        // @step When the shared shell splits the area
        let mut term = Terminal::new(TestBackend::new(80, 3)).expect("Terminal::new");
        term.draw(|frame| {
            render_full_screen_scaffold(
                frame.area(),
                frame.buffer_mut(),
                "Resume Session",
                0,
                "available",
                "footer",
                |body_area, _buf| {
                    reported.set(body_area.height);
                },
                None,
            );
        })
        .expect("draw");

        // @step Then the body sub-rect height is 0
        // @step And the body renderer receives height 0 and produces no output
        assert_eq!(reported.get(), 0);
    }
}

#[cfg(test)]
mod title_closure_tests;
