//! TUI-129 — the menu registry is the one table of shortcut keys.
//!
//! Feature: spec/features/advertised-keys-do-what-their-hints-say.feature
//!
//! Every row of [`CATEGORIES`] whose key is one character is also a
//! shortcut: on the board (views/board/keys.rs), in an open board or mux
//! dropdown (its own rows) and in the menu-bar help dialog. Letters match
//! in either case; a Ctrl or Alt chord is never a shortcut.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::items::{MenuAction, MenuEntry, CATEGORIES};

/// The character `key` types when it can be a shortcut.
fn shortcut_char(key: &KeyEvent) -> Option<char> {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match key.code {
        KeyCode::Char(typed) => Some(typed),
        _ => None,
    }
}

/// True iff `entry`'s key is the one character `typed`.
fn is_key_of(entry: &MenuEntry, typed: char) -> bool {
    let mut chars = entry.key.chars();
    match (chars.next(), chars.next()) {
        (Some(key), None) => key.eq_ignore_ascii_case(&typed),
        _ => false,
    }
}

/// The row of `category` whose key `key` presses.
pub fn row_for_key(category: usize, key: &KeyEvent) -> Option<usize> {
    let typed = shortcut_char(key)?;
    CATEGORIES
        .get(category)?
        .entries
        .iter()
        .position(|entry| is_key_of(entry, typed))
}

/// The registry row whose key `key` presses, as its index in display
/// order across every category (the menu-bar help dialog's row index).
pub fn flat_row_for_key(key: &KeyEvent) -> Option<usize> {
    let typed = shortcut_char(key)?;
    CATEGORIES
        .iter()
        .flat_map(|category| category.entries.iter())
        .position(|entry| is_key_of(entry, typed))
}

/// The action of the registry row whose key `key` presses.
pub fn shortcut_for_key(key: &KeyEvent) -> Option<MenuAction> {
    let row = flat_row_for_key(key)?;
    CATEGORIES
        .iter()
        .flat_map(|category| category.entries.iter())
        .nth(row)
        .map(|entry| entry.action)
}
