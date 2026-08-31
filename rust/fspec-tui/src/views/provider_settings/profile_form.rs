//! PROV-110 — profile create/edit form state + key routing.
//!
//! Feature: spec/features/provider-settings-profile-form.feature
//!
//! Rust port of the TS profile form (`src/tui/inputHandlers/
//! profileFormModeHandler.ts`, `src/tui/utils/providerSettingsHelpers.ts`,
//! `src/tui/constants/providerSettings.ts`). Pure state + parse-on-build,
//! mirroring the model_selector `CustomModelForm` shape. PROV-136 DIVERGES
//! from the TS name-lock: in edit mode the name IS editable (Up from the first
//! connection field re-enters it), so an existing profile can be renamed. The
//! `is_editing_name` flag still gates whether keystrokes edit the name;
//! `is_new` now only distinguishes the create-mode defaults. `customModels` is
//! deliberately NOT a form field — that array is owned by the model-selector
//! CRUD and preserved by the backend read-modify-write.

use codelet_rpc_types::ProfileDefinition;
use crossterm::event::{KeyCode, KeyEvent};

use crate::components::Action;

use super::profile_form_parse::{
    opt_num, parse_auto_continue, profile_compaction_trigger, render_threshold,
};
use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// Default base URL for a brand-new profile (TS `DEFAULT_PROFILE_BASE_URL`).
pub const DEFAULT_PROFILE_BASE_URL: &str = "http://localhost:8888";

/// The connection field labels in display order (TS `PROFILE_FORM_FIELDS`).
/// PROV-139 appends the boolean "Streaming" toggle as the 6th entry; PROV-142
/// appends the numeric "Auto-Continue" text field as the 7th; PROV-143
/// appends the boolean "Preserve Thinking" toggle as the 8th, last entry.
pub const PROFILE_FORM_FIELDS: [&str; 8] = [
    "Base URL",
    "API Key",
    "Context Window",
    "Max Output Tokens",
    "Compaction Threshold",
    "Streaming",
    "Auto-Continue",
    "Preserve Thinking",
];

/// In-progress profile form values. Number / threshold fields keep their raw
/// typed string and are parsed on build.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileForm {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub context_window: String,
    pub max_output_tokens: String,
    pub compaction_threshold: String,
    /// PROV-139: the Streaming boolean toggle (true ⇒ enabled); not a text field.
    pub streaming: bool,
    /// PROV-142: the Auto-Continue numeric text field (raw typed string,
    /// parsed on build). Empty or `0` ⇒ off; `n >= 1` ⇒ on with budget `n`.
    pub auto_continue: String,
    /// PROV-143: the Preserve Thinking boolean toggle. `false` (default) ⇒
    /// thinking blocks are STRIPPED from the chat history sent back to the LLM;
    /// `true` ⇒ preserved. Not a text field (Space/Left/Right flip it).
    pub preserve_thinking: bool,
    /// Focused field index into [`PROFILE_FORM_FIELDS`].
    pub field_index: usize,
    /// True while the cursor is in the name field (editable in both create and
    /// edit mode since PROV-136).
    pub is_editing_name: bool,
    /// True for create mode. PROV-136: no longer gates name re-entry (edit mode
    /// can rename too); retained to distinguish the create-mode default seed.
    pub is_new: bool,
}

impl ProfileForm {
    /// TS `initializeNewProfile`: empty name (being edited), default base URL.
    pub fn new_create() -> Self {
        Self {
            name: String::new(),
            base_url: DEFAULT_PROFILE_BASE_URL.to_string(),
            api_key: String::new(),
            context_window: String::new(),
            max_output_tokens: String::new(),
            compaction_threshold: String::new(),
            field_index: 0,
            is_editing_name: true,
            is_new: true,
            streaming: true,
            auto_continue: String::new(),
            // PROV-143: default to DISABLED — thinking blocks are stripped
            // from the outgoing chat history unless the user opts in.
            preserve_thinking: false,
        }
    }

    /// TS `initializeEditProfile`: prefill every field from the stored
    /// definition. PROV-136: the name starts un-focused (`is_editing_name =
    /// false`) but is now editable — Up from the first field re-enters it so
    /// the profile can be renamed.
    pub fn from_definition(name: &str, def: &ProfileDefinition) -> Self {
        Self {
            name: name.to_string(),
            base_url: def.base_url.clone(),
            api_key: def.api_key.clone(),
            context_window: opt_num(def.context_window),
            max_output_tokens: opt_num(def.max_output_tokens),
            compaction_threshold: render_threshold(
                def.compaction_threshold_type.as_deref(),
                def.compaction_threshold_value,
            ),
            field_index: 0,
            is_editing_name: false,
            is_new: false,
            streaming: def.streaming_enabled(),
            auto_continue: opt_num(def.auto_continue),
            // PROV-143: prefill the stored toggle; an absent key ⇒ disabled.
            preserve_thinking: def.preserve_thinking_enabled(),
        }
    }

    /// Mutable handle to the focused connection field's raw string (never the
    /// boolean Streaming field — the editing dispatch branches on it first;
    /// index 5 is only reached if the routing guard regresses).
    fn focused_text_mut(&mut self) -> &mut String {
        match self.field_index {
            0 => &mut self.base_url,
            1 => &mut self.api_key,
            2 => &mut self.context_window,
            3 => &mut self.max_output_tokens,
            6 => &mut self.auto_continue,
            _ => &mut self.compaction_threshold,
        }
    }

    fn move_down(&mut self) {
        if self.is_editing_name {
            self.is_editing_name = false;
            self.field_index = 0;
        } else if self.field_index < PROFILE_FORM_FIELDS.len() - 1 {
            self.field_index += 1;
        }
    }

    fn move_up(&mut self) {
        if self.is_editing_name {
            // Already at the top — Up is a no-op while editing the name.
        } else if self.field_index > 0 {
            self.field_index -= 1;
        } else {
            // PROV-136: Up from the first connection field re-enters the name
            // field in BOTH create and edit mode (the edit-mode rename path
            // diverges from the TS reference, which locks the name).
            self.is_editing_name = true;
        }
    }

    pub(super) fn backspace(&mut self) {
        if self.is_editing_name {
            self.name.pop();
        } else {
            self.focused_text_mut().pop();
        }
    }

    pub(super) fn push_char(&mut self, c: char) {
        if self.is_editing_name {
            self.name.push(c);
        } else {
            self.focused_text_mut().push(c);
        }
    }

    /// PROV-137: paste-time single-char insert (same routing as `push_char`).
    /// Exposed to the sibling `profile_form_paste` module which applies the
    /// printable-ASCII gate before calling this per char.
    pub(crate) fn push_paste_char(&mut self, c: char) {
        self.push_char(c);
    }

    /// Display value for a field index (PROV-139: Streaming renders its label;
    /// PROV-142: Auto-Continue renders its raw text; PROV-143: Preserve
    /// Thinking renders its Enabled/Disabled label).
    pub fn field_value(&self, idx: usize) -> &str {
        match idx {
            0 => &self.base_url,
            1 => &self.api_key,
            2 => &self.context_window,
            3 => &self.max_output_tokens,
            4 => &self.compaction_threshold,
            6 => &self.auto_continue,
            7 => super::profile_form_streaming::streaming_label(self.preserve_thinking),
            _ => super::profile_form_streaming::streaming_label(self.streaming),
        }
    }

    /// Build a [`ProfileDefinition`] from the current values.
    ///
    /// Returns `Err(hint)` when the save must be REJECTED with a visible hint:
    /// PROV-142 — a non-numeric Auto-Continue value (mirroring `/continue`'s
    /// invalid-argument rejection). Returns `Ok(None)` when base URL, API key,
    /// or the trimmed name is empty (TS `handleSave` guard — the form stays
    /// open silently). Returns `Ok(Some(def))` on success.
    pub fn build_definition(&self) -> Result<Option<ProfileDefinition>, String> {
        if self.base_url.is_empty() || self.api_key.is_empty() || self.name.trim().is_empty() {
            return Ok(None);
        }
        // PROV-142: parse the Auto-Continue field. Empty ⇒ None (off, today's
        // behavior); "0" ⇒ Some(0) (explicit-off sentinel); "n" (n >= 1) ⇒
        // Some(n) (on with budget n); non-numeric ⇒ reject with a hint.
        let auto_continue = parse_auto_continue(&self.auto_continue)?;
        let (compaction_threshold_type, compaction_threshold_value) =
            profile_compaction_trigger(&self.compaction_threshold);
        Ok(Some(ProfileDefinition {
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            context_window: self.context_window.trim().parse::<u32>().ok(),
            max_output_tokens: self.max_output_tokens.trim().parse::<u32>().ok(),
            compaction_threshold_type,
            compaction_threshold_value,
            streaming: Some(self.streaming),
            auto_continue,
            // PROV-143: always carry the explicit toggle so the on-disk
            // profile reflects the form (true ⇒ preserved, false ⇒ stripped).
            preserve_thinking: Some(self.preserve_thinking),
        }))
    }
}

/// Outcome of routing one key through the open form.
enum FormKey {
    Editing,
    Cancel,
    Submit,
}

/// Apply one key to the form (TS `handleProfileFormMode`).
fn route_key(form: &mut ProfileForm, key: KeyEvent) -> FormKey {
    match key.code {
        KeyCode::Esc => return FormKey::Cancel,
        KeyCode::Enter => return FormKey::Submit,
        // TUI-084: Tab is intentionally ignored in profile form mode.
        KeyCode::Tab => {}
        KeyCode::Down => form.move_down(),
        KeyCode::Up => form.move_up(),
        // PROV-139: remaining editing keys route through the sibling module.
        code => super::profile_form_streaming::route_edit_key(form, code),
    }
    FormKey::Editing
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
        FormKey::Cancel => {
            view.mode = ProviderSettingsMode::List;
            ProviderSettingsEvent::Consumed
        }
        FormKey::Submit => match form.build_definition() {
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
            // PROV-142: a non-numeric Auto-Continue value rejects the save —
            // surface the hint in the view status and keep the form open so
            // the user can fix the value (nothing is persisted).
            Err(hint) => {
                view.set_status(hint);
                restore_mode(view, provider_id, form, profile_name);
                ProviderSettingsEvent::Consumed
            }
        },
        FormKey::Editing => {
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
