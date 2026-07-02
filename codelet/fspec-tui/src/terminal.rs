//! Terminal lifecycle + panic hook (RPC-008 rule [13]).
//!
//! Feature: spec/features/fspec-tui-terminal.feature
//! Feature: spec/features/terminal-keyboard-enhancement-flags.feature
//!
//! `TerminalGuard::init()` enables alt-screen + raw mode + mouse capture
//! + bracketed paste, registers a `std::panic::set_hook` that restores
//!   the terminal before delegating to the previous panic hook, and
//!   returns a guard whose `Drop` re-runs the same restoration.
//!
//! RPC-402: after raw mode is enabled, `supports_keyboard_enhancement()`
//! is queried and — only when the terminal reports support — the kitty
//! keyboard-enhancement flags (`DISAMBIGUATE_ESCAPE_CODES`) are pushed
//! so modifier-carrying Enter (Shift+Enter / Alt+Enter) becomes
//! distinguishable from plain CR. The push is best-effort: a failed
//! query or a failed push never fails init. Whether flags were pushed
//! is recorded process-wide so EVERY teardown path (normal restore,
//! `Drop`, panic hook) pops them BEFORE leaving the alternate screen.
//!
//! The exact init/teardown command ordering is expressed as a pure,
//! testable plan — [`terminal_mode_plan`] — which both the real
//! enable/restore paths and the RPC-402 integration tests consume, so
//! the tests pin the real ordering.
//!
//! The panic-hook registration is idempotent — guarded by a
//! `std::sync::Once` so repeated calls to `TerminalGuard::init()` in
//! the same process re-use the previous hook chain instead of nesting.

use std::io::{stdout, Stdout};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

use anyhow::Result;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

static PANIC_HOOK_REGISTERED: Once = Once::new();

/// RPC-402: process-wide record of whether keyboard-enhancement flags
/// were actually pushed, so every teardown path (normal restore, Drop,
/// panic hook — all of which funnel into `restore_terminal_modes`)
/// knows whether to pop them. Cleared on restore so a second teardown
/// (e.g. Drop after the panic hook already restored) never issues a
/// stray pop.
static ENHANCEMENT_PUSHED: AtomicBool = AtomicBool::new(false);

/// RPC-402: one terminal-mode primitive in the init/teardown sequence.
/// The pure [`terminal_mode_plan`] emits ordered lists of these; the
/// executor [`execute_mode_command`] maps each onto the corresponding
/// crossterm call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeCommand {
    EnableRawMode,
    EnterAlternateScreen,
    EnableMouseCapture,
    EnableBracketedPaste,
    PushKeyboardEnhancement,
    PopKeyboardEnhancement,
    DisableBracketedPaste,
    DisableMouseCapture,
    LeaveAlternateScreen,
    DisableRawMode,
}

/// RPC-402: the ordered init (`setup`) and teardown (`teardown`) mode
/// command sequences for one process lifetime.
pub struct TerminalModePlan {
    pub setup: Vec<ModeCommand>,
    pub teardown: Vec<ModeCommand>,
}

/// RPC-402: pure planning function — the single source of truth for
/// terminal init/teardown ordering.
///
/// Setup: raw mode first (the `supports_keyboard_enhancement()` query
/// requires raw mode, so the push slot sits IMMEDIATELY after raw-mode
/// enable), then alt-screen + mouse capture + bracketed paste.
/// Teardown mirrors setup in reverse: the pop is issued BEFORE leaving
/// the alternate screen and before disabling raw mode. When the
/// terminal reports no support, neither push nor pop appears.
pub fn terminal_mode_plan(enhancement_supported: bool) -> TerminalModePlan {
    let mut setup = vec![ModeCommand::EnableRawMode];
    if enhancement_supported {
        setup.push(ModeCommand::PushKeyboardEnhancement);
    }
    setup.extend([
        ModeCommand::EnterAlternateScreen,
        ModeCommand::EnableMouseCapture,
        ModeCommand::EnableBracketedPaste,
    ]);

    let mut teardown = vec![
        ModeCommand::DisableBracketedPaste,
        ModeCommand::DisableMouseCapture,
    ];
    if enhancement_supported {
        teardown.push(ModeCommand::PopKeyboardEnhancement);
    }
    teardown.extend([
        ModeCommand::LeaveAlternateScreen,
        ModeCommand::DisableRawMode,
    ]);

    TerminalModePlan { setup, teardown }
}

/// RPC-402: executor — maps one [`ModeCommand`] onto the crossterm
/// primitive that realises it.
fn execute_mode_command(cmd: ModeCommand) -> Result<()> {
    let mut out = stdout();
    match cmd {
        ModeCommand::EnableRawMode => enable_raw_mode()?,
        ModeCommand::DisableRawMode => disable_raw_mode()?,
        ModeCommand::EnterAlternateScreen => execute!(out, EnterAlternateScreen)?,
        ModeCommand::LeaveAlternateScreen => execute!(out, LeaveAlternateScreen)?,
        ModeCommand::EnableMouseCapture => execute!(out, EnableMouseCapture)?,
        ModeCommand::DisableMouseCapture => execute!(out, DisableMouseCapture)?,
        ModeCommand::EnableBracketedPaste => execute!(out, EnableBracketedPaste)?,
        ModeCommand::DisableBracketedPaste => execute!(out, DisableBracketedPaste)?,
        ModeCommand::PushKeyboardEnhancement => execute!(
            out,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?,
        ModeCommand::PopKeyboardEnhancement => execute!(out, PopKeyboardEnhancementFlags)?,
    }
    Ok(())
}

/// RAII guard that owns the configured ratatui Terminal and restores
/// the terminal in `Drop`. Construction also enables alt-screen, raw
/// mode, mouse capture, and bracketed paste, and (idempotently)
/// registers a panic hook that performs the same restoration before
/// delegating to the previous panic hook.
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    /// Initialise the terminal — alt-screen + raw mode + mouse capture
    /// + bracketed paste (+ best-effort keyboard-enhancement flags,
    ///   RPC-402) — register the panic hook (once per process), and
    ///   return the guard owning the configured ratatui Terminal.
    ///
    /// All steps share the `enable_terminal_modes` helper with the
    /// panic hook so a single source of truth is responsible for what
    /// "terminal init" means.
    pub fn init() -> Result<Self> {
        register_panic_hook();
        enable_terminal_modes()?;
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Borrow the inner ratatui Terminal.
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal_modes();
    }
}

/// Enable the terminal modes RPC-008 requires (rule [13]) plus the
/// RPC-402 best-effort keyboard-enhancement push, in the order fixed
/// by [`terminal_mode_plan`].
///
/// Raw mode is enabled first, then `supports_keyboard_enhancement()`
/// is queried (it requires raw mode to be active; a query error is
/// treated as "unsupported"). The push itself is also best-effort: if
/// it errors, init proceeds without enhancement flags and teardown
/// will not pop.
fn enable_terminal_modes() -> Result<()> {
    // Raw mode FIRST — `supports_keyboard_enhancement()` needs it.
    execute_mode_command(ModeCommand::EnableRawMode)?;
    let supported = supports_keyboard_enhancement().unwrap_or(false);
    let plan = terminal_mode_plan(supported);
    for cmd in plan.setup {
        match cmd {
            // Already enabled above (the plan keeps the slot so tests
            // can assert push-comes-after-raw-mode ordering).
            ModeCommand::EnableRawMode => {}
            ModeCommand::PushKeyboardEnhancement => {
                if execute_mode_command(cmd).is_ok() {
                    ENHANCEMENT_PUSHED.store(true, Ordering::SeqCst);
                }
            }
            other => execute_mode_command(other)?,
        }
    }
    Ok(())
}

/// Restore the terminal modes RPC-008 enables (plus the RPC-402 pop
/// when flags were pushed). Used by `TerminalGuard::Drop` and by the
/// panic hook, in the order fixed by [`terminal_mode_plan`].
///
/// Teardown is BEST-EFFORT (feature doc string): every command in the
/// plan is attempted independently, so a failing
/// `DisableBracketedPaste` cannot leave the alternate screen active or
/// raw mode enabled. The first error (if any) is reported to callers
/// that care, after all commands have been attempted.
pub fn restore_terminal_modes() -> Result<()> {
    let pushed = ENHANCEMENT_PUSHED.swap(false, Ordering::SeqCst);
    let plan = terminal_mode_plan(pushed);
    let mut first_err: Option<anyhow::Error> = None;
    for cmd in plan.teardown {
        if let Err(err) = execute_mode_command(cmd) {
            first_err.get_or_insert(err);
        }
    }
    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// Idempotent panic-hook registration. The first call swaps in a new
/// hook that runs `restore_terminal_modes()` and then delegates to the
/// previous hook. Subsequent calls are no-ops.
fn register_panic_hook() {
    PANIC_HOOK_REGISTERED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = restore_terminal_modes();
            previous(info);
        }));
    });
}

/// Test-only helper: returns true iff the panic hook has been
/// registered. Used by the idempotency test.
#[cfg(test)]
pub fn panic_hook_registered() -> bool {
    PANIC_HOOK_REGISTERED.is_completed()
}
