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

use super::{
    DetailStatus, DetailSub, ProviderSettingsEvent, ProviderSettingsMode,
    ProviderSettingsView,
};

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
        DetailSub::Summary { last_status } => {
            handle_summary_key(view, key, provider_id, last_status)
        }
        DetailSub::EditApiKey { draft } => handle_edit_key(view, key, provider_id, draft),
        DetailSub::OAuthNotice => handle_oauth_notice_key(view, key),
    }
}

fn handle_summary_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    last_status: Option<DetailStatus>,
) -> ProviderSettingsEvent {
    match key.code {
        KeyCode::Esc => {
            view.mode = ProviderSettingsMode::List;
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            view.mode = ProviderSettingsMode::Detail {
                provider_id: provider_id.clone(),
                sub: DetailSub::Summary {
                    last_status: Some(DetailStatus::Testing),
                },
            };
            view.status = "Testing…".to_string();
            ProviderSettingsEvent::Emit(Action::TestProviderConnection(provider_id))
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            view.mode = ProviderSettingsMode::Detail {
                provider_id: provider_id.clone(),
                sub: DetailSub::Summary {
                    last_status: Some(DetailStatus::RefreshingModels),
                },
            };
            view.status = "Refreshing models…".to_string();
            ProviderSettingsEvent::Emit(Action::RefreshProviderModels(provider_id))
        }
        KeyCode::Enter => {
            // Enter on api_key row opens the inline edit form. OAuth
            // rows never reach Summary so we don't need to gate.
            view.mode = ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::EditApiKey {
                    draft: String::new(),
                },
            };
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        _ => {
            // Re-enter the Summary mode preserving last_status so an
            // unrelated keystroke doesn't drop the status text.
            view.mode = ProviderSettingsMode::Detail {
                provider_id,
                sub: DetailSub::Summary { last_status },
            };
            ProviderSettingsEvent::Consumed
        }
    }
}

fn handle_edit_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    mut draft: String,
) -> ProviderSettingsEvent {
    // RPC-162 — TS parity: every EditApiKey EXIT path returns to List
    // (Esc, empty-Enter, and successful Save). Previously these arms
    // all routed to Detail::Summary { … }. The Summary variant remains
    // on the enum for legacy callers; the EditApiKey form is just no
    // longer one of them. Empty-Enter is a silent cancel (no
    // "API key cannot be empty" status).
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
            ProviderSettingsEvent::Emit(Action::SaveProviderCredentials {
                provider_id,
                api_key,
            })
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

fn handle_oauth_notice_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
) -> ProviderSettingsEvent {
    if matches!(key.code, KeyCode::Esc) {
        view.mode = ProviderSettingsMode::List;
        view.status.clear();
    }
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
    let focused = view
        .providers
        .iter()
        .find(|p| p.provider_id == provider_id);
    let display_name = focused
        .map(|p| p.display_name.clone())
        .unwrap_or_else(|| provider_id.to_string());

    match sub {
        DetailSub::Summary { last_status } => {
            lines.push(Line::from(Span::styled(display_name, title_style)));
            lines.push(Line::from(format!("provider_id: {provider_id}")));
            if let Some(p) = focused {
                lines.push(Line::from(format!("credential type: {}", p.credential_type)));
                lines.push(Line::from(format!("models: {}", p.model_count)));
                let configured = if p.configured { "✓ configured" } else { "(not configured)" };
                lines.push(Line::from(configured.to_string()));
            }
            if let Some(status) = last_status {
                lines.push(Line::from(""));
                lines.push(Line::from(status.to_span()));
            }
        }
        DetailSub::EditApiKey { draft } => {
            lines.push(Line::from(Span::styled(
                format!("Edit API Key: {display_name}"),
                title_style,
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Key: ", Style::default().fg(Color::Cyan)),
                Span::raw("•".repeat(draft.len())),
                Span::styled("█", Style::default().fg(Color::White)),
            ]));
            if !view.status.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    view.status.clone(),
                    Style::default().fg(Color::Red),
                )));
            }
        }
        DetailSub::OAuthNotice => {
            lines.push(Line::from(Span::styled(display_name, title_style)));
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "{provider_id} uses OAuth which is not yet supported in the Rust frontend"
            )));
            lines.push(Line::from(
                "Use the legacy TS frontend or environment variables.",
            ));
        }
    }
    Paragraph::new(lines).render(area, buf);
}

#[cfg(test)]
mod tests {
    //! RPC-161 — branch-coverage unit tests for the printable-ASCII helper.
    //!
    //! Feature: spec/features/rpc161-provider-settings-api-key-printable-ascii-filter.feature
    //!
    //! Covers the same Gherkin scenario as the end-to-end integration test
    //! `is_printable_ascii_helper_classifies_characters_by_ascii_code_observed_end_to_end`
    //! in tests/provider_settings_api_key_charset_rpc161.rs, but exercises
    //! the helper directly so a regression that changed the range bounds
    //! without changing observable view state would still fail.

    use super::is_printable_ascii;

    /// Scenario: is_printable_ascii() helper classifies characters by ASCII code
    #[test]
    fn is_printable_ascii_returns_true_for_inclusive_boundaries_and_interior() {
        // @step Given the helper function is_printable_ascii(c: char) -> bool exists in views/provider_settings/detail.rs
        // @step When the helper is called with each of the chars ' ' (32), 'A' (65), '~' (126)
        // @step Then it returns true for every one
        assert!(is_printable_ascii(' '), "space (32) must be accepted");
        assert!(is_printable_ascii('A'), "interior char 'A' (65) must be accepted");
        assert!(is_printable_ascii('~'), "tilde (126) must be accepted");
    }

    #[test]
    fn is_printable_ascii_returns_false_for_control_del_and_non_ascii() {
        // @step When the helper is called with each of the chars '\t' (9), '\u{001F}' (31), '\u{007F}' (127), 'é' (233), '🔑' (128017)
        // @step Then it returns false for every one
        assert!(!is_printable_ascii('\t'), "tab (9) must be rejected");
        assert!(!is_printable_ascii('\u{001F}'), "unit-separator (31) must be rejected");
        assert!(!is_printable_ascii('\u{007F}'), "DEL (127) must be rejected");
        assert!(!is_printable_ascii('é'), "latin-1 é (233) must be rejected");
        assert!(!is_printable_ascii('🔑'), "non-BMP emoji (128017) must be rejected");
    }
}
