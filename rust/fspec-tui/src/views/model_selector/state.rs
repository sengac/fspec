//! State: construction, accessors, row projection & scroll reconciliation.
//!
//! Extracted from `mod.rs` (PROV-107) to keep that file under the
//! 300-LoC ceiling. Behaviour-preserving move of `impl ModelSelectorView`
//! methods; field/method visibility unchanged.

use super::*;
use crate::terminal::sanitize::sanitize_for_terminal;

impl ModelSelectorView {
    pub fn new() -> Self {
        Self {
            session_id: None,
            providers: Vec::new(),
            expanded: HashSet::new(),
            rows: Vec::new(),
            selected_index: 0,
            has_selection: false,
            scroll_offset: 0,
            filter: String::new(),
            filter_mode: false,
            current_model: None,
            is_refreshing: false,
            loaded: false,
            opening: true,
            newest_list: 0,
            load_error: None,
            focus_after_load: None,
            visible_rows: 12,
            wheel: crate::components::scroll_viewport::WheelVelocity::new(),
            custom_model_mode: CustomModelMode::Browse,
            form: CustomModelForm::default(),
            scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag::new(),
            last_scrollbar_rect: None,
            write_error: None,
        }
    }

    pub fn set_session(&mut self, session_id: Option<SessionId>) {
        self.session_id = session_id;
        self.write_error = None;
    }

    /// The session the selector was opened for.
    pub fn session(&self) -> Option<&SessionId> {
        self.session_id.as_ref()
    }

    /// BUG-325: record (or clear) the failure of the last custom-model write.
    pub fn set_write_error(&mut self, error: Option<String>) {
        self.write_error = error;
    }

    /// BUG-325: the failure of the last custom-model write, if any.
    pub fn write_error(&self) -> Option<&str> {
        self.write_error.as_deref()
    }

    /// BUG-576: the session's current `(section, model)`.
    pub fn set_current_model(&mut self, current: Option<(&str, &str)>) {
        self.current_model =
            current.map(|(section, model)| (section.to_string(), model.to_string()));
    }

    /// BUG-576: the session's model became known; when nothing is selected
    /// yet its section opens and the cursor goes to it.
    pub(crate) fn reconcile_current_model(&mut self, session: &SessionId, current: (&str, &str)) {
        if self.session_id.as_ref() != Some(session) {
            return;
        }
        self.set_current_model(Some(current));
        if self.loaded && !self.has_selection && self.seed_on_current() {
            self.adjust_scroll();
        }
    }

    /// Fold a provider list into the view: marks it loaded, clears
    /// `is_refreshing` and the load error, and rebuilds the rows. TUI-111:
    /// display names are sanitized on ingress.
    ///
    /// BUG-576 (product owner, 2026-11-07): the first list of a visit
    /// collapses every section but the current model's and seeds the cursor
    /// on it (RPC-341/RPC-342), or selects nothing (PROV-101). Every later
    /// list keeps the expanded sections that still exist and the cursor's
    /// row; after a custom-model write its section opens on its model.
    pub fn set_providers(&mut self, providers: Vec<ProviderInfo>) {
        let mut providers = providers;
        for provider in providers.iter_mut() {
            provider.display_name = sanitize_for_terminal(&provider.display_name);
            for model in provider.models.iter_mut() {
                model.display_name = sanitize_for_terminal(&model.display_name);
            }
        }
        let anchor = (!self.opening).then(|| self.cursor_row()).flatten();
        let focus = self.focus_after_load.take();
        self.expanded = self.expanded_for(&providers);
        if let Some((section, _)) = &focus {
            self.expanded.insert(section.clone());
        }
        self.providers = providers;
        self.loaded = true;
        self.is_refreshing = false;
        self.load_error = None;
        self.rebuild_rows();
        if std::mem::take(&mut self.opening) {
            if !self.seed_on_current() {
                self.selected_index = 0;
                self.has_selection = false;
            }
        } else if let Some((section, model)) = focus {
            self.place_cursor(&section, model.as_deref().unwrap_or_default());
            self.has_selection = true;
        } else if let Some((section, model)) = anchor {
            self.place_cursor(&section, &model);
        }
        self.clamp_cursor();
        self.adjust_scroll();
    }

    pub(crate) fn row_is_selectable(&self, idx: usize) -> bool {
        self.rows.get(idx).map(|r| r.selectable).unwrap_or(false)
    }

    /// PROV-101: whether the cursor currently points at an explicitly active
    /// selection. `false` after `set_providers` finds no current model — the
    /// UI shows "nothing selected" and Enter is a no-op.
    pub fn has_active_selection(&self) -> bool {
        self.has_selection
    }

    /// Anchor the cursor on the first selectable row as an explicit user
    /// action (Home, filter changes). Marks the selection active.
    pub(crate) fn anchor_first_selectable(&mut self) {
        self.selected_index =
            crate::components::model_selector_dialog_rows::first_selectable(&self.rows);
        self.has_selection = true;
    }

    /// Keep `selected_index` inside the visible window by reconciling
    /// `scroll_offset`. Reuses the shared `scroll_viewport::ensure_visible`
    /// helper (the same primitive `ProviderSettingsView::adjust_scroll`
    /// uses). Called after every mutation that moves the selection or
    /// rebuilds the row list, plus once at render-time once the real body
    /// height is known.
    /// BUG-578 F20: the scrollbar moved the window to `offset`; the
    /// selection moves to the nearest model row inside it, so the window
    /// stays where the thumb put it.
    pub(crate) fn scroll_to(&mut self, offset: usize) {
        let total = self.rows.len();
        self.scroll_offset = offset.min(total.saturating_sub(self.visible_rows));
        let end = (self.scroll_offset + self.visible_rows).min(total);
        let window = self.scroll_offset..end;
        let current = self.selected_index;
        let nearest = window
            .filter(|&index| self.rows.get(index).is_some_and(|row| row.selectable))
            .min_by_key(|&index| index.abs_diff(current));
        if let Some(index) = nearest {
            self.selected_index = index;
        }
        self.adjust_scroll();
    }

    pub(crate) fn adjust_scroll(&mut self) {
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
        if self.selected_index
            == crate::components::model_selector_dialog_rows::first_selectable(&self.rows)
            && self.selected_index < self.visible_rows
        {
            self.scroll_offset = 0;
        }
    }

    pub(crate) fn rebuild_rows(&mut self) {
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

    /// MODEL-007: the number of model rows the last render reserved for the
    /// windowed list (body height minus the legend, minus the filter line
    /// when a filter prompt is present). Exposed for the filter-reservation
    /// acceptance test, which asserts the value drops by one when the filter
    /// row steals a body line — mirrors the existing `selected_index` /
    /// `model_count` observability accessors.
    ///
    /// Integration tests live in a SEPARATE crate (`tests/`) and cannot see
    /// `#[cfg(test)]` lib items, so this accessor must stay ungated `pub`
    /// (matching the sibling `set_visible_rows_for_test` popup accessors).
    /// `#[doc(hidden)]` keeps it off the published API surface — the same
    /// convention those siblings use.
    #[doc(hidden)]
    pub fn visible_rows_for_test(&self) -> usize {
        self.visible_rows
    }

    /// RPC-344: the current custom-model CRUD sub-mode.
    pub fn custom_model_mode(&self) -> &CustomModelMode {
        &self.custom_model_mode
    }

    /// RPC-344: the in-progress custom-model form values.
    pub fn form(&self) -> &CustomModelForm {
        &self.form
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

    /// MODEL-008: the SINGLE source of truth for the browse-list title's
    /// dim count span. Produces the already-pluralized `(N models)` label
    /// with the `(refreshing...)` status annotation appended when a refresh
    /// is in flight. BOTH `render.rs` (the rendered dim span) and
    /// [`Self::title_text`] consume this, so the rendered UI and the tested
    /// string can never drift apart.
    pub fn title_count_label(&self) -> String {
        // PROV-127: share the singular/plural rule with the header rows.
        let base = super::rows::model_count_label(self.total_model_count());
        if self.is_refreshing {
            format!("{base} (refreshing...)")
        } else {
            base
        }
    }

    /// The bold-yellow name span of the browse-list title. A constant today,
    /// exposed as a method so render and tests share one definition.
    pub fn title_name(&self) -> &'static str {
        "Select Model"
    }

    /// The full flat title string (`"Select Model (N models)"`, plus a
    /// trailing ` (refreshing...)` while refreshing). Composed from the same
    /// [`Self::title_name`] + [`Self::title_count_label`] pieces that
    /// `render.rs` paints, so this remains a faithful projection of the UI.
    pub fn title_text(&self) -> String {
        format!("{} {}", self.title_name(), self.title_count_label())
    }
}
