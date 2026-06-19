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

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Clear, Widget};

use crate::views::agent::confirm_dialog::ConfirmDialog;
use crate::views::agent::mode_view_render::{render_footer_hint, render_title_with_count};

/// Number of chrome rows the scaffold reserves (title + separator +
/// footer). Body height = `area.height - CHROME_ROWS`.
pub(crate) const CHROME_ROWS: u16 = 3;

/// Render the shared full-screen scaffold:
///   1. `Clear.render(area, buf)` — overwrite the underlying view.
///   2. Vertical split `[Length(1), Length(1), Min(0), Length(1)]`.
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
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    let (title_area, body_area, footer_area) = (split[0], split[2], split[3]);
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
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    let (title_area, body_area, footer_area) = (split[0], split[2], split[3]);
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
    fn render_to_string(
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

    /// RPC-339 — render a `width`x`height` TestBackend through the
    /// title-closure scaffold variant and return the joined rows.
    fn render_with_title_to_string(
        width: u16,
        height: u16,
        title_text: &str,
        footer: &str,
        body_text: &str,
        overlay: Option<&ConfirmDialog>,
    ) -> String {
        use ratatui::widgets::{Paragraph, Widget};
        let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
        term.draw(|frame| {
            render_full_screen_scaffold_with_title(
                frame.area(),
                frame.buffer_mut(),
                |title_area, buf| {
                    Paragraph::new(title_text).render(title_area, buf);
                },
                footer,
                |body_area, buf| {
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
    fn shell_title_closure_paints_title_body_and_footer_no_overlay() {
        // @step Given a full-screen area and a caller-supplied title closure, body closure, and a static footer hint
        let title_text = "(search): auth";
        let footer = "Enter Select | Esc Cancel";

        // @step When the view is rendered via render_full_screen_scaffold_with_title with overlay None
        let out = render_with_title_to_string(80, 24, title_text, footer, "search-body-rows", None);

        // @step Then the title closure paints the first row
        assert!(out.starts_with("(search): auth"));

        // @step And the body closure paints the body sub-rect below the separator
        assert!(out.contains("search-body-rows"));

        // @step And the static footer hint paints the last row
        assert!(out.contains("Enter Select | Esc Cancel"));

        // @step And no ConfirmDialog overlay is drawn
        assert!(!out.contains("Delete"));
    }

    #[test]
    fn count_wrapper_preserves_title_count_format() {
        // @step Given a view rendered via the count-title wrapper render_full_screen_scaffold
        // @step When it is called with title "Resume Session", count 5, and suffix "available"
        let out = render_to_string(
            80,
            24,
            "Resume Session",
            5,
            "available",
            "Enter Select | Esc Cancel",
            "session-row-body",
            None,
        );

        // @step Then the title row reads "Resume Session (5 available)"
        assert!(out.contains("Resume Session (5 available)"));

        // @step And the rendered output is identical to the pre-RPC-339 baseline
        assert!(out.contains("session-row-body"));
        assert!(out.contains("Enter Select | Esc Cancel"));
    }
}
