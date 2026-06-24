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

use super::nav_item::{build_nav_items, NavItem, NavItemKind, OAuthMethod, ProviderDisplayInfo};
use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use crate::components::Action;
use crate::views::agent::confirm_dialog::ConfirmDialogOutcome;
use codelet_rpc_types::{ProfileDefinition, ProviderCredentialInfo};
use crossterm::event::KeyEvent;
use std::collections::HashMap;

impl ProviderSettingsView {
    /// Route a key through the open `delete_confirm` dialog. Extracted from
    /// `handle_key` (mod.rs) to keep that file under the 300-LoC ceiling.
    /// PROV-111: a pending per-profile delete takes priority and emits
    /// `ConfirmDeleteProfile`; otherwise the PROV-102 provider-credentials
    /// delete resolves by the focused NavItem's provider_id.
    pub(crate) fn handle_delete_confirm_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent {
        let outcome = match self.delete_confirm.as_mut() {
            Some(dialog) => dialog.handle_key(key.code, key.modifiers),
            None => return ProviderSettingsEvent::Consumed,
        };
        match outcome {
            ConfirmDialogOutcome::Primary => {
                if let Some((provider_id, profile_name)) = self.pending_profile_delete.take() {
                    self.delete_confirm = None;
                    return ProviderSettingsEvent::Emit(Action::ConfirmDeleteProfile {
                        provider_id,
                        profile_name,
                    });
                }
                let pid = self.delete_target_provider_id();
                self.delete_confirm = None;
                match pid {
                    Some(id) => {
                        ProviderSettingsEvent::Emit(Action::ConfirmDeleteProviderCredentials(id))
                    }
                    None => ProviderSettingsEvent::Consumed,
                }
            }
            ConfirmDialogOutcome::Secondary | ConfirmDialogOutcome::Cancel => {
                self.delete_confirm = None;
                self.pending_profile_delete = None;
                ProviderSettingsEvent::Consumed
            }
            ConfirmDialogOutcome::Continued | ConfirmDialogOutcome::Ignored => {
                ProviderSettingsEvent::Consumed
            }
        }
    }

    /// PROV-111: store the full per-profile ProfileConfig map (keyed by
    /// profile name) used to prefill the EditProfile form.
    pub fn set_profile_configs(&mut self, configs: HashMap<String, ProfileDefinition>) {
        self.profile_configs = configs;
    }

    /// PROV-111: look up the stored full ProfileConfig for a bare profile
    /// name (the EditProfile prefill source).
    pub fn profile_config_for(&self, name: &str) -> Option<&ProfileDefinition> {
        self.profile_configs.get(name)
    }

    /// RPC-103: the filtered provider ids (parent-anchored substring filter).
    pub fn visible_provider_ids(&self) -> Vec<String> {
        self.visible_providers()
            .iter()
            .map(|p| p.provider_id.clone())
            .collect()
    }

    /// RPC-103: providers surviving the case-insensitive `filter`.
    pub(crate) fn visible_providers(&self) -> Vec<&ProviderCredentialInfo> {
        if self.filter.is_empty() {
            return self.providers.iter().collect();
        }
        let lower = self.filter.to_lowercase();
        self.providers
            .iter()
            .filter(|p| {
                p.provider_id.to_lowercase().contains(&lower)
                    || p.display_name.to_lowercase().contains(&lower)
            })
            .collect()
    }

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
        let mut items = build_nav_items(&self.display_providers, &self.expanded, &self.filter);
        // PROV-113: gate the browser login rows to transports that can run the
        // providers-layer local HTTP server (embedded). When browser login is
        // disabled the Browser rows are dropped while the Headless/device rows
        // remain selectable (feature Rule 7).
        if !self.browser_login_enabled {
            items.retain(|item| {
                !matches!(
                    item.kind,
                    NavItemKind::OAuthLogin {
                        method: OAuthMethod::Browser,
                        ..
                    }
                )
            });
        }
        self.nav_items = items;
    }

    /// Returns the currently-focused NavItem from `nav_items`, or
    /// `None` when the nav list is empty or selected_index is out of
    /// bounds.
    pub fn focused_nav_item(&self) -> Option<&NavItem> {
        self.nav_items.get(self.selected_index)
    }

    /// PROV-112: record the provider whose row the next
    /// `ProviderCredentialsLoaded` reload should re-focus (TS
    /// `navigateToProviderRef`). Consumed by `apply_pending_navigate`.
    pub fn set_navigate_target(&mut self, provider_id: impl Into<String>) {
        self.pending_navigate_provider = Some(provider_id.into());
    }

    /// PROV-112: after a nav rebuild, move the cursor onto the pending
    /// navigate target's PROVIDER row (PROV-036 parity: a disconnect/delete
    /// returns focus to the parent provider row instead of stranding the
    /// cursor on a now-removed child row). No-op when no target is pending or
    /// the provider is no longer present.
    pub(crate) fn apply_pending_navigate(&mut self) {
        let Some(target) = self.pending_navigate_provider.take() else {
            return;
        };
        if let Some(idx) = self
            .nav_items
            .iter()
            .position(|i| matches!(i.kind, NavItemKind::Provider { .. }) && i.provider_id == target)
        {
            self.selected_index = idx;
            self.adjust_scroll();
        }
    }

    /// PROV-103 — Length of the list the cursor actually navigates and the
    /// body actually renders. Prefer the flat `nav_items` tree (the full
    /// expanded list incl. profile / api-key / oauth child rows) whenever
    /// it is populated; fall back to the legacy top-level
    /// `visible_providers()` count ONLY for pre-RPC-103 callers that drive
    /// the view via `set_providers` alone (nav_items stays empty).
    ///
    /// Mirrors the TS contract where Up/Down clamp by `navItems.length`
    /// (listModeHandler.ts) — every nav row is a valid landing target, so
    /// no header-skip is needed here (unlike `ModelSelectorView`, whose
    /// provider HEADER rows are non-selectable). Per PROV-101 this never
    /// introduces a silent selection fallback: when the count is 0,
    /// `move_clamped` is a no-op rather than snapping to row 0.
    pub(crate) fn nav_len(&self) -> usize {
        if self.nav_items.is_empty() {
            self.visible_providers().len()
        } else {
            self.nav_items.len()
        }
    }

    /// PROV-102 — Resolve the provider whose credentials a confirmed delete
    /// should target. In Detail mode the mode already carries the
    /// provider_id. In List mode prefer the focused NavItem's own
    /// provider_id (the flat tree the cursor actually navigates); fall back
    /// to the legacy `focused_provider()` only when `nav_items` is empty
    /// (pre-RPC-103 `set_providers`-only callers). This keeps the delete
    /// target from ever being re-derived by indexing the provider list with
    /// a `nav_items`-space `selected_index`.
    pub(crate) fn delete_target_provider_id(&self) -> Option<String> {
        match &self.mode {
            ProviderSettingsMode::Detail { provider_id, .. }
            | ProviderSettingsMode::CreateProfile { provider_id, .. }
            | ProviderSettingsMode::EditProfile { provider_id, .. }
            | ProviderSettingsMode::DisconnectOAuth { provider_id } => Some(provider_id.clone()),
            // PROV-113: the OAuth login modes never open the delete-confirm, so
            // they resolve to no delete target.
            // PROV-114: the copilot preamble modes likewise never open the
            // delete-confirm (testing stub — no delete target).
            ProviderSettingsMode::OAuthBrowserWaiting { .. }
            | ProviderSettingsMode::OAuthDeviceWaiting { .. }
            | ProviderSettingsMode::OAuthHeadlessCodeEntry { .. }
            | ProviderSettingsMode::OAuthSuccess { .. }
            | ProviderSettingsMode::OAuthError { .. }
            | ProviderSettingsMode::OAuthDeploymentTypeSelect { .. }
            | ProviderSettingsMode::OAuthEnterpriseUrlEntry { .. } => None,
            ProviderSettingsMode::List => self
                .focused_nav_item()
                .map(|item| item.provider_id.clone())
                .or_else(|| self.focused_provider().map(|p| p.provider_id.clone())),
        }
    }
}
