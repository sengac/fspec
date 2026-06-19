//! RPC-054 — ProviderSettingsView (revision 2026-06-01).
//!
//! Feature: spec/features/rpc054-provider-settings-view.feature
//!
//! Full-screen mode-view that ports the TS `ProviderSettingsScreen`
//! UX onto the Rust ratatui frontend. Mirrors the canonical
//! RPC-026 `ResumeSessionView` pattern: `Clear.render` first, then a
//! 4-constraint vertical Layout for title / separator / body / footer.
//! Destructive `d` opens a `ConfirmDialog` overlay BEFORE any backend
//! round-trip fires (`Action::ConfirmDeleteProviderCredentials`).

use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::collections::HashSet;

use crate::components::scroll_viewport::ensure_visible;
use crate::components::Action;
use crate::views::agent::confirm_dialog::{ConfirmDialog, ConfirmDialogOutcome};
use crate::views::agent::slash_commands::SlashCommandAction;

mod detail;
pub mod footer_hints;
pub mod icons;
mod list;
mod list_nav_render;
pub mod nav_item;
mod nav_tree_ops;
pub mod row_render;
mod status_text;
mod test_result;

pub use nav_item::{NavItem, NavItemKind, OAuthMethod, ProviderDisplayInfo};
pub use status_text::DetailStatus;
pub use test_result::{ProviderTestResult, ProviderTestStatus};

pub const DELETE_PROVIDER_CREDS_DIALOG_ID: &str = "delete-provider-creds";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProviderSettingsMode {
    #[default]
    List,
    Detail {
        provider_id: String,
        sub: DetailSub,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailSub {
    Summary { last_status: Option<DetailStatus> },
    EditApiKey { draft: String },
    OAuthNotice,
}

#[derive(Debug, Clone)]
pub enum ProviderSettingsEvent {
    Consumed,
    Ignored,
    Emit(Action),
    Close,
    /// RPC-160: list-mode Tab keybind emits this variant — distinct from
    /// `Close` and `Emit(Action)`. The Navigator translates it to the
    /// model-settings view transition (TS analog:
    /// `onSwitchToModels()` callback in
    /// src/tui/inputHandlers/listModeHandler.ts lines 56-60). Pure UI
    /// navigation event — no Action payload.
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
    /// RPC-158: inline test-result decoration (see `test_result.rs`).
    pub test_result: Option<ProviderTestResult>,
    visible_rows: usize,
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
            test_result: None,
            visible_rows: 18,
        }
    }

    pub fn set_providers(&mut self, providers: Vec<ProviderCredentialInfo>) {
        let max = providers.len().saturating_sub(1);
        if !providers.is_empty() && self.selected_index > max {
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

    pub fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows.max(1);
    }

    pub fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    pub fn title_text(&self) -> String {
        // RPC-105: count is the length of the flat NavItem tree — every
        // visible row the cursor can navigate to, including expanded
        // children and shrunk by the parent-anchored filter. Mirrors TS
        // ProviderSettingsPanel.tsx (`navItems.length`). The previous
        // `configured_count()` method has been removed (no callers).
        format!("Provider Settings ({} items)", self.nav_items.len())
    }

    pub fn footer_hint(&self) -> String {
        // RPC-106: Footer hint is context-sensitive on the currently-
        // focused NavItem kind in List mode, matching the TS
        // `getFooterHints(itemType)` dispatch. Detail-mode hints keep
        // their dedicated strings but adopt the bullet (`·`) + lowercase-
        // colon style for visual consistency. Summary hint will be
        // dropped wholesale by RPC-162 along with Detail::Summary itself.
        match &self.mode {
            ProviderSettingsMode::List => {
                footer_hints::footer_hint_for(footer_hints::focused_row_kind(self))
            }
            ProviderSettingsMode::Detail { sub, .. } => match sub {
                DetailSub::Summary { .. } => {
                    // RPC-154 — drop `t: test ·` (TS binds no `t` for
                    // the test-connection action). Hint now matches
                    // the actually-bound Summary keys.
                    "r: refresh models · Esc: back".to_string()
                }
                DetailSub::EditApiKey { .. } => "Enter: save · Esc: cancel".to_string(),
                DetailSub::OAuthNotice => "Esc: back".to_string(),
            },
        }
    }

    pub fn visible_provider_ids(&self) -> Vec<String> {
        self.visible_providers()
            .iter()
            .map(|p| p.provider_id.clone())
            .collect()
    }

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

    pub fn handle_key(&mut self, key: KeyEvent) -> ProviderSettingsEvent {
        if let Some(dialog) = self.delete_confirm.as_mut() {
            match dialog.handle_key(key.code, key.modifiers) {
                ConfirmDialogOutcome::Primary => {
                    let pid = match &self.mode {
                        ProviderSettingsMode::List => {
                            self.focused_provider().map(|p| p.provider_id.clone())
                        }
                        ProviderSettingsMode::Detail { provider_id, .. } => {
                            Some(provider_id.clone())
                        }
                    };
                    self.delete_confirm = None;
                    if let Some(id) = pid {
                        return ProviderSettingsEvent::Emit(
                            Action::ConfirmDeleteProviderCredentials(id),
                        );
                    }
                    return ProviderSettingsEvent::Consumed;
                }
                ConfirmDialogOutcome::Secondary | ConfirmDialogOutcome::Cancel => {
                    self.delete_confirm = None;
                    return ProviderSettingsEvent::Consumed;
                }
                ConfirmDialogOutcome::Continued | ConfirmDialogOutcome::Ignored => {
                    return ProviderSettingsEvent::Consumed;
                }
            }
        }
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return ProviderSettingsEvent::Consumed;
        }
        match self.mode.clone() {
            ProviderSettingsMode::List => list::handle_list_key(self, key),
            ProviderSettingsMode::Detail { provider_id, sub } => {
                detail::handle_detail_key(self, key, provider_id, sub)
            }
        }
    }

    pub fn reset_to_list(&mut self) {
        self.mode = ProviderSettingsMode::List;
        self.delete_confirm = None;
        self.status.clear();
        self.filter.clear();
        self.filter_mode = false;
        // RPC-105: keep nav_items in sync with the cleared filter so the
        // header count reflects the full provider list immediately.
        self.rebuild_nav_items();
    }

    pub(crate) fn adjust_scroll(&mut self) {
        let total = self.visible_providers().len();
        ensure_visible(
            &mut self.scroll_offset,
            self.selected_index,
            self.visible_rows,
            total,
        );
    }

    pub(crate) fn move_clamped(&mut self, delta: i32) {
        let total = self.visible_providers().len();
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
        // RPC-337: render via the shared full-screen scaffold. Title +
        // footer strings are computed up-front (owned) so the body
        // closure can borrow `self` mutably to capture body height.
        let title = "Provider Settings";
        let count = self.nav_items.len();
        let footer = self.footer_hint();
        let overlay = self.delete_confirm.clone();
        crate::views::full_screen_shell::render_full_screen_scaffold(
            area,
            buf,
            title,
            count,
            "items",
            &footer,
            |body_area, buf| {
                self.visible_rows = body_area.height as usize;
                match &self.mode {
                    ProviderSettingsMode::List => list::render_list(self, body_area, buf),
                    ProviderSettingsMode::Detail { provider_id, sub } => {
                        detail::render_detail(self, body_area, buf, provider_id, sub);
                    }
                }
            },
            overlay.as_ref(),
        );
    }

    pub fn visible_rows_for(area: Rect) -> usize {
        area.height
            .saturating_sub(crate::views::full_screen_shell::CHROME_ROWS) as usize
    }
}

pub fn is_provider_action(action: SlashCommandAction) -> bool {
    matches!(action, SlashCommandAction::Provider)
}
