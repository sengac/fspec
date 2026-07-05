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
use crate::views::model_selector::form::parse_compaction_trigger;

use super::{ProviderSettingsEvent, ProviderSettingsMode, ProviderSettingsView};

/// Default base URL for a brand-new profile (TS `DEFAULT_PROFILE_BASE_URL`).
pub const DEFAULT_PROFILE_BASE_URL: &str = "http://localhost:8888";

/// The five connection field labels in display order (TS `PROFILE_FORM_FIELDS`).
pub const PROFILE_FORM_FIELDS: [&str; 5] = [
    "Base URL",
    "API Key",
    "Context Window",
    "Max Output Tokens",
    "Compaction Threshold",
];

/// TS `compactionThresholdParser.ts` range constants (lines 15-21). Mirrored on
/// the profile save path only — the shared `parse_compaction_trigger` and the
/// model_selector custom-model form stay range-free (TS does not range-check
/// the custom-model form).
const MIN_PERCENTAGE: u32 = 1;
const MAX_PERCENTAGE: u32 = 100;
const MIN_TOKEN_THRESHOLD: u32 = 1000;

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
        }
    }

    /// Mutable handle to the focused connection field's raw string.
    fn focused_text_mut(&mut self) -> &mut String {
        match self.field_index {
            0 => &mut self.base_url,
            1 => &mut self.api_key,
            2 => &mut self.context_window,
            3 => &mut self.max_output_tokens,
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

    fn backspace(&mut self) {
        if self.is_editing_name {
            self.name.pop();
        } else {
            self.focused_text_mut().pop();
        }
    }

    fn push_char(&mut self, c: char) {
        if self.is_editing_name {
            self.name.push(c);
        } else {
            self.focused_text_mut().push(c);
        }
    }

    /// Display value for a field index (used by the renderer).
    pub fn field_value(&self, idx: usize) -> &str {
        match idx {
            0 => &self.base_url,
            1 => &self.api_key,
            2 => &self.context_window,
            3 => &self.max_output_tokens,
            _ => &self.compaction_threshold,
        }
    }

    /// Build a [`ProfileDefinition`] from the current values, or `None` when
    /// base URL, API key, or the trimmed name is empty (TS `handleSave` guard).
    pub fn build_definition(&self) -> Option<ProfileDefinition> {
        if self.base_url.is_empty() || self.api_key.is_empty() || self.name.trim().is_empty() {
            return None;
        }
        let (compaction_threshold_type, compaction_threshold_value) =
            profile_compaction_trigger(&self.compaction_threshold);
        Some(ProfileDefinition {
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            context_window: self.context_window.trim().parse::<u32>().ok(),
            max_output_tokens: self.max_output_tokens.trim().parse::<u32>().ok(),
            compaction_threshold_type,
            compaction_threshold_value,
        })
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
        KeyCode::Backspace | KeyCode::Delete => form.backspace(),
        KeyCode::Char(c) if (' '..='~').contains(&c) => form.push_char(c),
        _ => {}
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
            Some(definition) => {
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
            None => {
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
fn restore_mode(
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

fn opt_num(value: Option<u32>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}

/// Profile-scoped compaction-trigger parse: split via the shared
/// [`parse_compaction_trigger`], then enforce the TS range rules (percentage
/// 1..=100 inclusive, tokens >= 1000). Out-of-range → `(None, None)` so the
/// field is omitted from the saved profile, matching TS
/// `parseCompactionThreshold`. The shared splitter — and therefore the
/// model_selector custom-model form — is left range-free.
fn profile_compaction_trigger(raw: &str) -> (Option<String>, Option<u32>) {
    let (kind, value) = parse_compaction_trigger(raw);
    match (kind.as_deref(), value) {
        (Some("percentage"), Some(n)) if (MIN_PERCENTAGE..=MAX_PERCENTAGE).contains(&n) => {
            (kind, value)
        }
        (Some("tokens"), Some(n)) if n >= MIN_TOKEN_THRESHOLD => (kind, value),
        _ => (None, None),
    }
}

/// Render a stored compaction threshold back into its raw editable string
/// (`percentage` → `"80%"`, `tokens` → `"200000"`, otherwise empty).
fn render_threshold(kind: Option<&str>, value: Option<u32>) -> String {
    match (kind, value) {
        (Some("percentage"), Some(v)) => format!("{v}%"),
        (Some("tokens"), Some(v)) => v.to_string(),
        _ => String::new(),
    }
}
