//! PROV-110/PROV-144 — profile form key routing + submit handling, extracted
//! from `profile_form.rs` to keep that module under the 300-LoC ceiling.
//!
//! Feature: spec/features/provider-settings-profile-form.feature
//!
//! `handle_form_key` routes one key through an open create/edit form: on a
//! valid Enter it emits [`Action::SaveProfile`] and resets to List; Esc
//! cancels to List; every other key keeps the form open with its edits
//! persisted. A rejected build (PROV-142 non-numeric Auto-Continue, PROV-144
//! non-numeric Max Images) surfaces the hint in the view status and keeps the
//! form open — nothing is persisted.

use crate::components::Action;

use super::profile_form::{ProfileForm, ProfileFormKey};
use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};
use crossterm::event::{KeyCode, KeyEvent};

/// Apply one key to the form (TS `handleProfileFormMode`).
fn route_key(form: &mut ProfileForm, key: KeyEvent) -> ProfileFormKey {
    match key.code {
        KeyCode::Esc => return ProfileFormKey::Cancel,
        KeyCode::Enter => return ProfileFormKey::Submit,
        // TUI-084: Tab is intentionally ignored in profile form mode.
        KeyCode::Tab => {}
        KeyCode::Down => form.move_down(),
        KeyCode::Up => form.move_up(),
        // PROV-139: remaining editing keys route through the sibling module.
        code => super::profile_form_streaming::route_edit_key(form, code),
    }
    ProfileFormKey::Editing
}

/// Route a key event through an open create/edit form mode. On a valid Enter
/// it emits [`Action::SaveProfile`] and resets to List; Esc cancels to List;
/// every other key keeps the form open with its edits persisted.
pub(super) fn handle_form_key(
    view: &mut ProviderSettingsView,
    key: KeyEvent,
    provider_id: String,
    mut form: ProfileForm,
    profile_name: Option<String>,
) -> ProviderSettingsEvent {
    match route_key(&mut form, key) {
        ProfileFormKey::Cancel => {
            view.mode = ProviderSettingsMode::List;
            ProviderSettingsEvent::Consumed
        }
        ProfileFormKey::Submit => match form.build_definition() {
            Ok(Some(definition)) => {
                // PROV-136: the EMITTED name is the (possibly edited) form name;
                // `old_profile_name` carries the ORIGINAL edit-mode name so the
                // dispatch layer can detect + apply a rename. In create mode
                // `profile_name` is None → `old_profile_name` is None.
                let new_name = form.name.trim().to_string();
                view.mode = ProviderSettingsMode::List;
                ProviderSettingsEvent::Emit(Action::SaveProfile {
                    provider_id,
                    profile_name: new_name,
                    old_profile_name: profile_name,
                    definition,
                })
            }
            Ok(None) => {
                restore_mode(view, provider_id, form, profile_name);
                ProviderSettingsEvent::Consumed
            }
            // PROV-142/144: a non-numeric Auto-Continue or Max Images value
            // rejects the save — surface the hint in the view status and keep
            // the form open so the user can fix the value (nothing is
            // persisted).
            Err(hint) => {
                view.set_status(hint);
                restore_mode(view, provider_id, form, profile_name);
                ProviderSettingsEvent::Consumed
            }
        },
        ProfileFormKey::Editing => {
            restore_mode(view, provider_id, form, profile_name);
            ProviderSettingsEvent::Consumed
        }
    }
}

/// Persist the (possibly mutated) form back into the view's mode.
pub(super) fn restore_mode(
    view: &mut ProviderSettingsView,
    provider_id: String,
    form: ProfileForm,
    profile_name: Option<String>,
) {
    view.mode = match profile_name {
        Some(profile_name) => ProviderSettingsMode::EditProfile {
            provider_id,
            profile_name,
            form,
        },
        None => ProviderSettingsMode::CreateProfile { provider_id, form },
    };
}
