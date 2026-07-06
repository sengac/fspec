//! PROV-137 — profile-form paste insertion.
//!
//! Feature: spec/features/provider-settings-input-paste.feature
//!
//! Extracted from `profile_form.rs` to keep that module under the 300-LoC
//! ceiling. Mirrors the multiline_input_paste.rs split-out pattern: the paste
//! sink lives beside its owner and is called through the view's
//! `handle_paste`. Each pasted char passes the SAME `(' '..='~')` gate that
//! `route_key` applies to typed chars, so newlines / control chars / non-ASCII
//! are dropped and single-line fields stay single-line.

use super::profile_form::{restore_mode, ProfileForm};
use super::profile_form_streaming::is_streaming_field;
use super::{ProviderSettingsEvent, ProviderSettingsView};

/// Insert every gate-passing char of `text` into the focused field (or the
/// name while `is_editing_name`). Chars outside `(' '..='~')` are dropped.
/// PROV-139: a paste while the boolean Streaming field is focused is a no-op —
/// that field is not text and must never accumulate pasted characters.
fn insert_str(form: &mut ProfileForm, text: &str) {
    if !form.is_editing_name && is_streaming_field(form.field_index) {
        return;
    }
    for c in text.chars() {
        if (' '..='~').contains(&c) {
            form.push_paste_char(c);
        }
    }
}

/// Route a paste into an open create/edit form. Inserts the pasted text into
/// the focused field (or name), then persists the mutated form back into the
/// view's mode via `restore_mode`.
pub(super) fn handle_form_paste(
    view: &mut ProviderSettingsView,
    provider_id: String,
    mut form: ProfileForm,
    profile_name: Option<String>,
    text: &str,
) -> ProviderSettingsEvent {
    insert_str(&mut form, text);
    restore_mode(view, provider_id, form, profile_name);
    ProviderSettingsEvent::Consumed
}
