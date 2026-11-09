//! CMPCT-088 — compaction counts are never clamped or wrapped.
//!
//! Feature: spec/features/compaction-counts-are-never-clamped.feature

use codelet_common::narrow_count;

#[test]
fn a_count_that_fits_is_narrowed_exactly() {
    // @step Given an original token count of 4294967295
    let count = u64::from(u32::MAX);
    // @step When it is narrowed for a compaction result
    let narrowed = narrow_count("original tokens", count);
    // @step Then the result is 4294967295
    assert_eq!(narrowed, Ok(u32::MAX));
}

#[test]
fn a_count_that_does_not_fit_is_an_error_naming_it() {
    // @step Given an original token count of 4294967296
    let count = u64::from(u32::MAX) + 1;
    // @step When it is narrowed for a compaction result
    let narrowed = narrow_count("original tokens", count);
    // @step Then narrowing fails naming "original tokens" and 4294967296
    let error = narrowed.err().unwrap_or_default();
    assert!(
        error.contains("original tokens") && error.contains("4294967296"),
        "{error}"
    );
}
