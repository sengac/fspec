/**
 * Feature: spec/features/shift-enter-newline-doesn-t-work-in-real-terminals-terminal-eats-modifier-no-fallback-binding-no-capability-probe.feature
 *
 * This test file validates the acceptance criteria for the agent input
 * newline bindings — Ctrl+J universal fallback with Shift+Enter best-effort.
 * Scenarios map directly to Gherkin scenarios.
 */

use crossterm::event::{KeyCode, KeyModifiers};

use codelet_fspec_tui::views::agent::multiline_input::{
    InputEventOutcome, InputGate, MultiLineInput,
};

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Ctrl+J inserts a newline and grows the input area
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn ctrl_j_inserts_a_newline_and_grows_the_input_area() {
    // @step Given the agent input contains "hello" with the cursor at the end
    let mut input = MultiLineInput::new();
    input.set_value("hello");
    assert_eq!(input.cursor(), (0, 5), "precondition: cursor at end of 'hello'");

    // @step When I press Ctrl+J
    let outcome = input.handle_key(KeyCode::Char('j'), KeyModifiers::CONTROL);

    // @step Then the input buffer contains "hello" followed by a newline
    assert_eq!(
        input.value(),
        "hello\n",
        "buffer must contain 'hello' followed by newline"
    );

    // @step And the cursor is at the start of the second line
    assert_eq!(
        input.cursor(),
        (1, 0),
        "cursor must be at start of second line"
    );

    // @step And the input area reports 2 visible rows
    assert_eq!(input.visible_rows(), 2, "input must grow to 2 rows");

    // @step And the outcome is Continued (not Submitted)
    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "Ctrl+J must be handled internally (Continued), got {outcome:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Ctrl+J mid-word splits the line at cursor position
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn ctrl_j_mid_word_splits_the_line_at_cursor_position() {
    // @step Given the agent input contains "hello world" with the cursor between "hello " and "world"
    let mut input = MultiLineInput::new();
    input.set_value("hello world");
    // Walk left over "world" (5 chars) so cursor sits between "hello " and "world"
    for _ in 0..5 {
        let _ = input.handle_key(KeyCode::Left, KeyModifiers::NONE);
    }
    assert_eq!(
        input.cursor(),
        (0, 6),
        "precondition: cursor must sit between 'hello ' and 'world'"
    );

    // @step When I press Ctrl+J
    let outcome = input.handle_key(KeyCode::Char('j'), KeyModifiers::CONTROL);

    // @step Then the input buffer contains "hello " on the first line and "world" on the second line
    assert_eq!(
        input.value(),
        "hello \nworld",
        "buffer must be split at cursor into two lines"
    );

    // @step And the cursor is at the start of the second line
    assert_eq!(
        input.cursor(),
        (1, 0),
        "cursor must be at start of second line"
    );

    // @step And the input area reports 2 visible rows
    assert_eq!(input.visible_rows(), 2, "input must grow to 2 rows");

    // @step And the outcome is Continued
    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "Ctrl+J must be Continued, got {outcome:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Plain Enter submits the multi-line buffer and resets the input
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn plain_enter_submits_the_multi_line_buffer_and_resets_the_input() {
    // @step Given the agent input contains 3 lines composed with Ctrl+J
    let mut input = MultiLineInput::new();
    input.set_value("line1");
    let _ = input.handle_key(KeyCode::Char('j'), KeyModifiers::CONTROL);
    input.set_value("line1\nline2");
    let _ = input.handle_key(KeyCode::Char('j'), KeyModifiers::CONTROL);
    input.set_value("line1\nline2\nline3");
    assert_eq!(
        input.value(),
        "line1\nline2\nline3",
        "precondition: 3 lines in buffer"
    );

    // @step When I press plain Enter with no modifiers
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // @step Then the submitted value is the 3 lines joined by newline characters
    assert!(
        matches!(&outcome, InputEventOutcome::Submitted(v) if *v == "line1\nline2\nline3"),
        "submitted value must be 3 lines joined by newlines, got {outcome:?}"
    );

    // @step And the input buffer is empty
    assert!(
        input.is_empty(),
        "input buffer must be empty after submit"
    );

    // @step And the input area reports 1 visible row
    assert_eq!(
        input.visible_rows(),
        1,
        "input must reset to 1 visible row"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Enter inserts a newline on enhanced terminals
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn shift_enter_inserts_a_newline_on_enhanced_terminals() {
    // @step Given the agent input contains "first line" with the cursor at the end
    let mut input = MultiLineInput::new();
    input.set_value("first line");
    assert_eq!(
        input.cursor(),
        (0, 10),
        "precondition: cursor at end of 'first line'"
    );

    // @step And the terminal supports keyboard enhancement
    // (simulated by passing SHIFT modifier directly)

    // @step When I press Shift+Enter
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);

    // @step Then a newline is inserted at the cursor
    assert_eq!(
        input.value(),
        "first line\n",
        "a newline must be inserted at cursor"
    );

    // @step And the buffer is not submitted
    assert!(
        !matches!(outcome, InputEventOutcome::Submitted(_)),
        "Shift+Enter must never submit the buffer"
    );

    // @step And the input area reports 2 visible rows
    assert_eq!(input.visible_rows(), 2, "input must grow to 2 rows");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Enter submits on non-enhanced terminals (modifier eaten)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn shift_enter_submits_on_non_enhanced_terminals() {
    // @step Given the agent input contains "hello" with the cursor at the end
    let mut input = MultiLineInput::new();
    input.set_value("hello");

    // @step And the terminal does not support keyboard enhancement
    // (simulated by passing Enter with NO modifiers — terminal ate SHIFT)

    // @step When I press Shift+Enter
    // Terminal sends plain Enter (no SHIFT) because it ate the modifier
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // @step Then the buffer is submitted as "hello"
    assert!(
        matches!(&outcome, InputEventOutcome::Submitted(v) if *v == "hello"),
        "buffer must be submitted as 'hello', got {outcome:?}"
    );

    // @step And the input buffer is empty
    assert!(
        input.is_empty(),
        "input buffer must be empty after submit"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Alt+Enter inserts a newline as legacy fallback
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn alt_enter_inserts_a_newline_as_legacy_fallback() {
    // @step Given the agent input contains "first line" with the cursor at the end
    let mut input = MultiLineInput::new();
    input.set_value("first line");

    // @step When I press Alt+Enter
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::ALT);

    // @step Then a newline is inserted at the cursor
    assert_eq!(
        input.value(),
        "first line\n",
        "a newline must be inserted at cursor"
    );

    // @step And the buffer is not submitted
    assert!(
        !matches!(outcome, InputEventOutcome::Submitted(_)),
        "Alt+Enter must never submit the buffer"
    );

    // @step And the input area reports 2 visible rows
    assert_eq!(input.visible_rows(), 2, "input must grow to 2 rows");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Ctrl+J is swallowed while the session is compacting
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn ctrl_j_is_swallowed_while_the_session_is_compacting() {
    // @step Given the agent input contains "draft"
    let mut input = MultiLineInput::new();
    input.set_value("draft");

    // @step And the session is compacting
    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };

    // @step When I press Ctrl+J
    let outcome = input.handle_key_gated(KeyCode::Char('j'), KeyModifiers::CONTROL, gate);

    // @step Then the key is consumed without modifying the buffer
    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "Ctrl+J must be consumed (Continued) while compacting, got {outcome:?}"
    );

    // @step And the input buffer still contains "draft"
    assert_eq!(
        input.value(),
        "draft",
        "buffer must be unchanged while compacting"
    );

    // @step And the input area reports 1 visible row
    assert_eq!(
        input.visible_rows(),
        1,
        "input must stay at 1 row while compacting"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Input placeholder shows Ctrl+J as the primary newline hint
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn input_placeholder_shows_ctrl_j_as_the_primary_newline_hint() {
    // @step Given the agent input is empty
    // (placeholder is a constant in views/agent.rs)

    // @step When the placeholder is rendered
    let placeholder = codelet_fspec_tui::views::agent::INPUT_PLACEHOLDER_HINT;

    // @step Then the placeholder text contains "Ctrl+J"
    assert!(
        placeholder.contains("Ctrl+J"),
        "placeholder must contain 'Ctrl+J', got '{placeholder}'"
    );
}
