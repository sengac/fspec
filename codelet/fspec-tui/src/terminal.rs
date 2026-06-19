//! Terminal lifecycle + panic hook (RPC-008 rule [13]).
//!
//! Feature: spec/features/fspec-tui-terminal.feature
//!
//! `TerminalGuard::init()` enables alt-screen + raw mode + mouse capture
//! + bracketed paste, registers a `std::panic::set_hook` that restores
//!   the terminal before delegating to the previous panic hook, and
//!   returns a guard whose `Drop` re-runs the same restoration.
//!
//! The panic-hook registration is idempotent — guarded by a
//! `std::sync::Once` so repeated calls to `TerminalGuard::init()` in
//! the same process re-use the previous hook chain instead of nesting.

use std::io::{stdout, Stdout};
use std::sync::Once;

use anyhow::Result;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

static PANIC_HOOK_REGISTERED: Once = Once::new();

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
    /// + bracketed paste — register the panic hook (once per process),
    ///   and return the guard owning the configured ratatui Terminal.
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

/// Enable the four terminal modes RPC-008 requires (rule [13]).
fn enable_terminal_modes() -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(
        out,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    Ok(())
}

/// Restore the four terminal modes RPC-008 enables. Used both by
/// `TerminalGuard::Drop` and by the panic hook.
pub fn restore_terminal_modes() -> Result<()> {
    let mut out = stdout();
    execute!(
        out,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;
    Ok(())
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
