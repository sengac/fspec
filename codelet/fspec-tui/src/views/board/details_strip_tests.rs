//! Unit tests for `details_strip.rs`, split into a `#[path]`-included sibling
//! so the parent stays under the 300-LoC source-shape ceiling while keeping
//! canonical rustfmt formatting.

#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn wrap_short_text_fits_on_one_line() {
    let (a, b) = wrap_to_two_lines("hello world", 20);
    assert_eq!(a, "hello world");
    assert_eq!(b, "");
}

#[test]
fn wrap_long_text_breaks_on_whitespace_for_line_one() {
    // 25 chars — line1 fits 20 wide, remainder ("dog") fits line2.
    let (a, b) = wrap_to_two_lines("the quick brown fox done", 20);
    assert_eq!(a, "the quick brown fox");
    assert_eq!(b, "done");
}

#[test]
fn wrap_overflow_truncates_line_two_with_ellipsis() {
    let (a, b) = wrap_to_two_lines(
        "the quick brown fox jumps over the lazy dog one two three four",
        20,
    );
    assert_eq!(a, "the quick brown fox");
    // line2 must be exactly `width` chars long and end with `…`.
    assert_eq!(b.chars().count(), 20);
    assert!(b.ends_with('…'));
}

#[test]
fn wrap_hard_break_when_first_word_exceeds_width() {
    let (a, b) = wrap_to_two_lines("supercalifragilisticexpialidocious", 10);
    assert_eq!(a.chars().count(), 10);
    assert!(!b.is_empty());
}
