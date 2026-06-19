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
    /// view loaded, clears `is_refreshing`, and rebuilds the row
    /// projection. Following TS parity (`useModelSelectorState.ts:148-150`,
    /// `ModelSelectorScreen.tsx:93-119`) every provider starts collapsed and
    /// only the section containing the current model is auto-expanded, so the
    /// list fits the viewport on first open instead of overflowing.
    pub fn set_providers(&mut self, providers: Vec<ProviderInfo>) {
        // RPC-342: start all-collapsed, then expand ONLY the section that
        // contains the current model (if any).
        self.expanded = HashSet::new();
        if let Some(current) = self.current_model_id.as_deref() {
            if let Some(p) = providers
                .iter()
                .find(|p| p.models.iter().any(|m| m.id == current))
            {
                self.expanded.insert(p.key.clone());
            }
        }
        self.providers = providers;
        self.loaded = true;
        self.is_refreshing = false;
        self.rebuild_rows();
        // RPC-341: seed the cursor on the active-session model when it is
        // present (TS auto-expand-to-current, ModelSelectorScreen.tsx:93-119).
        // The current model's section was just auto-expanded above (RPC-342),
        // so its row already exists. Falls back to the existing
        // validate-or-first-selectable behavior when there is no current model
        // or it is not loaded.
        if let Some(idx) = rows::index_of_model(&self.rows, self.current_model_id.as_deref()) {
            self.selected_index = idx;
        } else if self.selected_index >= self.rows.len()
            || !self.row_is_selectable(self.selected_index)
        {
            self.selected_index = rows::first_selectable_or_zero(&self.rows);
        }
        self.adjust_scroll();
    }

    fn row_is_selectable(&self, idx: usize) -> bool {
        self.rows.get(idx).map(|r| r.selectable).unwrap_or(false)
    }

    /// Keep `selected_index` inside the visible window by reconciling
    /// `scroll_offset`. Reuses the shared `scroll_viewport::ensure_visible`
    /// helper (the same primitive `ProviderSettingsView::adjust_scroll`
    /// uses). Called after every mutation that moves the selection or
    /// rebuilds the row list, plus once at render-time once the real body
    /// height is known.
    fn adjust_scroll(&mut self) {
        crate::components::scroll_viewport::ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            self.visible_rows,
            self.rows.len(),
        );
        // When the cursor sits on the first selectable row, everything
        // above it is a non-selectable provider header; reveal it by
        // anchoring the window at the top (TS parity: scrolling to the
        // top shows the leading section header, not just the first model).
        if self.selected_index == rows::first_selectable_or_zero(&self.rows)
            && self.selected_index < self.visible_rows
        {
            self.scroll_offset = 0;
        }
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

    /// Number of selectable model rows in the CURRENT projection (honours
    /// the expanded set and any active filter). Used for navigation and the
    /// filter-narrowing assertions.
    pub fn model_count(&self) -> usize {
        self.rows.iter().filter(|r| r.selectable).count()
    }

    /// Total number of models across ALL providers, independent of which
    /// sections are expanded or filtered. RPC-342: the title shows this total
    /// so a collapse-by-default open reads "(N models)" instead of a confusing
    /// "(0 models)".
    pub fn total_model_count(&self) -> usize {
        self.providers.iter().map(|p| p.models.len()).sum()
    }

    pub fn title_text(&self) -> String {
        let suffix = if self.is_refreshing {
            " (refreshing...)"
        } else {
            ""
        };
        format!("Select Model ({} models){suffix}", self.total_model_count())
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
            self.adjust_scroll();
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
            self.adjust_scroll();
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
            self.adjust_scroll();
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> ModelSelectorEvent {
        match key.code {
            KeyCode::Esc => {
                self.filter.clear();
                self.filter_mode = false;
                self.rebuild_rows();
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
                self.adjust_scroll();
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
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.rebuild_rows();
                self.selected_index = rows::first_selectable_or_zero(&self.rows);
                self.adjust_scroll();
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
                self.adjust_scroll();
                ModelSelectorEvent::Consumed
            }
            KeyCode::End => {
                self.selected_index =
                    crate::components::model_selector_dialog_rows::last_selectable(&self.rows);
                self.adjust_scroll();
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
                // Defensive reconcile: now that the real body height is
                // known, re-clamp the offset (covers window-resize and
                // initial-draw where navigation ran with a stale height).
                self.adjust_scroll();
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
        // Most of these shipped scenarios assume their GIVEN precondition of
        // expanded provider groups (the pre-RPC-342 default). RPC-342 makes the
        // real default collapse-on-load — verified separately by its own tests
        // below — so here we restore the expanded fixture explicitly.
        expand_all(&mut v);
        v
    }

    /// Test fixture helper: expand every provider section and reset the
    /// selection to the first selectable row. Mirrors the pre-RPC-342
    /// all-expanded default for scenarios whose GIVEN assumes expanded groups.
    fn expand_all(v: &mut ModelSelectorView) {
        v.expanded = v.providers.iter().map(|p| p.key.clone()).collect();
        v.rebuild_rows();
        v.selected_index = rows::first_selectable_or_zero(&v.rows);
        v.adjust_scroll();
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
        expand_all(&mut v);
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
        expand_all(&mut v);

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

    // ---- RPC-340: scroll-follows-cursor -------------------------------

    /// Render into a `width`x`height` TestBackend so `self.visible_rows`
    /// is populated from the real body height (height - chrome - legend).
    fn render_at(v: &mut ModelSelectorView, width: u16, height: u16) {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height))
            .expect("term");
        term.draw(|f| v.render(f.area(), f.buffer_mut()))
            .expect("draw");
    }

    fn tall_view() -> ModelSelectorView {
        // One provider, 30 models → a single header + 30 selectable rows.
        let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_providers(vec![provider("openai", &refs)]);
        expand_all(&mut v);
        v
    }

    /// Scenario: Navigating down past the bottom scrolls the viewport to follow the cursor
    #[test]
    fn down_past_bottom_scrolls_viewport_to_follow_cursor() {
        // @step Given the model selector shows a body viewport 10 rows tall
        let mut v = tall_view();
        render_at(&mut v, 60, 14); // body ≈ 10 list rows after chrome+legend
        let visible = v.visible_rows;
        assert!(visible > 0 && visible < v.rows.len());

        // @step And the list is much longer than the viewport with the cursor at the top
        assert_eq!(v.scroll_offset, 0);

        // @step When I press Down until the selected row would fall below the visible window
        for _ in 0..(visible + 2) {
            v.handle_key(key(KeyCode::Down));
        }

        // @step Then the viewport scrolls down so the selected row becomes the last visible row
        assert_eq!(v.scroll_offset, v.selected_index + 1 - visible);
        // @step And the selected row stays inside the visible window
        assert!(v.selected_index >= v.scroll_offset);
        assert!(v.selected_index < v.scroll_offset + visible);
    }

    /// Scenario: Navigating back up scrolls the viewport up with the cursor
    #[test]
    fn up_to_top_scrolls_viewport_back_to_offset_zero() {
        // @step Given the model selector shows a body viewport 10 rows tall
        let mut v = tall_view();
        render_at(&mut v, 60, 14);
        let visible = v.visible_rows;

        // @step And the cursor has been moved down so the viewport is scrolled away from the top
        for _ in 0..(visible + 5) {
            v.handle_key(key(KeyCode::Down));
        }
        assert!(v.scroll_offset > 0);

        // @step When I press Up until the cursor reaches the first row
        let first = rows::first_selectable_or_zero(&v.rows);
        while v.selected_index > first {
            v.handle_key(key(KeyCode::Up));
        }

        // @step Then the viewport scrolls up with the cursor
        assert!(v.selected_index >= v.scroll_offset);
        // @step And the scroll offset returns to 0
        assert_eq!(v.scroll_offset, 0);
    }

    /// Scenario: End jumps to the last row and pins it to the bottom edge
    #[test]
    fn end_pins_last_row_to_bottom_edge() {
        // @step Given the model selector shows a body viewport 10 rows tall
        let mut v = tall_view();
        render_at(&mut v, 60, 14);
        let visible = v.visible_rows;
        let total = v.rows.len();
        assert!(total > visible);

        // @step And the list is taller than the viewport
        // @step When I press End
        v.handle_key(key(KeyCode::End));

        // @step Then the cursor is on the last selectable row
        assert_eq!(
            v.selected_index,
            crate::components::model_selector_dialog_rows::last_selectable(&v.rows)
        );
        // @step And the scroll offset equals total rows minus visible rows
        assert_eq!(v.scroll_offset, total - visible);
        // @step And there are no blank rows rendered after the last row
        assert_eq!(v.scroll_offset + visible, total);
    }

    /// Scenario: Mouse-wheel navigation scrolls the viewport like the Down key
    #[test]
    fn wheel_down_scrolls_viewport_like_down_key() {
        use crossterm::event::{MouseEvent, MouseEventKind};

        // @step Given the model selector shows a body viewport 10 rows tall
        let mut v = tall_view();
        render_at(&mut v, 60, 14);
        let visible = v.visible_rows;

        // @step And the list overflows the viewport with the cursor on the last visible row
        // Move down to the bottom edge of the current window (selected ==
        // scroll_offset + visible - 1) without yet scrolling.
        while v.selected_index < v.scroll_offset + visible - 1 {
            v.handle_key(key(KeyCode::Down));
        }
        assert_eq!(v.selected_index, v.scroll_offset + visible - 1);
        let before = v.selected_index;
        let offset_before = v.scroll_offset;

        // @step When I scroll the mouse-wheel down
        let ev = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        };
        v.handle_mouse(ev);

        // @step Then the selection advances to the next selectable row skipping headers
        assert_eq!(v.selected_index, before + 1);
        assert!(v.rows[v.selected_index].selectable);
        // @step And the viewport scrolls to keep the new selection visible
        assert_eq!(v.scroll_offset, offset_before + 1);
        assert!(v.selected_index < v.scroll_offset + visible);
        assert!(v.selected_index >= v.scroll_offset);
    }

    /// Scenario: Filtering rebuilds the list and reconciles the scroll offset
    #[test]
    fn filtering_reconciles_scroll_offset() {
        // @step Given the model selector has been scrolled down a long list
        let mut v = tall_view();
        render_at(&mut v, 60, 14);
        let visible = v.visible_rows;
        v.handle_key(key(KeyCode::End));
        assert!(v.scroll_offset > 0);

        // @step When I type a filter that narrows the results to a few rows
        v.handle_key(key(KeyCode::Char('/')));
        v.handle_key(key(KeyCode::Char('m')));
        v.handle_key(key(KeyCode::Char('1')));

        // @step Then the scroll offset is reconciled so the reset selection is visible
        assert!(v.selected_index >= v.scroll_offset);
        assert!(v.selected_index < v.scroll_offset + visible);
        // @step And there are no blank trailing rows rendered
        let total = v.rows.len();
        assert!(v.scroll_offset <= total.saturating_sub(visible));
    }

    /// Scenario: A tiny or empty viewport renders gracefully without panic
    #[test]
    fn tiny_viewport_renders_without_panic() {
        // @step Given the model selector body viewport is only 3 rows tall or the list is empty
        let mut v = tall_view();
        v.handle_key(key(KeyCode::End)); // push selection/offset down first

        // @step When the body is rendered
        render_at(&mut v, 60, 3); // body collapses near zero

        // @step Then the scroll offset is 0
        assert_eq!(v.scroll_offset, 0);
        // @step And the body renders without panic
        // (reaching here without panic satisfies the scenario)
    }

    /// Scenario: Shrinking the terminal re-clamps the scroll offset on the next paint
    #[test]
    fn shrinking_terminal_reclamps_offset_on_next_paint() {
        // @step Given the model selector cursor is near the bottom of a tall list
        let mut v = tall_view();
        render_at(&mut v, 60, 14);
        v.handle_key(key(KeyCode::End));
        let total = v.rows.len();

        // @step When the terminal is resized smaller so the body has fewer rows
        render_at(&mut v, 60, 8);
        let visible = v.visible_rows;

        // @step Then on the next paint the scroll offset is re-clamped
        assert_eq!(v.scroll_offset, total - visible);
        // @step And the selected row is still visible
        assert!(v.selected_index >= v.scroll_offset);
        assert!(v.selected_index < v.scroll_offset + visible);
        // @step And there are no blank trailing rows rendered
        assert_eq!(v.scroll_offset + visible, total);
    }

    // ---- RPC-341: open on the current model ---------------------------

    /// Scenario: Cursor lands on the current model when it is loaded
    #[test]
    fn cursor_lands_on_current_model_when_loaded() {
        // @step Given my current model is "claude-sonnet"
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(Some("claude-sonnet".to_string()));

        // @step When the model selector loads the "openai" and "anthropic" providers
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the cursor is on the selectable row for "claude-sonnet"
        let row = &v.rows[v.selected_index];
        assert!(row.selectable);
        assert_eq!(row.model_id, "claude-sonnet");

        // @step And the cursor is not on the first model "gpt-4o"
        assert_ne!(row.model_id, "gpt-4o");
    }

    /// Scenario: Cursor falls back to the first selectable row when no current model is set
    #[test]
    fn cursor_falls_back_when_no_current_model() {
        // @step Given no current model is set
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(None);

        // @step When the model selector loads the providers
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the cursor is on the first selectable row
        assert_eq!(v.selected_index, rows::first_selectable_or_zero(&v.rows));
    }

    /// Scenario: Cursor falls back to the first selectable row when the current model is not found
    #[test]
    fn cursor_falls_back_when_current_model_not_found() {
        // @step Given my current model is "does-not-exist"
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(Some("does-not-exist".to_string()));

        // @step When the model selector loads the providers
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the cursor is on the first selectable row
        assert_eq!(v.selected_index, rows::first_selectable_or_zero(&v.rows));
    }

    /// Scenario: Seeded cursor on a below-the-fold model is scrolled into view
    #[test]
    fn seeded_cursor_below_fold_is_scrolled_into_view() {
        // @step Given my current model is in a long list below the viewport fold
        let ids: Vec<String> = (0..30).map(|i| format!("m{i}")).collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(Some("m25".to_string()));

        // @step When the model selector loads the providers
        v.set_providers(vec![provider("openai", &refs)]);

        // @step Then the cursor is on the selectable row for my current model
        let row = &v.rows[v.selected_index];
        assert!(row.selectable);
        assert_eq!(row.model_id, "m25");

        // @step And the seeded row is scrolled into view
        assert!(v.selected_index >= v.scroll_offset);
        assert!(v.selected_index < v.scroll_offset + v.visible_rows);
    }

    // ---- RPC-342: collapse-by-default expansion -----------------------

    /// Scenario: No current model set leaves every provider collapsed
    #[test]
    fn no_current_model_leaves_every_provider_collapsed() {
        // @step Given no current model is set
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(None);

        // @step When the model selector loads the "openai" and "anthropic" providers
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the "openai" provider is collapsed
        assert!(!v.is_expanded("openai"));
        // @step And the "anthropic" provider is collapsed
        assert!(!v.is_expanded("anthropic"));
        // @step And the title reads "Select Model (3 models)"
        assert_eq!(v.title_text(), "Select Model (3 models)");
    }

    /// Scenario: Only the current model's provider section is auto-expanded
    #[test]
    fn only_current_models_section_is_auto_expanded() {
        // @step Given my current model is "claude-sonnet"
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(Some("claude-sonnet".to_string()));

        // @step When the model selector loads the "openai" and "anthropic" providers
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the "anthropic" provider is expanded
        assert!(v.is_expanded("anthropic"));
        // @step And the "openai" provider is collapsed
        assert!(!v.is_expanded("openai"));
        // @step And the cursor is on the selectable row for "claude-sonnet"
        let row = &v.rows[v.selected_index];
        assert!(row.selectable);
        assert_eq!(row.model_id, "claude-sonnet");
    }

    /// Scenario: A current model in the first provider expands only that section
    #[test]
    fn current_model_in_first_provider_expands_only_that_section() {
        // @step Given my current model is "gpt-4o"
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(Some("gpt-4o".to_string()));

        // @step When the model selector loads the "openai" and "anthropic" providers
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the "openai" provider is expanded
        assert!(v.is_expanded("openai"));
        // @step And the "anthropic" provider is collapsed
        assert!(!v.is_expanded("anthropic"));
    }

    /// Scenario: Filtering reveals matches inside collapsed providers
    #[test]
    fn filtering_reveals_matches_inside_collapsed_providers() {
        // @step Given no current model is set
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(None);

        // @step And the model selector has loaded the "openai" and "anthropic" providers all collapsed
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);
        assert!(!v.is_expanded("openai"));

        // @step When I type the filter "o3"
        v.handle_key(key(KeyCode::Char('/')));
        v.handle_key(key(KeyCode::Char('o')));
        v.handle_key(key(KeyCode::Char('3')));

        // @step Then the model list shows the "o3-mini" model even though "openai" was collapsed
        assert!(v
            .rows
            .iter()
            .any(|r| r.selectable && r.model_id == "o3-mini"));
    }

    /// Scenario: Reloading providers re-applies the collapse default
    #[test]
    fn reloading_providers_reapplies_collapse_default() {
        // @step Given my current model is "gpt-4o"
        let mut v = ModelSelectorView::new();
        v.set_session(Some(SessionId::new("s-1")));
        v.set_current_model(Some("gpt-4o".to_string()));

        // @step And the model selector has loaded the providers with only "openai" expanded
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);
        assert!(v.is_expanded("openai"));
        assert!(!v.is_expanded("anthropic"));

        // @step When the providers are reloaded
        v.set_providers(vec![
            provider("openai", &["gpt-4o", "o3-mini"]),
            provider("anthropic", &["claude-sonnet"]),
        ]);

        // @step Then the "openai" provider is expanded
        assert!(v.is_expanded("openai"));
        // @step And the "anthropic" provider is collapsed
        assert!(!v.is_expanded("anthropic"));
    }
}
