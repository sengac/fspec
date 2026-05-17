//! RPC-020 — Shared popup body adapter for the slash + file search
//! palettes.
//!
//! Tiny `tui_popup::SizedWidgetRef` adapter that paints a multi-line
//! string with one highlighted row. Used by both SlashCommandPopup
//! and FileSearchPopup so the rendering math + WidgetRef plumbing
//! lives in one place instead of being duplicated per widget (also
//! keeps each popup file under the 300-LoC ceiling).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Widget, WidgetRef};
use tui_popup::SizedWidgetRef;

/// Computed dimensions + body text + the index of the row to paint
/// with a highlighted style. Constructed inline by each popup's
/// `render` method.
#[derive(Debug)]
pub struct PopupBody {
    pub text: String,
    pub selected_index: usize,
    pub width: u16,
    pub height: u16,
}

impl WidgetRef for PopupBody {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        for (i, line) in self.text.lines().enumerate() {
            if y >= area.y.saturating_add(area.height) {
                break;
            }
            let style = if i == self.selected_index {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let row = Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            };
            Line::from(Span::styled(line.to_string(), style)).render(row, buf);
            y = y.saturating_add(1);
        }
    }
}

impl SizedWidgetRef for PopupBody {
    fn width(&self) -> usize {
        self.width as usize
    }

    fn height(&self) -> usize {
        self.height as usize
    }
}

/// Compute the widest line in `text` (capped at a floor of 40 cols so
/// the popup is never claustrophobically narrow).
pub fn widest_line(text: &str) -> u16 {
    text.lines()
        .map(|l| l.chars().count() as u16)
        .max()
        .unwrap_or(40)
        .max(40)
}
