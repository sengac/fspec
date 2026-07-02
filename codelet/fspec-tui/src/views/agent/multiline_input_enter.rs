//! RPC-402 — Enter-key handling for [`super::multiline_input::MultiLineInput`].
//!
//! Feature: spec/features/agent-input-multiline-newline-keys.feature
//!
//! Extracted from `multiline_input.rs` so that file stays under the
//! 300-LoC ceiling. Owns the Enter dispositions:
//!
//!   - Plain Enter (no modifiers) → submit the buffer, unless the
//!     RPC-095 gate suppresses Enter (Compacting).
//!   - ANY modifier-carrying Enter (Shift, Alt, Ctrl, combos) →
//!     insert a literal newline at the cursor. Alt+Enter is the
//!     EXPLICIT legacy-terminal fallback (terminals that send
//!     `ESC CR` deliver Enter+ALT even without kitty enhancement
//!     flags); treating the remaining combos identically closes the
//!     accidental tui-textarea fallthrough COMPLETELY — Ctrl+Enter
//!     previously reached `textarea.input()` ungated and could edit
//!     the buffer while Compacting.
//!
//! Every newline chord is an edit, so `gate.block_edits` (Compacting)
//! swallows them without mutating the buffer.

use crossterm::event::{KeyCode, KeyModifiers};

use super::multiline_input::{InputEventOutcome, InputGate, MultiLineInput};

/// Route an Enter keystroke. Returns `None` when `code` is not Enter
/// so `handle_key_gated` falls through to its remaining branches;
/// every Enter (any modifier combination) is handled here.
pub(super) fn handle_enter(
    input: &mut MultiLineInput,
    code: KeyCode,
    mods: KeyModifiers,
    gate: InputGate,
) -> Option<InputEventOutcome> {
    if code != KeyCode::Enter {
        return None;
    }
    // Plain Enter → submit, unless suppressed.
    if mods.is_empty() {
        if gate.suppress_enter {
            return Some(InputEventOutcome::Continued);
        }
        let buf = input.value();
        input.reset();
        return Some(InputEventOutcome::Submitted(buf));
    }
    // Any modifier-Enter combo (Shift/Alt/Ctrl/…) → literal newline
    // (Continued), gated as an edit while Compacting.
    if gate.block_edits {
        return Some(InputEventOutcome::Continued);
    }
    input.insert_newline();
    Some(InputEventOutcome::Continued)
}

/// RPC-095 — returns true if the keystroke would otherwise insert
/// text or delete characters. Used by `handle_key_gated` to swallow
/// input while the session is Compacting (`gate.block_edits = true`).
/// Moved here from `multiline_input.rs` (RPC-403) to keep that file
/// under the 300-LoC ceiling.
pub(super) fn is_edit_keystroke(code: KeyCode, _mods: KeyModifiers) -> bool {
    matches!(
        code,
        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete | KeyCode::Tab
    )
}
