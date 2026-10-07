//! TUI-112 — shared paint + hit-test builder for the three-button
//! dialog style (`CreateSessionDialog` / `ExitConfirmationDialog` /
//! `BoardExitConfirmationDialog`).
//!
//! Feature: spec/features/left-click-activates-buttons-rows-in-all-enter-selectable-modal-dialogs-new-agent-exit-session-etc.feature
//! Card: TUI-112.
//!
//! The three-button dialogs paint an identical row — a 1-space lead,
//! ` <label> ` buttons separated by 2-column dim gaps, 1-space trail,
//! centred in the body with `pad = (body_w - raw_w) / 2`. This function
//! is the single source of truth for that span vector AND for the
//! `LastLayout` hit-test geometry: it builds the descriptor, paints it
//! via `dialog_theme::render_dialog`, and returns the geometry captured
//! from the same descriptor — so the click rects line up with the
//! painted pixels (R4/R5). If the paint changes, change it here once.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

use super::dialog_button_hits::{three_button_layout, LastLayout};
use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};

/// Paint a three-button dialog into `buf` at `area` and return the
/// last-rendered button-row hit-test geometry (row index 1).
///
/// `description` is the single non-selectable content row above the
/// button row (rows = [description, button_row]); `selected` is the
/// currently highlighted option index; `labels` are the button texts in
/// paint order.
#[allow(clippy::too_many_arguments)]
pub fn render_three_button_dialog(
    area: Rect,
    buf: &mut Buffer,
    accent: Accent,
    title: &str,
    description: &str,
    footer: &str,
    min_width: u16,
    selected: usize,
    labels: &[&str],
) -> LastLayout {
    let dim_style = Style::default()
        .add_modifier(Modifier::DIM)
        .bg(Color::Black);
    let description_row = DialogRow {
        spans: vec![Span::styled(description.to_string(), dim_style)],
        selectable: false,
        selected: false,
    };

    // EXACT TS Ink parity (src/components/CreateSessionDialog.tsx /
    // ThreeButtonDialog.tsx): selected button = bg=Blue/fg=White/bold on
    // " <label> "; unselected = fg=Gray; NO ▸/○ marker glyphs.
    let selected_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let unselected_style = Style::default().fg(Color::Gray).bg(Color::Black);

    // Layout mirrors TS marginX={1}: 1 leading space, ` <label> `
    // buttons, 2-space dim gaps, 1 trailing space. All span content is
    // owned (owned strings), so the row is `Span<'static>` regardless
    // of the descriptor's borrow lifetime.
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled(" ".to_string(), dim_style));
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ".to_string(), dim_style));
        }
        let style = if i == selected {
            selected_style
        } else {
            unselected_style
        };
        spans.push(Span::styled(format!(" {label} "), style));
    }
    spans.push(Span::styled(" ".to_string(), dim_style));

    // Centre the button row within the body content width — mirrors
    // `dialog_theme::inner_content_width`'s max() computation so the
    // leading pad equals (final_body_w - raw_row_w) / 2.
    let raw_w: usize = spans.iter().map(|s| s.content.width()).sum();
    let body_w = [
        title.width(),
        description.width(),
        raw_w,
        footer.width(),
        min_width as usize,
    ]
    .into_iter()
    .max()
    .unwrap_or(min_width as usize);
    if body_w > raw_w {
        let pad = (body_w - raw_w) / 2;
        if pad > 0 {
            spans.insert(
                0,
                Span::styled(" ".repeat(pad), Style::default().bg(Color::Black)),
            );
        }
    }

    let button_row = DialogRow {
        spans,
        selectable: false,
        selected: false,
    };

    let dialog = FspecDialog {
        accent,
        title,
        rows: vec![description_row, button_row],
        footer,
        min_width,
        query_row: None,
    };
    // The layout is derived from the SAME descriptor that is painted so
    // the button rects line up with the pixels (TUI-112 R4/R5).
    let last_layout = three_button_layout(area, &dialog, 1, labels);
    render_dialog(area, buf, &dialog);
    last_layout
}
