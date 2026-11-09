//! BUG-642: a quoted-context turn (AMGR-011 turns, range and query paths)
//! is truncated on a char boundary.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature

use codelet_agent_loop::agent_manager_handler::truncate_context_turn;

#[test]
fn a_quoted_context_turn_cuts_before_a_multi_byte_character() {
    // @step Given turn content of 1999 ASCII bytes followed by "😀" and more text
    let ascii = "t".repeat(1999);
    let content = format!("{ascii}😀{}", "u".repeat(100));
    // @step When the content is truncated for a quoted-context block
    let truncated = truncate_context_turn(content);
    // @step Then it is the 1999 ASCII bytes followed by "... [truncated]"
    assert_eq!(truncated, format!("{ascii}... [truncated]"));
}
