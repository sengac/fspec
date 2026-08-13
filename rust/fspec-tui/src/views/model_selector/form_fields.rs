//! RPC-344 — static field metadata for the custom-model form.
//!
//! Feature: spec/features/model-selector-custom-model-crud.feature
//!
//! Extracted from `form.rs` (PROV-107) to keep that file under the
//! 300-LoC ceiling. Pure constant data + the field-kind enum; no state,
//! no behaviour. Re-exported from `form.rs` so existing
//! `super::form::{FieldType, FORM_FIELDS}` paths keep resolving.

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
