//! BUG-642: mask_api_key cuts its prefix and suffix on char boundaries.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature

use codelet_providers::credentials::mask_api_key;

#[test]
fn a_non_ascii_api_key_is_masked_on_char_boundaries() {
    // @step Given the API key "abcde€fghijklm€€"
    let key = "abcde€fghijklm€€";
    // @step When it is masked
    let masked = mask_api_key(key);
    // @step Then the mask is "abcde••••••••€"
    assert_eq!(masked, "abcde••••••••€");
    // @step And an ASCII key "sk-ant-abcdefghijk1234" still masks as "sk-ant-••••••••1234"
    assert_eq!(
        mask_api_key("sk-ant-abcdefghijk1234"),
        "sk-ant-••••••••1234"
    );
}
