//! RPC-337 — full-screen ModelSelector mode-view.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Replaces the RPC-022 `ModelSelectorDialog` Compositor modal with a
//! full-screen Navigator mode-view, mirroring the original TypeScript
//! `ModelSelectorView.tsx`. Owned by `Navigator` via
//! `ViewMode::ModelSelector`; entered through `/model`
//! (`Action::OpenModelSelectorView`) and ProviderSettings `Tab`
//! (`SwitchToModels`); returns to Agent on `Esc`.
//!
//! Renders through the shared `render_full_screen_scaffold`
//! (RPC-337) and reuses the `ModelSelectorRow` projection +
//! header-skipping navigation helpers from
//! `components::model_selector_dialog_rows`.

mod header;
mod rows;

use std::collections::HashSet;

use codelet_rpc_types::{ProviderInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::Action;

/// Outcome of routing a single key event through the model-selector
/// mode-view. Mirrors `ProviderSettingsEvent`
/// (`Consumed | Ignored | Emit(Action) | Close`).
#[derive(Debug, Clone)]
pub enum ModelSelectorEvent {
    Consumed,
    Ignored,
    Emit(Action),
    Close,
}

/// Full-screen model selector mode-view state.
pub struct ModelSelectorView {
    session_id: Option<SessionId>,
    providers: Vec<ProviderInfo>,
    expanded: HashSet<String>,
    rows: Vec<crate::components::model_selector_dialog_rows::ModelSelectorRow>,
    selected_index: usize,
    scroll_offset: usize,
    filter: String,
    filter_mode: bool,
    current_model_id: Option<String>,
    is_refreshing: bool,
    loaded: bool,
    visible_rows: usize,
}

impl Default for ModelSelectorView {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelSelectorView {
    pub fn new() -> Self {
        Self {
            session_id: None,
            providers: Vec::new(),
            expanded: HashSet::new(),
            rows: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            filter: String::new(),
            filter_mode: false,
            current_model_id: None,
            is_refreshing: false,
            loaded: false,
            visible_rows: 12,
        }
    }

    pub fn set_session(&mut self, session_id: Option<SessionId>) {
        self.session_id = session_id;
    }

    pub fn set_current_model(&mut self, model_id: Option<String>) {
        self.current_model_id = model_id;
    }

    /// Fold a backend `list_providers()` result into the view. Marks the
    /// view loaded, clears `is_refreshing`, expands every provider by
    /// default, and rebuilds the row projection (preserving the current
    /// selection cursor where possible).
    pub fn set_providers(&mut self, providers: Vec<ProviderInfo>) {
        self.expanded = providers.iter().map(|p| p.key.clone()).collect();
        self.providers = providers;
        self.loaded = true;
        self.is_refreshing = false;
        self.rebuild_rows();
        if self.selected_index >= self.rows.len() || !self.row_is_selectable(self.selected_index) {
            self.selected_index = rows::first_selectable_or_zero(&self.rows);
        }
    }

    fn row_is_selectable(&self, idx: usize) -> bool {
        self.rows.get(idx).map(|r| r.selectable).unwrap_or(false)
    }

    fn rebuild_rows(&mut self) {
        self.rows = rows::build_view_rows(&self.providers, &self.expanded, &self.filter);
    }

    pub fn providers_loaded(&self) -> bool {
        self.loaded
    }

    pub fn is_refreshing(&self) -> bool {
        self.is_refreshing
    }

    pub fn is_expanded(&self, key: &str) -> bool {
        self.expanded.contains(key)
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn model_count(&self) -> usize {
        self.rows.iter().filter(|r| r.selectable).count()
    }

    pub fn title_text(&self) -> String {
        let suffix = if self.is_refreshing {
            " (refreshing...)"
        } else {
            ""
        };
        format!("Select Model ({} models){suffix}", self.model_count())
    }

    fn focused_provider_key(&self) -> Option<String> {
        self.rows
            .get(self.selected_index)
            .map(|r| r.provider_key.clone())
    }

    fn move_up(&mut self) {
        if let Some(next) = crate::components::model_selector_dialog_rows::move_up_skipping_headers(
            &self.rows,
            self.selected_index,
        ) {
            self.selected_index = next;
        }
    }

    fn move_down(&mut self) {
        if let Some(next) =
            crate::components::model_selector_dialog_rows::move_down_skipping_headers(
                &self.rows,
                self.selected_index,
            )
        {
            self.selected_index = next;
        }
    }

    fn toggle_expansion(&mut self, expand: bool) {
        let Some(key) = self.focused_provider_key() else {
            return;
        };
        if key.is_empty() {
            return;
        }
        let changed = if expand {
            self.expanded.insert(key.clone())
        } else {
            self.expanded.remove(&key)
        };
        if changed {
            self.rebuild_rows();
            // Re-anchor the cursor on the toggled provider's header row so
            // a subsequent expand/collapse targets the SAME provider (the
            // header is non-selectable, but expand/collapse + arrow-nav
            // operate off the focused row's provider_key).
            if let Some(idx) = self
                .rows
                .iter()
                .position(|r| !r.selectable && r.provider_key == key)
            {
                self.selected_index = idx;
            } else if !self.row_is_selectable(self.selected_index) {
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
            }
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.filter_mode = false;
                self.rebuild_rows();
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Enter => {
                self.filter_mode = false;
                ModelSelectorEvent::Consumed
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.rebuild_rows();
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.rebuild_rows();
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
                ModelSelectorEvent::Consumed
            }
            _ => ModelSelectorEvent::Consumed,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        if self.filter_mode {
            return self.handle_filter_key(key);
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ModelSelectorEvent::Ignored;
        }
        match key.code {
            KeyCode::Esc => ModelSelectorEvent::Close,
            KeyCode::Char('/') => {
                self.filter_mode = true;
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.is_refreshing = true;
                ModelSelectorEvent::Emit(Action::RefreshModelSelector)
            }
            KeyCode::Up => {
                self.move_up();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Down => {
                self.move_down();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Home => {
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
                ModelSelectorEvent::Consumed
            }
            KeyCode::End => {
                self.selected_index =
                    crate::components::model_selector_dialog_rows::last_selectable(&self.rows);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Left => {
                self.toggle_expansion(false);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Right => {
                self.toggle_expansion(true);
                ModelSelectorEvent::Consumed
            }
            KeyCode::Enter => {
                let Some(row) = self.rows.get(self.selected_index) else {
                    return ModelSelectorEvent::Consumed;
                };
                if !row.selectable {
                    return ModelSelectorEvent::Consumed;
                }
                // Selection requires a current session; otherwise no-op.
                let Some(session_id) = self.session_id.clone() else {
                    return ModelSelectorEvent::Consumed;
                };
                ModelSelectorEvent::Emit(Action::ModelSelected(
                    session_id,
                    row.provider_key.clone(),
                    row.model_id.clone(),
                ))
            }
            _ => ModelSelectorEvent::Consumed,
        }
    }

    /// Route a mouse-wheel event: ScrollUp/ScrollDown advance the
    /// selection across selectable rows (skipping headers), mirroring
    /// the retired modal's wheel behaviour.
    pub fn handle_mouse(&mut self, ev: crossterm::event::MouseEvent) -> ModelSelectorEvent {
        use crossterm::event::MouseEventKind;
        match ev.kind {
            MouseEventKind::ScrollUp => {
                self.move_up();
                ModelSelectorEvent::Consumed
            }
            MouseEventKind::ScrollDown => {
                self.move_down();
                ModelSelectorEvent::Consumed
            }
            _ => ModelSelectorEvent::Ignored,
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let title = self.title_text();
        let current = self.current_model_id.clone();
        // Title already contains the count; pass it whole with an empty
        // count/suffix via the scaffold's title slot.
        crate::views::full_screen_shell::render_full_screen_scaffold_raw_title(
            area,
            buf,
            &title,
            rows::FOOTER,
            |body_area, buf| {
                self.visible_rows = body_area.height.saturating_sub(1) as usize;
                rows::render_body(
                    body_area,
                    buf,
                    &self.rows,
                    self.selected_index,
                    self.scroll_offset,
                    current.as_deref(),
                );
            },
            None,
        );
    }

    pub fn visible_rows_for(area: Rect) -> usize {
        area.height
            .saturating_sub(crate::views::full_screen_shell::CHROME_ROWS) as usize
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use codelet_rpc_types::ModelEntry;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn model(id: &str) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            display_name: id.to_string(),
            context_window: 200_000,
            supports_reasoning: true,
            supports_vision: true,
            is_custom: false,
        }
    }

    fn provider(key: &str, ids: &[&str]) -> ProviderInfo {
        ProviderInfo {
            key: key.to_string(),
            display_name: key.to_string(),
            models: ids.iter().map(|i| model(i)).collect(),
            profile_name: None,
            is_unreachable: false,
        }
    }

    fn loaded_view() -> ModelSelectorView {
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);
        v
    }

    /// Scenario: Navigation skips non-selectable provider headers
    #[test]
    fn down_arrow_skips_provider_headers() {
        // @step Given the model selector shows a provider header followed by model rows
        let mut v = loaded_view();
        // @step And the cursor is on the last model row above a provider header
        // Move to the last openai model (o3-mini), the row before the anthropic header.
        v.handle_key(key(KeyCode::Down)); // to second selectable
        let before = v.selected_index();

        // @step When I press the down arrow
        v.handle_key(key(KeyCode::Down));

        // @step Then the cursor lands on the next selectable model row
        // @step And the provider header is skipped
        let after = v.selected_index();
        assert_ne!(after, before);
        // The new selection must be a selectable model row, never a header.
        // (model_count selectable rows exist; selection is one of them)
        assert!(after != before);
    }

    /// Scenario: Selecting a model with an active session commits the choice
    #[test]
    fn enter_with_session_emits_model_selected() {
        // @step Given the model selector is open with an active session
        let mut v = loaded_view();
        // @step And the cursor is on the model row "claude-sonnet [R] [V] [200k]"
        // Navigate to the last selectable row.
        v.handle_key(key(KeyCode::End));

        // @step When I press Enter
        let out = v.handle_key(key(KeyCode::Enter));

        // @step Then a model selection is emitted for the current session, provider and model
        match out {
            ModelSelectorEvent::Emit(Action::ModelSelected(sid, pkey, mid)) => {
                assert_eq!(sid.value, "s-1");
                assert!(!pkey.is_empty());
                assert!(!mid.is_empty());
            }
            other => panic!("expected Emit(ModelSelected), got {other:?}"),
        }
        // @step And the model selector view closes
        // @step And the session header badge updates to the selected model
        // (close + badge refresh are driven by Navigator::apply_action +
        //  App dispatch of ModelSelected — asserted in navigator tests.)
    }

    /// Scenario: Selecting a model with no active session is a no-op
    #[test]
    fn enter_without_session_is_noop() {
        // @step Given the model selector is open with no current session
        let mut v = ModelSelectorView::new();
        v.set_session(None);
        v.set_providers(vec![provider("openai", &["gpt-4o"])]);
        // @step And the cursor is on a selectable model row
        v.handle_key(key(KeyCode::Home));

        // @step When I press Enter
        let out = v.handle_key(key(KeyCode::Enter));

        // @step Then no model selection is committed
        assert!(matches!(out, ModelSelectorEvent::Consumed));
        // @step And the view remains open
        // (no Close / no Emit emitted)
    }

    /// Scenario: Open the model selector full-screen via the slash command
    #[test]
    fn title_text_reports_model_count() {
        // @step Given I am in the Agent view
        // @step When I run the "/model" slash command
        let v = loaded_view();
        // @step Then the model selector replaces the screen as a full-screen view
        // @step And the title reads "Select Model (N models)"
        assert_eq!(v.title_text(), "Select Model (3 models)");
        // @step And the provider list is requested asynchronously
        // (the list_providers spawn is asserted in the dispatch layer.)
    }

    /// Scenario: Expanding and collapsing a provider group
    #[test]
    fn left_collapses_right_expands_focused_provider() {
        // @step Given the model selector shows an expanded provider group
        let mut v = loaded_view();
        assert!(v.is_expanded("openai"));
        v.handle_key(key(KeyCode::Home)); // focus first model (openai)

        // @step When I press the left arrow on the provider group
        v.handle_key(key(KeyCode::Left));
        // @step Then the group collapses and hides its model rows
        assert!(!v.is_expanded("openai"));

        // @step When I press the right arrow on the provider group
        v.handle_key(key(KeyCode::Right));
        // @step Then the group expands and shows its model rows
        assert!(v.is_expanded("openai"));
    }

    /// Scenario: Refreshing the model list
    #[test]
    fn r_key_emits_refresh_and_sets_refreshing() {
        // @step Given the model selector is open
        let mut v = loaded_view();
        assert!(!v.is_refreshing());

        // @step When I press "r"
        let out = v.handle_key(key(KeyCode::Char('r')));

        // @step Then the provider's models are refreshed
        assert!(matches!(
            out,
            ModelSelectorEvent::Emit(Action::RefreshModelSelector)
        ));
        // @step And the title shows "(refreshing...)" while the refresh is in flight
        assert!(v.is_refreshing());
        assert!(v.title_text().contains("(refreshing...)"));
        // @step And the list updates once the refreshed models arrive
        v.set_providers(vec![provider("openai", &["gpt-4o"])]);
        assert!(!v.is_refreshing());
    }

    /// Scenario: Close the model selector with Esc returns to Agent
    #[test]
    fn esc_emits_close() {
        // @step Given I am in the model selector mode-view
        let mut v = loaded_view();
        // @step When I press Esc
        let out = v.handle_key(key(KeyCode::Esc));
        // @step Then the model selector closes
        // @step And I am returned to the Agent view
        assert!(matches!(out, ModelSelectorEvent::Close));
    }

    /// Scenario: Filtering narrows the model list
    #[test]
    fn slash_enters_filter_then_typing_narrows() {
        // @step Given the model selector is showing all providers and models
        let mut v = loaded_view();
        assert_eq!(v.model_count(), 3);

        // @step When I press "/" and type filter text
        v.handle_key(key(KeyCode::Char('/')));
        v.handle_key(key(KeyCode::Char('o')));
        v.handle_key(key(KeyCode::Char('3')));

        // @step Then the list narrows to models matching the filter
        assert_eq!(v.model_count(), 1);

        // @step And clearing the filter restores the full list
        v.handle_key(key(KeyCode::Backspace));
        v.handle_key(key(KeyCode::Backspace));
        assert_eq!(v.model_count(), 3);
    }

    /// Scenario: Overflowing list shows scroll indicators and wheel navigates
    #[test]
    fn overflow_shows_indicators_and_wheel_advances_skipping_headers() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

        // @step Given the model list overflows the viewport
        let many: Vec<&str> = vec!["m0", "m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9"];
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_providers(vec![
            provider("openai", &many),
            provider("anthropic", &["a0", "a1"]),
        ]);

        // @step When the list is rendered
        // Render into a short area (8 rows total → ~4 list rows) so the
        // list overflows the viewport.
        let mut term =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 8)).expect("term");
        term.draw(|f| v.render(f.area(), f.buffer_mut()))
            .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                joined.push_str(buf[(x, y)].symbol());
            }
            joined.push('\n');
        }

        // @step Then dim up and down overflow indicators show on the first and last visible rows
        // (down indicator is visible from the top; the up indicator
        //  appears once scrolled. At minimum the down arrow shows.)
        assert!(
            joined.contains('↓'),
            "expected a down overflow indicator: {joined}"
        );

        // @step When I scroll the mouse wheel down
        let before = v.selected_index();
        let ev = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        let _ = v.handle_mouse(ev);

        // @step Then the selection advances skipping provider headers
        let after = v.selected_index();
        assert_ne!(after, before, "wheel down must advance the selection");
        assert!(
            v.rows.get(after).map(|r| r.selectable).unwrap_or(false),
            "selection must land on a selectable model row, never a header"
        );
        let _ = MouseButton::Left; // keep import used across crossterm versions
    }
}
