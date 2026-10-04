//! MENU-001 — dropdown row text helpers: key-column formatting and the
//! description truncation.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R7).
//!
//! [`key`] pads/truncates a registry key hint into the 4-cell dim
//! key-hint column; [`truncate`] caps a description at `max_width`
//! display columns, appending `…` when it was cut (the ellipsis counts
//! toward `max_width`).

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The 4-cell dim key-hint column (R7).
pub const KEY_COL: u16 = 4;

/// `key` padded/truncated into the 4-cell key column.
pub fn key(key: &str) -> String {
    let width = key.width();
    if width >= KEY_COL as usize {
        truncate(key, KEY_COL as usize)
    } else {
        format!("{key:width$}", width = KEY_COL as usize)
    }
}

/// Truncate `text` to at most `max_width` display columns, appending
/// `…` when it was cut (the ellipsis counts toward `max_width`).
pub fn truncate(text: &str, max_width: usize) -> String {
    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    let budget = max_width - 1; // leave room for the ellipsis
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + w > budget {
            break;
        }
        out.push(ch);
        used += w;
    }
    // Never end on a dangling space before the ellipsis.
    while out.ends_with(' ') {
        out.pop();
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_descriptions_truncate_with_an_ellipsis() {
        // @step And it shows a dim description truncated with an ellipsis when the row is too wide
        let desc = truncate("Open the attachment picker for the selected work unit", 10);
        assert_eq!(desc, "Open the…");
        assert_eq!(desc.width(), 9, "truncation stays within the budget");
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abc", 0), "");
        assert_eq!(truncate("exactly-10", 10), "exactly-10");
    }

    #[test]
    fn key_hints_fit_the_four_cell_column() {
        assert_eq!(key("."), ".   ");
        assert_eq!(key("Esc"), "Esc ");
        assert_eq!(key("Esc"), "Esc ", "short keys pad to 4 cells");
    }
}
