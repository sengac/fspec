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

mod crud;
mod dispatch;
pub(crate) mod form;
mod form_render;
mod header;
mod model_id;
mod navigation;
mod render;
mod rows;
mod state;

#[cfg(test)]
#[path = "scroll_tests.rs"]
mod scroll_tests;

use form::FormOutcome;
pub use form::{CustomModelForm, CustomModelMode};

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
    /// RPC-345: Tab keybind — pure UI navigation, no Action payload.
    /// The Navigator translates it to `Action::OpenProviderSettingsView`
    /// (→ `ViewMode::ProviderSettings`), the reciprocal of
    /// `ProviderSettingsEvent::SwitchToModels`. TS analog:
    /// `onSwitchToSettings()` in ModelSelectorScreen.tsx:145.
    SwitchToProviders,
}

/// Full-screen model selector mode-view state.
pub struct ModelSelectorView {
    session_id: Option<SessionId>,
    providers: Vec<ProviderInfo>,
    expanded: HashSet<String>,
    rows: Vec<crate::components::model_selector_dialog_rows::ModelSelectorRow>,
    selected_index: usize,
    /// PROV-101: whether `selected_index` currently points at an explicitly
    /// active selection. `false` means "nothing selected" — the cursor must
    /// NOT auto-snap to index 0, and Enter is a no-op. Set `true` when the
    /// current model seeds the cursor or when the user navigates explicitly.
    has_selection: bool,
    scroll_offset: usize,
    filter: String,
    filter_mode: bool,
    current_model_id: Option<String>,
    is_refreshing: bool,
    loaded: bool,
    visible_rows: usize,
    /// RPC-344: custom-model CRUD sub-mode (browse / add / edit /
    /// delete-confirm) and the in-progress form values.
    custom_model_mode: CustomModelMode,
    form: CustomModelForm,
}

impl Default for ModelSelectorView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "tests_core.rs"]
mod tests_core;

#[cfg(test)]
#[path = "tests_scroll.rs"]
mod tests_scroll;

#[cfg(test)]
#[path = "tests_current_model.rs"]
mod tests_current_model;

#[cfg(test)]
#[path = "tests_collapse.rs"]
mod tests_collapse;

#[cfg(test)]
#[path = "tests_tab.rs"]
mod tests_tab;

#[cfg(test)]
#[path = "tests_crud_add.rs"]
mod tests_crud_add;

#[cfg(test)]
#[path = "tests_crud_delete.rs"]
mod tests_crud_delete;

#[cfg(test)]
#[path = "tests_loading_empty.rs"]
mod tests_loading_empty;

#[cfg(test)]
#[path = "tests_enter_expand.rs"]
mod tests_enter_expand;
