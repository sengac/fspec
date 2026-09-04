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

use super::profile_form_parse::{
    opt_num, parse_auto_continue, parse_max_images, profile_compaction_trigger, render_threshold,
};

/// Default base URL for a brand-new profile (TS `DEFAULT_PROFILE_BASE_URL`).
pub const DEFAULT_PROFILE_BASE_URL: &str = "http://localhost:8888";

/// Key routing + submit for the open form (TS `handleProfileFormMode`),
/// extracted into the sibling `profile_form_submit` module to keep this file
/// under the 300-LoC ceiling. Re-exported so the view's `handle_key` keeps its
/// `profile_form::handle_form_key` call-site.
pub(super) use super::profile_form_submit::{handle_form_key, restore_mode};

/// The connection field labels in display order (TS `PROFILE_FORM_FIELDS`).
/// PROV-139 appends the boolean "Streaming" toggle as the 6th entry; PROV-142
/// appends the numeric "Auto-Continue" text field as the 7th; PROV-143
/// appends the boolean "Preserve Thinking" toggle as the 8th; PROV-144
/// appends the numeric "Max Images" text field as the 9th, last entry.
pub const PROFILE_FORM_FIELDS: [&str; 9] = [
    "Base URL",
    "API Key",
    "Context Window",
    "Max Output Tokens",
    "Compaction Threshold",
    "Streaming",
    "Auto-Continue",
    "Preserve Thinking",
    "Max Images",
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
    /// PROV-144: the Max Images numeric text field (raw typed string, parsed
    /// on build). Empty or absent ⇒ the tool-layer default of 4; `"0"` ⇒
    /// no-vision profile (the Read tool fails image reads); `"n"` (n >= 1) ⇒
    /// a single Read result may return at most `n` images.
    pub max_images: String,
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
            // PROV-144: empty ⇒ absent on save ⇒ the tool-layer default of 4
            // applies. (The edit form prefills the effective value via
            // `from_definition`; see that path.)
            max_images: String::new(),
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
            // PROV-144: prefill the EFFECTIVE limit — the stored value, or the
            // default 4 when the key is absent (assumption: disk and form
            // always agree). An explicit 0 prefills "0" (no vision).
            max_images: def.max_images_limit().to_string(),
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
            4 => &mut self.compaction_threshold,
            6 => &mut self.auto_continue,
            // PROV-144: the Max Images field (index 8, the last text field).
            8 => &mut self.max_images,
            // 5 (Streaming) and 7 (Preserve Thinking) are boolean toggles —
            // the routing guard intercepts them before this is reached.
            _ => &mut self.compaction_threshold,
        }
    }

    pub(super) fn move_down(&mut self) {
        if self.is_editing_name {
            self.is_editing_name = false;
            self.field_index = 0;
        } else if self.field_index < PROFILE_FORM_FIELDS.len() - 1 {
            self.field_index += 1;
        }
    }

    pub(super) fn move_up(&mut self) {
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
    /// Thinking renders its Enabled/Disabled label; PROV-144: Max Images
    /// renders its raw text).
    pub fn field_value(&self, idx: usize) -> &str {
        match idx {
            0 => &self.base_url,
            1 => &self.api_key,
            2 => &self.context_window,
            3 => &self.max_output_tokens,
            4 => &self.compaction_threshold,
            6 => &self.auto_continue,
            7 => super::profile_form_streaming::streaming_label(self.preserve_thinking),
            // PROV-144: the Max Images field (index 8) renders its raw text.
            8 => &self.max_images,
            _ => super::profile_form_streaming::streaming_label(self.streaming),
        }
    }

    /// Build a [`ProfileDefinition`] from the current values.
    ///
    /// Returns `Err(hint)` when the save must be REJECTED with a visible hint:
    /// PROV-142 — a non-numeric Auto-Continue value (mirroring `/continue`'s
    /// invalid-argument rejection); PROV-144 — a non-numeric Max Images value.
    /// Returns `Ok(None)` when base URL, API key, or the trimmed name is empty
    /// (TS `handleSave` guard — the form stays open silently). Returns
    /// `Ok(Some(def))` on success.
    pub fn build_definition(&self) -> Result<Option<ProfileDefinition>, String> {
        if self.base_url.is_empty() || self.api_key.is_empty() || self.name.trim().is_empty() {
            return Ok(None);
        }
        // PROV-142: parse the Auto-Continue field. Empty ⇒ None (off, today's
        // behavior); "0" ⇒ Some(0) (explicit-off sentinel); "n" (n >= 1) ⇒
        // Some(n) (on with budget n); non-numeric ⇒ reject with a hint.
        let auto_continue = parse_auto_continue(&self.auto_continue)?;
        // PROV-144: parse the Max Images field. Empty ⇒ None (absent ⇒ default
        // 4); "0" ⇒ Some(0) (no-vision sentinel); "n" (n >= 1) ⇒ Some(n)
        // (cap of n images per Read result); non-numeric ⇒ reject with a hint.
        let max_images = parse_max_images(&self.max_images)?;
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
            // PROV-144: carry the parsed Max Images limit (empty ⇒ None so the
            // persistence read-modify-write REMOVES the key ⇒ default 4).
            max_images,
        }))
    }
}

/// Outcome of routing one key through the open form. Owned here (form
/// internals drive it); consumed by the sibling `profile_form_submit` module,
/// which handles the submit / cancel / restore side effects.
pub(super) enum ProfileFormKey {
    Editing,
    Cancel,
    Submit,
}
