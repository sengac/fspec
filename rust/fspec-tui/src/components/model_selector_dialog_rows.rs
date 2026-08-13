//! Helpers extracted from the (now retired) RPC-022 model-selector dialog.
//! The flat `ModelSelectorRow` projection plus the header-skipping navigation
//! helpers are reused by the live full-screen `views/model_selector/` mode-view.
//!
//! Feature: spec/features/model-selector-no-auto-select.feature
//!
//! PROV-101 FIX 2: the dead RPC-022 `ModelSelectorDialog` (and its index-0
//! `page_step_selectable` / `build_dialog_rows` selection fallbacks) was deleted
//! so the fallback cannot resurface. Only the row projection + navigation
//! helpers used by the live mode-view remain here.

/// One displayable row in the selector. Provider headers are not
/// selectable (`selectable = false`); model rows are.
///
/// RPC-337: re-scoped from `pub(super)` to `pub(crate)` so the new
/// full-screen `views/model_selector/` mode-view can reuse the row
/// projection + header-skipping navigation helpers.
#[derive(Debug, Clone)]
pub(crate) struct ModelSelectorRow {
    /// Render text for the row (without the leading `▸ ` marker).
    pub(crate) label: String,
    /// Capability/context badge suffix (e.g. `" [R] [V] [200k]"`),
    /// rendered DIM on unselected rows.
    pub(crate) badges: String,
    /// True for model rows (Enter emits ModelSelected); false for
    /// provider headers.
    pub(crate) selectable: bool,
    /// Provider key — populated only when `selectable` is true.
    pub(crate) provider_key: String,
    /// Model id — populated only when `selectable` is true.
    pub(crate) model_id: String,
    /// RPC-338: true for a local-server profile section header (drives the
    /// magenta 📁 icon). Header rows only; always false for model rows.
    pub(crate) is_profile: bool,
    /// RPC-338: true when an (unreachable) profile header (drives the red
    /// `(unreachable)` marker). Header rows only.
    pub(crate) is_unreachable: bool,
    /// RPC-344: true for a selectable custom-model row (drives the `[C]`
    /// badge AND gates the `e`/`d` keybinds). Always false for headers.
    pub(crate) is_custom: bool,
    /// RPC-344: the profile name of the section this row belongs to, when it
    /// is a local-server profile section. Set on BOTH the profile header and
    /// its model rows so the a/e/d guards can read it off the focused row.
    pub(crate) profile_name: Option<String>,
    /// RPC-344: context window of a model row (0 for headers) — used to
    /// prefill the edit form without re-walking providers.
    pub(crate) context_window: u32,
    /// RPC-344: reasoning capability of a model row — edit-form prefill.
    pub(crate) supports_reasoning: bool,
    /// RPC-344: vision capability of a model row — edit-form prefill.
    pub(crate) supports_vision: bool,
}

/// PROV-104 (TS parity): move the cursor up by one row over the FULL flat
/// row list INCLUDING provider headers, CLAMPED to `[0, len-1]` with NO
/// wrap-around. Mirrors `useModelSelectorState.ts` `navigateUp`
/// (`Math.max(currentIdx - 1, 0)`). The cursor may rest on a non-selectable
/// header so the user can press Right/Enter to expand it. Returns `None`
/// when there are no rows.
pub(crate) fn move_up_clamped(rows: &[ModelSelectorRow], current: usize) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    Some(current.saturating_sub(1))
}

/// PROV-104 (TS parity): move the cursor down by one row over the FULL flat
/// row list INCLUDING provider headers, CLAMPED to `[0, len-1]` with NO
/// wrap-around. Mirrors `useModelSelectorState.ts` `navigateDown`
/// (`Math.min(currentIdx + 1, filteredFlatItems.length - 1)`).
pub(crate) fn move_down_clamped(rows: &[ModelSelectorRow], current: usize) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    Some((current + 1).min(rows.len() - 1))
}

/// First selectable row index (Home).
pub(crate) fn first_selectable(rows: &[ModelSelectorRow]) -> usize {
    rows.iter().position(|r| r.selectable).unwrap_or(0)
}

/// Last selectable row index (End).
pub(crate) fn last_selectable(rows: &[ModelSelectorRow]) -> usize {
    rows.iter()
        .rposition(|r| r.selectable)
        .unwrap_or(rows.len().saturating_sub(1))
}
