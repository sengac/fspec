//! RPC-344 — custom-model CRUD form state for the full-screen
//! ModelSelector mode-view.
//!
//! Feature: spec/features/model-selector-custom-model-crud.feature
//!
//! Mirrors the TypeScript `customModelMode.ts` / `customModelForm.ts` /
//! `useCustomModelFormState.ts` trio: the mode union (browse / add / edit /
//! delete-confirm), the eight-field form definition, the in-progress field
//! values, and the helpers that turn those values into a transport
//! `CustomModelDefinition` (the RPC-347 wire type). Pure state + pure
//! transforms — no rendering, no key routing (those live in the parent
//! `mod.rs`).

use codelet_rpc_types::CustomModelDefinition;
use crossterm::event::{KeyCode, KeyEvent};

/// Outcome of routing one key through an open add/edit form. `Submit` and
/// `Cancel` ask the parent view to leave form mode (building the matching
/// Action on submit); `Editing` keeps the form open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormOutcome {
    Editing,
    Cancel,
    Submit,
}

/// The custom-model sub-mode of the model selector. `Browse` is the normal
/// list; the other three drive the form / confirm overlays. Mirrors the TS
/// `CustomModelMode` union (`customModelMode.ts:13-41`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CustomModelMode {
    #[default]
    Browse,
    Add {
        provider_id: String,
        profile_name: String,
    },
    Edit {
        provider_id: String,
        profile_name: String,
        original_model_id: String,
    },
    DeleteConfirm {
        provider_id: String,
        profile_name: String,
        model_id: String,
        display_name: String,
    },
}

/// Input kind of a custom-model form field (`customModelForm.ts:12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Number,
    Select,
    Boolean,
}

/// One form field's static metadata (`customModelForm.ts:38-96`).
#[derive(Debug, Clone, Copy)]
pub struct FormField {
    pub label: &'static str,
    pub field_type: FieldType,
    pub required: bool,
    pub placeholder: &'static str,
    pub options: &'static [&'static str],
}

/// The eight custom-model form fields in display order. Index positions are
/// load-bearing: the form's `field_index` indexes straight into this slice.
pub const FORM_FIELDS: [FormField; 8] = [
    FormField {
        label: "Model ID",
        field_type: FieldType::Text,
        required: true,
        placeholder: "e.g., meta-llama/Meta-Llama-3.1-405B",
        options: &[],
    },
    FormField {
        label: "Display Name",
        field_type: FieldType::Text,
        required: false,
        placeholder: "e.g., Llama 3.1 405B",
        options: &[],
    },
    FormField {
        label: "Facade",
        field_type: FieldType::Select,
        required: false,
        placeholder: "(default: openai)",
        options: &["openai", "codex", "claude", "gemini", "zai"],
    },
    FormField {
        label: "Context Window",
        field_type: FieldType::Number,
        required: false,
        placeholder: "128000",
        options: &[],
    },
    FormField {
        label: "Max Output Tokens",
        field_type: FieldType::Number,
        required: false,
        placeholder: "16384",
        options: &[],
    },
    FormField {
        label: "Compaction Trigger",
        field_type: FieldType::Text,
        required: false,
        placeholder: "80% or 200000",
        options: &[],
    },
    FormField {
        label: "Reasoning",
        field_type: FieldType::Boolean,
        required: false,
        placeholder: "false",
        options: &[],
    },
    FormField {
        label: "Vision",
        field_type: FieldType::Boolean,
        required: false,
        placeholder: "false",
        options: &[],
    },
];

/// In-progress form field values. Text/number fields keep their raw typed
/// string (number fields are validated on build); the Compaction Trigger
/// keeps its raw `"80%"` / `"200000"` text and is parsed on build.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CustomModelForm {
    pub id: String,
    pub display_name: String,
    pub facade: Option<String>,
    pub context_window: String,
    pub max_output_tokens: String,
    pub compaction_trigger: String,
    pub reasoning: Option<bool>,
    pub has_vision: Option<bool>,
    /// Focused field index into [`FORM_FIELDS`].
    pub field_index: usize,
}

impl CustomModelForm {
    /// Prefill the form from a focused custom-model row's wire fields. Per the
    /// RPC-344 known divergence, only id / display name / context window /
    /// reasoning / vision are available on the wire `ModelEntry`; facade,
    /// max-output and compaction start blank.
    pub fn prefill_from_entry(
        id: &str,
        display_name: &str,
        context_window: u32,
        reasoning: bool,
        has_vision: bool,
    ) -> Self {
        Self {
            id: id.to_string(),
            // Parity with TS: the wire row's display name falls back to the id
            // when no custom display name is stored (see merge_profile_models).
            // Treat label == id as "no stored display name" so editing and
            // re-saving an unnamed custom model does NOT materialize
            // displayName == id on disk (a write the TS build never makes).
            display_name: if display_name == id {
                String::new()
            } else {
                display_name.to_string()
            },
            facade: None,
            context_window: if context_window > 0 {
                context_window.to_string()
            } else {
                String::new()
            },
            max_output_tokens: String::new(),
            compaction_trigger: String::new(),
            reasoning: Some(reasoning),
            has_vision: Some(has_vision),
            field_index: 0,
        }
    }

    /// Mutable handle to the focused text/number field's raw string, or `None`
    /// when the focused field is a select or boolean.
    fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.field_index {
            0 => Some(&mut self.id),
            1 => Some(&mut self.display_name),
            3 => Some(&mut self.context_window),
            4 => Some(&mut self.max_output_tokens),
            5 => Some(&mut self.compaction_trigger),
            _ => None,
        }
    }

    /// Route one key event through the open form. Returns `Submit` on Enter,
    /// `Cancel` on Esc, and `Editing` for every field mutation.
    pub fn handle_key(&mut self, key: KeyEvent) -> FormOutcome {
        match key.code {
            KeyCode::Esc => return FormOutcome::Cancel,
            KeyCode::Enter => return FormOutcome::Submit,
            KeyCode::Up => {
                self.field_index = self.field_index.saturating_sub(1);
            }
            KeyCode::Down => {
                self.field_index = (self.field_index + 1).min(FORM_FIELDS.len() - 1);
            }
            KeyCode::Left | KeyCode::Right => {
                self.cycle_focused_field(matches!(key.code, KeyCode::Right));
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if let Some(s) = self.focused_text_mut() {
                    s.pop();
                }
            }
            KeyCode::Char(c) if (' '..='~').contains(&c) => {
                if let Some(s) = self.focused_text_mut() {
                    s.push(c);
                }
            }
            _ => {}
        }
        FormOutcome::Editing
    }

    /// Left/Right on the focused field: cycle select options (wrapping) or
    /// toggle boolean fields. Text/number fields ignore arrows.
    fn cycle_focused_field(&mut self, forward: bool) {
        match FORM_FIELDS[self.field_index].field_type {
            FieldType::Select => {
                let options = FORM_FIELDS[self.field_index].options;
                if options.is_empty() {
                    return;
                }
                let cur = self
                    .facade
                    .as_deref()
                    .and_then(|v| options.iter().position(|o| *o == v));
                let next = match cur {
                    Some(i) if forward => (i + 1) % options.len(),
                    Some(i) => (i + options.len() - 1) % options.len(),
                    None if forward => 0,
                    None => options.len() - 1,
                };
                self.facade = Some(options[next].to_string());
            }
            FieldType::Boolean => {
                let slot = match self.field_index {
                    6 => &mut self.reasoning,
                    7 => &mut self.has_vision,
                    _ => return,
                };
                *slot = Some(!slot.unwrap_or(false));
            }
            _ => {}
        }
    }

    /// Build a transport [`CustomModelDefinition`] from the current values,
    /// or `None` when the required Model ID is blank. Empty optional fields
    /// are omitted (left `None`) so they round-trip to skipped JSON keys.
    pub fn build_definition(&self) -> Option<CustomModelDefinition> {
        let id = self.id.trim();
        if id.is_empty() {
            return None;
        }
        let (compaction_threshold_type, compaction_threshold_value) =
            parse_compaction_trigger(&self.compaction_trigger);
        let display_name = {
            let dn = self.display_name.trim();
            if dn.is_empty() {
                None
            } else {
                Some(dn.to_string())
            }
        };
        Some(CustomModelDefinition {
            id: id.to_string(),
            display_name,
            facade: self.facade.clone(),
            context_window: self.context_window.trim().parse::<u32>().ok(),
            max_output_tokens: self.max_output_tokens.trim().parse::<u32>().ok(),
            compaction_threshold_type,
            compaction_threshold_value,
            reasoning: self.reasoning,
            has_vision: self.has_vision,
        })
    }
}

/// Parse a Compaction Trigger string into the split wire fields
/// `(compaction_threshold_type, compaction_threshold_value)`: a trailing-%
/// string → `("percentage", n)`, a bare integer → `("tokens", n)`, anything
/// else → `(None, None)`.
pub fn parse_compaction_trigger(raw: &str) -> (Option<String>, Option<u32>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    if let Some(pct) = trimmed.strip_suffix('%') {
        if let Ok(n) = pct.trim().parse::<u32>() {
            return (Some("percentage".to_string()), Some(n));
        }
        return (None, None);
    }
    if let Ok(n) = trimmed.parse::<u32>() {
        return (Some("tokens".to_string()), Some(n));
    }
    (None, None)
}
