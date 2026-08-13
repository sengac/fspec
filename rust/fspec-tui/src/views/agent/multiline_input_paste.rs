//! RPC-403 — bracketed-paste routing for [`super::multiline_input::MultiLineInput`].
//!
//! Feature: spec/features/agent-input-bracketed-paste-routing.feature
//!
//! Extracted alongside `multiline_input_enter.rs` so `multiline_input.rs`
//! stays under the 300-LoC ceiling. Owns:
//!
//!   - CRLF / lone-CR → LF normalization (rule [4], via the shared
//!     [`crate::text_normalize::normalize_line_endings`]) so carriage
//!     returns never enter the buffer, and
//!   - the RPC-095 `block_edits` (Compacting) gate applied to paste
//!     exactly like typed edits (rule [5]).

use super::multiline_input::{InputEventOutcome, InputGate, MultiLineInput};
use crate::text_normalize::normalize_line_endings;

/// Route one bracketed-paste payload into the input: swallowed while
/// the compacting edit gate is active (buffer unchanged), otherwise
/// the normalized text is inserted verbatim at the cursor with
/// embedded `\n` preserved (the input auto-grows via `visible_rows`).
pub(super) fn handle_paste(
    input: &mut MultiLineInput,
    text: &str,
    gate: InputGate,
) -> InputEventOutcome {
    if gate.block_edits {
        return InputEventOutcome::Continued;
    }
    input.insert_str(&normalize_line_endings(text));
    InputEventOutcome::Continued
}
