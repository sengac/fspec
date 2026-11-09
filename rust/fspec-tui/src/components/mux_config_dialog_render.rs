//! MUX-010 — MuxConfigDialog rows, footer and the live layout preview.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! Content rows (fixed, so the dialog never resizes while typing):
//! the Layout field, two error rows, [`PREVIEW_ROWS`] preview rows, the
//! Enabled row, the Apply row and the Save row. The preview is painted
//! over its blank rows after the shared dialog frame: one box per pane,
//! laid out by the same geometry as the live grid and labelled with the
//! pane kind.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::views::multiplex::layout_tree::{compute, pane_name};

use super::dialog_button_hits::row_list_layout;
use super::dialog_theme::{render_dialog, Accent, DialogRow, FspecDialog};
use super::mux_config_dialog::{MuxConfigDialog, MuxDialogFocus};

/// The dialog title.
pub const MUX_CONFIG_DIALOG_TITLE: &str = "Mux Layout";

/// The fixed footer (the dialog's keys).
pub const MUX_CONFIG_DIALOG_FOOTER: &str =
    "Tab/↑↓ Move · Type to edit · Space Toggle · Enter Apply · Esc Cancel";

/// Rows of the drawn preview.
pub const PREVIEW_ROWS: usize = 8;
/// Body width of the dialog (stable while typing).
const BODY_WIDTH: u16 = 64;
const FIELD_LABEL: &str = "Layout  ";
const ERROR_ROWS: usize = 2;
const PREVIEW_START: usize = 1 + ERROR_ROWS;
const ENABLED_ROW: usize = PREVIEW_START + PREVIEW_ROWS;
const ROW_COUNT: usize = ENABLED_ROW + 3;

/// What a clicked content row addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowTarget {
    Field,
    Control(MuxDialogFocus),
}

/// The target of content row `index` (`None` for the error / preview rows).
pub fn row_target(index: usize) -> Option<RowTarget> {
    match index {
        0 => Some(RowTarget::Field),
        i if i == ENABLED_ROW => Some(RowTarget::Control(MuxDialogFocus::Enabled)),
        i if i == ENABLED_ROW + 1 => Some(RowTarget::Control(MuxDialogFocus::Apply)),
        i if i == ENABLED_ROW + 2 => Some(RowTarget::Control(MuxDialogFocus::Save)),
        _ => None,
    }
}

fn row(spans: Vec<Span<'static>>, selected: bool) -> DialogRow {
    DialogRow {
        spans,
        selectable: true,
        selected,
    }
}

fn blank() -> DialogRow {
    DialogRow {
        spans: vec![Span::raw("")],
        selectable: false,
        selected: false,
    }
}

/// Paint one cell when it lies inside the buffer.
fn put(buf: &mut Buffer, x: u16, y: u16, symbol: &str, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(symbol).set_style(style);
    }
}

/// Split `text` into lines of at most `width` chars.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(width.max(1))
        .map(|chunk| chunk.iter().collect())
        .collect()
}

impl MuxConfigDialog {
    fn rows(&self) -> Vec<DialogRow> {
        let field_focused = self.focus == MuxDialogFocus::Field;
        let visible = usize::from(BODY_WIDTH) - FIELD_LABEL.len();
        let (before, at, after) = self.field.window(visible);
        let cursor_style = if field_focused {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let mut rows = vec![row(
            vec![
                Span::raw(FIELD_LABEL),
                Span::raw(before),
                Span::styled(at.to_string(), cursor_style),
                Span::raw(after),
            ],
            field_focused,
        )];
        let error_lines = self
            .error
            .as_deref()
            .map(|e| wrap(e, usize::from(BODY_WIDTH)))
            .unwrap_or_default();
        for i in 0..ERROR_ROWS {
            rows.push(match error_lines.get(i) {
                Some(line) => DialogRow {
                    spans: vec![Span::styled(line.clone(), Style::default().fg(Color::Red))],
                    selectable: false,
                    selected: false,
                },
                None => blank(),
            });
        }
        rows.extend((0..PREVIEW_ROWS).map(|_| blank()));
        let enabled = if self.draft.enabled { "On" } else { "Off" };
        rows.push(row(
            vec![Span::raw(format!("Enabled  {enabled}"))],
            self.focus == MuxDialogFocus::Enabled,
        ));
        rows.push(row(
            vec![Span::raw("[ Apply ]")],
            self.focus == MuxDialogFocus::Apply,
        ));
        rows.push(row(
            vec![Span::raw("[ Save ]")],
            self.focus == MuxDialogFocus::Save,
        ));
        rows
    }

    pub(super) fn render_dialog(&mut self, area: Rect, buf: &mut Buffer) {
        let dialog = FspecDialog {
            accent: Accent::Cyan,
            title: MUX_CONFIG_DIALOG_TITLE,
            rows: self.rows(),
            footer: MUX_CONFIG_DIALOG_FOOTER,
            min_width: BODY_WIDTH,
            query_row: None,
        };
        self.last_layout = row_list_layout(area, &dialog, ROW_COUNT);
        render_dialog(area, buf, &dialog);
        let controls = &self.last_layout.controls;
        if let (Some(first), Some(last)) = (
            controls.get(PREVIEW_START),
            controls.get(PREVIEW_START + PREVIEW_ROWS - 1),
        ) {
            let preview = Rect::new(first.x, first.y, first.width, last.y + 1 - first.y);
            self.paint_preview(preview, buf);
        }
    }

    /// Draw one labelled box per pane of the draft layout in `area`.
    fn paint_preview(&self, area: Rect, buf: &mut Buffer) {
        let geometry = compute(area, &self.draft.layout, &[]);
        let border = Style::default().fg(Color::Cyan);
        for (rect, kind) in geometry.panes.iter().zip(&geometry.kinds) {
            if rect.width >= 2 && rect.height >= 2 {
                let (right, bottom) = (rect.right() - 1, rect.bottom() - 1);
                for x in rect.x..=right {
                    for y in [rect.y, bottom] {
                        put(buf, x, y, "─", border);
                    }
                }
                for y in rect.y..=bottom {
                    for x in [rect.x, right] {
                        put(buf, x, y, "│", border);
                    }
                }
                for (x, y, glyph) in [
                    (rect.x, rect.y, "┌"),
                    (right, rect.y, "┐"),
                    (rect.x, bottom, "└"),
                    (right, bottom, "┘"),
                ] {
                    put(buf, x, y, glyph, border);
                }
            }
            let inner = if rect.width > 2 && rect.height > 2 {
                Rect::new(rect.x + 1, rect.y + 1, rect.width - 2, rect.height - 2)
            } else {
                *rect
            };
            for (i, ch) in pane_name(*kind)
                .chars()
                .take(usize::from(inner.width))
                .enumerate()
            {
                let label = Style::default().fg(Color::White);
                put(buf, inner.x + i as u16, inner.y, &ch.to_string(), label);
            }
        }
    }
}
