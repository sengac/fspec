//! RPC-054 — ProviderSettingsView (revision 2026-06-01).
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
use codelet_rpc_types::{ProfileDefinition, ProviderCredentialInfo};
use crossterm::event::{KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::collections::{HashMap, HashSet};

use crate::components::scroll_viewport::ensure_visible;
use crate::components::Action;
use crate::views::agent::confirm_dialog::ConfirmDialog;

mod body_render;
mod copy;
mod detail;
pub mod footer_hints;
pub mod icons;
mod list;
mod list_actions;
mod list_nav_render;
mod mode;
mod mouse;
pub mod nav_item;
mod nav_tree_ops;
mod oauth_confirm;
mod oauth_copilot;
mod oauth_login;
mod oauth_login_render;
mod paste;
pub mod profile_form;
mod profile_form_context;
mod profile_form_parse;
mod profile_form_paste;
mod profile_form_render;
mod profile_form_streaming;
mod profile_form_submit;
pub mod profiles_config;
pub mod projection;
pub mod refresh;
pub mod row_render;
mod row_segments;
mod settings_config;
mod status_text;

pub(crate) use copy::mask_secret;
pub use mode::{DetailSub, ProviderSettingsMode};
pub use nav_item::{NavItem, NavItemKind, OAuthMethod, ProviderDisplayInfo};

pub const DELETE_PROVIDER_CREDS_DIALOG_ID: &str = "delete-provider-creds";

#[derive(Debug, Clone)]
pub enum ProviderSettingsEvent {
    Consumed,
    Ignored,
    /// Boxed — see `BlocklistEvent::Emit` for the `large_enum_variant`
    /// rationale.
    Emit(Box<Action>),
    Close,
    /// RPC-160: list-mode Tab keybind — Navigator translates it to the
    /// model-settings transition (pure UI navigation, no Action payload).
    SwitchToModels,
}

pub struct ProviderSettingsView {
    pub providers: Vec<ProviderCredentialInfo>,
    pub display_providers: Vec<ProviderDisplayInfo>,
    pub expanded: HashSet<String>,
    pub nav_items: Vec<NavItem>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub mode: ProviderSettingsMode,
    pub filter: String,
    pub filter_mode: bool,
    pub delete_confirm: Option<ConfirmDialog>,
    pub status: String,
    pub profile_configs: HashMap<(String, String), ProfileDefinition>,
    pub provider_configs: HashMap<String, ProfileDefinition>,
    /// PROV-111: pending delete-confirm target; when set the Primary arm emits `ConfirmDeleteProfile`.
    pub(crate) pending_profile_delete: Option<(String, String)>,
    /// BUG-575: refresh numbering, load states and the navigation epoch.
    pub(crate) refresh: refresh::RefreshState,
    /// PROV-113: the login number; mismatched results are dropped. BUG-575: each
    /// view and login start takes a new one (process-wide); Esc moves past it.
    pub(crate) oauth_generation: u64,
    /// PROV-113: last login (provider, method) so the OAuthError screen retries it.
    pub(crate) oauth_last_provider: Option<String>,
    pub(crate) oauth_last_method: Option<OAuthMethod>,
    pub(crate) oauth_tasks: Vec<tokio::task::AbortHandle>,
    pub(crate) oauth_browser_url: Option<String>,
    visible_rows: usize,
    /// RPC-353: mouse-wheel 1×–5× velocity accelerator (shared chat-view ramp).
    wheel: crate::components::scroll_viewport::WheelVelocity,
    /// TUI-101: scrollbar click-and-drag state machine.
    scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag,
    /// TUI-101: cached scrollbar rect from last render for hit-testing.
    last_scrollbar_rect: Option<Rect>,
}

impl Default for ProviderSettingsView {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderSettingsView {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            display_providers: Vec::new(),
            expanded: HashSet::new(),
            nav_items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            mode: ProviderSettingsMode::List,
            filter: String::new(),
            filter_mode: false,
            delete_confirm: None,
            status: String::new(),
            profile_configs: HashMap::new(),
            provider_configs: HashMap::new(),
            pending_profile_delete: None,
            refresh: refresh::RefreshState::default(),
            oauth_generation: refresh::fresh_oauth_generation(),
            oauth_last_provider: None,
            oauth_last_method: None,
            oauth_tasks: Vec::new(),
            oauth_browser_url: None,
            visible_rows: 18,
            wheel: crate::components::scroll_viewport::WheelVelocity::new(),
            scrollbar_drag: crate::mouse::scrollbar_drag::ScrollbarDrag::new(),
            last_scrollbar_rect: None,
        }
    }

    pub fn set_providers(&mut self, providers: Vec<ProviderCredentialInfo>) {
        // A provider-space clamp, only while there is no nav tree (BUG-575:
        // a folded tree clamps in nav space, in `restore_focus`).
        let max = providers.len().saturating_sub(1);
        if self.nav_items.is_empty() && !providers.is_empty() && self.selected_index > max {
            self.selected_index = max;
        }
        self.providers = providers;
    }

    pub fn focused_provider(&self) -> Option<&ProviderCredentialInfo> {
        self.providers.get(self.selected_index)
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    /// PROV-113: current login generation (bumped on Esc-cancel); results with
    /// a mismatched generation are dropped as stale.
    pub fn oauth_generation(&self) -> u64 {
        self.oauth_generation
    }

    pub fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows.max(1);
    }

    pub fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    pub fn title_text(&self) -> String {
        // RPC-105: count is the flat NavItem tree length (mirrors TS navItems).
        format!("Provider Settings ({} items)", self.nav_items.len())
    }

    pub fn footer_hint(&self) -> String {
        // RPC-106 / PROV-110: mode-sensitive dispatch lives in footer_hints.rs.
        footer_hints::compute_footer_hint(self)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent {
        if self.delete_confirm.is_some() {
            return self.handle_delete_confirm_key(key);
        }
        // PROV-138: Ctrl+C in a text-entry mode copies the focused field.
        if let Some(ev) = copy::intercept_ctrl_c(self, key) {
            return ev;
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ProviderSettingsEvent::Consumed;
        }
        match self.mode.clone() {
            ProviderSettingsMode::List => list::handle_list_key_tracked(self, key),
            ProviderSettingsMode::Detail { provider_id, sub } => {
                detail::handle_detail_key(self, key, provider_id, sub)
            }
            ProviderSettingsMode::CreateProfile { provider_id, form }
            | ProviderSettingsMode::EditSettings { provider_id, form } => {
                profile_form::handle_form_key(self, key, provider_id, form, None)
            }
            ProviderSettingsMode::EditProfile {
                provider_id,
                profile_name,
                form,
            } => profile_form::handle_form_key(self, key, provider_id, form, Some(profile_name)),
            ProviderSettingsMode::DisconnectOAuth { provider_id } => {
                oauth_confirm::handle_disconnect_oauth_key(self, key, provider_id)
            }
            mode @ (ProviderSettingsMode::OAuthBrowserWaiting { .. }
            | ProviderSettingsMode::OAuthDeviceWaiting { .. }
            | ProviderSettingsMode::OAuthHeadlessCodeEntry { .. }
            | ProviderSettingsMode::OAuthSuccess { .. }
            | ProviderSettingsMode::OAuthError { .. }) => {
                oauth_login::handle_oauth_login_key(self, key, mode)
            }
            // PROV-114: the github-copilot deployment-type / enterprise-host
            // preamble modes route to their own sibling handler.
            mode @ (ProviderSettingsMode::OAuthDeploymentTypeSelect { .. }
            | ProviderSettingsMode::OAuthEnterpriseUrlEntry { .. }) => {
                oauth_copilot::handle_copilot_preamble_key(self, key, mode)
            }
        }
    }

    /// PROV-137: bracketed-paste sink (see `paste::handle_paste`).
    pub fn handle_paste(&mut self, text: &str) -> ProviderSettingsEvent {
        paste::handle_paste(self, text)
    }
    pub fn reset_to_list(&mut self) {
        self.mode = ProviderSettingsMode::List;
        self.delete_confirm = None;
        self.pending_profile_delete = None;
        self.status.clear();
        self.filter.clear();
        self.filter_mode = false;
        // RPC-105: keep nav_items in sync with the cleared filter so the
        // header count reflects the full provider list immediately.
        self.rebuild_nav_items();
    }

    pub(crate) fn adjust_scroll(&mut self) {
        let total = self.nav_len();
        ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            self.visible_rows,
            total,
        );
    }

    pub(crate) fn move_clamped(&mut self, delta: i32) {
        let total = self.nav_len();
        if total == 0 {
            return;
        }
        let max_idx = (total - 1) as i32;
        let current = self.selected_index as i32;
        let new_idx = (current + delta).clamp(0, max_idx);
        self.selected_index = new_idx as usize;
        self.adjust_scroll();
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // RPC-337 / RPC-350 R1: title-closure full-screen scaffold (two-span title, shared blue title).
        let title = "Provider Settings";
        let count = self.nav_items.len();
        let footer = self.footer_hint();
        let overlay = self.delete_confirm.clone();
        crate::views::full_screen_shell::render_full_screen_scaffold_with_title(
            area,
            buf,
            |title_area, buf| {
                crate::views::agent::mode_view_render::render_two_span_title(
                    title_area, buf, title, count, "items",
                );
            },
            &footer,
            |body_area, buf| {
                body_render::render_mode_body(self, body_area, buf);
            },
            overlay.as_ref(),
        );
    }
}
