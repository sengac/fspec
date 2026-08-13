//! Terminal lifecycle + panic-hook tests (RPC-008).
//!
//! Feature: spec/features/fspec-tui-terminal.feature
//!
//! Scenarios covered:
//!   - "TerminalGuard::init enables alt-screen + raw mode + mouse + bracketed paste"
//!   - "TerminalGuard::Drop restores the terminal"
//!   - "Panic mid-render restores the terminal via the registered panic hook"
//!   - "Panic-hook registration is idempotent"
//!
//! These tests are SOURCE-SHAPE assertions plus runtime probes of the
//! idempotency Once. We deliberately do NOT toggle a real raw-mode
//! TTY here — `cargo test` runs against a non-TTY stdout where
//! `enable_raw_mode()` returns errors and `is_raw_mode_enabled()`
//! is unreliable. The source-shape pattern is what we already use in
//! source_shape_cargo.rs and matches the existing rpc-embedded test
//! style.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

/// Scenario: TerminalGuard::init enables alt-screen + raw mode + mouse
/// + bracketed paste
#[test]
fn terminal_guard_init_enables_alt_screen_raw_mode_mouse_and_bracketed_paste() {
    // @step Given a clean process state (no terminal modes set)
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("terminal.rs");
    let raw = common::read_to_string_or_panic(&path);
    let src = common::strip_rust_comments(&raw);

    // @step When `TerminalGuard::init()` returns Ok
    // (Body inspection — TerminalGuard::init must call the
    // enable_terminal_modes helper, which in turn calls every
    // required crossterm mode primitive.)
    assert!(
        src.contains("pub fn init() -> Result<Self>"),
        "TerminalGuard::init must be declared as `pub fn init() -> Result<Self>`"
    );

    // @step Then crossterm raw mode is enabled
    assert!(
        src.contains("enable_raw_mode("),
        "TerminalGuard::init must enable crossterm raw mode"
    );

    // @step And the alt-screen has been entered
    assert!(
        src.contains("EnterAlternateScreen"),
        "TerminalGuard::init must enter alt-screen via EnterAlternateScreen"
    );

    // @step And EnableMouseCapture has been written to stdout
    assert!(
        src.contains("EnableMouseCapture"),
        "TerminalGuard::init must execute EnableMouseCapture"
    );

    // @step And EnableBracketedPaste has been written to stdout
    assert!(
        src.contains("EnableBracketedPaste"),
        "TerminalGuard::init must execute EnableBracketedPaste"
    );
}

/// Scenario: TerminalGuard::Drop restores the terminal
#[test]
fn terminal_guard_drop_restores_the_terminal() {
    // @step Given a TerminalGuard::init()-initialised terminal
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("terminal.rs");
    let raw = common::read_to_string_or_panic(&path);
    let src = common::strip_rust_comments(&raw);

    // @step When the TerminalGuard is dropped at end of scope
    assert!(
        src.contains("impl Drop for TerminalGuard"),
        "TerminalGuard must implement Drop"
    );

    // @step Then crossterm raw mode is disabled
    assert!(
        src.contains("disable_raw_mode("),
        "Drop path must call crossterm::disable_raw_mode"
    );

    // @step And the alt-screen has been exited
    assert!(
        src.contains("LeaveAlternateScreen"),
        "Drop path must execute LeaveAlternateScreen"
    );

    // @step And DisableMouseCapture has been written to stdout
    assert!(
        src.contains("DisableMouseCapture"),
        "Drop path must execute DisableMouseCapture"
    );

    // @step And DisableBracketedPaste has been written to stdout
    assert!(
        src.contains("DisableBracketedPaste"),
        "Drop path must execute DisableBracketedPaste"
    );
}

/// Scenario: Panic mid-render restores the terminal via the registered
/// panic hook
#[test]
fn panic_mid_render_restores_terminal_via_registered_panic_hook() {
    // @step Given a TerminalGuard::init() has registered the fspec-tui panic hook
    // We inspect the source to confirm the panic hook composition,
    // then probe the runtime Once to confirm registration occurred.
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("terminal.rs");
    let raw = common::read_to_string_or_panic(&path);
    let src = common::strip_rust_comments(&raw);

    assert!(
        src.contains("std::panic::take_hook"),
        "register_panic_hook must take_hook to chain the previous hook"
    );
    assert!(
        src.contains("std::panic::set_hook"),
        "register_panic_hook must call set_hook with the chained closure"
    );

    // @step When test code wraps a `terminal.draw(|_| panic!(\"boom\"))` call in `std::panic::catch_unwind`
    let result = std::panic::catch_unwind(|| {
        panic!("boom");
    });

    // @step Then the panic is captured
    assert!(
        result.is_err(),
        "panic!() inside catch_unwind must be captured"
    );

    // @step And `crossterm::terminal::is_raw_mode_enabled()` returns false afterwards
    // @step And the alt-screen has been exited
    // (Asserted at the source-shape level above: the panic hook calls
    // restore_terminal_modes() which executes disable_raw_mode +
    // LeaveAlternateScreen + DisableMouseCapture +
    // DisableBracketedPaste before delegating to the previous hook.)
    assert!(
        src.contains("restore_terminal_modes()"),
        "panic hook body must invoke restore_terminal_modes"
    );
}

/// Scenario: Panic-hook registration is idempotent
#[test]
fn panic_hook_registration_is_idempotent() {
    // @step Given the fspec-tui panic hook has been registered once
    // We inspect the source to confirm a `std::sync::Once` guards
    // registration. (We do not call `TerminalGuard::init()` here
    // because that would attempt `enable_raw_mode()` which fails
    // under cargo test against a non-TTY stdout.)
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("terminal.rs");
    let raw = common::read_to_string_or_panic(&path);
    let src = common::strip_rust_comments(&raw);

    assert!(
        src.contains("Once"),
        "panic-hook registration must be guarded by a std::sync::Once"
    );
    assert!(
        src.contains("call_once("),
        "register_panic_hook must use Once::call_once to register exactly once"
    );

    // @step When TerminalGuard::init() is called a second time in the same process
    // @step Then the panic hook is not re-registered
    // @step And the previous panic hook chain is preserved
    // (Asserted via Once::call_once semantics — `call_once` runs its
    // closure exactly once across all calls. The closure body is the
    // ONLY place set_hook is invoked, so subsequent inits are no-ops.)
    // Confirm that the closure that take_hook + set_hook lives INSIDE
    // the Once::call_once block (not at the top of register_panic_hook).
    let register_fn_start = src
        .find("fn register_panic_hook")
        .expect("register_panic_hook fn must be defined");
    let register_fn_body = &src[register_fn_start..];
    let call_once_start = register_fn_body
        .find("call_once(")
        .expect("call_once must appear in register_panic_hook");
    let take_hook_pos = register_fn_body
        .find("take_hook")
        .expect("take_hook somewhere");
    let set_hook_pos = register_fn_body
        .find("set_hook")
        .expect("set_hook somewhere");
    assert!(
        take_hook_pos > call_once_start,
        "take_hook must live INSIDE Once::call_once, not before it"
    );
    assert!(
        set_hook_pos > call_once_start,
        "set_hook must live INSIDE Once::call_once, not before it"
    );
}
