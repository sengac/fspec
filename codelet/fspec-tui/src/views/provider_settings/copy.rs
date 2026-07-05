//! PROV-138 — Ctrl+C copy dispatch for `ProviderSettingsView` input areas.
//!
//! Feature: spec/features/provider-settings-input-copy.feature
//!
//! Extracted from `mod.rs` (mirrors `paste.rs`) so that module stays under the
//! 300-LoC ceiling. Only the text-entry modes copy: the create/edit profile
//! form and the inline API-key entry. The focused field's value is copied
//! PLAINTEXT, except the secret API-key surfaces which are MASKED via the
//! shared `mask_secret` helper BEFORE the action is built — so the plaintext
//! secret can never enter the action bus / clipboard.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{
    Action, DetailSub, ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView,
};

/// PROV-138: single source of truth for secret masking. The on-screen mask
/// (profile form API-key field + inline EditApiKey draft) and the Ctrl+C copy
/// mask both call this so they can never drift: one bullet per Unicode scalar.
pub(crate) fn mask_secret(s: &str) -> String {
    "•".repeat(s.chars().count())
}

/// If `key` is Ctrl+C while a copyable text-entry mode is active, emit the
/// copy event; otherwise return `None` so `handle_key` falls through to its
/// existing behaviour.
pub(super) fn intercept_ctrl_c(
    view: &ProviderSettingsView,
    key: KeyEvent,
) -> Option<ProviderSettingsEvent> {
    if matches!(key.code, KeyCode::Char('c'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
        && is_copyable_mode(&view.mode)
    {
        Some(handle_copy(view))
    } else {
        None
    }
}

/// True for the modes whose Ctrl+C copies a focused input field. All other
/// modes (List, Detail summary, OAuth flows, …) fall through to the existing
/// no-op consume.
fn is_copyable_mode(mode: &ProviderSettingsMode) -> bool {
    matches!(
        mode,
        ProviderSettingsMode::CreateProfile { .. }
            | ProviderSettingsMode::EditProfile { .. }
            | ProviderSettingsMode::Detail {
                sub: DetailSub::EditApiKey { .. },
                ..
            }
    )
}

/// Compute the focused field's copy text and emit `CopyToClipboard`. The API
/// Key field (profile-form index 1) and the inline EditApiKey draft are masked;
/// every other field is copied plaintext.
fn handle_copy(view: &ProviderSettingsView) -> ProviderSettingsEvent {
    let text = match &view.mode {
        ProviderSettingsMode::CreateProfile { form, .. }
        | ProviderSettingsMode::EditProfile { form, .. } => {
            if !form.is_editing_name && form.field_index == 1 {
                // API Key field — mask; never expose the plaintext secret.
                mask_secret(form.field_value(1))
            } else if form.is_editing_name {
                // The Name field is not secret.
                form.name.clone()
            } else {
                form.field_value(form.field_index).to_string()
            }
        }
        ProviderSettingsMode::Detail {
            sub: DetailSub::EditApiKey { draft },
            ..
        } => mask_secret(draft),
        // is_copyable_mode gates callers, so no other mode reaches here.
        _ => String::new(),
    };
    ProviderSettingsEvent::Emit(Action::CopyToClipboard(text))
}
