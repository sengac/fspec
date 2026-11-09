//! BOARD-024 — Unicode-width-aware soft wrapping that never drops visible
//! characters (only the single space a break replaces).

use unicode_width::UnicodeWidthChar;

/// Wrap one logical line to rows no wider than `width` display columns.
/// Breaks at the last space when possible, otherwise between characters.
pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    // Byte index of the last space in `current` and the width before it.
    let mut last_space: Option<(usize, usize)> = None;
    for character in text.chars() {
        let char_width = character.width().unwrap_or(0);
        if current_width + char_width > width && !current.is_empty() {
            match last_space {
                Some((index, before)) if index > 0 && character != ' ' => {
                    let rest = current.split_off(index);
                    rows.push(std::mem::take(&mut current));
                    current = rest.chars().skip(1).collect();
                    current_width -= before + 1;
                }
                _ => {
                    rows.push(std::mem::take(&mut current));
                    current_width = 0;
                    if character == ' ' {
                        last_space = None;
                        continue;
                    }
                }
            }
            last_space = None;
        }
        if character == ' ' {
            last_space = Some((current.len(), current_width));
        }
        current.push(character);
        current_width += char_width;
    }
    rows.push(current);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn wraps_at_spaces_and_keeps_every_visible_character() {
        let text = "日本語 café 🦀 wrapped words ".repeat(5);
        let rows = wrap(&text, 11);
        assert!(rows.iter().all(|row| row.width() <= 11));
        let joined: String = rows.concat().chars().filter(|c| *c != ' ').collect();
        let original: String = text.chars().filter(|c| *c != ' ').collect();
        assert_eq!(joined, original);
    }

    #[test]
    fn long_words_break_between_characters() {
        assert_eq!(wrap("abcdef", 4), vec!["abcd", "ef"]);
        assert_eq!(wrap("", 4), vec![String::new()]);
    }
}
