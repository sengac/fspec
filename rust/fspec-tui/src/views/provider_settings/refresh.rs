//! BUG-575: one numbered refresh of the provider tree.
//!
//! Feature: spec/features/provider-settings-loads-and-writes-never-corrupt-the-view.feature
//!
//! The credentials and the settings are read in one task and shown
//! together, so Settings and profile rows never appear before their data
//! (product owner, 2026-11-07: no settings form opens before that
//! provider's settings have loaded). The App numbers each refresh when it is
//! requested; the view shows a refresh only when it is newer than the one
//! shown. A write that moves the cursor (logout, sign-in, profile save or
//! delete) puts its target in its own refresh, and the target holds only
//! while the user has not moved the cursor since the write began.

use codelet_rpc_types::{ProviderCredentialInfo, ProviderSettingsConfig};

use super::nav_item::{NavItem, NavItemKind};
use super::ProviderSettingsView;

/// Where a write returns the cursor once its refresh is shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigateTarget {
    /// The provider's own row.
    Provider(String),
    /// A profile row, by the profile's name.
    Profile { provider_id: String, name: String },
}

/// A write's cursor target and the navigation epoch when the write began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigateRequest {
    pub target: NavigateTarget,
    pub nav_epoch: u64,
}

/// What one refresh read.
#[derive(Debug, Clone)]
pub struct ProviderSettingsRefresh {
    pub generation: u64,
    pub credentials: Result<Vec<ProviderCredentialInfo>, String>,
    pub configs: Result<Vec<ProviderSettingsConfig>, String>,
    pub navigate: Option<NavigateRequest>,
}

/// Whether a read has answered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LoadState {
    #[default]
    Loading,
    Loaded,
    Failed(String),
}

/// The view's load bookkeeping.
#[derive(Debug, Default)]
pub struct RefreshState {
    /// The newest refresh shown (or the floor set when the view opened).
    shown: u64,
    credentials: LoadState,
    settings: LoadState,
    /// Bumped whenever the user moves the cursor.
    nav_epoch: u64,
}

impl ProviderSettingsView {
    /// Ignore every refresh numbered `floor` or lower (requested before
    /// this view opened, or before a list shown outside the numbering).
    pub(crate) fn refresh_floor(&mut self, floor: u64) {
        self.refresh.shown = self.refresh.shown.max(floor);
    }

    /// The number of the newest refresh shown.
    pub fn shown_refresh(&self) -> u64 {
        self.refresh.shown
    }

    /// The navigation epoch: a write captures it when it begins.
    pub fn nav_epoch(&self) -> u64 {
        self.refresh.nav_epoch
    }

    /// The user moved the cursor: a pending return to a row is dropped.
    pub(crate) fn note_navigation(&mut self) {
        self.refresh.nav_epoch = self.refresh.nav_epoch.wrapping_add(1);
    }

    /// Whether the settings read has answered successfully.
    pub fn settings_loaded(&self) -> bool {
        self.refresh.settings == LoadState::Loaded
    }

    pub(super) fn mark_settings_loaded(&mut self) {
        self.refresh.settings = LoadState::Loaded;
    }

    /// Why the Settings form cannot open yet, if it cannot.
    pub(super) fn settings_not_ready(&self) -> Option<String> {
        match &self.refresh.settings {
            LoadState::Loaded => None,
            LoadState::Loading => Some("Settings are still loading…".to_string()),
            LoadState::Failed(error) => Some(settings_failed(error)),
        }
    }

    /// What the list says when it has no rows.
    pub(super) fn empty_list_text(&self) -> String {
        match &self.refresh.credentials {
            LoadState::Loading => "Loading providers…".to_string(),
            LoadState::Failed(error) => providers_failed(error),
            LoadState::Loaded => "(no providers configured)".to_string(),
        }
    }

    /// Show `refresh` unless a newer one is already shown. Returns whether
    /// it was shown.
    pub fn apply_refresh(&mut self, refresh: ProviderSettingsRefresh) -> bool {
        if refresh.generation <= self.refresh.shown {
            return false;
        }
        self.refresh.shown = refresh.generation;
        let focused = self.focused_nav_item().cloned();
        match refresh.configs {
            Ok(configs) => {
                self.store_settings_configs(configs);
                self.refresh.settings = LoadState::Loaded;
            }
            Err(error) => {
                self.status = settings_failed(&error);
                self.refresh.settings = LoadState::Failed(error);
            }
        }
        match refresh.credentials {
            Ok(list) => {
                self.refresh.credentials = LoadState::Loaded;
                self.fold_provider_credentials(list);
            }
            Err(error) => {
                if !self.nav_items.is_empty() {
                    self.status = providers_failed(&error);
                }
                self.refresh.credentials = LoadState::Failed(error);
                self.relabel_profiles();
                self.rebuild_nav_items();
            }
        }
        self.restore_focus(focused);
        if let Some(navigate) = refresh.navigate {
            self.navigate_unless_moved(&navigate);
        }
        true
    }

    /// Return the cursor to the request's row, unless the user moved it
    /// since the write began.
    fn navigate_unless_moved(&mut self, navigate: &NavigateRequest) {
        if navigate.nav_epoch == self.refresh.nav_epoch {
            self.navigate_to(&navigate.target);
        }
    }

    /// After a rebuild: keep the cursor on the same row when it is still
    /// there, and inside the tree either way.
    pub(crate) fn restore_focus(&mut self, focused: Option<NavItem>) {
        if let Some(index) = focused.as_ref().and_then(|focused| {
            self.nav_items
                .iter()
                .position(|item| same_row(item, focused))
        }) {
            self.selected_index = index;
        }
        let last = self.nav_len().saturating_sub(1);
        self.selected_index = self.selected_index.min(last);
        self.scroll_offset = self.scroll_offset.min(last);
        self.adjust_scroll();
    }

    fn navigate_to(&mut self, target: &NavigateTarget) {
        let found = self.nav_items.iter().position(|item| match target {
            NavigateTarget::Provider(provider_id) => {
                &item.provider_id == provider_id
                    && matches!(item.kind, NavItemKind::Provider { .. })
            }
            NavigateTarget::Profile { provider_id, name } => {
                &item.provider_id == provider_id
                    && matches!(&item.kind, NavItemKind::Profile { profile_name }
                        if &super::list_actions::bare_profile_name(profile_name) == name)
            }
        });
        if let Some(index) = found {
            self.selected_index = index;
            self.adjust_scroll();
        }
    }
}

/// BUG-575: a login number never handed out before in this process, so a
/// result from a cancelled login or an earlier visit to /provider never
/// matches the current one.
pub(crate) fn fresh_oauth_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn settings_failed(error: &str) -> String {
    format!("✗ Settings could not be loaded: {error}")
}

fn providers_failed(error: &str) -> String {
    format!("✗ Providers could not be loaded: {error}")
}

/// The same row across a rebuild: same provider and kind, and for a profile
/// the same name (its label changes with its base URL and missing limits).
fn same_row(item: &NavItem, focused: &NavItem) -> bool {
    if item.provider_id != focused.provider_id {
        return false;
    }
    match (&item.kind, &focused.kind) {
        (NavItemKind::Profile { profile_name: a }, NavItemKind::Profile { profile_name: b }) => {
            super::list_actions::bare_profile_name(a) == super::list_actions::bare_profile_name(b)
        }
        (NavItemKind::Provider { .. }, NavItemKind::Provider { .. }) => true,
        (a, b) => a == b,
    }
}
