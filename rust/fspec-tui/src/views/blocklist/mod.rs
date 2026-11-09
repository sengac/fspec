//! RPC-056 — BlocklistView.
//!
//! Feature: spec/features/rpc056-blocklist-view-dispatch.feature
//! Feature: spec/features/blocklist-view-scrolling.feature
//!
//! New child view for the `/blocklist` slash command. Renders a left-pane
//! list of blocklist rules (id + source tag + category tag + action) and
//! a right-pane details panel (id, pattern, action, source, reason,
//! guidance, session status). Arrow keys (plus PageUp/PageDown/Home/End)
//! navigate, `Enter`/`Space` toggles the focused rule's session-disabled
//! status, `Esc` dismisses the view.
//!
//! The view emits Actions onto the dispatcher rather than mutating
//! AgentViewStore directly. BUG-601: the turned-off rules live in the
//! daemon, which enforces them for the session the view was opened from
//! (`BlocklistView::session`); `AgentViewStore.blocklist_disabled_by_session`
//! only caches the daemon's last answer. The view receives that set through
//! `render()` so render unit tests can drive it without an AgentViewStore.
//!
//! TS parity: `src/tui/components/BlocklistListView.tsx`.
//!
//! BLOCK-008 adds viewport scrolling (parity with the
//! `model_selector`/`changed_files` scroll pattern): the view owns a
//! `scroll_offset` + `visible_rows` reconciled via the shared
//! `scroll_viewport::ensure_visible` primitive, windows the rendered rows
//! and paints an overflow scrollbar gutter. Split into sibling modules
//! (`render`, `tests`) to stay under 300 LoC.

use codelet_rpc_types::BlocklistRuleInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;

use crate::components::Action;
use crate::terminal::sanitize::sanitize_for_terminal;

mod panes;
mod render;

mod mouse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod framing_tests;

#[cfg(test)]
mod keybindings_tests;

#[cfg(test)]
mod mouse_scroll_tests;

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
    ///
    /// The `Action` payload is boxed: `Action` is a 216+ byte enum, and the
    /// other variants carry no data — boxing keeps the event enum itself
    /// small (clippy `large_enum_variant`).
    Emit(Box<Action>),
    /// View consumed the key and wants the App to dismiss it.
    Close,
}

/// The BlocklistView state. Owns the rules list + selected_index and the
/// session it was opened from; the daemon's turned-off rules for that
/// session are cached on the AgentViewStore and supplied to render.
///
/// BLOCK-008: also owns a `scroll_offset` + `visible_rows` viewport
/// state reconciled via `scroll_viewport::ensure_visible`.
#[derive(Debug, Clone, Default)]
pub struct BlocklistView {
    pub rules: Vec<BlocklistRuleInfo>,
    pub selected_index: usize,
    /// BUG-601: the session the view was opened from; Toggle Rule turns
    /// rules off and on for this session only (None: no session open).
    pub session: Option<codelet_rpc_types::SessionId>,
    /// Index of the first rule painted in the left pane. Reconciled by
    /// [`adjust_scroll`](BlocklistView::adjust_scroll) so the selection
    /// always stays inside the visible window.
    scroll_offset: usize,
    /// Number of rule rows the left pane can show at once. Seeded
    /// defensively from the real body height at render time; also
    /// settable by nav-time tests.
    visible_rows: usize,
    /// BLOCK-011: mouse-wheel velocity accumulator (1×–5× ramp) shared
    /// with the model_selector / provider_settings wheel handlers.
    wheel: crate::components::scroll_viewport::WheelVelocity,
}

impl BlocklistView {
    /// Construct a fresh view with no rules and selected_index 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the rules list with a fresh snapshot from
    /// `Action::BlocklistRulesLoaded`. Caps `selected_index` so it
    /// stays in range and resets `scroll_offset` so a shorter list can
    /// never leave a stale offset (BLOCK-008).
    ///
    /// **TUI-111**: user-visible fields (id, pattern, reason,
    /// guidance, source) are sanitized on ingress. `action` is a
    /// closed vocabulary ("block" | "allow" | "prompt") and is
    /// skipped.
    pub fn set_rules(&mut self, rules: Vec<BlocklistRuleInfo>) {
        let mut rules = rules;
        for rule in rules.iter_mut() {
            rule.id = sanitize_for_terminal(&rule.id);
            rule.pattern = sanitize_for_terminal(&rule.pattern);
            rule.reason = sanitize_for_terminal(&rule.reason);
            rule.guidance = rule.guidance.as_ref().map(|g| sanitize_for_terminal(g));
            rule.source = sanitize_for_terminal(&rule.source);
        }
        let max = rules.len().saturating_sub(1);
        if self.selected_index > max {
            self.selected_index = max;
        }
        self.rules = rules;
        self.scroll_offset = 0;
        self.adjust_scroll();
    }

    /// Borrow the currently-focused rule (or `None` when the list is
    /// empty).
    pub fn focused_rule(&self) -> Option<&BlocklistRuleInfo> {
        self.rules.get(self.selected_index)
    }

    /// Current scroll offset — first rule index painted in the left
    /// pane. Exposed for scroll-parity assertions (BLOCK-008).
    #[cfg(test)]
    pub(crate) fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Seed the visible-window height directly. Used by nav-time tests
    /// to drive the scroll reconciliation without a render pass.
    #[cfg(test)]
    pub(crate) fn set_visible_rows(&mut self, visible_rows: usize) {
        self.visible_rows = visible_rows;
    }

    /// Keep `selected_index` inside the visible window by reconciling
    /// `scroll_offset`. Delegates to the shared
    /// `scroll_viewport::ensure_visible` primitive (BLOCK-008) — the
    /// same helper `model_selector` / `changed_files` use. Called after
    /// every navigation mutation and once again defensively at render
    /// time once the real body height is known.
    pub(crate) fn adjust_scroll(&mut self) {
        crate::components::scroll_viewport::ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            self.visible_rows,
            self.rules.len(),
        );
    }

    /// Handle a key event. The view itself never changes a rule's state:
    /// toggle actions emit `Action::ToggleBlocklistRule(id)` and the App
    /// asks the daemon (BUG-601, `handle_toggle_blocklist_rule`), whose
    /// answer is the only state the view shows.
    ///
    /// BLOCK-008: each navigation arm clamp-moves `selected_index` then
    /// calls `adjust_scroll()` so the selection stays inside the window.
    pub fn handle_key(&mut self, key: KeyEvent) -> BlocklistEvent {
        // TUI-130: Ctrl+C, the copy wrapper, copies the pattern like c.
        if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
            return self.copy(|rule| rule.pattern.clone());
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return BlocklistEvent::Consumed;
        }
        match key.code {
            KeyCode::Esc => BlocklistEvent::Close,
            KeyCode::Down => {
                self.move_down();
                BlocklistEvent::Consumed
            }
            KeyCode::Up => {
                self.move_up();
                BlocklistEvent::Consumed
            }
            KeyCode::PageDown => {
                self.page_down();
                BlocklistEvent::Consumed
            }
            KeyCode::PageUp => {
                self.page_up();
                BlocklistEvent::Consumed
            }
            KeyCode::Home => {
                self.jump_top();
                BlocklistEvent::Consumed
            }
            KeyCode::End => {
                self.jump_bottom();
                BlocklistEvent::Consumed
            }
            KeyCode::Char(' ') => self.toggle_focused(),
            // TUI-130: c copies the focused rule's pattern, C the whole rule.
            KeyCode::Char('c') => self.copy(|rule| rule.pattern.clone()),
            KeyCode::Char('C') => self.copy(rule_detail),
            KeyCode::Char(_) => BlocklistEvent::Consumed,
            KeyCode::Enter => self.toggle_focused(),
            _ => BlocklistEvent::Consumed,
        }
    }

    /// TUI-130: copy `text` of the focused rule through `CopyToClipboard`.
    fn copy(&self, text: impl Fn(&BlocklistRuleInfo) -> String) -> BlocklistEvent {
        match self.focused_rule() {
            Some(rule) => BlocklistEvent::Emit(Box::new(Action::CopyToClipboard(text(rule)))),
            None => BlocklistEvent::Consumed,
        }
    }

    fn move_down(&mut self) {
        if self.selected_index + 1 < self.rules.len() {
            self.selected_index += 1;
        }
        self.adjust_scroll();
    }

    fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        self.adjust_scroll();
    }

    /// Advance the selection by up to one viewport (`visible_rows`),
    /// clamped to the last rule. No-op on an empty list.
    fn page_down(&mut self) {
        if self.rules.is_empty() {
            return;
        }
        let step = self.visible_rows.max(1);
        let last = self.rules.len() - 1;
        self.selected_index = (self.selected_index + step).min(last);
        self.adjust_scroll();
    }

    /// Retreat the selection by up to one viewport (`visible_rows`),
    /// clamped to the first rule. No-op on an empty list.
    fn page_up(&mut self) {
        if self.rules.is_empty() {
            return;
        }
        let step = self.visible_rows.max(1);
        self.selected_index = self.selected_index.saturating_sub(step);
        self.adjust_scroll();
    }

    /// Jump the selection to the first rule. No-op on an empty list.
    fn jump_top(&mut self) {
        if self.rules.is_empty() {
            return;
        }
        self.selected_index = 0;
        self.adjust_scroll();
    }

    /// Jump the selection to the last rule. No-op on an empty list.
    fn jump_bottom(&mut self) {
        if self.rules.is_empty() {
            return;
        }
        self.selected_index = self.rules.len() - 1;
        self.adjust_scroll();
    }

    fn toggle_focused(&self) -> BlocklistEvent {
        let Some(rule) = self.focused_rule() else {
            return BlocklistEvent::Consumed;
        };
        BlocklistEvent::Emit(Box::new(Action::ToggleBlocklistRule(rule.id.clone())))
    }
}

pub(crate) fn action_color(action: &str) -> Color {
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

/// TUI-130: the focused rule's whole detail, as the detail pane lists it.
fn rule_detail(rule: &BlocklistRuleInfo) -> String {
    let mut text = format!(
        "ID: {}\nAction: {}\nSource: {}\nPattern: {}\nReason: {}",
        rule.id, rule.action, rule.source, rule.pattern, rule.reason
    );
    if let Some(guidance) = &rule.guidance {
        text.push_str(&format!("\nGuidance: {guidance}"));
    }
    text
}
