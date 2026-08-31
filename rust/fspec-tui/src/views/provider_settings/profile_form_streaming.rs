//! PROV-139 — Streaming boolean-field helpers extracted from `profile_form.rs`
//! to keep that module under the 300-LoC ceiling.
//!
//! Feature: spec/features/provider-settings-profile-streaming.feature
//!
//! The Streaming field is a boolean toggle, not a free-text field: Space,
//! Left, and Right flip it; printable chars are swallowed (never appended to a
//! text field). This module owns the toggle routing and the Enabled/Disabled
//! display label so `profile_form.rs` only wires the field in.

use crossterm::event::KeyCode;

use super::profile_form::ProfileForm;

/// Index of the Streaming field in `PROFILE_FORM_FIELDS` (the 6th entry).
pub(super) const STREAMING_FIELD_INDEX: usize = 5;

/// PROV-143: index of the Preserve Thinking field in `PROFILE_FORM_FIELDS`
/// (the 8th, last entry).
pub(super) const PRESERVE_THINKING_FIELD_INDEX: usize = 7;

/// True when `field_index` targets a boolean toggle field (Streaming or
/// PROV-143's Preserve Thinking).
pub(super) fn is_streaming_field(field_index: usize) -> bool {
    field_index == STREAMING_FIELD_INDEX || field_index == PRESERVE_THINKING_FIELD_INDEX
}

/// Display string for the Streaming toggle (rendered by `profile_form_render`).
pub(super) fn streaming_label(streaming: bool) -> &'static str {
    if streaming {
        "Enabled"
    } else {
        "Disabled"
    }
}

/// Dispatch one editing key (everything except Esc/Enter/Tab/Up/Down).
///
/// While a boolean toggle field (Streaming, or the PROV-143 Preserve Thinking
/// field) is focused the field is a boolean toggle: Space/Left/Right flip it
/// and every other key (including printable chars) is swallowed so no text is
/// appended. Otherwise the key edits the focused text field exactly as before.
pub(super) fn route_edit_key(form: &mut ProfileForm, code: KeyCode) {
    if !form.is_editing_name && is_streaming_field(form.field_index) {
        toggle_on_key(form, code);
        return;
    }
    match code {
        KeyCode::Backspace | KeyCode::Delete => form.backspace(),
        KeyCode::Char(c) if (' '..='~').contains(&c) => form.push_char(c),
        _ => {}
    }
}

/// Flip the focused boolean toggle on Space/Left/Right; ignore all other keys.
fn toggle_on_key(form: &mut ProfileForm, code: KeyCode) {
    if matches!(code, KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right) {
        match form.field_index {
            STREAMING_FIELD_INDEX => form.streaming = !form.streaming,
            PRESERVE_THINKING_FIELD_INDEX => form.preserve_thinking = !form.preserve_thinking,
            _ => {}
        }
    }
}
