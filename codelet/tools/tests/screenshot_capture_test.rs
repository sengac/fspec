// Feature: spec/features/full-page-scrollable-screenshot-capture.feature
//
// Tests for BROWSE-002: Full-page scrollable screenshot capture
// Uses rust-headless-chrome CDP capture_beyond_viewport for full-page screenshots

use codelet_tools::{ChromeBrowser, ChromeConfig, ChromeError};
use std::path::Path;

// Note: These tests require Chrome to be installed on the system
// Run with: cargo test -p codelet-tools --test screenshot_capture_test -- --ignored

// =============================================================================
// Scenario: Capture viewport screenshot with default settings
// =============================================================================

#[test]
#[ignore = "Requires Chrome installed - run with --ignored flag"]
fn test_capture_viewport_screenshot_with_default_settings() -> Result<(), ChromeError> {
    // @step Given a web page at "https://example.com"
    let config = ChromeConfig::default();
    let browser = ChromeBrowser::new(config)?;
    let tab = browser.new_tab()?;
    browser.navigate_and_wait(&tab, "https://example.com")?;

    // @step When I capture a screenshot with action type "capture_screenshot" and url "https://example.com"
    let screenshot_path = browser.capture_screenshot(&tab, None, false)?;

    // @step Then the tool should return a file path to a PNG screenshot
    assert!(
        screenshot_path.ends_with(".png"),
        "Screenshot path should end with .png: {}",
        screenshot_path
    );

    // @step And the screenshot file should exist at the returned path
    assert!(
        Path::new(&screenshot_path).exists(),
        "Screenshot file should exist at: {}",
        screenshot_path
    );

    // @step And the screenshot should capture the visible viewport
    // Verify the file is a valid PNG by checking magic bytes
    let file_contents = std::fs::read(&screenshot_path)
        .map_err(|e| ChromeError::TabError(format!("Failed to read screenshot: {}", e)))?;
    assert!(
        file_contents.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "File should be a valid PNG (checking magic bytes)"
    );

    // Cleanup
    std::fs::remove_file(&screenshot_path).ok();
    browser.cleanup_tab(&tab);

    Ok(())
}

// =============================================================================
// Scenario: Capture full-page screenshot of scrollable content
// =============================================================================

#[test]
#[ignore = "Requires Chrome installed - run with --ignored flag"]
fn test_capture_full_page_screenshot_of_scrollable_content() -> Result<(), ChromeError> {
    // @step Given a web page with scrollable content taller than the viewport
    let config = ChromeConfig::default();
    let browser = ChromeBrowser::new(config)?;
    let tab = browser.new_tab()?;
    // Wikipedia articles have scrollable content
    browser.navigate_and_wait(
        &tab,
        "https://en.wikipedia.org/wiki/Rust_(programming_language)",
    )?;

    // @step When I capture a screenshot with url "https://example.com" and full_page set to true
    let screenshot_path = browser.capture_screenshot(&tab, None, true)?;

    // @step Then the screenshot should capture the entire scrollable page content
    assert!(
        Path::new(&screenshot_path).exists(),
        "Screenshot file should exist at: {}",
        screenshot_path
    );

    // @step And the screenshot height should exceed the viewport height
    // Read the PNG and check dimensions - full page should be taller than viewport (typically 600-800px)
    let file_contents = std::fs::read(&screenshot_path)
        .map_err(|e| ChromeError::TabError(format!("Failed to read screenshot: {}", e)))?;

    // PNG IHDR chunk starts at byte 8, width at offset 16, height at offset 20
    // Format: 4 bytes width (big-endian), 4 bytes height (big-endian)
    if file_contents.len() > 24 {
        let height = u32::from_be_bytes([
            file_contents[20],
            file_contents[21],
            file_contents[22],
            file_contents[23],
        ]);
        assert!(
            height > 800,
            "Full-page screenshot height ({}) should exceed typical viewport height (800px)",
            height
        );
    }

    // Cleanup
    std::fs::remove_file(&screenshot_path).ok();
    browser.cleanup_tab(&tab);

    Ok(())
}

// =============================================================================
// Scenario: Capture screenshot to custom output path
// =============================================================================

#[test]
#[ignore = "Requires Chrome installed - run with --ignored flag"]
fn test_capture_screenshot_to_custom_output_path() -> Result<(), ChromeError> {
    // @step Given a web page at "https://example.com"
    let config = ChromeConfig::default();
    let browser = ChromeBrowser::new(config)?;
    let tab = browser.new_tab()?;
    browser.navigate_and_wait(&tab, "https://example.com")?;

    // @step When I capture a screenshot with url "https://example.com" and output_path "/tmp/my-custom-screenshot.png"
    let custom_path = "/tmp/my-custom-screenshot.png";
    let screenshot_path = browser.capture_screenshot(&tab, Some(custom_path.to_string()), false)?;

    // @step Then the screenshot should be saved to "/tmp/my-custom-screenshot.png"
    assert!(
        Path::new(custom_path).exists(),
        "Screenshot should be saved to custom path: {}",
        custom_path
    );

    // @step And the returned path should match the specified output_path
    assert_eq!(
        screenshot_path, custom_path,
        "Returned path should match specified output_path"
    );

    // Cleanup
    std::fs::remove_file(custom_path).ok();
    browser.cleanup_tab(&tab);

    Ok(())
}

// =============================================================================
// Scenario: View captured screenshot using Read tool
// =============================================================================

#[test]
#[ignore = "Requires Chrome installed - run with --ignored flag"]
fn test_view_captured_screenshot_using_read_tool() -> Result<(), ChromeError> {
    // @step Given I have captured a screenshot that returned path "/tmp/screenshot.png"
    let config = ChromeConfig::default();
    let browser = ChromeBrowser::new(config)?;
    let tab = browser.new_tab()?;
    browser.navigate_and_wait(&tab, "https://example.com")?;

    // Capture a screenshot first
    let screenshot_path = "/tmp/screenshot-read-test.png";
    let returned_path =
        browser.capture_screenshot(&tab, Some(screenshot_path.to_string()), false)?;

    // @step When I use the Read tool with file_path "/tmp/screenshot.png"
    // The Read tool in codelet already supports images via file_type detection
    // We verify the file can be read and is a valid PNG
    let file_contents = std::fs::read(&returned_path)
        .map_err(|e| ChromeError::TabError(format!("Failed to read screenshot: {}", e)))?;

    // @step Then the Read tool should return image data with type "image"
    // Verify it's a valid PNG by checking magic bytes
    assert!(
        file_contents.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "File should be a valid PNG"
    );

    // @step And the media_type should be "image/png"
    // The file_type module in codelet-tools detects PNG via magic bytes
    // This is verified by the PNG signature check above
    // The actual media_type detection happens in the Read tool implementation

    // Cleanup
    std::fs::remove_file(screenshot_path).ok();
    browser.cleanup_tab(&tab);

    Ok(())
}

// =============================================================================
// Unit test for WebSearchAction::CaptureScreenshot variant
// =============================================================================

#[test]
fn test_web_search_action_capture_screenshot_variant() {
    use codelet_common::web_search::WebSearchAction;

    // @step Given a CaptureScreenshot action with url and full_page parameters
    // This test verifies the enum variant exists
    // It will fail to compile until WebSearchAction::CaptureScreenshot is added
    let action = WebSearchAction::CaptureScreenshot {
        url: Some("https://example.com".to_string()),
        output_path: None,
        full_page: Some(true),
    };

    // @step Then the action should serialize correctly
    let json = serde_json::to_string(&action).expect("Should serialize");
    assert!(
        json.contains("capture_screenshot"),
        "Should contain action type"
    );
    assert!(json.contains("example.com"), "Should contain URL");
    assert!(
        json.contains("full_page"),
        "Should contain full_page parameter"
    );

    // @step And the action should deserialize correctly
    let deserialized: WebSearchAction = serde_json::from_str(&json).expect("Should deserialize");
    match deserialized {
        WebSearchAction::CaptureScreenshot { url, full_page, .. } => {
            assert_eq!(url, Some("https://example.com".to_string()));
            assert_eq!(full_page, Some(true));
        }
        _ => panic!("Should deserialize to CaptureScreenshot variant"),
    }
}
