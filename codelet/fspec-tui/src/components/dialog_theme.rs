//! RPC-027 — Canonical fspec dialog theme.
//!
//! Feature: spec/features/rpc027-dialog-theme.feature
//!
//! Single source of truth for the rounded/black/accent-color look
//! shared with the TypeScript Ink dialogs (src/components/Dialog.tsx,
//! src/tui/components/ThinkingLevelDialog.tsx). Every dialog/popup
//! under `codelet/fspec-tui` delegates to `render_dialog` so a change
//! to the visual contract lands in exactly one place.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Widget};

use super::dialog_theme_rows::{paint_left_aligned, paint_text};

/// One canonical accent color per dialog kind. The accent drives the
/// border, the inner title, and the selection-row inverse highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Accent {
    #[default]
    Cyan,
    Yellow,
    Red,
}

impl Accent {
    /// Map the variant to its ratatui `Color`.
    pub fn color(self) -> Color {
        match self {
            Accent::Cyan => Color::Cyan,
            Accent::Yellow => Color::Yellow,
            Accent::Red => Color::Red,
        }
    }
}

/// One content row inside the body. `selected = true` paints the row
/// with the inverse highlight; `selectable = false` is used for
/// provider headers in ModelSelectorDialog (they can never be
/// highlighted even when the cursor is over them).
#[derive(Debug, Clone)]
pub struct DialogRow {
    pub spans: Vec<Span<'static>>,
    pub selectable: bool,
    pub selected: bool,
}

/// Per-render dialog input. `min_width` is the minimum body width in
/// columns (before border + padding). Pass 0 for "shrink-to-content".
#[derive(Default)]
pub struct FspecDialog<'a> {
    pub accent: Accent,
    pub title: &'a str,
    pub rows: Vec<DialogRow>,
    pub footer: &'a str,
    pub min_width: u16,
}

/// `▸ ` (U+25B8 BLACK RIGHT-POINTING SMALL TRIANGLE + space). Used as
/// the prefix for the selected row.
pub const MARKER_SELECTED: &str = "▸ ";

/// Two ASCII spaces. Used as the prefix for unselected rows so column
/// alignment matches `MARKER_SELECTED`.
pub const MARKER_UNSELECTED: &str = "  ";

/// `" │ "` (space + U+2502 + space). Used between footer hint chunks
/// and between button labels in `ConfirmDialog`.
pub const FOOTER_SEPARATOR: &str = " │ ";

fn span_width(span: &Span<'_>) -> u16 {
    span.content.chars().count() as u16
}

fn row_width(row: &DialogRow) -> u16 {
    row.spans.iter().map(span_width).sum()
}

fn line_width(line: &str) -> u16 {
    line.chars().count() as u16
}

fn inner_content_width(dialog: &FspecDialog<'_>) -> u16 {
    let mut w = line_width(dialog.title);
    for row in &dialog.rows {
        w = w.max(row_width(row));
    }
    for ln in dialog.footer.lines() {
        w = w.max(line_width(ln));
    }
    w.max(dialog.min_width)
}

fn footer_line_count(footer: &str) -> u16 {
    if footer.is_empty() {
        0
    } else {
        footer.lines().count() as u16
    }
}

/// Compute the centered dialog rect inside `area`. The returned rect
/// is clamped to `area` if the natural content size exceeds it.
pub fn dialog_rect(area: Rect, dialog: &FspecDialog<'_>) -> Rect {
    let content_w = inner_content_width(dialog);
    // 2 border + 2 padding (1 each side)
    let width = (content_w + 4).min(area.width);
    let footer_h = footer_line_count(dialog.footer);
    let body_h = dialog.rows.len() as u16;
    // 2 border + 2 padding + 1 title + 1 gap-after-title + body + (1 gap + footer if footer)
    let footer_block = if footer_h == 0 { 0 } else { 1 + footer_h };
    let natural = 2 + 2 + 1 + 1 + body_h + footer_block;
    let height = natural.min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Paint the dialog onto `buf` inside `area`. Renders:
///   1. opaque black background (Clear + Block with bg=Black),
///   2. rounded border in `dialog.accent.color()`,
///   3. bold accent-colored inner title row,
///   4. one blank gap row,
///   5. body rows (selected rows use inverse highlight),
///   6. one blank gap row + dim centered footer.
pub fn render_dialog(area: Rect, buf: &mut Buffer, dialog: &FspecDialog<'_>) {
    let rect = dialog_rect(area, dialog);
    render_dialog_at(rect, buf, dialog);
}

/// RPC-383: paint `dialog` at an explicit, caller-computed `rect`
/// instead of the shrink-to-content [`dialog_rect`]. Used by the
/// full-screen `TurnContentModal` (with [`fixed_dialog_rect`]); the body
/// painting / title / footer logic is shared so there is exactly one
/// implementation of the visual contract.
pub fn render_dialog_at(rect: Rect, buf: &mut Buffer, dialog: &FspecDialog<'_>) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    Clear.render(rect, buf);
    let accent = dialog.accent.color();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent).bg(Color::Black))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(rect);
    block.render(rect, buf);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    // 1-cell padding inside the border
    let body = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(2),
        height: inner.height.saturating_sub(2),
    };
    if body.width == 0 || body.height == 0 {
        return;
    }

    let bg_style = Style::default().bg(Color::Black);
    // Paint the inner body rows with a black background so unrendered
    // cells (gaps, footer padding) keep the dialog's opaque look.
    for y in body.y..body.y + body.height {
        for x in body.x..body.x + body.width {
            buf[(x, y)].set_style(bg_style);
            buf[(x, y)].set_symbol(" ");
        }
    }

    // Title: row 0 of body.
    let title_style = Style::default()
        .fg(accent)
        .add_modifier(Modifier::BOLD)
        .bg(Color::Black);
    paint_left_aligned(
        buf,
        Rect {
            x: body.x,
            y: body.y,
            width: body.width,
            height: 1,
        },
        &[Span::styled(dialog.title.to_string(), title_style)],
        bg_style,
    );

    // Body rows. The DEFAULT (spacious) layout has a blank gap row
    // between the title and the first content row (title at body.y, gap
    // at body.y+1, content from body.y+2) plus a gap before the footer —
    // unchanged for every shrink-to-content dialog. Only when the body is
    // too short for the spacious layout to show even ONE content row does
    // it fall back to a COMPACT layout (drop the gaps; then, if still too
    // short, drop the footer). This keeps the full-screen
    // `TurnContentModal` showing content on small terminals without
    // regressing the canonical dialog spacing.
    let inverse = Style::default()
        .bg(accent)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);
    let raw_footer_h = footer_line_count(dialog.footer);
    // Spacious needs: title(1) + gap(1) + >=1 content + (gap(1)+footer if
    // any). Use it whenever it yields at least one visible content row.
    let spacious_min = 3 + if raw_footer_h > 0 {
        raw_footer_h + 1
    } else {
        0
    };
    let spacious = body.height >= spacious_min;
    // Compact drops the gaps; drop the footer too if it still won't fit.
    let footer_h = if raw_footer_h == 0 {
        0
    } else if spacious || body.height >= 2 + raw_footer_h {
        raw_footer_h
    } else {
        0
    };
    let content_start = body.y + if spacious { 2 } else { 1 };
    let body_row_end = if footer_h > 0 {
        let reserved = if spacious { footer_h + 1 } else { footer_h };
        body.y + body.height.saturating_sub(reserved)
    } else {
        body.y + body.height
    };
    for (y, row) in (content_start..).zip(dialog.rows.iter()) {
        if y >= body_row_end {
            break;
        }
        let row_rect = Rect {
            x: body.x,
            y,
            width: body.width,
            height: 1,
        };
        if row.selected {
            // Paint the full inner-row inverse highlight (border-to-border).
            // The highlight covers padding cells on either side of the
            // body content so the visual matches "full width inverse"
            // from RPC-027 rule [3].
            for x in inner.x..inner.x + inner.width {
                buf[(x, y)].set_style(inverse);
                buf[(x, y)].set_symbol(" ");
            }
            let styled: Vec<Span<'static>> = row
                .spans
                .iter()
                .map(|s| Span::styled(s.content.clone(), s.style.patch(inverse)))
                .collect();
            paint_left_aligned(buf, row_rect, &styled, inverse);
        } else {
            paint_left_aligned(buf, row_rect, &row.spans, bg_style);
        }
    }

    // Footer pinned to the bottom of the body block.
    if footer_h > 0 && body.height >= footer_h {
        let footer_y = body.y + body.height - footer_h;
        for (i, line) in dialog.footer.lines().enumerate() {
            let r = Rect {
                x: body.x,
                y: footer_y + i as u16,
                width: body.width,
                height: 1,
            };
            let dim_style = Style::default()
                .add_modifier(Modifier::DIM)
                .bg(Color::Black);
            // Center horizontally.
            let line_len = line_width(line);
            let offset = if r.width > line_len {
                (r.width - line_len) / 2
            } else {
                0
            };
            for x in r.x..r.x + r.width {
                buf[(x, r.y)].set_style(bg_style);
                buf[(x, r.y)].set_symbol(" ");
            }
            paint_text(
                buf,
                r.x + offset,
                r.y,
                r.width.saturating_sub(offset),
                line,
                dim_style,
            );
        }
    }
}
