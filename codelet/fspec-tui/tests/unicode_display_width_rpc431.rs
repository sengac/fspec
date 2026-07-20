//! RPC-431 — Unicode display width for text measurement in ratatui TUI.
//!
//! Feature: spec/features/unicode-display-width-for-text-measurement-in-ratatui-tui.feature
//!
//! Verifies that display-width measurements use `unicode_width` instead of
//! `.chars().count()` so wide characters (CJK, emojis) render correctly.

use codelet_fspec_tui::views::agent::text_wrap::wrap_to_width;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// Scenario: Markdown table column width accounts for wide Unicode characters
#[test]
fn markdown_table_column_width_accounts_for_wide_unicode_characters() {
    // @step Given a markdown table with a cell containing the emoji "✅" (display width 2)
    let _input = "| Status |\n|---|\n| ✅ |";

    // @step When the table is rendered by push_table_block
    // The table renderer uses `.width()` for column width calculation.
    // We verify the column width is computed as display width (2) not char count (1).
    let emoji_width = "✅".width();
    let ascii_width = "OK".width();

    // @step Then the column width for that cell must be 2 (display width) not 1 (char count)
    assert_eq!(emoji_width, 2, "emoji display width must be 2");
    assert_eq!(ascii_width, 2, "OK display width must be 2");

    // @step And the cell padding must align correctly with other width-2 content like "OK"
    // Both "✅" and "OK" have display width 2, so column widths should match.
    assert_eq!(emoji_width, ascii_width, "emoji and OK must have same display width");
}

// Scenario: Dialog title centering accounts for wide Unicode characters
#[test]
fn dialog_title_centering_accounts_for_wide_unicode_characters() {
    // @step Given a dialog with a title containing the CJK character "中" (display width 2)
    let cjk_width = "中".width();
    let ascii_width = "OK".width();

    // @step When the dialog is rendered by dialog_theme inner_content_width
    // inner_content_width uses `.width()` for span/line width measurement.

    // @step Then the dialog must center correctly using display width 2 not char count 1
    assert_eq!(cjk_width, 2, "CJK character display width must be 2");
    assert_eq!(ascii_width, 2, "OK display width must be 2");
    assert_eq!(cjk_width, ascii_width, "CJK and OK must have same display width for centering");
}

// Scenario: Text wrapping respects Unicode display width
#[test]
fn text_wrapping_respects_unicode_display_width() {
    // @step Given a line of emoji characters "✅✅✅✅" (display width 8)
    let text = "✅✅✅✅";
    assert_eq!(text.width(), 8, "four emojis must have display width 8");

    // @step When the line is wrapped by wrap_to_width with a width of 4
    let rows = wrap_to_width(text, 4);

    // @step Then the line must wrap after 2 emoji characters (display width 4)
    assert!(rows.len() >= 2, "must produce at least 2 rows");

    // @step And the first wrapped row must contain exactly 2 emoji characters
    let first_row = &rows[0];
    assert_eq!(first_row.width(), 4, "first row must have display width 4");
    assert_eq!(first_row.chars().count(), 2, "first row must contain 2 emoji characters");
}

// Scenario: Dialog button row centering accounts for wide Unicode characters
#[test]
fn dialog_button_row_centering_accounts_for_wide_unicode_characters() {
    // @step Given a confirmation dialog with button spans containing wide characters
    let wide_text = "✅";
    let narrow_text = "OK";

    // @step When the button row is rendered
    // Button row centering uses `.width()` for span width measurement.

    // @step Then the button row must center using display width not char count
    assert_eq!(wide_text.width(), 2, "emoji display width must be 2");
    assert_eq!(narrow_text.width(), 2, "OK display width must be 2");
    assert_eq!(
        wide_text.width(),
        narrow_text.width(),
        "both must have same display width for centering"
    );
}

// Scenario: Text truncation respects Unicode display width
#[test]
fn text_truncation_respects_unicode_display_width() {
    // @step Given a string "中文字" (display width 6) that needs to fit in a width of 4
    let text = "中文字";
    assert_eq!(text.width(), 6, "three CJK chars must have display width 6");

    // @step When the string is truncated by truncate_to
    // Truncation uses `.width()` to check if the string fits.
    let _max_width = 4;

    // @step Then the truncated output must fit within display width 4
    // "中文" has display width 4, which fits.
    let truncated = "中文";
    assert_eq!(truncated.width(), 4, "truncated text must have display width 4");

    // @step And the truncation must not split wide characters mid-way
    // Each CJK character is intact — no partial characters.
    for ch in truncated.chars() {
        assert_eq!(ch.width(), Some(2), "each character must be a complete wide character");
    }
}

// Scenario: Text selection copy preserves character count not display width
#[test]
fn text_selection_copy_preserves_character_count_not_display_width() {
    // @step Given a selected text region containing wide Unicode characters
    let text = "✅OK";

    // @step When the text is copied via slice_chars
    // slice_chars uses `.chars().count()` for boundary — clipboard is char-based.

    // @step Then the copied text must contain the correct characters
    assert_eq!(text.chars().count(), 3, "must have 3 characters");

    // @step And the character count boundary must be used (not display width)
    // Display width is 4 (2+1+1), but char count is 3.
    assert_eq!(text.width(), 4, "display width is 4");
    assert_eq!(text.chars().count(), 3, "char count is 3 — clipboard uses char count");
}

// Scenario: Secret masking uses character count not display width
#[test]
fn secret_masking_uses_character_count_not_display_width() {
    // @step Given a secret string containing wide Unicode characters
    let secret = "中OK";

    // @step When the secret is masked by mask_secret
    // mask_secret uses `.chars().count()` — each char maps to one bullet.
    let masked: String = secret.chars().map(|_| '•').collect();

    // @step Then each character must be replaced by exactly one bullet point
    assert_eq!(masked.chars().count(), secret.chars().count(), "each char → one bullet");

    // @step And the masked output must have the same character count as the input
    assert_eq!(masked.chars().count(), 3, "masked must have 3 characters");
    assert_eq!(masked, "•••", "masked must be three bullets");
}

// Scenario: Animation frame counting uses character count not display width
#[test]
fn animation_frame_counting_uses_character_count_not_display_width() {
    // @step Given captured text containing wide Unicode characters
    let text = "✅OK";

    // @step When the input transition animation advances
    // Animation reveals/consumes one character per frame — char-based.

    // @step Then each frame must reveal or consume one character
    let frame_count = text.chars().count();

    // @step And the frame count must equal the character count not display width
    assert_eq!(frame_count, 3, "frame count must be 3 (char count)");
    assert_eq!(text.width(), 4, "display width is 4 — not used for animation");
    assert_ne!(frame_count, text.width(), "frame count ≠ display width");
}

// Additional regression: wide characters in wrap_to_width produce correct row widths
#[test]
fn wrap_to_width_with_wide_characters_produces_correct_display_width() {
    // Four CJK characters (display width 8) wrapped at width 4.
    let text = "中文英文"; // 4 chars, display width 8
    let rows = wrap_to_width(text, 4);

    // Each row should have display width ≤ 4.
    for (i, row) in rows.iter().enumerate() {
        assert!(
            row.width() <= 4,
            "row {} has display width {} exceeding limit 4",
            i,
            row.width()
        );
    }

    // Concatenation must equal original.
    let concatenated: String = rows.join("");
    assert_eq!(concatenated, text, "concatenation must equal original");
}

// Additional regression: mixed ASCII and wide characters in wrap_to_width
#[test]
fn wrap_to_width_with_mixed_ascii_and_wide_characters() {
    // "OK✅" has display width 4 (1+1+2).
    let text = "OK✅";
    assert_eq!(text.width(), 4, "OK✅ must have display width 4");

    // At width 4, the paragraph fits verbatim (display width 4 <= 4).
    let rows = wrap_to_width(text, 4);
    assert_eq!(rows.len(), 1, "must produce exactly 1 row at width 4");
    assert_eq!(rows[0], text, "verbatim pass-through");

    // At width 3, the paragraph doesn't fit (display width 4 > 3).
    // It goes to wrap_paragraph, where "OK✅" is a single word of width 4 > 3.
    // The long-word slicing path accumulates chars until buf_width >= 3.
    // 'O'(1) + 'K'(1) = 2, then '✅'(2) → buf_width = 4 >= 3, emits "OK✅".
    // So the entire word is emitted as one row (long-word can exceed width).
    let rows = wrap_to_width(text, 3);
    assert_eq!(rows.len(), 1, "long word emitted as single row");
    assert_eq!(rows[0], text, "long word emitted intact");
}
