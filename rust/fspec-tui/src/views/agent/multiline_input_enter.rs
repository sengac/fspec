//! RPC-426 — Enter-key and Ctrl+J newline handling for
//! [`super::multiline_input::MultiLineInput`].
//!
//! Feature: spec/features/shift-enter-newline-doesn-t-work-in-real-terminals-terminal-eats-modifier-no-fallback-binding-no-capability-probe.feature
//!
//! Extracted from `multiline_input.rs` so that file stays under the
//! 300-LoC ceiling. Owns the newline insertion dispositions:
//!
//!   - Plain Enter (no modifiers) → submit the buffer, unless the
//!     RPC-095 gate suppresses Enter (Compacting).
//!   - Ctrl+J → universal newline insertion (Emacs-style). Works on
//!     every terminal because it uses character codes, not modifier
//!     detection. This is the PRIMARY newline binding.
//!   - Shift+Enter → newline ONLY on terminals with kitty keyboard
//!     enhancement (best-effort). Terminal may eat the modifier.
//!   - Alt+Enter → newline as legacy-terminal fallback (terminals
//!     that send `ESC CR` deliver Enter+ALT even without enhancement
//!     flags).
//!
//! Every newline chord is an edit, so `gate.block_edits` (Compacting)
//! swallows them without mutating the buffer.

use crossterm::event::{KeyCode, KeyModifiers};

use super::multiline_input::{InputEventOutcome, InputGate, MultiLineInput};

/// Insert a newline at the cursor position. Used by Ctrl+J, Shift+Enter,
/// and Alt+Enter handlers. Gated by `block_edits` while Compacting.
fn insert_newline_gated(input: &mut MultiLineInput, gate: InputGate) -> InputEventOutcome {
    if gate.block_edits {
        return InputEventOutcome::Continued;
    }
    input.insert_newline();
    InputEventOutcome::Continued
}

/// Route Enter and Ctrl+J keystrokes. Returns `Some(outcome)` when the
/// key is handled here (Enter or Ctrl+J), `None` to fall through to
/// remaining branches in `handle_key_gated`.
///
/// Ctrl+J is the universal newline binding — works on every terminal.
/// Shift+Enter is best-effort (only on enhanced terminals). Alt+Enter
/// is legacy fallback.
pub(super) fn handle_enter(
    input: &mut MultiLineInput,
    code: KeyCode,
    mods: KeyModifiers,
    gate: InputGate,
) -> Option<InputEventOutcome> {
    // Ctrl+J → universal newline (Emacs-style).
    // Works on every terminal because Ctrl+J is a character code,
    // not a modifier-dependent key event.
    if code == KeyCode::Char('j') && mods.contains(KeyModifiers::CONTROL) {
        return Some(insert_newline_gated(input, gate));
    }
    // Enter key handling.
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
    // Shift+Enter only works on terminals with keyboard enhancement.
    // Alt+Enter works as legacy fallback (ESC CR → Enter+ALT).
    Some(insert_newline_gated(input, gate))
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
