//! RPC-054 — ProviderSettingsView.
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! New child view for the `/provider` slash command. Renders a left-pane
//! list of providers (display name + configured indicator + model count)
//! and a right-pane status area. Pressing Enter on an `api_key`-type row
//! opens an inline edit form; `t` runs a connection test; `r` refreshes
//! the cached model list; `d` clears credentials; Esc dismisses the view.
//!
//! The view holds NO async state — it is purely a per-frame painter +
//! key handler that emits Actions onto the bus. `App::dispatch_rpc054`
//! drives the backend round-trips and folds responses into the view via
//! the same `Action::ProviderCredentialsLoaded` / `ProviderTestComplete`
//! / `ProviderModelsRefreshed` pattern that the rest of the AgentView
//! uses.

use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::components::Action;
use crate::views::agent::slash_commands::SlashCommandAction;

/// Per-view mode. List mode shows the provider list + status text;
/// EditApiKey mode overlays an inline edit form for the focused
/// provider's API key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProviderSettingsMode {
    #[default]
    List,
    EditApiKey { provider_id: String, draft: String },
}

/// Outcome of a single key event consumed by the view. The dispatcher
/// translates `Action(_)` outcomes onto the action bus; `Consumed` and
/// `Ignored` short-circuit the dispatch chain.
#[derive(Debug, Clone)]
pub enum ProviderSettingsEvent {
    /// View consumed the key; no action to emit.
    Consumed,
    /// View did not consume the key.
    Ignored,
    /// View consumed the key and wants the App to emit this action.
    Emit(Action),
    /// View consumed the key and wants the App to dismiss it.
    Close,
}

/// The ProviderSettings view state.
#[derive(Debug, Clone, Default)]
pub struct ProviderSettingsView {
    pub providers: Vec<ProviderCredentialInfo>,
    pub selected_index: usize,
    pub mode: ProviderSettingsMode,
    /// Right-pane status text — drives "Testing…", "✓ ok (42ms)",
    /// "✗ unreachable: …", "models refreshed", and the OAuth read-only
    /// notice.
    pub status: String,
}

impl ProviderSettingsView {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the provider list with a fresh snapshot from
    /// `Action::ProviderCredentialsLoaded`. Caps `selected_index` so it
    /// stays in range.
    pub fn set_providers(&mut self, providers: Vec<ProviderCredentialInfo>) {
        let max = providers.len().saturating_sub(1);
        if self.selected_index > max {
            self.selected_index = max;
        }
        self.providers = providers;
    }

    /// Convenience for tests + the dispatcher.
    pub fn focused_provider(&self) -> Option<&ProviderCredentialInfo> {
        self.providers.get(self.selected_index)
    }

    /// Set the right-pane status text. Always overrides — most callers
    /// want "✓ ok (Xms)" to replace "Testing…" etc.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    /// Handle a key event. Returns the dispatcher hint.
    pub fn handle_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent {
        match &self.mode {
            ProviderSettingsMode::List => self.handle_list_key(key),
            ProviderSettingsMode::EditApiKey { .. } => self.handle_edit_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent {
        match key.code {
            KeyCode::Esc => ProviderSettingsEvent::Close,
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                ProviderSettingsEvent::Consumed
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.providers.len() {
                    self.selected_index += 1;
                }
                ProviderSettingsEvent::Consumed
            }
            KeyCode::Enter => match self.focused_provider().cloned() {
                Some(info) if info.credential_type == "api_key" => {
                    self.mode = ProviderSettingsMode::EditApiKey {
                        provider_id: info.provider_id,
                        draft: String::new(),
                    };
                    self.status.clear();
                    ProviderSettingsEvent::Consumed
                }
                Some(info) if info.credential_type == "oauth" => {
                    self.status = format!(
                        "{}: OAuth flow not yet supported in Rust frontend — use the legacy TS frontend or env vars",
                        info.provider_id
                    );
                    ProviderSettingsEvent::Consumed
                }
                _ => ProviderSettingsEvent::Consumed,
            },
            KeyCode::Char(c)
                if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                match c {
                    't' | 'T' => {
                        if let Some(info) = self.focused_provider().cloned() {
                            self.status = "Testing…".to_string();
                            return ProviderSettingsEvent::Emit(
                                Action::TestProviderConnection(info.provider_id),
                            );
                        }
                        ProviderSettingsEvent::Consumed
                    }
                    'r' | 'R' => {
                        if let Some(info) = self.focused_provider().cloned() {
                            self.status = "Refreshing models…".to_string();
                            return ProviderSettingsEvent::Emit(
                                Action::RefreshProviderModels(info.provider_id),
                            );
                        }
                        ProviderSettingsEvent::Consumed
                    }
                    'd' | 'D' => {
                        if let Some(info) = self.focused_provider().cloned() {
                            if info.configured {
                                self.status = "Deleting credentials…".to_string();
                                return ProviderSettingsEvent::Emit(
                                    Action::DeleteProviderCredentials(info.provider_id),
                                );
                            }
                        }
                        ProviderSettingsEvent::Consumed
                    }
                    _ => ProviderSettingsEvent::Consumed,
                }
            }
            _ => ProviderSettingsEvent::Consumed,
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent {
        let ProviderSettingsMode::EditApiKey { provider_id, draft } = &mut self.mode else {
            return ProviderSettingsEvent::Consumed;
        };
        match key.code {
            KeyCode::Esc => {
                self.mode = ProviderSettingsMode::List;
                self.status.clear();
                ProviderSettingsEvent::Consumed
            }
            KeyCode::Enter => {
                if draft.is_empty() {
                    self.status = "API key cannot be empty".to_string();
                    return ProviderSettingsEvent::Consumed;
                }
                let provider_id = provider_id.clone();
                let draft = draft.clone();
                self.mode = ProviderSettingsMode::List;
                self.status = format!("Saving credentials for {provider_id}…");
                ProviderSettingsEvent::Emit(Action::SaveProviderCredentials {
                    provider_id,
                    api_key: draft,
                })
            }
            KeyCode::Backspace => {
                draft.pop();
                ProviderSettingsEvent::Consumed
            }
            KeyCode::Char(c)
                if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                draft.push(c);
                ProviderSettingsEvent::Consumed
            }
            _ => ProviderSettingsEvent::Consumed,
        }
    }

    /// Paint the view into `area`. Single-pass renderer — no internal
    /// scroll state yet (the longest provider list is 6-8 entries; if a
    /// future card adds custom providers in volume we can add the same
    /// scroll viewport pattern used elsewhere).
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        // Outer block: title + footer hint.
        let outer = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Provider Settings ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = outer.inner(area);
        outer.render(area, buf);

        // Reserve one row at the bottom for the footer hint.
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        let body = layout[0];
        let footer = layout[1];

        // Split body into two panes.
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body);

        self.render_left_pane(panes[0], buf);
        self.render_right_pane(panes[1], buf);

        // Footer hint.
        let hint = match &self.mode {
            ProviderSettingsMode::List => {
                " Enter: edit  |  t: test  |  r: refresh models  |  d: delete  |  Esc: back "
            }
            ProviderSettingsMode::EditApiKey { .. } => " Enter: save  |  Esc: cancel ",
        };
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )))
        .render(footer, buf);
    }

    fn render_left_pane(&self, area: Rect, buf: &mut Buffer) {
        let mut lines: Vec<Line<'_>> = Vec::with_capacity(self.providers.len());
        for (idx, info) in self.providers.iter().enumerate() {
            let selected = idx == self.selected_index;
            let configured_span = if info.configured {
                Span::styled(" ✓ configured ", Style::default().fg(Color::Green))
            } else {
                Span::styled(" (not configured) ", Style::default().fg(Color::Gray))
            };
            let row = Line::from(vec![
                Span::raw(if selected { "> " } else { "  " }),
                Span::styled(
                    info.display_name.clone(),
                    if selected {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                ),
                configured_span,
                Span::raw(format!(" — {} models", info.model_count)),
                Span::raw(format!(
                    " [{}]",
                    info.credential_type
                )),
            ]);
            lines.push(row);
        }
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "(no providers — list will refresh on /provider open)",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Paragraph::new(lines).render(area, buf);
    }

    fn render_right_pane(&self, area: Rect, buf: &mut Buffer) {
        let mut lines: Vec<Line<'_>> = Vec::new();
        match &self.mode {
            ProviderSettingsMode::List => {
                if let Some(info) = self.focused_provider() {
                    lines.push(Line::from(Span::styled(
                        info.display_name.clone(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(format!("provider_id: {}", info.provider_id)));
                    lines.push(Line::from(format!(
                        "credential type: {}",
                        info.credential_type
                    )));
                    lines.push(Line::from(format!("models: {}", info.model_count)));
                } else {
                    lines.push(Line::from(Span::styled(
                        "(select a provider on the left)",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            ProviderSettingsMode::EditApiKey { provider_id, draft } => {
                lines.push(Line::from(Span::styled(
                    format!("Edit API Key: {provider_id}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Key: ", Style::default().fg(Color::Cyan)),
                    Span::raw("•".repeat(draft.len())),
                    Span::styled("█", Style::default().fg(Color::White)),
                ]));
                lines.push(Line::from(Span::styled(
                    "(Enter to save · Esc to cancel)",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
        if !self.status.is_empty() {
            lines.push(Line::from(""));
            let color = if self.status.starts_with("✗") {
                Color::Red
            } else if self.status.starts_with("✓") {
                Color::Green
            } else {
                Color::Cyan
            };
            lines.push(Line::from(Span::styled(
                self.status.clone(),
                Style::default().fg(color),
            )));
        }
        Paragraph::new(lines).render(area, buf);
    }
}

/// Module-level helper to detect whether the `Provider` slash action
/// should route through this view. Exposed so `dispatch_rpc054` can
/// stay decoupled from the slash command registry's internal enum.
pub fn is_provider_action(action: SlashCommandAction) -> bool {
    matches!(action, SlashCommandAction::Provider)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use codelet_rpc_types::ProviderCredentialInfo;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn pinfo(id: &str, ctype: &str, configured: bool, models: u32) -> ProviderCredentialInfo {
        ProviderCredentialInfo {
            provider_id: id.to_string(),
            display_name: id.to_string(),
            configured,
            credential_type: ctype.to_string(),
            model_count: models,
        }
    }

    #[test]
    fn enter_on_api_key_row_opens_edit_form() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("anthropic", "api_key", false, 0)]);
        view.handle_key(key(KeyCode::Enter));
        match &view.mode {
            ProviderSettingsMode::EditApiKey { provider_id, draft } => {
                assert_eq!(provider_id, "anthropic");
                assert!(draft.is_empty());
            }
            _ => panic!("expected EditApiKey mode"),
        }
    }

    #[test]
    fn enter_on_oauth_row_surfaces_notice() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("codex", "oauth", false, 0)]);
        view.handle_key(key(KeyCode::Enter));
        assert!(matches!(view.mode, ProviderSettingsMode::List));
        assert!(view.status.contains("OAuth flow not yet supported"));
    }

    #[test]
    fn t_key_emits_test_action() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("openai", "api_key", true, 4)]);
        let out = view.handle_key(key(KeyCode::Char('t')));
        match out {
            ProviderSettingsEvent::Emit(Action::TestProviderConnection(id)) => {
                assert_eq!(id, "openai");
            }
            _ => panic!("expected TestProviderConnection action"),
        }
        assert_eq!(view.status, "Testing…");
    }

    #[test]
    fn r_key_emits_refresh_action() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("openai", "api_key", true, 4)]);
        let out = view.handle_key(key(KeyCode::Char('r')));
        assert!(matches!(
            out,
            ProviderSettingsEvent::Emit(Action::RefreshProviderModels(_))
        ));
    }

    #[test]
    fn d_key_only_emits_when_configured() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("openai", "api_key", false, 0)]);
        let out = view.handle_key(key(KeyCode::Char('d')));
        // Not configured → no emit
        assert!(matches!(out, ProviderSettingsEvent::Consumed));

        view.set_providers(vec![pinfo("openai", "api_key", true, 4)]);
        let out = view.handle_key(key(KeyCode::Char('d')));
        assert!(matches!(
            out,
            ProviderSettingsEvent::Emit(Action::DeleteProviderCredentials(_))
        ));
    }

    #[test]
    fn esc_closes_view() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("openai", "api_key", true, 4)]);
        let out = view.handle_key(key(KeyCode::Esc));
        assert!(matches!(out, ProviderSettingsEvent::Close));
    }

    #[test]
    fn edit_form_save_emits_credentials_action() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("anthropic", "api_key", false, 0)]);
        view.handle_key(key(KeyCode::Enter)); // open edit form
        view.handle_key(key(KeyCode::Char('s')));
        view.handle_key(key(KeyCode::Char('k')));
        view.handle_key(key(KeyCode::Char('-')));
        view.handle_key(key(KeyCode::Char('1')));
        let out = view.handle_key(key(KeyCode::Enter));
        match out {
            ProviderSettingsEvent::Emit(Action::SaveProviderCredentials {
                provider_id,
                api_key,
            }) => {
                assert_eq!(provider_id, "anthropic");
                assert_eq!(api_key, "sk-1");
            }
            _ => panic!("expected SaveProviderCredentials action"),
        }
        assert!(matches!(view.mode, ProviderSettingsMode::List));
    }

    #[test]
    fn edit_form_esc_cancels_without_emitting() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![pinfo("anthropic", "api_key", false, 0)]);
        view.handle_key(key(KeyCode::Enter));
        let out = view.handle_key(key(KeyCode::Esc));
        assert!(matches!(out, ProviderSettingsEvent::Consumed));
        assert!(matches!(view.mode, ProviderSettingsMode::List));
    }

    #[test]
    fn arrow_keys_move_selection() {
        let mut view = ProviderSettingsView::new();
        view.set_providers(vec![
            pinfo("anthropic", "api_key", false, 0),
            pinfo("openai", "api_key", false, 0),
        ]);
        assert_eq!(view.selected_index, 0);
        view.handle_key(key(KeyCode::Down));
        assert_eq!(view.selected_index, 1);
        view.handle_key(key(KeyCode::Up));
        assert_eq!(view.selected_index, 0);
        view.handle_key(key(KeyCode::Up));
        assert_eq!(view.selected_index, 0); // clamped
    }
}
