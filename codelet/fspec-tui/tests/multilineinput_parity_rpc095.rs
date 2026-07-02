//! Feature: spec/features/agentview-multilineinput-parity.feature
//!
//! RPC-095 — AgentView MultiLineInput parity: spinner/busy, placeholder, blocking, Esc cascade.
//!
//! This integration test exercises the *pure* helpers from the new
//! `views/agent/spinner.rs`, `views/agent/input_transition.rs`, the
//! `InputGate` extension to `multiline_input.rs`, and the L6
//! input-clear branch added to `app/dispatch_esc_cascade.rs`.
//!
//! Scenarios mapped 1:1 to feature file. Tests must FAIL before
//! implementation lands (red phase).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::input_transition::{
    render_input_transition, InputTransitionState,
};
use codelet_fspec_tui::views::agent::multiline_input::{
    InputEventOutcome, InputGate, MultiLineInput,
};
use codelet_fspec_tui::views::agent::spinner::{
    current_frame_glyph, paint_spinner_line, DOTS_FRAMES, DOTS_INTERVAL_MS,
};
use codelet_fspec_tui::views::agent::INPUT_PLACEHOLDER_HINT;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

// --------------------------------------------------------------------
// Scenario: Running session paints Thinking spinner on first frame
// --------------------------------------------------------------------
#[test]
fn running_session_paints_thinking_spinner_first_frame() {
    // @step Given I am viewing the AgentView for a session whose status has just become Running
    // @step And the spinner elapsed-time counter is zero
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    let state = InputTransitionState::Loading { elapsed_ms: 0 };

    // @step When the AgentView renders the input row
    render_input_transition(area, &mut buf, &state);

    // @step Then the input row shows the text "⠋ Thinking... (Esc to stop)"
    let mut line = String::new();
    for x in area.x..area.x + area.width {
        line.push_str(buf[(x, 0)].symbol());
    }
    let trimmed = line.trim_end();
    assert!(
        trimmed.starts_with("⠋ Thinking... (Esc to stop)"),
        "expected Thinking spinner line, got {trimmed:?}"
    );

    // @step And the entire line is rendered with the DIM style modifier
    for x in area.x..area.x + (trimmed.chars().count() as u16) {
        let cell = &buf[(x, 0)];
        if cell.symbol() != " " {
            assert!(
                cell.modifier.contains(Modifier::DIM),
                "cell at x={x} ({:?}) missing DIM modifier",
                cell.symbol()
            );
        }
    }
}

// --------------------------------------------------------------------
// Scenario: Running session advances spinner frame at 80ms cadence
// --------------------------------------------------------------------
#[test]
fn spinner_advances_one_frame_per_80ms() {
    // @step Given I am viewing the AgentView for a Running session
    // @step When 240 milliseconds have elapsed since the spinner started
    let glyph = current_frame_glyph(240);

    // @step Then the spinner glyph in the input row is "⠸"
    assert_eq!(glyph, "⠸");
    assert_eq!(DOTS_FRAMES[3], "⠸");
    assert_eq!(DOTS_INTERVAL_MS, 80);

    // @step And the spinner message remains "Thinking... (Esc to stop)"
    // (covered by the input_transition render path — see prior test.)
}

// --------------------------------------------------------------------
// Scenario: Compacting session paints Compacting spinner on first frame
// --------------------------------------------------------------------
#[test]
fn compacting_session_paints_compacting_spinner_first_frame() {
    // @step Given I am viewing the AgentView for a session whose status has just become Compacting
    // @step And the spinner elapsed-time counter is zero
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    let state = InputTransitionState::Compacting { elapsed_ms: 0 };

    // @step When the AgentView renders the input row
    render_input_transition(area, &mut buf, &state);

    // @step Then the input row shows the text "⠋ Compacting... (Esc to stop)"
    let mut line = String::new();
    for x in area.x..area.x + area.width {
        line.push_str(buf[(x, 0)].symbol());
    }
    let trimmed = line.trim_end();
    assert!(
        trimmed.starts_with("⠋ Compacting... (Esc to stop)"),
        "expected Compacting spinner line, got {trimmed:?}"
    );

    // @step And the entire line is rendered with the DIM style modifier
    let cell = &buf[(area.x, 0)];
    assert!(cell.modifier.contains(Modifier::DIM));
}

// --------------------------------------------------------------------
// Scenario: Compacting blocks printable character insertion
// --------------------------------------------------------------------
#[test]
fn compacting_blocks_printable_insert() {
    // @step Given I am viewing the AgentView for a Compacting session
    // @step And the input buffer contains the text "hello"
    let mut input = MultiLineInput::new();
    input.set_value("hello");
    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };

    // @step When I press the printable character "a"
    let outcome = input.handle_key_gated(KeyCode::Char('a'), KeyModifiers::NONE, gate);

    // @step Then the input buffer still contains exactly "hello"
    assert_eq!(input.value(), "hello");
    // @step And no submit action is dispatched
    assert!(matches!(outcome, InputEventOutcome::Continued));
}

// --------------------------------------------------------------------
// Scenario: Compacting blocks Backspace
// --------------------------------------------------------------------
#[test]
fn compacting_blocks_backspace() {
    // @step Given I am viewing the AgentView for a Compacting session
    // @step And the input buffer contains the text "hello"
    let mut input = MultiLineInput::new();
    input.set_value("hello");
    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };

    // @step When I press Backspace
    let _ = input.handle_key_gated(KeyCode::Backspace, KeyModifiers::NONE, gate);

    // @step Then the input buffer still contains exactly "hello"
    assert_eq!(input.value(), "hello");
}

// --------------------------------------------------------------------
// Scenario: Compacting blocks Delete and forward-delete
// --------------------------------------------------------------------
#[test]
fn compacting_blocks_delete() {
    // @step Given I am viewing the AgentView for a Compacting session
    // @step And the input buffer contains the text "hello"
    // @step And the cursor is at position 0
    let mut input = MultiLineInput::new();
    input.set_value("hello");
    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };

    // @step When I press Delete
    let _ = input.handle_key_gated(KeyCode::Delete, KeyModifiers::NONE, gate);

    // @step Then the input buffer still contains exactly "hello"
    assert_eq!(input.value(), "hello");
}

// --------------------------------------------------------------------
// Scenario: Compacting swallows Enter so input is not submitted
// --------------------------------------------------------------------
#[test]
fn compacting_swallows_enter() {
    // @step Given I am viewing the AgentView for a Compacting session
    // @step And the input buffer contains the text "hello"
    let mut input = MultiLineInput::new();
    input.set_value("hello");
    let gate = InputGate {
        block_edits: true,
        suppress_enter: true,
    };

    // @step When I press Enter
    let outcome = input.handle_key_gated(KeyCode::Enter, KeyModifiers::NONE, gate);

    // @step Then no submit action is dispatched
    assert!(!matches!(outcome, InputEventOutcome::Submitted(_)));
    // @step And the input buffer still contains exactly "hello"
    assert_eq!(input.value(), "hello");
}

// --------------------------------------------------------------------
// Scenario: Cursor movement still works during Running
// --------------------------------------------------------------------
#[test]
fn running_preserves_cursor_movement_and_spinner_animates() {
    // @step Given I am viewing the AgentView for a Running session
    // @step And the input buffer contains the text "hello"
    // @step And the cursor is at position 0
    let mut input = MultiLineInput::new();
    input.set_value("hello");
    // Move cursor home first.
    input.move_cursor_home();
    assert_eq!(input.cursor(), (0, 0));

    // Gate during Running: edits NOT blocked (Running !== Compacting),
    // suppress_enter NOT set when no popup active.
    let gate = InputGate {
        block_edits: false,
        suppress_enter: false,
    };

    // @step When I press the Right arrow key
    let _ = input.handle_key_gated(KeyCode::Right, KeyModifiers::NONE, gate);

    // @step Then the cursor moves to position 1
    assert_eq!(input.cursor(), (0, 1));

    // @step And the spinner continues to animate
    //   (frame picker is pure — advancing elapsed_ms gives a new glyph.)
    assert_ne!(current_frame_glyph(0), current_frame_glyph(80));
}

// --------------------------------------------------------------------
// Scenario: Idle placeholder text is verbatim
// --------------------------------------------------------------------
#[test]
fn idle_placeholder_text_verbatim() {
    // @step Given I am viewing the AgentView for an Idle session
    // @step And the input buffer is empty
    // @step When the AgentView renders the input row
    // @step Then the input row shows the placeholder text "Type a message..."

    // The placeholder constant gained the RPC-402 Shift+Enter newline
    // hint (leading, so it survives 80-col truncation; the whole
    // string fits a 100-col render) — verbatim assert.
    assert_eq!(
        INPUT_PLACEHOLDER_HINT,
        "Type a message... 'Shift+Enter' newline, 'Shift+↑/↓' history, 'Shift+←/→' sessions, 'Tab' turns"
    );
}

// --------------------------------------------------------------------
// Scenario: Dangling unused constant PLACEHOLDER_FOOTER_HINTS removed
// --------------------------------------------------------------------
// NOTE: This scenario was descoped during specifying. The RPC-013
// source-shape test pins the existence of the "Enter=send" literal in
// views/agent.rs (see codelet/fspec-tui/tests/source_shape_rpc013.rs:124-130).
// Removing PLACEHOLDER_FOOTER_HINTS would break that historic
// regression test. Filed as documentation-only — the constant remains
// dead-wired and harmless.

// --------------------------------------------------------------------
// Scenario: New modules stay under the 300-LoC source-shape ceiling
// --------------------------------------------------------------------
#[test]
fn new_modules_stay_under_300_loc() {
    // @step Given codelet/fspec-tui/src/views/agent/spinner.rs exists
    let spinner_src = include_str!("../src/views/agent/spinner.rs");

    // @step And codelet/fspec-tui/src/views/agent/input_transition.rs exists
    let transition_src = include_str!("../src/views/agent/input_transition.rs");

    // @step When the source-shape test runs
    let spinner_loc = spinner_src.lines().count();
    let transition_loc = transition_src.lines().count();

    // @step Then both files have fewer than 300 lines of code
    assert!(
        spinner_loc < 300,
        "spinner.rs has {spinner_loc} lines (>= 300)"
    );
    assert!(
        transition_loc < 300,
        "input_transition.rs has {transition_loc} lines (>= 300)"
    );
}

// --------------------------------------------------------------------
// Spinner unit: pure frame-picker is verbatim TS math
// --------------------------------------------------------------------
#[test]
fn spinner_frames_match_typescript_dots_set() {
    assert_eq!(
        DOTS_FRAMES,
        ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    );
}

#[test]
fn spinner_painter_paints_into_target_area() {
    let area = Rect::new(2, 5, 40, 1);
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 10));
    paint_spinner_line(area, &mut buf, 0, "Thinking", "(Esc to stop)");

    // First cell of `area` should be the spinner glyph.
    assert_eq!(buf[(2, 5)].symbol(), "⠋");
    // Cell BEFORE area should be untouched.
    assert_eq!(buf[(1, 5)].symbol(), " ");
}
