//! PROV-114 — renderers for the GitHub Copilot sign-in preamble: the
//! deployment-type select and the GitHub Enterprise host entry.
//!
//! Feature: spec/features/provider-settings-oauth-copilot-device.feature
//!
//! BUG-622: split out of `oauth_login_render.rs` to keep it under the
//! 300-LoC ceiling; the renderers are unchanged.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::{hint_style, title_style};
use crate::views::agent::text_wrap::wrapped_lines;

/// PROV-114: the github-copilot deployment-type-select preamble — a title plus
/// the two deployment options with the selected one marked.
pub(in crate::views::provider_settings) fn render_deployment_type_select(
    area: Rect,
    buf: &mut Buffer,
    selected_index: usize,
) {
    let options = [
        "GitHub.com (Public)",
        "GitHub Enterprise (Self-hosted / data residency)",
    ];
    let mut lines = vec![
        Line::from(Span::styled(
            "GitHub Copilot Login — Select deployment type".to_string(),
            title_style(),
        )),
        Line::from(""),
    ];
    for (idx, label) in options.iter().enumerate() {
        let marker = if idx == selected_index { "> " } else { "  " };
        let style = if idx == selected_index {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(format!("{marker}{label}"), style)));
    }
    lines.push(Line::from(Span::styled(
        "↑/↓: select · Enter: continue · Esc: cancel",
        hint_style(),
    )));
    Paragraph::new(lines).render(area, buf);
}

/// PROV-114: the github-copilot enterprise-host entry preamble — a prompt, the
/// current input, and (when present) the red validation error.
pub(in crate::views::provider_settings) fn render_enterprise_url_entry(
    area: Rect,
    buf: &mut Buffer,
    url_input: &str,
    validation_error: Option<&str>,
) {
    let mut lines = vec![
        Line::from(Span::styled(
            "GitHub Enterprise host".to_string(),
            title_style(),
        )),
        Line::from(""),
    ];
    lines.extend(wrapped_lines(
        &format!("URL or domain: {url_input}"),
        usize::from(area.width),
        Style::default(),
    ));
    if let Some(err) = validation_error {
        lines.push(Line::from(Span::styled(
            err.to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::from(Span::styled(
        "Enter: continue · Esc: cancel",
        hint_style(),
    )));
    Paragraph::new(lines).render(area, buf);
}
