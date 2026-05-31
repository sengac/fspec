//! RPC-056 — BlocklistView.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//!
//! New child view for the `/blocklist` slash command. Renders a left-pane
//! list of blocklist rules (id + source tag + category tag + action) and
//! a right-pane details panel (id, pattern, action, source, reason,
//! guidance, session status). Pressing `j`/`k` (or arrows) navigates,
//! `Enter`/`Space` toggles the focused rule's session-disabled status,
//! `Esc` dismisses the view.
//!
//! The view emits Actions onto the dispatcher rather than mutating
//! AgentViewStore directly — the per-session disabled set lives on
//! `AgentViewStore.blocklist_disabled_by_session` (RPC-056) so it
//! survives close/reopen cycles. The view receives the disabled set
//! through `render()` / `handle_key()` parameters so test-time render +
//! key-handler unit tests can drive it without an AgentViewStore.
//!
//! TS parity: `src/tui/components/BlocklistListView.tsx`. The TS frontend
//! also uses an in-memory `Set<string>` lifted to the AgentView component
//! state — disabled rules are purely a UI affordance and do NOT
//! flow back into the tool-execution path (a follow-up card can wire
//! enforcement when the broader rule-management UX lands).

use std::collections::HashSet;

use codelet_rpc_types::BlocklistRuleInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::components::Action;

/// Outcome of a single key event consumed by the view. Mirrors the
/// `ProviderSettingsEvent` shape — `Emit(Action)` is the canonical way
/// the view asks the App to fold state into the dispatcher.
#[derive(Debug, Clone)]
pub enum BlocklistEvent {
    /// View consumed the key; no action to emit.
    Consumed,
    /// View did not consume the key.
    Ignored,
    /// View consumed the key and wants the App to emit this action.
    Emit(Action),
    /// View consumed the key and wants the App to dismiss it.
    Close,
}

/// The BlocklistView state. Owns the rules list + selected_index; the
/// session-disabled `HashSet<String>` lives ON the AgentViewStore and
/// is supplied to render + key-handling by reference so the view
/// stays stateless w.r.t. the per-session lift.
#[derive(Debug, Clone, Default)]
pub struct BlocklistView {
    pub rules: Vec<BlocklistRuleInfo>,
    pub selected_index: usize,
}

impl BlocklistView {
    /// Construct a fresh view with no rules and selected_index 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the rules list with a fresh snapshot from
    /// `Action::BlocklistRulesLoaded`. Caps `selected_index` so it
    /// stays in range.
    pub fn set_rules(&mut self, rules: Vec<BlocklistRuleInfo>) {
        let max = rules.len().saturating_sub(1);
        if self.selected_index > max {
            self.selected_index = max;
        }
        self.rules = rules;
    }

    /// Borrow the currently-focused rule (or `None` when the list is
    /// empty).
    pub fn focused_rule(&self) -> Option<&BlocklistRuleInfo> {
        self.rules.get(self.selected_index)
    }

    /// Handle a key event. The view itself never mutates the
    /// session-disabled set — instead, toggle actions emit
    /// `Action::ToggleBlocklistRule(id)` onto the dispatcher and the
    /// App task folds the toggle into
    /// `AgentViewStore.blocklist_disabled_by_session` via
    /// `handle_toggle_blocklist_rule` (RPC-056 dispatch helper).
    ///
    /// This single-source-of-truth design avoids the duplicate-state
    /// trap that would otherwise let the view's local set drift from
    /// the store's authoritative set across session switches.
    pub fn handle_key(&mut self, key: KeyEvent) -> BlocklistEvent {
        if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return BlocklistEvent::Consumed;
        }
        match key.code {
            KeyCode::Esc => BlocklistEvent::Close,
            KeyCode::Down => {
                if self.selected_index + 1 < self.rules.len() {
                    self.selected_index += 1;
                }
                BlocklistEvent::Consumed
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                BlocklistEvent::Consumed
            }
            KeyCode::Char(c) => match c {
                'j' | 'J' => {
                    if self.selected_index + 1 < self.rules.len() {
                        self.selected_index += 1;
                    }
                    BlocklistEvent::Consumed
                }
                'k' | 'K' => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    BlocklistEvent::Consumed
                }
                ' ' => self.toggle_focused(),
                _ => BlocklistEvent::Consumed,
            },
            KeyCode::Enter => self.toggle_focused(),
            _ => BlocklistEvent::Consumed,
        }
    }

    fn toggle_focused(&self) -> BlocklistEvent {
        let Some(rule) = self.focused_rule() else {
            return BlocklistEvent::Consumed;
        };
        BlocklistEvent::Emit(Action::ToggleBlocklistRule(rule.id.clone()))
    }

    /// Paint the view into `area`. Two-pane layout: 50% left (list),
    /// 50% right (details). The supplied `session_disabled` set drives
    /// the per-row glyph (●/○) and the right-pane Session Status field.
    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        session_disabled: &HashSet<String>,
    ) {
        // Outer block: title + footer hint.
        let outer = Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                " Blocklist ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        let inner = outer.inner(area);
        outer.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        let body = layout[0];
        let footer = layout[1];

        // Empty-state placeholder
        if self.rules.is_empty() {
            let lines: Vec<Line<'_>> = vec![
                Line::from(Span::styled(
                    "No blocklist rules configured.",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Blocklist rules prevent dangerous commands and guide",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "AI agents to use proper tools and patterns.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "System config: ~/.fspec/blocklist.json",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    "Project config: .fspec/blocklist.json",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            Paragraph::new(lines).render(body, buf);
        } else {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(body);
            self.render_left_pane(panes[0], buf, session_disabled);
            self.render_right_pane(panes[1], buf, session_disabled);
        }

        // Footer hint.
        let hint = " j/k: navigate  |  Enter/Space: toggle  |  Esc: back ";
        Paragraph::new(Line::from(Span::styled(
            hint,
            Style::default().fg(Color::DarkGray),
        )))
        .render(footer, buf);
    }

    fn render_left_pane(
        &self,
        area: Rect,
        buf: &mut Buffer,
        session_disabled: &HashSet<String>,
    ) {
        let mut lines: Vec<Line<'_>> = Vec::with_capacity(self.rules.len());
        for (idx, rule) in self.rules.iter().enumerate() {
            let selected = idx == self.selected_index;
            let disabled = session_disabled.contains(&rule.id);
            let glyph = if disabled { '○' } else { '●' };
            let prefix = if selected { "> " } else { "  " };

            let row_style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if disabled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let id_line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("{glyph} {}", rule.id), row_style),
            ]);
            lines.push(id_line);

            let action_color = action_color(&rule.action);
            let category = derive_category(&rule.pattern);
            let mut meta_spans: Vec<Span<'_>> = vec![
                Span::raw("    "),
                Span::styled(format!("[{}]", rule.action), Style::default().fg(action_color)),
                Span::raw(" "),
                Span::styled(format!("[{category}]"), Style::default().fg(Color::Magenta)),
                Span::raw(" "),
                Span::styled(rule.source.clone(), Style::default().fg(Color::DarkGray)),
            ];
            if disabled {
                meta_spans.push(Span::styled(
                    " (disabled)",
                    Style::default().fg(Color::Yellow),
                ));
            }
            lines.push(Line::from(meta_spans));
        }
        Paragraph::new(lines).render(area, buf);
    }

    fn render_right_pane(
        &self,
        area: Rect,
        buf: &mut Buffer,
        session_disabled: &HashSet<String>,
    ) {
        let mut lines: Vec<Line<'_>> = Vec::new();
        if let Some(rule) = self.focused_rule() {
            lines.push(Line::from(Span::styled(
                "Rule Details",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::White)),
                Span::raw(rule.id.clone()),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Action: ", Style::default().fg(Color::White)),
                Span::styled(rule.action.clone(), Style::default().fg(action_color(&rule.action))),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Source: ", Style::default().fg(Color::White)),
                Span::styled(rule.source.clone(), Style::default().fg(Color::DarkGray)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("Category: ", Style::default().fg(Color::White)),
                Span::styled(
                    derive_category(&rule.pattern).to_string(),
                    Style::default().fg(Color::Magenta),
                ),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Pattern:",
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(Span::styled(
                rule.pattern.clone(),
                Style::default().fg(Color::DarkGray),
            )));
            if !rule.reason.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "Reason:",
                    Style::default().fg(Color::White),
                )));
                lines.push(Line::from(rule.reason.clone()));
            }
            if let Some(g) = rule.guidance.as_ref() {
                if !g.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "Guidance:",
                        Style::default().fg(Color::White),
                    )));
                    lines.push(Line::from(Span::styled(
                        g.clone(),
                        Style::default().fg(Color::Green),
                    )));
                }
            }
            lines.push(Line::from(""));
            let disabled = session_disabled.contains(&rule.id);
            let (status_color, status_text) = if disabled {
                (Color::Yellow, "disabled (session)")
            } else {
                (Color::Green, "enabled")
            };
            lines.push(Line::from(vec![
                Span::styled("Session Status: ", Style::default().fg(Color::White)),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                "(no rule selected)",
                Style::default().fg(Color::DarkGray),
            )));
        }
        Paragraph::new(lines).render(area, buf);
    }
}

fn action_color(action: &str) -> Color {
    match action {
        "block" => Color::Red,
        "allow" => Color::Green,
        "prompt" => Color::Yellow,
        _ => Color::White,
    }
}

/// Derive the category label for a blocklist rule's regex pattern.
///
/// Heuristic:
///   * Patterns whose source contains a `/` separator, or starts with
///     `~`, `./`, or `/` are classified as `"file_path"` — they look
///     like file-path rules consumed by `check_file_path`.
///   * Everything else is classified as `"bash"` — they look like
///     command rules consumed by `check_bash_command`.
///
/// Pure helper — no storage state, no I/O. Exposed at the module level
/// so unit tests can pin the heuristic deterministically.
pub fn derive_category(pattern: &str) -> &'static str {
    if pattern.starts_with('~')
        || pattern.starts_with("./")
        || pattern.starts_with('/')
        || pattern.contains('/')
    {
        "file_path"
    } else {
        "bash"
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn rule(id: &str, pattern: &str, action: &str, source: &str) -> BlocklistRuleInfo {
        BlocklistRuleInfo {
            id: id.to_string(),
            pattern: pattern.to_string(),
            action: action.to_string(),
            reason: String::new(),
            guidance: None,
            source: source.to_string(),
        }
    }

    #[test]
    fn set_rules_resets_selection_when_index_exceeds_new_len() {
        let mut view = BlocklistView::new();
        view.set_rules(vec![
            rule("a", "a", "block", "system"),
            rule("b", "b", "block", "system"),
        ]);
        view.selected_index = 1;
        view.set_rules(vec![rule("c", "c", "block", "system")]);
        assert_eq!(view.selected_index, 0);
    }

    #[test]
    fn derive_category_classifies_bash_and_file_path() {
        assert_eq!(derive_category("^cat\\s+"), "bash");
        assert_eq!(derive_category("/etc/passwd"), "file_path");
        assert_eq!(derive_category("~/.aws/.*"), "file_path");
        assert_eq!(derive_category("./scripts/deploy.sh"), "file_path");
        assert_eq!(derive_category("git checkout"), "bash");
    }
}
