//! State: construction, accessors, row projection & scroll reconciliation.
//!
//! Extracted from `mod.rs` (PROV-107) to keep that file under the
//! 300-LoC ceiling. Behaviour-preserving move of `impl ModelSelectorView`
//! methods; field/method visibility unchanged.

use super::*;

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
            current_model_id: None,
            is_refreshing: false,
            loaded: false,
            visible_rows: 12,
            custom_model_mode: CustomModelMode::Browse,
            form: CustomModelForm::default(),
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
            if let Some(p) = providers.iter().find(|p| {
                p.models
                    .iter()
                    .any(|m| model_id::model_ids_match(&m.id, current))
            }) {
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
        // RPC-341 / PROV-101: seed the cursor on the active-session model when
        // it is present. When there is NO current model (or it is not loaded),
        // do NOT auto-snap a selection to the first selectable row — surface a
        // "nothing selected" state instead (has_selection stays false). Enter
        // is a no-op until the user navigates explicitly.
        if let Some(idx) = rows::index_of_model(&self.rows, self.current_model_id.as_deref()) {
            self.selected_index = idx;
            self.has_selection = true;
        } else {
            self.has_selection = false;
        }
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

    pub fn title_text(&self) -> String {
        let suffix = if self.is_refreshing {
            " (refreshing...)"
        } else {
            ""
        };
        format!("Select Model ({} models){suffix}", self.total_model_count())
    }
}
