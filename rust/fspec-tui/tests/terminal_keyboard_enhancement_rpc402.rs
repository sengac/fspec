// Feature: spec/features/terminal-keyboard-enhancement-flags.feature
//! RPC-402 — TerminalGuard keyboard-enhancement flag plumbing,
//! scenarios 6–7 of the feature file.
//!
//! `TerminalGuard::init` performs real terminal I/O and cannot run in
//! CI, so these tests target the seam in `terminal.rs`: the pure
//! planning function `terminal_mode_plan` that, given whether the
//! terminal reports keyboard-enhancement support (via crossterm's
//! `supports_keyboard_enhancement()`, queried AFTER raw mode is
//! enabled), returns the ORDERED list of mode commands for setup and
//! teardown. Both `TerminalGuard::init` / `restore_terminal_modes` and
//! these tests consume the same plan, so asserting on the plan pins
//! the real init/teardown ordering.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::terminal::{terminal_mode_plan, ModeCommand};

/// Index of the first occurrence of `cmd` in `list`, or None.
fn position_of(list: &[ModeCommand], cmd: ModeCommand) -> Option<usize> {
    list.iter().position(|c| *c == cmd)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Terminal without keyboard-enhancement support skips flag
//           push and pop
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn terminal_without_keyboard_enhancement_support_skips_flag_push_and_pop() {
    // @step Given the terminal does not support keyboard enhancement flags
    let supported = false;

    // @step When the terminal modes are initialized
    let plan = terminal_mode_plan(supported);

    // @step Then keyboard enhancement flags are not pushed
    assert!(
        position_of(&plan.setup, ModeCommand::PushKeyboardEnhancement).is_none(),
        "setup must not push keyboard enhancement flags on an unsupported terminal; got {:?}",
        plan.setup
    );

    // @step And teardown does not issue a pop of keyboard enhancement flags
    assert!(
        position_of(&plan.teardown, ModeCommand::PopKeyboardEnhancement).is_none(),
        "teardown must not pop keyboard enhancement flags when none were pushed; got {:?}",
        plan.teardown
    );
    // The app must otherwise work as before: the pre-RPC-402 modes are
    // all still present in the plan.
    for cmd in [
        ModeCommand::EnableRawMode,
        ModeCommand::EnterAlternateScreen,
        ModeCommand::EnableMouseCapture,
        ModeCommand::EnableBracketedPaste,
    ] {
        assert!(
            position_of(&plan.setup, cmd).is_some(),
            "setup must still include {cmd:?}; got {:?}",
            plan.setup
        );
    }
    for cmd in [
        ModeCommand::DisableBracketedPaste,
        ModeCommand::DisableMouseCapture,
        ModeCommand::LeaveAlternateScreen,
        ModeCommand::DisableRawMode,
    ] {
        assert!(
            position_of(&plan.teardown, cmd).is_some(),
            "teardown must still include {cmd:?}; got {:?}",
            plan.teardown
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Supporting terminal pushes flags on init and pops them
//           before leaving the alternate screen
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn supporting_terminal_pushes_flags_on_init_and_pops_them_before_leaving_the_alternate_screen() {
    // @step Given the terminal supports keyboard enhancement flags
    let supported = true;

    // @step When the terminal modes are initialized and then torn down
    let plan = terminal_mode_plan(supported);

    // @step Then keyboard enhancement flags are pushed after raw mode is enabled
    let raw_idx =
        position_of(&plan.setup, ModeCommand::EnableRawMode).expect("setup must enable raw mode");
    let push_idx = position_of(&plan.setup, ModeCommand::PushKeyboardEnhancement)
        .expect("setup must push keyboard enhancement flags on a supporting terminal");
    assert!(
        push_idx > raw_idx,
        "flags must be pushed AFTER raw mode is enabled \
         (supports_keyboard_enhancement() requires raw mode); \
         push at {push_idx}, raw enable at {raw_idx}: {:?}",
        plan.setup
    );

    // @step And a pop of keyboard enhancement flags is issued before leaving the alternate screen
    let pop_idx = position_of(&plan.teardown, ModeCommand::PopKeyboardEnhancement)
        .expect("teardown must pop keyboard enhancement flags when they were pushed");
    let leave_idx = position_of(&plan.teardown, ModeCommand::LeaveAlternateScreen)
        .expect("teardown must leave the alternate screen");
    assert!(
        pop_idx < leave_idx,
        "flags must be popped BEFORE leaving the alternate screen; \
         pop at {pop_idx}, leave at {leave_idx}: {:?}",
        plan.teardown
    );
}
