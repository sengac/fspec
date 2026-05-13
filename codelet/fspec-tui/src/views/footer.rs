//! Footer hint bar (RPC-009).
//!
//! Feature: spec/features/fspec-tui-root-layout-rpc009.feature
//!
//! 1-row Component rendering keybinding hints `?`-help, `q`-quit,
//! `Tab`-switch-pane via styled `Spans` against the existing Theme.
//! NO `tui-prompts`, NO `throbber-widgets-tui` — those are deferred.

use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::components::{Component, Priority};
use crate::theme::Theme;

/// Bottom-row hint bar shared by every screen in RPC-009.
pub struct FooterView {
    pub theme: Arc<Theme>,
}

impl Default for FooterView {
    fn default() -> Self {
        Self {
            theme: Arc::new(Theme::default()),
        }
    }
}

impl FooterView {
    pub fn new(theme: Arc<Theme>) -> Self {
        Self { theme }
    }
}

impl Component for FooterView {
    fn priority(&self) -> Priority {
        Priority::Background
    }

    fn id(&self) -> &str {
        "footer"
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let dim = Style::default().fg(self.theme.dim);
        let key_style = Style::default().fg(self.theme.fg).bold();
        let line = Line::from(vec![
            Span::styled("? ", key_style),
            Span::styled("help  ", dim),
            Span::styled("q ", key_style),
            Span::styled("quit  ", dim),
            Span::styled("Tab ", key_style),
            Span::styled("switch pane", dim),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}
