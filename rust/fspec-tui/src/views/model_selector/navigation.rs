//! Navigation: cursor movement, paging and provider expand/collapse.
//!
//! Extracted from `mod.rs` (PROV-107) to keep that file under the
//! 300-LoC ceiling. Behaviour-preserving move of `impl ModelSelectorView`
//! methods; field/method visibility unchanged.

use super::*;

impl ModelSelectorView {
    pub(crate) fn focused_provider_key(&self) -> Option<String> {
        self.rows
            .get(self.selected_index)
            .map(|r| r.provider_key.clone())
    }

    pub(crate) fn move_up(&mut self) {
        // PROV-124: `has_selection` gates Enter ONLY. The first explicit
        // navigation from a "nothing selected" state must ACTIVATE the
        // selection AND perform the clamped move on the SAME press (no
        // swallowed first press). `anchor_first_selectable` is reserved for
        // the Home/filter paths.
        self.has_selection = true;
        if let Some(next) = crate::components::model_selector_dialog_rows::move_up_clamped(
            &self.rows,
            self.selected_index,
        ) {
            self.selected_index = next;
        }
        self.adjust_scroll();
    }

    pub(crate) fn move_down(&mut self) {
        // PROV-124: activate the selection and move on the same first press;
        // `has_selection` gates Enter only.
        self.has_selection = true;
        if let Some(next) = crate::components::model_selector_dialog_rows::move_down_clamped(
            &self.rows,
            self.selected_index,
        ) {
            self.selected_index = next;
        }
        self.adjust_scroll();
    }

    /// PageDown / PageUp: move the selection by one viewport height across
    /// selectable rows (skipping headers), matching the TS navigate-by-page
    /// semantics. PROV-124: the first explicit navigation from a "nothing
    /// selected" state activates the selection AND pages on the same press;
    /// `has_selection` gates Enter only.
    pub(crate) fn page_down(&mut self) {
        self.has_selection = true;
        let step = self.visible_rows.max(1);
        for _ in 0..step {
            match crate::components::model_selector_dialog_rows::move_down_clamped(
                &self.rows,
                self.selected_index,
            ) {
                Some(next) => self.selected_index = next,
                None => break,
            }
        }
        self.adjust_scroll();
    }

    pub(crate) fn page_up(&mut self) {
        // PROV-124: activate the selection and page on the same first press;
        // `has_selection` gates Enter only.
        self.has_selection = true;
        let step = self.visible_rows.max(1);
        for _ in 0..step {
            match crate::components::model_selector_dialog_rows::move_up_clamped(
                &self.rows,
                self.selected_index,
            ) {
                Some(next) => self.selected_index = next,
                None => break,
            }
        }
        self.adjust_scroll();
    }

    pub(crate) fn toggle_expansion(&mut self, expand: bool) {
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
                // Explicit expand/collapse: anchor on a selectable row.
                self.anchor_first_selectable();
            }
            self.adjust_scroll();
        }
    }
}
