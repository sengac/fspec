//! PROV-137 — bracketed-paste dispatch for `ProviderSettingsView`.
//!
//! Feature: spec/features/provider-settings-input-paste.feature
//!
//! Extracted from `mod.rs` so that module stays under the 300-LoC ceiling.
//! Dispatches on `self.mode` like `handle_key`, but acts ONLY in the
//! text-entry modes (create/edit profile form + inline API-key entry). All
//! other modes ignore the paste so it falls through untouched. Pasted text is
//! filtered through the same per-field charset gate used for typed input,
//! which also strips newlines.

use super::{
    detail, profile_form_paste, DetailSub, ProviderSettingsEvent, ProviderSettingsMode,
    ProviderSettingsView,
};

/// Route a bracketed-paste blob to the active input mode's paste sink.
pub(super) fn handle_paste(
    view: &mut ProviderSettingsView,
    text: &str,
) -> ProviderSettingsEvent {
    match view.mode.clone() {
        ProviderSettingsMode::CreateProfile { provider_id, form } => {
            profile_form_paste::handle_form_paste(view, provider_id, form, None, text)
        }
        ProviderSettingsMode::EditProfile {
            provider_id,
            profile_name,
            form,
        } => profile_form_paste::handle_form_paste(
            view,
            provider_id,
            form,
            Some(profile_name),
            text,
        ),
        ProviderSettingsMode::Detail {
            provider_id,
            sub: DetailSub::EditApiKey { draft },
        } => detail::handle_edit_paste(view, provider_id, draft, text),
        _ => ProviderSettingsEvent::Ignored,
    }
}
