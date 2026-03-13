
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for ViewImageTool
//! Feature: spec/features/codex-view-image.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.

use codelet_tools::view_image::{ViewImageArgs, ViewImageTool};
use rig::tool::Tool;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;
use uuid::Uuid;

// ============================================================================
// Helpers
// ============================================================================

/// Create a minimal valid PNG file (8-byte header + IHDR with small dimensions)
fn create_small_png(dir: &TempDir, name: &str) -> String {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();
    // PNG signature
    let png_sig: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    file.write_all(&png_sig).unwrap();
    // Minimal IHDR chunk (13 bytes data): width=1, height=1, bit_depth=8, color_type=2 (RGB)
    // Length (4 bytes) + "IHDR" (4 bytes) + data (13 bytes) + CRC (4 bytes)
    let ihdr: [u8; 25] = [
        0x00, 0x00, 0x00, 0x0D, // length = 13
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x00, 0x01, // width = 1
        0x00, 0x00, 0x00, 0x01, // height = 1
        0x08,                   // bit depth = 8
        0x02,                   // color type = 2 (RGB)
        0x00,                   // compression
        0x00,                   // filter
        0x00,                   // interlace
        0x1E, 0x92, 0x6A, 0x57, // CRC (precomputed for this exact IHDR)
    ];
    file.write_all(&ihdr).unwrap();
    // Add some padding bytes to make it a realistic file
    file.write_all(&[0u8; 100]).unwrap();
    path.to_string_lossy().to_string()
}

/// Create a minimal valid JPEG file
fn create_small_jpeg(dir: &TempDir, name: &str) -> String {
    let path = dir.path().join(name);
    let mut file = File::create(&path).unwrap();
    // JPEG SOI + APP0 marker
    let jpeg_header: [u8; 3] = [0xFF, 0xD8, 0xFF];
    file.write_all(&jpeg_header).unwrap();
    file.write_all(&[0u8; 200]).unwrap();
    // JPEG EOI
    file.write_all(&[0xFF, 0xD9]).unwrap();
    path.to_string_lossy().to_string()
}

// ============================================================================
// Scenario: View a PNG image file
// ============================================================================

#[tokio::test]
async fn test_view_png_image_file() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());
    let temp_dir = TempDir::new().unwrap();

    // @step And a PNG image file exists at a known path
    let file_path = create_small_png(&temp_dir, "test.png");

    // @step When view_image is called with the path to the PNG file
    let args = ViewImageArgs {
        path: file_path,
    };
    let result = tool.call(args).await;

    // @step Then the result is a JSON object with type "image"
    assert!(result.is_ok(), "Expected success, got: {:?}", result.err());
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(output["type"], "image");

    // @step And the media_type is "image/png"
    assert_eq!(output["media_type"], "image/png");

    // @step And the data field contains base64-encoded PNG data
    assert!(output["data"].is_string());
    let data = output["data"].as_str().unwrap();
    assert!(!data.is_empty());
}

// ============================================================================
// Scenario: View a JPEG image file
// ============================================================================

#[tokio::test]
async fn test_view_jpeg_image_file() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());
    let temp_dir = TempDir::new().unwrap();

    // @step And a JPEG image file exists at a known path
    let file_path = create_small_jpeg(&temp_dir, "photo.jpg");

    // @step When view_image is called with the path to the JPEG file
    let args = ViewImageArgs {
        path: file_path,
    };
    let result = tool.call(args).await;

    // @step Then the result is a JSON object with type "image"
    assert!(result.is_ok(), "Expected success, got: {:?}", result.err());
    let output: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(output["type"], "image");

    // @step And the media_type is "image/jpeg"
    assert_eq!(output["media_type"], "image/jpeg");

    // @step And the data field contains base64-encoded JPEG data
    assert!(output["data"].is_string());
    let data = output["data"].as_str().unwrap();
    assert!(!data.is_empty());
}

// ============================================================================
// Scenario: Reject a text file as not an image
// ============================================================================

#[tokio::test]
async fn test_reject_text_file_as_not_image() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());
    let temp_dir = TempDir::new().unwrap();

    // @step And a plain text file exists at a known path
    let path = temp_dir.path().join("readme.txt");
    std::fs::write(&path, "Hello, world!").unwrap();
    let file_path = path.to_string_lossy().to_string();

    // @step When view_image is called with the path to the text file
    let args = ViewImageArgs {
        path: file_path,
    };
    let result = tool.call(args).await;

    // @step Then the tool returns an error indicating the file is not a supported image
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not a supported image") || err.contains("not an image"),
        "Error should indicate non-image file, got: {err}"
    );
}

// ============================================================================
// Scenario: Reject an SVG file as not a binary image
// ============================================================================

#[tokio::test]
async fn test_reject_svg_file_as_not_binary_image() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());
    let temp_dir = TempDir::new().unwrap();

    // @step And an SVG file exists at a known path
    let path = temp_dir.path().join("diagram.svg");
    std::fs::write(&path, "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect/></svg>").unwrap();
    let file_path = path.to_string_lossy().to_string();

    // @step When view_image is called with the path to the SVG file
    let args = ViewImageArgs {
        path: file_path,
    };
    let result = tool.call(args).await;

    // @step Then the tool returns an error indicating SVG is not a supported binary image format
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.to_lowercase().contains("svg") || err.contains("not a supported image"),
        "Error should mention SVG or unsupported image, got: {err}"
    );
}

// ============================================================================
// Scenario: Return error for non-existent file
// ============================================================================

#[tokio::test]
async fn test_error_for_nonexistent_file() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());

    // @step When view_image is called with a path that does not exist
    let args = ViewImageArgs {
        path: "/tmp/definitely_does_not_exist_view_image_test.png".to_string(),
    };
    let result = tool.call(args).await;

    // @step Then the tool returns an error indicating the file was not found
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not found") || err.contains("does not exist") || err.contains("No such file"),
        "Error should indicate file not found, got: {err}"
    );
}

// ============================================================================
// Scenario: Reject an oversized image
// ============================================================================

#[tokio::test]
async fn test_reject_oversized_image() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());
    let temp_dir = TempDir::new().unwrap();

    // @step And an image file exists whose base64 encoding exceeds 5MB
    let path = temp_dir.path().join("huge.png");
    let mut file = File::create(&path).unwrap();
    let png_sig: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    file.write_all(&png_sig).unwrap();
    // 6MB raw → base64 will be > 5MB
    let size = 6 * 1024 * 1024;
    file.write_all(&vec![0u8; size]).unwrap();
    let file_path = path.to_string_lossy().to_string();

    // @step When view_image is called with the path to the oversized image
    let args = ViewImageArgs {
        path: file_path,
    };
    let result = tool.call(args).await;

    // @step Then the tool returns an error indicating the image is too large for LLM processing
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("too large") || err.contains("size"),
        "Error should indicate image is too large, got: {err}"
    );
}

// ============================================================================
// Scenario: Tool definition matches Codex CLI spec
// ============================================================================

#[tokio::test]
async fn test_tool_definition_matches_codex_spec() {
    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());

    // @step When the tool definition is requested
    let def = tool.definition(String::new()).await;

    // @step Then the tool name is "view_image"
    assert_eq!(def.name, "view_image");

    // @step And the parameters schema has a required "path" property of type string
    let params = &def.parameters;
    // Check properties contains "path"
    let properties = params.get("properties")
        .expect("Schema should have properties");
    assert!(
        properties.get("path").is_some(),
        "Schema should have 'path' property"
    );
    // Check required includes "path"
    let required = params.get("required")
        .and_then(|r| r.as_array())
        .expect("Schema should have required array");
    let required_names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        required_names.contains(&"path"),
        "Required should include 'path', got: {:?}", required_names
    );
}

// ============================================================================
// Scenario: Reject a blocklisted path
// ============================================================================

#[tokio::test]
async fn test_reject_blocklisted_path() {
    use codelet_tools::{BlocklistConfig, BlocklistRule, BlocklistAction, init_blocklist};

    // @step Given a ViewImageTool instance with a valid session ID
    let tool = ViewImageTool::new(Uuid::nil());
    let temp_dir = TempDir::new().unwrap();

    // Create a PNG file at a path containing "secret"
    let file_path = create_small_png(&temp_dir, "secret.png");

    // @step And a blocklist is initialized with a rule blocking the target path
    // Write a blocklist config file
    let fspec_dir = temp_dir.path().join(".fspec");
    std::fs::create_dir_all(&fspec_dir).unwrap();
    let blocklist_config = BlocklistConfig {
        version: "1.0.0".to_string(),
        rules: vec![BlocklistRule {
            id: "block-secret-images".to_string(),
            pattern: r"secret\.png$".to_string(),
            action: BlocklistAction::Block,
            reason: "Blocked: access to secret images is not allowed".to_string(),
            guidance: None,
        }],
    };
    let config_json = serde_json::to_string_pretty(&blocklist_config).unwrap();
    std::fs::write(fspec_dir.join("blocklist.json"), config_json).unwrap();
    init_blocklist(Some(temp_dir.path()));

    // @step When view_image is called with the blocklisted path
    let args = ViewImageArgs {
        path: file_path,
    };
    let result = tool.call(args).await;

    // @step Then the tool returns a blocked error
    assert!(result.is_err(), "Should be blocked");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Blocked") || err.contains("blocked"),
        "Error should indicate blocked, got: {err}"
    );

    // Clean up: reinitialize blocklist with no project root
    init_blocklist(None);
}

// ============================================================================
// Scenario: ViewImageTool is registered in Codex agent
// Note: This is verified by the test in codelet/providers/src/codex/mod.rs
// (create_rig_agent_does_not_expose_non_native_glob_tool) which checks all
// expected tool names including view_image. We verify the tool NAME constant
// here as a compile-time check that the tool can be constructed and registered.
// ============================================================================

#[tokio::test]
async fn test_view_image_registered_in_codex_agent() {
    // @step Given a CodexProvider with create_rig_agent configured
    // We verify structurally: ViewImageTool implements rig::tool::Tool with NAME = "view_image"
    // and can be constructed with a session_id — the same pattern used by create_rig_agent.
    let session_id = Uuid::new_v4();

    // @step When the agent is built with a session_id
    let tool = ViewImageTool::new(session_id);
    let def = tool.definition(String::new()).await;

    // @step Then the agent's tool list includes a tool named "view_image"
    assert_eq!(def.name, "view_image", "Tool should register as 'view_image'");
}

// ============================================================================
// Scenario: view_image tool NAME constant is "view_image"
// ============================================================================

#[test]
fn test_tool_name_constant() {
    assert_eq!(ViewImageTool::NAME, "view_image");
}
