//! MUX-010 — the MuxConfigDialog's single-line layout-string editor.
//!
//! Feature: spec/features/mux-nested-column-and-row-layouts-from-a-bracket-layout-string.feature
//!
//! A char-indexed text + cursor: printable keys insert at the cursor,
//! Left/Right/Home/End move it, Backspace/Delete edit.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The editable layout text and its cursor (a char index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutField {
    chars: Vec<char>,
    cursor: usize,
}

impl LayoutField {
    /// A field holding `text` with the cursor at the end.
    pub fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let cursor = chars.len();
        Self { chars, cursor }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Apply an editing key. Returns `Some(true)` when the text changed,
    /// `Some(false)` for a cursor move, `None` when the key is not an
    /// editing key.
    pub fn handle_key(&mut self, key: &KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.chars.insert(self.cursor, c);
                self.cursor += 1;
                Some(true)
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                self.chars.remove(self.cursor);
                Some(true)
            }
            KeyCode::Delete if self.cursor < self.chars.len() => {
                self.chars.remove(self.cursor);
                Some(true)
            }
            KeyCode::Backspace | KeyCode::Delete => Some(false),
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                Some(false)
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.chars.len());
                Some(false)
            }
            KeyCode::Home => {
                self.cursor = 0;
                Some(false)
            }
            KeyCode::End => {
                self.cursor = self.chars.len();
                Some(false)
            }
            _ => None,
        }
    }

    /// The `(before, at, after)` slices of the window of `width` cells
    /// that keeps the cursor visible (`at` is the cursor cell, a space at
    /// the end of the text).
    pub fn window(&self, width: usize) -> (String, char, String) {
        let width = width.max(1);
        let start = (self.cursor + 1).saturating_sub(width);
        let end = (start + width).min(self.chars.len());
        let before: String = self.chars[start..self.cursor].iter().collect();
        let at = self.chars.get(self.cursor).copied().unwrap_or(' ');
        let after: String = self
            .chars
            .get(self.cursor + 1..end.max(self.cursor + 1))
            .map(|s| s.iter().collect())
            .unwrap_or_default();
        (before, at, after)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn edits_at_the_cursor() {
        let mut f = LayoutField::new("board, agent");
        assert_eq!(f.handle_key(&key(KeyCode::Char(']'))), Some(true));
        assert_eq!(f.text(), "board, agent]");
        f.handle_key(&key(KeyCode::Backspace));
        f.handle_key(&key(KeyCode::Home));
        f.handle_key(&key(KeyCode::Char('[')));
        f.handle_key(&key(KeyCode::End));
        f.handle_key(&key(KeyCode::Char(']')));
        assert_eq!(f.text(), "[board, agent]");
        f.handle_key(&key(KeyCode::Home));
        f.handle_key(&key(KeyCode::Delete));
        assert_eq!(f.text(), "board, agent]");
        assert_eq!(f.handle_key(&key(KeyCode::Tab)), None);
    }

    #[test]
    fn the_window_follows_the_cursor() {
        let f = LayoutField::new("abcdefgh");
        assert_eq!(f.window(4), ("fgh".to_string(), ' ', String::new()));
        let mut g = LayoutField::new("abcdefgh");
        g.handle_key(&key(KeyCode::Home));
        assert_eq!(g.window(4), (String::new(), 'a', "bcd".to_string()));
    }
}
