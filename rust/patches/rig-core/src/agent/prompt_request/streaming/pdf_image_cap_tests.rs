//! Feature: spec/features/defense-in-depth-pdf-image-count-cap-in-rig-patch.feature
//!
//! BUG-168 tests for the image count cap on PDF-shape tool results, moved out of
//! streaming.rs's inline test module so each test file maps one feature (COV-075).

use super::*;
use base64::Engine as _;

// =========================================================================
// BUG-168: Defense-in-depth image count cap on PDF-shape tool results
// Feature: spec/features/defense-in-depth-pdf-image-count-cap-in-rig-patch.feature
//
// These tests exercise `parse_tool_result_content` end-to-end for the
// pages-array cap and `pdf_pages_to_contents` directly. All four scenarios
// from the feature file are covered one-to-one, with @step comments
// matching the Gherkin step text.
// =========================================================================

/// Helper: build a PNG header base64 with the given dimensions.
fn bug168_png_header_b64(width: u32, height: u32) -> String {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0]);
    base64::engine::general_purpose::STANDARD.encode(&bytes)
}

/// Helper: build a PDF-shape pages array with `count` valid small pages.
fn bug168_pages_array(count: usize) -> serde_json::Value {
    let pages: Vec<serde_json::Value> = (1..=count)
        .map(|i| {
            serde_json::json!({
                "page_number": i,
                "data": bug168_png_header_b64(100, 100),
                "media_type": "image/png"
            })
        })
        .collect();
    serde_json::Value::Array(pages)
}

/// Scenario: A PDF-shape result exceeding the cap is truncated with a continue notice
#[test]
fn test_bug_168_pdf_shape_result_exceeding_cap_is_truncated_with_continue_notice() {
    // @step Given parse_tool_result_content is called with a result containing 67 pages
    let result_json = serde_json::json!({
        "path": "doc.pdf",
        "total_pages": 67,
        "pages": bug168_pages_array(67)
    })
    .to_string();
    let result = parse_tool_result_content(&result_json);

    // @step When the helper converts the pages array to ToolResultContent parts
    let images = result
        .iter()
        .filter(|c| matches!(c, ToolResultContent::Image(_)))
        .count();
    let texts = result
        .iter()
        .filter(|c| matches!(c, ToolResultContent::Text(_)))
        .count();

    // @step Then at most MAX_TOOL_RESULT_PDF_PAGES (20) Image parts are returned
    assert_eq!(
        images, MAX_TOOL_RESULT_PDF_PAGES,
        "image count must equal the cap"
    );

    // @step And a trailing text part states how many pages were dropped and the next offset
    assert_eq!(texts, 1, "exactly one trailing truncation text part");
    let trailing = result.last().expect("parts must be non-empty");
    match trailing {
        ToolResultContent::Text(t) => {
            assert!(
                t.text.contains("47"),
                "truncation notice must state the dropped count: {}",
                t.text
            );
            assert!(
                t.text.contains("offset=21"),
                "truncation notice must name the next offset: {}",
                t.text
            );
        }
        other => panic!("trailing part must be Text, got {other:?}"),
    }
}

/// Scenario: A PDF-shape result at or under the cap is unchanged
#[test]
fn test_bug_168_pdf_shape_result_at_or_under_cap_is_unchanged() {
    // @step Given parse_tool_result_content is called with a result containing 4 pages
    let result_json = serde_json::json!({
        "path": "doc.pdf",
        "total_pages": 4,
        "pages": bug168_pages_array(4)
    })
    .to_string();
    let result = parse_tool_result_content(&result_json);

    // @step When the helper converts the pages array to ToolResultContent parts
    let images = result
        .iter()
        .filter(|c| matches!(c, ToolResultContent::Image(_)))
        .count();

    // @step Then exactly 4 Image parts are returned
    assert_eq!(images, 4, "all 4 pages must become Image parts");

    // @step And no truncation text part is present
    assert_eq!(
        result.len(),
        4,
        "no extra parts for a payload under the cap"
    );
}

/// Scenario: The nested-text variant obeys the same cap
#[test]
fn test_bug_168_nested_text_variant_obeyes_the_same_cap() {
    // @step Given parse_tool_result_content is called with a nested-text result whose content holds 67 pages
    let inner = serde_json::json!({
        "path": "doc.pdf",
        "total_pages": 67,
        "pages": bug168_pages_array(67)
    })
    .to_string();
    let envelope = serde_json::json!({"type": "text", "content": inner}).to_string();
    let result = parse_tool_result_content(&envelope);

    // @step When the helper converts the nested pages array to ToolResultContent parts
    let images = result
        .iter()
        .filter(|c| matches!(c, ToolResultContent::Image(_)))
        .count();

    // @step Then at most MAX_TOOL_RESULT_PDF_PAGES (20) Image parts are returned
    assert_eq!(
        images, MAX_TOOL_RESULT_PDF_PAGES,
        "nested variant must obey the cap"
    );

    // @step And a trailing text part states how many pages were dropped and the next offset
    let trailing = result.last().expect("parts must be non-empty");
    match trailing {
        ToolResultContent::Text(t) => {
            assert!(t.text.contains("47"), "truncation notice: {}", t.text);
            assert!(
                t.text.contains("offset=21"),
                "truncation notice: {}",
                t.text
            );
        }
        other => panic!("trailing part must be Text, got {other:?}"),
    }
}

/// Scenario: The per-image dimension defense remains independent of the count cap
#[test]
fn test_bug_168_per_image_dimension_defense_remains_independent_of_count_cap() {
    // @step Given parse_tool_result_content is called with a result whose first page is oversized (fails EXT-016)
    let mut pages: Vec<serde_json::Value> = vec![serde_json::json!({
        "page_number": 1,
        "data": bug168_png_header_b64(10000, 10000),
        "media_type": "image/png"
    })];
    pages.extend(
        (2..=67)
            .map(|i| {
                serde_json::json!({
                    "page_number": i,
                    "data": bug168_png_header_b64(100, 100),
                    "media_type": "image/png"
                })
            })
            .collect::<Vec<_>>(),
    );
    let result_json = serde_json::json!({
        "path": "doc.pdf",
        "total_pages": 67,
        "pages": pages
    })
    .to_string();
    let result = parse_tool_result_content(&result_json);

    // @step When the helper converts the pages array
    let images = result
        .iter()
        .filter(|c| matches!(c, ToolResultContent::Image(_)))
        .count();

    // @step Then the oversized page is still rejected by the existing dimension check
    let first = &result[0];
    match first {
        ToolResultContent::Text(t) => {
            assert!(
                t.text.contains("10000"),
                "dimension rejection must be first: {}",
                t.text
            );
        }
        other => panic!("first page must be rejected as Text, got {other:?}"),
    }

    // @step And the count cap is applied to the remaining pages independently
    assert_eq!(
        images, MAX_TOOL_RESULT_PDF_PAGES,
        "the cap counts Image parts only, not rejections"
    );
}
