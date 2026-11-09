//! BUG-576: a visit starts from a clean list, every list is numbered when it
//! is requested and only the newest one is applied, and a reload keeps the
//! expanded sections and the cursor.
//!
//! Feature: spec/features/model-selector-keeps-its-place-across-loads.feature

use super::*;

impl ModelSelectorView {
    /// Start a visit from a clean list: no rows, filter, custom model form,
    /// refresh mark or error from an earlier visit. The first list of the
    /// visit applies the collapse default. List numbers keep counting, so a
    /// list requested on an earlier visit is dropped.
    pub fn reset_for_open(&mut self) {
        self.providers.clear();
        self.rows.clear();
        self.expanded.clear();
        self.selected_index = 0;
        self.has_selection = false;
        self.scroll_offset = 0;
        self.filter.clear();
        self.filter_mode = false;
        self.is_refreshing = false;
        self.loaded = false;
        self.custom_model_mode = CustomModelMode::Browse;
        self.form = CustomModelForm::default();
        self.write_error = None;
        self.load_error = None;
        self.focus_after_load = None;
        self.opening = true;
    }

    /// Number a new provider list request; only the newest number is applied.
    pub fn request_list(&mut self) -> u64 {
        self.newest_list += 1;
        self.newest_list
    }

    /// Apply the list numbered `number` when it is the newest one requested.
    pub fn apply_list(&mut self, number: u64, providers: Vec<ProviderInfo>) {
        if number == self.newest_list {
            self.set_providers(providers);
        }
    }

    /// The newest list requested failed: stop loading and refreshing, and
    /// say why.
    pub fn fail_list(&mut self, number: u64, error: String) {
        if number != self.newest_list {
            return;
        }
        self.loaded = true;
        self.is_refreshing = false;
        self.load_error = Some(error);
    }

    /// Why the newest list failed, if it did.
    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    /// The errors shown above the list: the last failed custom-model write
    /// (BUG-325) and the failed list (BUG-576).
    pub(super) fn error_lines(&self) -> Vec<String> {
        let write = self.write_error.iter().map(|e| format!("[error] {e}"));
        let load = self
            .load_error
            .iter()
            .map(|e| format!("[error] Models could not be loaded: {e}"));
        write.chain(load).collect()
    }

    /// The text of an empty list: loading, no models, or nothing when the
    /// list failed (its error is shown above the list).
    pub(super) fn placeholder(&self) -> Option<String> {
        match (self.loaded, self.load_error.is_some()) {
            (_, true) => None,
            // BUG-578 F23: an empty list under a filter says the filter
            // matched nothing, not that there are no models.
            (true, false) if !self.filter.is_empty() => {
                Some(format!("No models match '{}'", self.filter))
            }
            (true, false) => Some(rows::EMPTY_PLACEHOLDER.to_string()),
            (false, false) => Some(rows::LOADING_PLACEHOLDER.to_string()),
        }
    }

    /// After a custom-model write, the next list opens `section` and puts
    /// the cursor on `model` (or on the section when it has none).
    pub(crate) fn focus_after_load(&mut self, section: String, model: Option<String>) {
        self.focus_after_load = Some((section, model));
    }

    /// The section and model under the cursor (model empty on a header).
    pub(super) fn cursor_row(&self) -> Option<(String, String)> {
        self.rows
            .get(self.selected_index)
            .map(|row| (row.provider_key.clone(), row.model_id.clone()))
    }

    /// The expanded sections for a new list: on open only the current
    /// model's section; otherwise the open ones that still exist.
    pub(super) fn expanded_for(&self, providers: &[ProviderInfo]) -> HashSet<String> {
        if self.opening {
            return current_section(providers, self.current_model.as_ref())
                .into_iter()
                .collect();
        }
        self.expanded
            .iter()
            .filter(|key| providers.iter().any(|p| &p.key == *key))
            .cloned()
            .collect()
    }

    /// Put the cursor on `section`/`model`, else on `section`'s header,
    /// else keep it inside the list.
    pub(super) fn place_cursor(&mut self, section: &str, model: &str) {
        let exact = self
            .rows
            .iter()
            .position(|r| r.provider_key == section && r.model_id == model);
        let header = || {
            self.rows
                .iter()
                .position(|r| !r.selectable && r.provider_key == section)
        };
        if let Some(index) = exact.or_else(header) {
            self.selected_index = index;
        }
        self.clamp_cursor();
    }

    /// Keep the cursor on a row of the list.
    pub(super) fn clamp_cursor(&mut self) {
        self.selected_index = self.selected_index.min(self.rows.len().saturating_sub(1));
    }

    /// Open the current model's section and put the cursor on it; `false`
    /// when the list does not hold the current model.
    pub(super) fn seed_on_current(&mut self) -> bool {
        let Some(section) = current_section(&self.providers, self.current_model.as_ref()) else {
            return false;
        };
        self.expanded.insert(section);
        self.rebuild_rows();
        let current = self
            .current_model
            .as_ref()
            .map(|(p, m)| (p.as_str(), m.as_str()));
        match rows::index_of_model(&self.rows, current) {
            Some(index) => {
                self.selected_index = index;
                self.has_selection = true;
                true
            }
            None => false,
        }
    }
}

/// The section holding the current `(section, model)`.
fn current_section(
    providers: &[ProviderInfo],
    current: Option<&(String, String)>,
) -> Option<String> {
    let (section, model) = current?;
    providers
        .iter()
        .find(|p| {
            &p.key == section
                && p.models
                    .iter()
                    .any(|m| model_id::model_ids_match(&m.id, model))
        })
        .map(|p| p.key.clone())
}
