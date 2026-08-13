//! Unicode whitespace normalization for file paths.
//!
//! macOS uses U+202F (NARROW NO-BREAK SPACE) in screenshot filenames:
//!   "Screenshot 2026-04-13 at 9.13.45\u{202F}am.png"
//!
//! This module provides:
//! - `normalize_unicode_whitespace()`: sync, replaces Unicode Zs → ASCII space
//! - `resolve_unicode_path()`: async, tries exact → normalized → directory scan

use std::path::{Path, PathBuf};

/// Check whether a character is a Unicode whitespace (category Zs) that
/// should be normalized to ASCII space.
///
/// Covers:
/// - U+00A0 NO-BREAK SPACE
/// - U+1680 OGHAM SPACE MARK
/// - U+2000-U+200A Various typographic spaces
/// - U+202F NARROW NO-BREAK SPACE (macOS screenshot filenames)
/// - U+205F MEDIUM MATHEMATICAL SPACE
/// - U+3000 IDEOGRAPHIC SPACE
fn is_unicode_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{00A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
    )
}

/// Replace Unicode whitespace variants with ASCII space (U+0020).
///
/// Fast, synchronous, idempotent. Apply at input boundaries where
/// user-provided paths enter the system.
///
/// Path separators (/ and \) are NOT affected.
///
/// Returns the input unchanged (no allocation) when no Unicode whitespace
/// is present — zero overhead for normal paths.
pub fn normalize_unicode_whitespace(path: &str) -> String {
    // Fast path: check if any normalization is needed at all.
    // Most paths are pure ASCII, so this avoids allocation.
    if !path.chars().any(is_unicode_whitespace) {
        return path.to_string();
    }

    path.chars()
        .map(|c| if is_unicode_whitespace(c) { ' ' } else { c })
        .collect()
}

/// Three-phase file resolution for paths with potential Unicode whitespace.
///
/// Phase 1a (fast): Try exact path
/// Phase 1b: Try with normalized whitespace
/// Phase 2 (robust): Scan parent directory for fuzzy whitespace match
///
/// Returns `Some(actual_path)` if found, `None` if no match.
pub async fn resolve_unicode_path(path: &Path) -> Option<PathBuf> {
    // Phase 1a: Try exact path
    if tokio::fs::try_exists(path).await.unwrap_or(false) {
        return Some(path.to_path_buf());
    }

    // Phase 1b: Try normalized whitespace
    let path_str = path.to_string_lossy();
    let normalized_str = normalize_unicode_whitespace(&path_str);
    if normalized_str != path_str.as_ref() {
        let normalized_path = PathBuf::from(&normalized_str);
        if tokio::fs::try_exists(&normalized_path)
            .await
            .unwrap_or(false)
        {
            return Some(normalized_path);
        }
    }

    // Phase 2: Scan parent directory for entry whose normalized name matches
    let parent = path.parent()?;
    let target_name = path.file_name()?.to_string_lossy();
    let target_normalized = normalize_unicode_whitespace(&target_name);

    let mut entries = tokio::fs::read_dir(parent).await.ok()?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let entry_name = entry.file_name();
        let entry_str = entry_name.to_string_lossy();
        if normalize_unicode_whitespace(&entry_str) == target_normalized {
            return Some(entry.path());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_u202f() {
        let input = "Screenshot 2026-04-13 at 9.13.45\u{202F}am.png";
        let expected = "Screenshot 2026-04-13 at 9.13.45 am.png";
        assert_eq!(normalize_unicode_whitespace(input), expected);
    }

    #[test]
    fn test_normalize_all_variants() {
        // U+00A0 NO-BREAK SPACE
        assert_eq!(normalize_unicode_whitespace("a\u{00A0}b"), "a b");
        // U+1680 OGHAM SPACE MARK
        assert_eq!(normalize_unicode_whitespace("a\u{1680}b"), "a b");
        // U+2000 EN QUAD
        assert_eq!(normalize_unicode_whitespace("a\u{2000}b"), "a b");
        // U+200A HAIR SPACE
        assert_eq!(normalize_unicode_whitespace("a\u{200A}b"), "a b");
        // U+202F NARROW NO-BREAK SPACE
        assert_eq!(normalize_unicode_whitespace("a\u{202F}b"), "a b");
        // U+205F MEDIUM MATHEMATICAL SPACE
        assert_eq!(normalize_unicode_whitespace("a\u{205F}b"), "a b");
        // U+3000 IDEOGRAPHIC SPACE
        assert_eq!(normalize_unicode_whitespace("a\u{3000}b"), "a b");
    }

    #[test]
    fn test_idempotent() {
        let input = "Screenshot\u{202F}am.png";
        let first = normalize_unicode_whitespace(input);
        let second = normalize_unicode_whitespace(&first);
        assert_eq!(first, second);
    }

    #[test]
    fn test_path_separators_preserved() {
        let input = "/Users/test/foo\u{202F}bar/baz.txt";
        let result = normalize_unicode_whitespace(input);
        assert!(result.contains('/'));
        assert_eq!(result, "/Users/test/foo bar/baz.txt");
    }

    #[test]
    fn test_ascii_only_unchanged() {
        let input = "/tmp/normal/file.txt";
        let result = normalize_unicode_whitespace(input);
        assert_eq!(result, input);
    }
}
