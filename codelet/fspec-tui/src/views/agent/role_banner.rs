//! RoleBanner — inline single-row banner above scrollback when a
//! session role overlay is active.
//!
//! Feature: spec/features/rpc022-role-banner.feature
//! Card: RPC-022 (parent RPC-002).
//!
//! Rust port of `src/tui/components/RoleBanner.tsx` (TUI-081). Unlike
//! ModelSelectorDialog / ThinkingLevelDialog this is NOT a Compositor
//! layer — it is an inline ratatui [`Widget`] painted by
//! [`crate::views::AgentView::render_with_store`] in the area carved
//! out above the scrollback Block when
//! `AgentViewStore::role_for(current_session).is_some()`.
//!
//! Multi-line role text is collapsed to a single line by replacing
//! every whitespace run (including newlines) with a single space —
//! mirroring the TS `roleText.replace(/\\s+/g, ' ').trim()`
//! behaviour.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Owned snapshot of the only input the banner needs to render.
///
/// AgentView constructs this fresh per frame from
/// `AgentViewStore::role_for(current_session)`.
pub struct RoleBanner<'a> {
    pub role_text: &'a str,
}

impl<'a> Widget for RoleBanner<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let collapsed = collapse_whitespace(self.role_text);
        let line = Line::from(vec![
            Span::styled("Role: ", Style::default().fg(Color::Cyan)),
            Span::styled(collapsed, Style::default().fg(Color::DarkGray)),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}

/// Replace any whitespace run (including newlines) with a single
/// ASCII space and trim leading/trailing whitespace. Public so unit
/// tests can exercise the same helper the render path uses.
pub fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_ws = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_text(buf: &Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn render_banner_into(width: u16, text: &str) -> Buffer {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).expect("Terminal::new");
        terminal
            .draw(|frame| {
                let area = frame.area();
                RoleBanner { role_text: text }.render(area, frame.buffer_mut());
            })
            .expect("draw");
        terminal.backend().buffer().clone()
    }

    #[test]
    fn collapse_whitespace_replaces_newlines_with_spaces() {
        let out = collapse_whitespace("You are a security reviewer.\nAnalyze code for vulnerabilities.");
        assert_eq!(
            out,
            "You are a security reviewer. Analyze code for vulnerabilities."
        );
    }

    #[test]
    fn collapse_whitespace_collapses_runs() {
        let out = collapse_whitespace("  hello   world  \n\t  ");
        assert_eq!(out, "hello world");
    }

    #[test]
    fn render_paints_role_prefix_and_text() {
        let buf = render_banner_into(60, "You are a reviewer");
        let text = buffer_text(&buf);
        assert!(text.starts_with("Role: "), "expected `Role: ` prefix, got {text:?}");
        assert!(text.contains("You are a reviewer"));
    }

    #[test]
    fn render_collapses_multiline_role() {
        let buf = render_banner_into(80, "You are a reviewer.\nLook for bugs.");
        let text = buffer_text(&buf);
        assert!(text.contains("You are a reviewer. Look for bugs."));
        assert!(!text.contains('\n') || text.matches('\n').count() == 1);
    }

    #[test]
    fn render_truncates_to_terminal_width() {
        let long_role = "X".repeat(200);
        let buf = render_banner_into(40, &long_role);
        // Single row, exactly 40 columns wide.
        assert_eq!(buf.area.width, 40);
        assert_eq!(buf.area.height, 1);
    }
}
