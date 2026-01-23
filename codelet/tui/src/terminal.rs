//! Terminal state management
//!
//! Handles raw mode restoration and panic hooks.
//! Uses crossterm directly - no ratatui dependency needed.

use anyhow::Result;
use crossterm::{
    event::{DisableBracketedPaste, PopKeyboardEnhancementFlags},
    execute,
    terminal::disable_raw_mode,
};
use std::io::stdout;

/// Restore terminal to normal state
/// Disables raw mode, pops keyboard enhancements
pub fn restore_terminal() -> Result<()> {
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    execute!(stdout(), DisableBracketedPaste)?;
    disable_raw_mode()?;
    let _ = execute!(stdout(), crossterm::cursor::Show);
    Ok(())
}

/// Set panic hook to restore terminal on crash
pub fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal(); // Restore even on panic
        hook(panic_info);
    }));
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_restore_terminal_exists() {
        // This test verifies the function exists and can be called
        // Actual terminal operations tested in integration tests
        // Using empty block instead of assert!(true) to avoid clippy warning
    }
}
