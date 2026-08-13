//! PROV-113 — renderers for the OAuth login modes.
//!
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! Split out of `mod.rs` to keep it under the 300-LoC ceiling. Each renderer
//! paints the body area for one login mode; the exact user-facing strings
//! (waiting titles, success labels, the "Press Esc to cancel" / retry hints)
//! mirror the TS `ProviderSettingsScreen` OAuth sub-screens so the Rust TUI is
//! a faithful port.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Title shown while a browser login is awaiting authorization.
pub(super) fn browser_waiting_title(provider_id: &str) -> &'static str {
    match provider_id {
        "anthropic" => "Claude OAuth Login",
        "codex" => "Codex OAuth Login",
        _ => "OAuth Login",
    }
}

/// Title shown while a device login is awaiting authorization.
pub(super) fn device_waiting_title(provider_id: &str) -> &'static str {
    match provider_id {
        "codex" => "Codex Device Login",
        "github-copilot" => "GitHub Copilot Device Login",
        _ => "Device Login",
    }
}

/// Success banner shown once a login completes.
pub(super) fn success_label(provider_id: &str) -> &'static str {
    match provider_id {
        "anthropic" => "✓ Connected to Claude",
        "codex" => "✓ Connected to ChatGPT",
        "github-copilot" => "✓ Connected to GitHub Copilot",
        _ => "✓ Connected",
    }
}

fn title_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn hint_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub(super) fn render_browser_waiting(area: Rect, buf: &mut Buffer, provider_id: &str) {
    let lines = vec![
        Line::from(Span::styled(
            browser_waiting_title(provider_id).to_string(),
            title_style(),
        )),
        Line::from(""),
        Line::from("Waiting for authorization..."),
        Line::from(Span::styled("Press Esc to cancel", hint_style())),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub(super) fn render_device_waiting(
    area: Rect,
    buf: &mut Buffer,
    provider_id: &str,
    user_code: &str,
    verification_url: &str,
) {
    let lines = vec![
        Line::from(Span::styled(
            device_waiting_title(provider_id).to_string(),
            title_style(),
        )),
        Line::from(""),
        Line::from(format!("Your code: {user_code}")),
        Line::from(format!("Visit: {verification_url}")),
        Line::from(Span::styled("⠿ Waiting for authorization...", hint_style())),
        Line::from(Span::styled("Press Esc to cancel", hint_style())),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub(super) fn render_headless_code_entry(
    area: Rect,
    buf: &mut Buffer,
    authorize_url: &str,
    code_input: &str,
) {
    let lines = vec![
        Line::from(Span::styled(
            "Claude OAuth Login".to_string(),
            title_style(),
        )),
        Line::from(""),
        Line::from(format!("Visit: {authorize_url}")),
        Line::from(""),
        Line::from(format!("Code: {code_input}")),
        Line::from(Span::styled(
            "Enter: submit · c: copy URL · o: open URL · Esc: cancel",
            hint_style(),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub(super) fn render_success(area: Rect, buf: &mut Buffer, provider_id: &str) {
    let lines = vec![
        Line::from(Span::styled(
            success_label(provider_id).to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Press Enter or Esc to continue", hint_style())),
    ];
    Paragraph::new(lines).render(area, buf);
}

pub(super) fn render_error(area: Rect, buf: &mut Buffer, error: &str) {
    let lines = vec![
        Line::from(Span::styled(
            "OAuth Login error".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(error.to_string()),
        Line::from(Span::styled(
            "Press Enter to retry | Esc to go back",
            hint_style(),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}

/// PROV-114: the github-copilot deployment-type-select preamble — a title plus
/// the two deployment options with the selected one marked.
pub(super) fn render_deployment_type_select(area: Rect, buf: &mut Buffer, selected_index: usize) {
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
pub(super) fn render_enterprise_url_entry(
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
        Line::from(format!("URL or domain: {url_input}")),
    ];
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
