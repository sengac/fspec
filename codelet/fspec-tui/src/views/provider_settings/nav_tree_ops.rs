//! RPC-103 — Flat tree nav-mechanics methods on `ProviderSettingsView`.
//!
//! Feature: spec/features/rpc103-provider-settings-flat-tree-nav-model.feature
//!
//! Extracted from `mod.rs` to keep that file under the 300-LoC ceiling
//! (RPC-054 source-shape contract). This module owns the public
//! `set_provider_display_infos`, `toggle_expansion`, `rebuild_nav_items`
//! and `focused_nav_item` methods that drive the flat NavItem tree
//! introduced by RPC-103. All methods are pure with respect to the
//! supplied inputs — they only mutate `display_providers`, `expanded`
//! and `nav_items` fields and never touch `selected_index` (Rule 3).

use super::nav_item::{build_nav_items, NavItem, ProviderDisplayInfo};
use super::ProviderSettingsView;

impl ProviderSettingsView {
    /// Replace the cached `display_providers` and rebuild `nav_items`.
    /// Crucially, this **does NOT** clear the `expanded` set — the
    /// HashSet survives reload() per Rule 2, mirroring the TS
    /// `useRef<Set<string>> expandedProviderIds` lifetime.
    pub fn set_provider_display_infos(&mut self, infos: Vec<ProviderDisplayInfo>) {
        self.display_providers = infos;
        self.rebuild_nav_items();
    }

    /// Flip membership of `provider_id` in the `expanded` set and
    /// rebuild `nav_items`. Selected_index is intentionally left
    /// untouched (Rule 3) — the cursor stays on the same provider row
    /// while children appear/disappear below it.
    pub fn toggle_expansion(&mut self, provider_id: &str) {
        if self.expanded.contains(provider_id) {
            self.expanded.remove(provider_id);
        } else {
            self.expanded.insert(provider_id.to_string());
        }
        self.rebuild_nav_items();
    }

    /// Recompute `nav_items` from the current `display_providers`,
    /// `expanded` set, and `filter`. Called whenever any of the three
    /// inputs change. Mirrors the TS `useMemo([providers, filter])`
    /// reactive dependency at useProviderSettingsState.ts:361-364.
    pub fn rebuild_nav_items(&mut self) {
        self.nav_items = build_nav_items(&self.display_providers, &self.expanded, &self.filter);
    }

    /// Returns the currently-focused NavItem from `nav_items`, or
    /// `None` when the nav list is empty or selected_index is out of
    /// bounds.
    pub fn focused_nav_item(&self) -> Option<&NavItem> {
        self.nav_items.get(self.selected_index)
    }
}
