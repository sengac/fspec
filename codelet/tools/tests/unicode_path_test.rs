
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/unicode-path-normalization.feature
//!
//! Tests for Unicode whitespace normalization in Rust codelet tools — BUG-130
//!
//! These tests verify that Read, Write, Edit, and Ls tools can handle file paths
//! where macOS uses U+202F (NARROW NO-BREAK SPACE) before am/pm in screenshot
//! filenames, but users/agents type regular ASCII spaces.

use codelet_tools::{
    edit::{EditArgs, EditTool},
    ls::LsArgs,
    read::{ReadArgs, ReadTool},
    write::{WriteArgs, WriteTool},
    LsTool,
};
use rig::tool::Tool;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

// Helper: U+202F NARROW NO-BREAK SPACE (the macOS screenshot character)
const NBSP_NARROW: &str = "\u{202F}";
// Helper: U+00A0 NO-BREAK SPACE
const NBSP: &str = "\u{00A0}";

/// Create a file with U+202F in its name (simulating macOS screenshot filename).
/// Returns (temp_dir, actual_path_with_unicode, user_typed_path_with_regular_space)
fn create_file_with_unicode_whitespace(
    dir: &std::path::Path,
    extension: &str,
    content: &[u8],
) -> (std::path::PathBuf, String) {
    // Actual filename macOS would create (U+202F before "am")
    let actual_name = format!("Screenshot 2026-04-13 at 9.13.45{NBSP_NARROW}am.{extension}");
    let actual_path = dir.join(&actual_name);
    fs::write(&actual_path, content).unwrap();

    // Path a user would type (regular ASCII space before "am")
    let user_typed_name = format!("Screenshot 2026-04-13 at 9.13.45 am.{extension}");
    let user_typed_path = dir.join(&user_typed_name).to_string_lossy().to_string();

    (actual_path, user_typed_path)
}

// ========================================
// Rust utility: normalize_unicode_whitespace
// ========================================

/// Scenario: Resolve file with U+202F on disk when user types regular space
#[tokio::test]
async fn test_resolve_file_with_u202f_when_user_types_regular_space() {
    // @step Given a file on disk named with U+202F in its name
    let temp_dir = TempDir::new().unwrap();
    let (actual_path, _user_typed_path) =
        create_file_with_unicode_whitespace(temp_dir.path(), "txt", b"hello unicode");

    // Verify the file actually exists with U+202F
    assert!(actual_path.exists(), "File with U+202F should exist on disk");

    // @step When I call resolve_unicode_path with a path using a regular space instead of U+202F
    // This calls the resolve_unicode_path function from unicode_path module
    let user_path_str = actual_path
        .to_string_lossy()
        .replace(NBSP_NARROW, " ");
    let user_path = std::path::Path::new(&user_path_str);

    // The file should NOT exist with the user-typed path (proving the bug)
    assert!(
        !user_path.exists(),
        "File with regular space should NOT exist — that's the whole bug"
    );

    // @step Then the file should be found via parent directory scan
    // Call the actual resolve function from the new unicode_path module
    let resolved = codelet_tools::unicode_path::resolve_unicode_path(user_path).await;

    // @step And the returned path should point to the actual file on disk containing U+202F
    assert!(resolved.is_some(), "resolve_unicode_path should find the file via directory scan");
    assert_eq!(
        resolved.unwrap(),
        actual_path,
        "Resolved path should point to the actual file with U+202F"
    );
}

/// Scenario: Resolve file with regular space on disk when user pastes U+00A0
#[tokio::test]
async fn test_resolve_file_with_regular_space_when_user_pastes_nbsp() {
    // @step Given a file on disk named with regular ASCII spaces
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("my file.txt");
    fs::write(&file_path, "content with regular spaces").unwrap();

    // @step When I call resolve_unicode_path with U+00A0 NO-BREAK SPACE instead of regular space
    let nbsp_path_str = file_path
        .to_string_lossy()
        .replace(' ', NBSP);
    let nbsp_path = std::path::Path::new(&nbsp_path_str);

    // The path with U+00A0 should NOT exist (proving normalization is needed)
    assert!(
        !nbsp_path.exists(),
        "File with U+00A0 should NOT exist on disk"
    );

    // @step Then the file should be found via normalized path lookup in phase 1b
    let resolved = codelet_tools::unicode_path::resolve_unicode_path(nbsp_path).await;

    assert!(resolved.is_some(), "resolve_unicode_path should find the file via normalization");
    assert_eq!(
        resolved.unwrap(),
        file_path,
        "Resolved path should point to the regular-space file"
    );
}

// ========================================
// Integration: validate_and_resolve_path (wrapper.rs)
// ========================================

/// Scenario: validate_and_resolve_path normalizes Unicode whitespace before canonicalization
#[tokio::test]
async fn test_validate_and_resolve_path_normalizes_unicode_whitespace() {
    // @step Given a directory on disk containing a file with U+202F in its name
    let temp_dir = TempDir::new().unwrap();
    // Create a file with a regular space (the normalized form) so validate_and_resolve_path
    // returns a path that exists after normalization
    let regular_name = "Screenshot 2026-04-13 at 9.13.45 am.txt";
    let regular_path = temp_dir.path().join(regular_name);
    fs::write(&regular_path, "test content").unwrap();

    // Build a user path with U+00A0 (NO-BREAK SPACE) instead of regular space
    let nbsp_name = regular_name.replace(' ', NBSP);
    let nbsp_path = temp_dir.path().join(&nbsp_name).to_string_lossy().to_string();

    // @step When I call validate_and_resolve_path with a path using regular ASCII spaces
    let result = codelet_tools::facade::validate_and_resolve_path(
        Uuid::nil(),
        &nbsp_path,
        "read",
    );

    // @step Then the returned PathBuf should point to the actual file on disk
    assert!(result.is_ok(), "validate_and_resolve_path should succeed: {:?}", result.err());
    let resolved = result.unwrap();

    // @step And the normalization should have occurred before any canonicalize or exists checks
    // The U+00A0 should have been normalized to regular space, matching the file on disk
    assert!(
        resolved.exists(),
        "Resolved path should point to an existing file after Unicode normalization"
    );
    assert_eq!(
        resolved.file_name().unwrap().to_string_lossy(),
        regular_name,
        "Filename should have regular spaces after normalization"
    );
}

// ========================================
// Integration: require_file_exists directory-scan fallback (validation.rs)
// ========================================

/// Scenario: require_file_exists finds file via directory scan when normalized path also fails
#[tokio::test]
async fn test_require_file_exists_directory_scan_fallback() {
    // @step Given a file on disk named "Screenshot 2026-04-13 at 9.13.45\u202fam.txt"
    let temp_dir = TempDir::new().unwrap();
    let actual_name = format!("Screenshot 2026-04-13 at 9.13.45{NBSP_NARROW}am.txt");
    let actual_path = temp_dir.path().join(&actual_name);
    fs::write(&actual_path, "test file content").unwrap();

    // @step When I call require_file_exists with path "Screenshot 2026-04-13 at 9.13.45 am.txt" containing regular space
    let user_name = "Screenshot 2026-04-13 at 9.13.45 am.txt";
    let user_path = temp_dir.path().join(user_name);
    let user_path_str = user_path.to_string_lossy().to_string();

    // The raw path doesn't exist
    assert!(!user_path.exists());

    // @step Then it should succeed by finding the file via directory scan fallback
    let result = codelet_tools::validation::require_file_exists(
        &user_path,
        &user_path_str,
    )
    .await;

    // @step And the resolved path used for subsequent I/O should point to the actual file
    assert!(
        result.is_ok(),
        "require_file_exists should succeed via directory scan: {:?}",
        result.err()
    );
}

// ========================================
// Integration: Read tool end-to-end
// ========================================

/// Scenario: Read tool reads file with U+202F when user provides regular space path
#[tokio::test]
async fn test_read_tool_unicode_text_file() {
    // @step Given a text file on disk named with U+202F before "am" containing known content
    let temp_dir = TempDir::new().unwrap();
    let known_content = "This is the content of the screenshot description file.\nLine two.\n";
    let (_actual_path, user_typed_path) =
        create_file_with_unicode_whitespace(temp_dir.path(), "txt", known_content.as_bytes());

    // @step When I call ReadTool.call() with file_path using regular ASCII space instead of U+202F
    let tool = ReadTool::new(Uuid::nil());
    let result = tool
        .call(ReadArgs {
            file_path: user_typed_path.clone(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the tool should return the file content successfully
    assert!(
        result.is_ok(),
        "ReadTool should find the file despite Unicode whitespace mismatch: {:?}",
        result.err()
    );

    // @step And the output should contain the known content with line numbers
    let output = result.unwrap();
    assert!(
        output.contains("This is the content"),
        "Output should contain the file content, got: {output}"
    );
    assert!(
        output.contains("1:"),
        "Output should have line numbers"
    );
}

/// Scenario: Read tool reads image with U+202F when user provides regular space path
#[tokio::test]
async fn test_read_tool_unicode_image_file() {
    // @step Given a PNG image file on disk named "Screenshot 2026-04-13 at 9.13.45\u202fam.png"
    let temp_dir = TempDir::new().unwrap();
    // Minimal valid PNG: 8-byte header + IHDR chunk (1x1 pixel)
    let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    // IHDR: length(13) + "IHDR" + width(1) + height(1) + bit_depth(8) + color_type(2) + rest(3) + CRC
    let ihdr: [u8; 26] = [
        0x00, 0x00, 0x00, 0x0D, // length = 13
        0x49, 0x48, 0x44, 0x52, // "IHDR"
        0x00, 0x00, 0x00, 0x01, // width = 1
        0x00, 0x00, 0x00, 0x01, // height = 1
        0x08, 0x02,             // bit depth=8, color=RGB
        0x00, 0x00, 0x00,       // compression, filter, interlace
        0x90, 0x77, 0x53, 0xDE, // CRC
        0x00,                   // padding
    ];
    let mut png_data = Vec::new();
    png_data.extend_from_slice(&png_header);
    png_data.extend_from_slice(&ihdr);
    // Add IEND chunk
    let iend: [u8; 12] = [
        0x00, 0x00, 0x00, 0x00, // length = 0
        0x49, 0x45, 0x4E, 0x44, // "IEND"
        0xAE, 0x42, 0x60, 0x82, // CRC
    ];
    png_data.extend_from_slice(&iend);

    let (_actual_path, user_typed_path) =
        create_file_with_unicode_whitespace(temp_dir.path(), "png", &png_data);

    // @step When I call ReadTool.call() with file_path "Screenshot 2026-04-13 at 9.13.45 am.png" using regular space
    let tool = ReadTool::new(Uuid::nil());
    let result = tool
        .call(ReadArgs {
            file_path: user_typed_path.clone(),
            offset: None,
            limit: None,
            pdf_mode: None,
        })
        .await;

    // @step Then the tool should return base64-encoded image data
    assert!(
        result.is_ok(),
        "ReadTool should find the PNG despite Unicode whitespace mismatch: {:?}",
        result.err()
    );

    let output = result.unwrap();

    // @step And the media_type should be "image/png"
    assert!(
        output.contains("image/png"),
        "Output should contain image/png media type, got: {output}"
    );
}

// ========================================
// Integration: Edit tool end-to-end
// ========================================

/// Scenario: Edit tool edits file with U+202F when user provides regular space path
#[tokio::test]
async fn test_edit_tool_unicode_file() {
    // @step Given a text file on disk named with U+202F containing "old content"
    let temp_dir = TempDir::new().unwrap();
    let (actual_path, user_typed_path) =
        create_file_with_unicode_whitespace(temp_dir.path(), "txt", b"old content here");

    // @step When I call EditTool.call() with file_path using regular space and old_string "old content" new_string "new content"
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: user_typed_path.clone(),
            old_string: "old content".to_string(),
            new_string: "new content".to_string(),
        })
        .await;

    // @step Then the edit should succeed
    assert!(
        result.is_ok(),
        "EditTool should find the file despite Unicode whitespace mismatch: {:?}",
        result.err()
    );

    // @step And the file on disk should contain "new content"
    let content = fs::read_to_string(&actual_path).unwrap();
    assert!(
        content.contains("new content"),
        "File should contain 'new content' after edit, got: {content}"
    );
    assert!(
        !content.contains("old content"),
        "File should NOT contain 'old content' after edit"
    );
}

// ========================================
// Integration: Write tool normalizes Unicode in new file path
// ========================================

/// Scenario: Write tool normalizes Unicode whitespace in file path for new files
#[tokio::test]
async fn test_write_tool_normalizes_unicode_in_path() {
    // @step Given a target directory exists
    let temp_dir = TempDir::new().unwrap();

    // @step When I call WriteTool.call() with file_path containing U+00A0 NO-BREAK SPACE and some content
    let nbsp_name = format!("my{NBSP}file.txt");
    let nbsp_path = temp_dir.path().join(&nbsp_name).to_string_lossy().to_string();
    let expected_name = "my file.txt"; // should normalize to regular space

    let tool = WriteTool::new(Uuid::nil());
    let result = tool
        .call(WriteArgs {
            file_path: nbsp_path.clone(),
            content: "written content".to_string(),
        })
        .await;

    // @step Then the file should be created with regular ASCII spaces in its name
    assert!(
        result.is_ok(),
        "WriteTool should succeed with Unicode whitespace in path: {:?}",
        result.err()
    );

    let expected_path = temp_dir.path().join(expected_name);
    assert!(
        expected_path.exists(),
        "File should exist with normalized (regular space) filename"
    );

    // @step And the file content should be written correctly
    let content = fs::read_to_string(&expected_path).unwrap();
    assert_eq!(content, "written content");
}

// ========================================
// Integration: Ls tool with Unicode path
// ========================================

/// Scenario: Ls tool lists directory when path contains Unicode whitespace
#[tokio::test]
async fn test_ls_tool_with_unicode_directory_path() {
    // @step Given a directory on disk whose path contains U+202F
    let temp_dir = TempDir::new().unwrap();
    let unicode_dir_name = format!("my{NBSP_NARROW}dir");
    let unicode_dir = temp_dir.path().join(&unicode_dir_name);
    fs::create_dir(&unicode_dir).unwrap();

    // Put a file inside so we can verify contents
    fs::write(unicode_dir.join("test.txt"), "hello").unwrap();

    // @step When I call LsTool with the path using regular ASCII space
    let user_dir_path = temp_dir
        .path()
        .join("my dir") // regular space
        .to_string_lossy()
        .to_string();

    let tool = LsTool::new(Uuid::nil());
    let result = tool
        .call(LsArgs {
            path: Some(user_dir_path),
        })
        .await;

    // @step Then the directory listing should be returned successfully
    assert!(
        result.is_ok(),
        "LsTool should find directory despite Unicode whitespace mismatch: {:?}",
        result.err()
    );

    let output = result.unwrap();
    assert!(
        output.contains("test.txt"),
        "Directory listing should show the file inside, got: {output}"
    );
}
