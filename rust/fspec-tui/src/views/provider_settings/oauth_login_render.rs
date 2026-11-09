//! PROV-113 — renderers for the OAuth login modes.
//!
//! Feature: spec/features/provider-settings-oauth-login.feature
//!
//! Split out of `mod.rs` to keep it under the 300-LoC ceiling. Each renderer
//! paints the body area for one login mode; the exact user-facing strings
//! (waiting titles, success labels, the "Press Esc to cancel" / retry hints)
//! mirror the TS `ProviderSettingsScreen` OAuth sub-screens so the Rust TUI is
//! a faithful port.
//!
//! TUI-126: every long value (the authorize URL, the typed code, the device
//! code and verification URL, the error) wraps to the screen width instead of
//! being cut off, a URL starts on a row of its own, and the view status (a
//! browser-open failure) is painted under the hints.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::views::agent::text_wrap::wrapped_lines;

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

/// The status line under the hints (red for a failure), wrapped.
fn push_status(lines: &mut Vec<Line<'static>>, status: &str, width: usize) {
    if status.is_empty() {
        return;
    }
    let style = if status.starts_with('✗') {
        Style::default().fg(Color::Red)
    } else {
        hint_style()
    };
    lines.push(Line::from(""));
    lines.extend(wrapped_lines(status, width, style));
}

/// A labelled URL: the label on its own row, then the URL's rows.
fn push_url(lines: &mut Vec<Line<'static>>, label: &str, url: &str, width: usize, style: Style) {
    lines.push(Line::from(Span::styled(label.to_string(), style)));
    lines.extend(wrapped_lines(url, width, Style::default()));
}

pub(super) fn render_browser_waiting(
    area: Rect,
    buf: &mut Buffer,
    provider_id: &str,
    authorize_url: Option<&str>,
    status: &str,
) {
    let width = usize::from(area.width);
    let mut lines = vec![
        Line::from(Span::styled(
            browser_waiting_title(provider_id).to_string(),
            title_style(),
        )),
        Line::from(""),
        Line::from("Waiting for authorization..."),
    ];
    if let Some(url) = authorize_url {
        push_url(&mut lines, "Visit:", url, width, Style::default());
        lines.extend(wrapped_lines(
            "c: copy URL · o: open URL",
            width,
            hint_style(),
        ));
    } else {
        lines.extend(wrapped_lines(
            "Browser didn't open? Press Esc and use Sign in with code.",
            width,
            hint_style(),
        ));
    }
    lines.push(Line::from(Span::styled(
        "Press Esc to cancel",
        hint_style(),
    )));
    push_status(&mut lines, status, width);
    Paragraph::new(lines).render(area, buf);
}

/// The device-login screen: the user code and verification URL once they
/// arrived ("Requesting device code…" before), the keys, and the status.
pub(super) fn render_device_waiting(
    area: Rect,
    buf: &mut Buffer,
    provider_id: &str,
    device: (&str, &str),
    status: &str,
) {
    let (user_code, verification_url) = device;
    let width = usize::from(area.width);
    let mut lines = vec![
        Line::from(Span::styled(
            device_waiting_title(provider_id).to_string(),
            title_style(),
        )),
        Line::from(""),
    ];
    if user_code.is_empty() {
        lines.push(Line::from("Requesting device code…"));
    } else {
        lines.extend(wrapped_lines(
            &format!("Your code: {user_code}"),
            width,
            Style::default(),
        ));
        push_url(
            &mut lines,
            "Visit:",
            verification_url,
            width,
            Style::default(),
        );
    }
    lines.push(Line::from(Span::styled(
        "⠿ Waiting for authorization...",
        hint_style(),
    )));
    if !user_code.is_empty() {
        lines.push(Line::from(Span::styled(
            "c: copy code · o: open URL",
            hint_style(),
        )));
    }
    lines.push(Line::from(Span::styled(
        "Press Esc to cancel",
        hint_style(),
    )));
    push_status(&mut lines, status, width);
    Paragraph::new(lines).render(area, buf);
}

/// The sign-in-with-code screen: the URL part and the Code field, the
/// focused one in the title style, then the keys for the focused part.
pub(super) fn render_headless_code_entry(
    area: Rect,
    buf: &mut Buffer,
    authorize_url: &str,
    code: (&str, bool),
    status: &str,
) {
    let (code_input, code_focused) = code;
    let width = usize::from(area.width);
    let label = |focused: bool| {
        if focused {
            title_style()
        } else {
            Style::default()
        }
    };
    let mut lines = vec![
        Line::from(Span::styled(
            "Claude OAuth Login".to_string(),
            title_style(),
        )),
        Line::from(""),
    ];
    push_url(
        &mut lines,
        "Visit:",
        authorize_url,
        width,
        label(!code_focused),
    );
    lines.push(Line::from(""));
    let cursor = if code_focused { "█" } else { "" };
    let code_lines = wrapped_lines(
        &format!("Code: {code_input}{cursor}"),
        width,
        Style::default(),
    );
    for (index, line) in code_lines.into_iter().enumerate() {
        lines.push(if index == 0 {
            line.patch_style(label(code_focused))
        } else {
            line
        });
    }
    lines.push(Line::from(Span::styled(
        super::footer_hints::code_entry_hint(code_focused),
        hint_style(),
    )));
    push_status(&mut lines, status, width);
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
    let width = usize::from(area.width);
    let mut lines = vec![
        Line::from(Span::styled(
            "OAuth Login error".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(wrapped_lines(error, width, Style::default()));
    lines.push(Line::from(Span::styled(
        "Press Enter to retry | Esc to go back",
        hint_style(),
    )));
    lines.push(Line::from(Span::styled("c: copy error", hint_style())));
    Paragraph::new(lines).render(area, buf);
}

// BUG-622: the GitHub Copilot preamble renderers live in a child module.
mod copilot_preamble;
pub(super) use copilot_preamble::{render_deployment_type_select, render_enterprise_url_entry};
