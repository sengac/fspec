// Feature: spec/features/agent-input-multiline-newline-keys.feature
//! RPC-402 — Shift+Enter / Alt+Enter newline behavior in the agent-view
//! MultiLineInput.
//!
//! Scenarios 1–5 of the feature file pin the contract:
//!
//!   - Shift+Enter splits the line at the cursor (Continued).
//!   - Alt+Enter inserts a newline INTENTIONALLY (explicit branch, not
//!     the accidental tui-textarea fallthrough) and never submits.
//!   - Plain Enter submits the full multi-line buffer joined by '\n'.
//!   - KeyEventKind::Release events are ignored on the REAL dispatch
//!     path (App::handle_event → Navigator → AgentView dispatch.rs).
//!   - The RPC-095 compacting gate swallows Shift+Enter without
//!     mutating the buffer.
//!
//! Scenarios 6–7 (terminal keyboard-enhancement flag plumbing) live in
//! tests/terminal_keyboard_enhancement_rpc402.rs.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::multiline_input::{
    InputEventOutcome, InputGate, MultiLineInput,
};
use codelet_fspec_tui::Action;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

mod common;

use common::harness::AppTestHarness;

/// Build a full crossterm key Event with an explicit `kind`, so tests
/// can deliver Release events the way the kitty keyboard protocol does
/// once enhancement flags are pushed (RPC-402).
fn key_event_with_kind(code: KeyCode, mods: KeyModifiers, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: mods,
        kind,
        state: KeyEventState::NONE,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Enter mid-word splits the line and grows the input to
//           2 rows
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn shift_enter_mid_word_splits_the_line_and_grows_the_input_to_2_rows() {
    // @step Given the agent input contains "hello world" with the cursor between "hello " and "world"
    let mut input = MultiLineInput::new();
    input.set_value("hello world");
    // set_value parks the cursor at the end; walk left over "world"
    // (5 chars) so the cursor sits between "hello " and "world".
    for _ in 0..5 {
        let _ = input.handle_key(KeyCode::Left, KeyModifiers::NONE);
    }
    assert_eq!(
        input.cursor(),
        (0, 6),
        "precondition: cursor must sit between \"hello \" and \"world\""
    );

    // @step When I press Shift+Enter
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "Shift+Enter must be handled internally (Continued), got {outcome:?}"
    );

    // @step Then the input buffer contains "hello " on the first line and "world" on the second line
    assert_eq!(
        input.value(),
        "hello \nworld",
        "buffer must be split at the cursor into two logical lines"
    );

    // @step And the cursor is at the start of the second line
    assert_eq!(
        input.cursor(),
        (1, 0),
        "cursor must land at row 1, col 0 after the newline"
    );

    // @step And the input area reports 2 visible rows
    assert_eq!(input.visible_rows(), 2, "input must auto-grow to 2 rows");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Alt+Enter inserts a newline instead of submitting
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn alt_enter_inserts_a_newline_instead_of_submitting() {
    // @step Given the agent input contains "first line" with the cursor at the end
    let mut input = MultiLineInput::new();
    input.set_value("first line");
    assert_eq!(
        input.cursor(),
        (0, 10),
        "precondition: cursor must be at the end of \"first line\""
    );

    // @step When I press Alt+Enter
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::ALT);

    // @step Then a newline is inserted at the cursor
    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "Alt+Enter must be handled as a newline edit (Continued), got {outcome:?}"
    );
    assert_eq!(
        input.value(),
        "first line\n",
        "a newline must be inserted at the cursor"
    );

    // @step And the buffer is not submitted
    assert!(
        !matches!(outcome, InputEventOutcome::Submitted(_)),
        "Alt+Enter must never submit the buffer"
    );
    assert!(
        !input.is_empty(),
        "the buffer must not have been reset by a submit"
    );

    // @step And the input area reports 2 visible rows
    assert_eq!(input.visible_rows(), 2, "input must auto-grow to 2 rows");
}

/// Supplementary (business rule 5 covers BOTH chords): Alt+Enter is an
/// EDIT — the RPC-095 compacting gate must swallow it exactly like
/// Shift+Enter. Pins the INTENTIONAL Alt+Enter branch in
/// `multiline_input_enter.rs` (the pre-RPC-402 accidental tui-textarea
/// fallthrough bypassed `gate.block_edits`). (No @step comments: this
/// augments the two gated/Alt scenarios rather than mapping 1:1.)
#[test]
fn alt_enter_is_gated_while_compacting() {
    let mut input = MultiLineInput::new();
    input.set_value("first line");

    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };
    let outcome = input.handle_key_gated(KeyCode::Enter, KeyModifiers::ALT, gate);

    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "gated Alt+Enter must be consumed (Continued), got {outcome:?}"
    );
    assert_eq!(
        input.value(),
        "first line",
        "gated Alt+Enter must not insert a newline"
    );
    assert!(
        !matches!(outcome, InputEventOutcome::Submitted(_)),
        "gated Alt+Enter must never submit"
    );
    assert_eq!(
        input.visible_rows(),
        1,
        "gated Alt+Enter must not grow the input"
    );
}

/// Supplementary (rule 2/5 review follow-up): ANY modifier-Enter combo
/// is a gated newline. Ctrl+Enter previously fell through to
/// `textarea.input()` UNGATED and could insert a newline while
/// Compacting. (No @step comments: augments the modifier-Enter
/// scenarios rather than mapping 1:1.)
#[test]
fn ctrl_enter_inserts_a_newline_instead_of_submitting() {
    let mut input = MultiLineInput::new();
    input.set_value("first line");

    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::CONTROL);

    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "Ctrl+Enter must be handled as a newline edit (Continued), got {outcome:?}"
    );
    assert_eq!(
        input.value(),
        "first line\n",
        "Ctrl+Enter must insert a newline at the cursor"
    );
}

/// Supplementary: Ctrl+Enter is an EDIT — the RPC-095 compacting gate
/// must swallow it exactly like Shift+Enter / Alt+Enter. (No @step
/// comments: augments the gated scenarios rather than mapping 1:1.)
#[test]
fn ctrl_enter_is_gated_while_compacting() {
    let mut input = MultiLineInput::new();
    input.set_value("first line");

    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };
    let outcome = input.handle_key_gated(KeyCode::Enter, KeyModifiers::CONTROL, gate);

    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "gated Ctrl+Enter must be consumed (Continued), got {outcome:?}"
    );
    assert_eq!(
        input.value(),
        "first line",
        "gated Ctrl+Enter must not insert a newline"
    );
    assert!(
        !matches!(outcome, InputEventOutcome::Submitted(_)),
        "gated Ctrl+Enter must never submit"
    );
    assert_eq!(
        input.visible_rows(),
        1,
        "gated Ctrl+Enter must not grow the input"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Plain Enter submits the full multi-line buffer and resets
//           the input
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn plain_enter_submits_the_full_multi_line_buffer_and_resets_the_input() {
    // @step Given the agent input contains 3 lines composed with Shift+Enter
    let mut input = MultiLineInput::new();
    for (i, line) in ["line1", "line2", "line3"].iter().enumerate() {
        for ch in line.chars() {
            let _ = input.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
        }
        if i < 2 {
            let _ = input.handle_key(KeyCode::Enter, KeyModifiers::SHIFT);
        }
    }
    assert_eq!(
        input.value(),
        "line1\nline2\nline3",
        "precondition: 3 lines composed via Shift+Enter"
    );
    assert_eq!(input.visible_rows(), 3, "precondition: 3 visible rows");

    // @step When I press plain Enter with no modifiers
    let outcome = input.handle_key(KeyCode::Enter, KeyModifiers::NONE);

    // @step Then the submitted value is the 3 lines joined by newline characters
    match outcome {
        InputEventOutcome::Submitted(value) => {
            assert_eq!(
                value, "line1\nline2\nline3",
                "submitted value must be all 3 lines joined by '\\n'"
            );
        }
        other => panic!("plain Enter must submit; got {other:?}"),
    }

    // @step And the input buffer is empty
    assert!(input.is_empty(), "buffer must be reset after submit");
    assert_eq!(input.value(), "", "buffer content must be empty");

    // @step And the input area reports 1 visible row
    assert_eq!(input.visible_rows(), 1, "input must reset to 1 row");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Key release events are ignored by the input
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn key_release_events_are_ignored_by_the_input() {
    // Drives the REAL production path: App::handle_event → Navigator →
    // AgentView::handle_event (dispatch.rs), where the Press-kind
    // filter must drop Release/Repeat events BEFORE any branch
    // (shortcuts, chords, or the MultiLineInput).
    // @step Given the agent input contains "draft"
    let mut h = AppTestHarness::new();
    h.app.navigator_mut().agent.input.set_value("draft");

    // @step When a Shift+Enter key event with kind Release arrives
    let release = key_event_with_kind(KeyCode::Enter, KeyModifiers::SHIFT, KeyEventKind::Release);
    let _ = h.app.handle_event(&release);
    // A plain-Enter Release would SUBMIT if the kind were ignored —
    // deliver one too so a double-submit regression cannot hide.
    let plain_release =
        key_event_with_kind(KeyCode::Enter, KeyModifiers::NONE, KeyEventKind::Release);
    let _ = h.app.handle_event(&plain_release);

    // @step Then no newline is inserted
    assert!(
        !h.app.navigator().agent.input.value().contains('\n'),
        "a Release event must not insert a newline; buffer = {:?}",
        h.app.navigator().agent.input.value()
    );
    assert_eq!(
        h.app.navigator().agent.input.visible_rows(),
        1,
        "input must stay at 1 row after a Release event"
    );
    // No submit action may have been emitted on the App's action bus.
    let mut actions = Vec::new();
    while let Some(action) = h.app.try_recv_action() {
        actions.push(action);
    }
    assert!(
        !actions
            .iter()
            .any(|a| matches!(a, Action::InputSubmitted(_))),
        "a Release event must never submit; emitted actions: {actions:?}"
    );

    // @step And the input buffer still contains "draft"
    assert_eq!(
        h.app.navigator().agent.input.value(),
        "draft",
        "buffer must be unchanged by a Release event"
    );
}

/// Supplementary (rule 3, defense-in-depth): the widget-level
/// `MultiLineInput::handle_event` boundary ALSO drops Release events,
/// protecting direct callers that bypass dispatch.rs. (No @step
/// comments: the scenario maps 1:1 to the dispatch-path test above.)
#[test]
fn key_release_events_are_ignored_at_the_widget_boundary() {
    let mut input = MultiLineInput::new();
    input.set_value("draft");

    let release = key_event_with_kind(KeyCode::Enter, KeyModifiers::SHIFT, KeyEventKind::Release);
    let outcome = input.handle_event(&release);

    assert!(
        !input.value().contains('\n'),
        "a Release event must not insert a newline; buffer = {:?}",
        input.value()
    );
    assert!(
        !matches!(outcome, InputEventOutcome::Submitted(_)),
        "a Release event must never submit"
    );
    assert_eq!(
        input.value(),
        "draft",
        "buffer must be unchanged by a Release event"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Enter is swallowed while the session is compacting
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn shift_enter_is_swallowed_while_the_session_is_compacting() {
    // @step Given the agent input contains "draft" and the compacting edit gate is active
    let mut input = MultiLineInput::new();
    input.set_value("draft");
    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };

    // @step When I press Shift+Enter
    let outcome = input.handle_key_gated(KeyCode::Enter, KeyModifiers::SHIFT, gate);

    // @step Then the key is consumed without submitting
    assert!(
        matches!(outcome, InputEventOutcome::Continued),
        "gated Shift+Enter must be consumed (Continued), got {outcome:?}"
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
        "input must not grow while compacting"
    );
}
