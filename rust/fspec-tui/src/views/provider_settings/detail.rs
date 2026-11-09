//! RPC-054 — Detail mode key handling + rendering.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::components::Action;

use super::{DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// RPC-161 — TS-canonical printable-ASCII filter for API-key edit input.
///
/// Mirrors `filterPrintableChars` at
/// `src/tui/utils/providerSettingsHelpers.ts:39-47`, which accepts only
/// characters whose code lies in the inclusive range 32..=126. Control
/// chars (0..=31), DEL (127), and any non-ASCII char (>127) are rejected
/// so they cannot leak into the API-key buffer.
fn is_printable_ascii(c: char) -> bool {
    let code = c as u32;
    (32..=126).contains(&code)
}

pub(super) fn handle_detail_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    sub: DetailSub,
) -> ProviderSettingsEvent {
    match sub {
        DetailSub::EditApiKey { draft } => handle_edit_key(view, key, provider_id, draft),
    }
}

fn handle_edit_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    mut draft: String,
) -> ProviderSettingsEvent {
    // RPC-162 — TS parity: every EditApiKey EXIT path returns to List
    // (Esc, empty-Enter, and successful Save). Empty-Enter is a silent
    // cancel (no "API key cannot be empty" status).
    match key.code {
        KeyCode::Esc => {
            view.mode = ProviderSettingsMode::List;
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Enter => {
            if draft.is_empty() {
                view.mode = ProviderSettingsMode::List;
                view.status.clear();
                return ProviderSettingsEvent::Consumed;
            }
            let api_key = draft.clone();
            view.mode = ProviderSettingsMode::List;
            view.status.clear();
            ProviderSettingsEvent::Emit(Box::new(Action::SaveProviderCredentials {
                provider_id,
                api_key,
            }))
        }
        KeyCode::Backspace | KeyCode::Delete => {
            // RPC-163 — TS parity: Ink's useInput exposes key.backspace and
            // key.delete as sibling boolean flags both wired to
            // draft.slice(0, -1) (see src/tui/inputHandlers/apiKeyEditModeHandler.ts:46).
            // Mirror that here with a merged match arm so the two key paths
            // can never diverge.
            draft.pop();
            view.mode = ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::EditApiKey { draft },
            };
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Char(c) => {
            // RPC-161 — drop control chars / DEL / non-ASCII so only
            // printable ASCII (32..=126) lands in the draft buffer.
            if is_printable_ascii(c) {
                draft.push(c);
                // Clear validation message on any further typing — but
                // only when an ACCEPTED printable char was appended;
                // dropping a non-printable must NOT clear the message.
                if view.status == "API key cannot be empty" {
                    view.status.clear();
                }
            }
            view.mode = ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::EditApiKey { draft },
            };
            ProviderSettingsEvent::Consumed
        }
        _ => {
            view.mode = ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::EditApiKey { draft },
            };
            ProviderSettingsEvent::Consumed
        }
    }
}

/// PROV-137 — inline API-key paste sink. Appends each printable-ASCII char of
/// `text` (same 32..=126 gate as typed input) to the draft, dropping newlines,
/// control chars, and non-ASCII. Re-stores the mutated draft so the existing
/// masked render (`"•".repeat(draft.len())`) keeps the secret hidden.
pub(super) fn handle_edit_paste(
    view: &mut ProviderSettingsView,
    provider_id: String,
    mut draft: String,
    text: &str,
) -> ProviderSettingsEvent {
    for c in text.chars() {
        if is_printable_ascii(c) {
            draft.push(c);
        }
    }
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::EditApiKey { draft },
    };
    ProviderSettingsEvent::Consumed
}

pub(super) fn render_detail(
    view: &ProviderSettingsView,
    area: Rect,
    buf: &mut Buffer,
    provider_id: &str,
    sub: &DetailSub,
) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let title_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let focused = view.providers.iter().find(|p| p.provider_id == provider_id);
    let display_name = focused
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| provider_id.to_string());

    match sub {
        DetailSub::EditApiKey { draft } => {
            lines.push(Line::from(Span::styled(
                format!("Edit API Key: {display_name}"),
                title_style,
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Key: ", Style::default().fg(Color::Cyan)),
                Span::raw(super::mask_secret(draft)),
                Span::styled("█", Style::default().fg(Color::White)),
            ]));
            if !view.status.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    view.status.clone(),
                    super::status_text::status_style(&view.status, Style::default().fg(Color::Red)),
                )));
            }
        }
    }
    Paragraph::new(lines).render(area, buf);
}

#[cfg(test)]
#[path = "detail_tests.rs"]
mod tests;
