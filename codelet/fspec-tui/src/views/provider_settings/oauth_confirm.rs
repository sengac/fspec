//! PROV-112 — DisconnectOAuth confirm dialog: key handling + rendering.
//!
//! Feature: spec/features/provider-settings-oauth-disconnect.feature
//!
//! A dedicated logout/disconnect confirm keyed by `provider_id`, reached from
//! Enter or `d`/`D` on an `oauth-status` (Logout) row. It is intentionally NOT
//! the generic `delete_confirm` credentials dialog — it never opens that and
//! never falls through to the api-key delete path.
//!
//! Keyboard contract (mirrors TS `handleConfirmation`,
//! `deleteConfirmModeHandler.ts:14-29`):
//!   * `y` / `Y` → emit `Action::OAuthDisconnect { provider_id }` then return
//!     to list.
//!   * `n` / `N` / Esc → return to list with NO backend call (tokens kept).
//!   * any other key → consumed, the confirm stays open (no mode change).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::components::Action;

use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// Route a key through the DisconnectOAuth confirm. Only `y`/`Y` performs the
/// disconnect; `n`/`N`/Esc cancels; everything else is consumed without
/// leaving the confirm.
pub(super) fn handle_disconnect_oauth_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
) -> ProviderSettingsEvent {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            view.mode = ProviderSettingsMode::List;
            view.status.clear();
            ProviderSettingsEvent::Emit(Action::OAuthDisconnect { provider_id })
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            view.mode = ProviderSettingsMode::List;
            view.status.clear();
            ProviderSettingsEvent::Consumed
        }
        // Any other key is consumed; the confirm stays open (mode unchanged).
        _ => ProviderSettingsEvent::Consumed,
    }
}

/// Render the disconnect confirm body. The feature asserts mode/backend
/// behaviour rather than exact strings, so this is a minimal, honest prompt.
pub(super) fn render_disconnect_oauth(area: Rect, buf: &mut Buffer, provider_id: &str) {
    let title_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let lines = vec![
        Line::from(Span::styled(
            format!("Disconnect OAuth for {provider_id}?"),
            title_style,
        )),
        Line::from(""),
        Line::from("This clears the stored OAuth tokens for this provider."),
        Line::from(Span::styled(
            "y: confirm · n/Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    Paragraph::new(lines).render(area, buf);
}
