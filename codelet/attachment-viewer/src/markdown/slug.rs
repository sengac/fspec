//! GitHub-style heading slug generation — port of `marked-gfm-heading-id`.
//!
//! Given a heading's plain-text content, [`slugify`] produces an anchor id that
//! matches GitHub's behavior: lowercase, drop characters that are not
//! alphanumeric/space/hyphen, collapse whitespace runs into a single hyphen, and
//! de-duplicate repeats within one document via `-1`, `-2`, … suffixes.

use std::collections::HashMap;

/// Convert heading `text` into a unique GitHub-style slug.
///
/// `seen` is a document-scoped counter: the first occurrence of a base slug is
/// emitted as-is; each subsequent identical base slug gets the next numeric
/// suffix (`-1`, `-2`, …).
pub fn slugify(text: &str, seen: &mut HashMap<String, u32>) -> String {
    let base = base_slug(text);
    match seen.get_mut(&base) {
        Some(count) => {
            *count += 1;
            format!("{base}-{count}")
        }
        None => {
            seen.insert(base.clone(), 0);
            base
        }
    }
}

/// Compute the (non-deduplicated) base slug for a heading's text content.
fn base_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_hyphen = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            // Defer emitting a hyphen until a real character follows, so leading
            // and trailing whitespace never produce stray hyphens.
            if !out.is_empty() {
                pending_hyphen = true;
            }
            continue;
        }
        if ch.is_alphanumeric() || ch == '-' {
            if pending_hyphen {
                out.push('-');
                pending_hyphen = false;
            }
            out.extend(ch.to_lowercase());
        }
        // All other characters (?, !, ', ., :, parentheses, …) are dropped.
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_word_lowercases() {
        let mut seen = HashMap::new();
        assert_eq!(slugify("Summary", &mut seen), "summary");
    }

    #[test]
    fn multi_word_hyphenates() {
        let mut seen = HashMap::new();
        assert_eq!(
            slugify("Domain-to-Tag Mapping Rules", &mut seen),
            "domain-to-tag-mapping-rules"
        );
    }

    #[test]
    fn special_characters_are_stripped() {
        let mut seen = HashMap::new();
        assert_eq!(slugify("What's New?", &mut seen), "whats-new");
    }

    #[test]
    fn duplicates_get_numeric_suffixes() {
        let mut seen = HashMap::new();
        assert_eq!(slugify("Summary", &mut seen), "summary");
        assert_eq!(slugify("Summary", &mut seen), "summary-1");
        assert_eq!(slugify("Summary", &mut seen), "summary-2");
    }
}
