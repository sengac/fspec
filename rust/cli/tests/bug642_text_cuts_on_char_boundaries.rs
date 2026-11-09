//! BUG-642: the CLI tool-result preview and the thinking-exhaustion
//! recovery message cut text on a char boundary.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature

use codelet_cli::interactive::build_thinking_exhaustion_recovery_message;
use codelet_cli::interactive::output::tool_result_preview;

#[test]
fn a_tool_result_preview_cuts_before_a_multi_byte_character() {
    // @step Given a tool result of 499 ASCII bytes followed by "é" and more text
    let ascii = "a".repeat(499);
    let content = format!("{ascii}é{}", "b".repeat(100));
    // @step When the CLI builds its preview
    let preview = tool_result_preview(&content);
    // @step Then the preview is the 499 ASCII bytes followed by "..."
    assert_eq!(preview, format!("{ascii}..."));
}

#[test]
fn recovery_reasoning_cuts_before_a_multi_byte_character() {
    // @step Given captured reasoning of 1999 ASCII bytes followed by "€" and more text
    let ascii = "r".repeat(1999);
    let reasoning = format!("{ascii}€{}", "s".repeat(100));
    // @step When the thinking-exhaustion recovery message is built
    let message = build_thinking_exhaustion_recovery_message(10, 20, Some(&reasoning));
    // @step Then it quotes exactly the 1999 ASCII bytes of reasoning
    let quoted = format!("Your previous reasoning (preserved as context):\n{ascii}");
    assert!(message.ends_with(&quoted), "{message}");
}
