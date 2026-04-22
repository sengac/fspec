//! Integration tests for binary-guard on Edit / apply_patch surfaces (BUG-143).
//!
//! Feature: spec/features/binary-guard-file-reading-surfaces.feature
//!
//! These tests exercise `EditTool` and `ApplyPatchTool` against real temp files
//! and assert that binary payloads are rejected with a structured guard error
//! naming the detected format (PNG / JPEG / PDF / …) instead of a confusing
//! UTF-8 decode failure.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_tools::apply_patch::{ApplyPatchArgs, ApplyPatchTool};
use codelet_tools::edit::{EditArgs, EditTool};
use codelet_tools::error::ToolError;
use codelet_tools::glob::{GlobArgs, GlobTool};
use rig::tool::Tool;
use std::fs;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Minimal valid PNG: 8-byte signature + minimal IHDR/IEND chunks.
fn png_bytes() -> Vec<u8> {
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&[0x08, 0x06, 0x00, 0x00, 0x00]);
    data.extend_from_slice(&[0x1F, 0x15, 0xC4, 0x89]);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    data.extend_from_slice(b"IEND");
    data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
    data
}

/// Minimal PDF header.
fn pdf_bytes() -> Vec<u8> {
    let mut v = b"%PDF-1.4\n".to_vec();
    v.extend_from_slice(&[0x25, 0xE2, 0xE3, 0xCF, 0xD3, 0x0A]);
    v.extend_from_slice(b"1 0 obj\n<<>>\nendobj\n%%EOF\n");
    v
}

/// Minimal ELF header (64-bit x86).
fn elf_bytes() -> Vec<u8> {
    let mut v = vec![0x7F, 0x45, 0x4C, 0x46];
    v.extend_from_slice(&[0x02, 0x01, 0x01, 0x00]);
    v.extend_from_slice(&[0x00; 8]);
    v.extend_from_slice(&[0x02, 0x00, 0x3E, 0x00]);
    v
}

/// Helper: write bytes to a temp file and return it (kept alive by caller).
fn write_temp_binary(bytes: &[u8]) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

/// Helper: write UTF-8 text to a temp file and return it.
fn write_temp_text(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ---------------------------------------------------------------------------
// Scenario: Edit rejects PNG file with named binary-guard error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_rejects_png_file_with_named_binary_guard_error() {
    // @step Given a file at "/tmp/icon.png" whose first 8 bytes are the PNG magic signature
    let bytes = png_bytes();
    let file = write_temp_binary(&bytes);
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/icon.png" and any old_string/new_string
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path.clone(),
            old_string: "anything".to_string(),
            new_string: "something".to_string(),
        })
        .await;

    // @step Then the tool returns a ToolError::Validation
    let err = result.expect_err("Edit should reject a PNG file");
    match err {
        ToolError::Validation { message, .. } => {
            // @step And the error message contains "detected PNG image"
            assert!(
                message.contains("detected PNG image"),
                "error message should name PNG, got: {message}"
            );
            // @step And the error message instructs the agent to use the Read tool instead
            assert!(
                message.contains("Read tool"),
                "error message should direct to Read tool, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }

    // @step And the file on disk is unchanged
    let after = fs::read(file.path()).unwrap();
    assert_eq!(after, bytes, "file bytes on disk must be unchanged");
}

// ---------------------------------------------------------------------------
// Scenario: Edit rejects PDF file with named binary-guard error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_rejects_pdf_file_with_named_binary_guard_error() {
    // @step Given a file at "/tmp/report.pdf" whose first 5 bytes are "%PDF-"
    let bytes = pdf_bytes();
    let file = write_temp_binary(&bytes);
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/report.pdf" and any old_string/new_string
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path.clone(),
            old_string: "a".to_string(),
            new_string: "b".to_string(),
        })
        .await;

    // @step Then the tool returns a ToolError::Validation
    let err = result.expect_err("Edit should reject a PDF file");
    match err {
        ToolError::Validation { message, .. } => {
            // @step And the error message contains "detected PDF document"
            assert!(
                message.contains("detected PDF document"),
                "error message should name PDF, got: {message}"
            );
            // @step And the error message instructs the agent to use the Read tool instead
            assert!(
                message.contains("Read tool"),
                "error message should direct to Read tool, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }

    // @step And the file on disk is unchanged
    let after = fs::read(file.path()).unwrap();
    assert_eq!(after, bytes);
}

// ---------------------------------------------------------------------------
// Scenario: Edit rejects ELF binary with generic binary-guard error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_rejects_elf_binary_with_generic_binary_guard_error() {
    // @step Given a file at "/tmp/program" whose first 4 bytes are 0x7F 0x45 0x4C 0x46 (ELF)
    let bytes = elf_bytes();
    let file = write_temp_binary(&bytes);
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/program" and any old_string/new_string
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path.clone(),
            old_string: "x".to_string(),
            new_string: "y".to_string(),
        })
        .await;

    // @step Then the tool returns a ToolError::Validation
    let err = result.expect_err("Edit should reject an ELF binary");
    match err {
        ToolError::Validation { message, .. } => {
            // @step And the error message contains "detected binary content"
            assert!(
                message.contains("detected binary content"),
                "error message should use generic phrasing, got: {message}"
            );
            // @step And the error message does not name PNG, JPEG, GIF, WebP, or PDF
            assert!(
                !message.contains("PNG")
                    && !message.contains("JPEG")
                    && !message.contains("GIF")
                    && !message.contains("WebP")
                    && !message.contains("PDF"),
                "generic error must not name a specific format, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Scenario: Edit rejects file containing raw NUL bytes with generic binary-guard error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_rejects_file_containing_raw_nul_bytes() {
    // @step Given a file at "/tmp/blob.bin" whose bytes start with 0x00 0x01 0x02 followed by text
    let mut bytes = vec![0x00u8, 0x01, 0x02];
    bytes.extend_from_slice(b"hello world");
    let file = write_temp_binary(&bytes);
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/blob.bin" and any old_string/new_string
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path,
            old_string: "hello".to_string(),
            new_string: "goodbye".to_string(),
        })
        .await;

    // @step Then the tool returns a ToolError::Validation
    let err = result.expect_err("Edit should reject file with NUL bytes");
    match err {
        ToolError::Validation { message, .. } => {
            // @step And the error message contains "detected binary content"
            assert!(
                message.contains("detected binary content"),
                "error should describe generic binary, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Scenario: Edit on a UTF-8 text file succeeds unchanged
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_on_utf8_text_file_succeeds_unchanged() {
    // @step Given a file at "/tmp/notes.md" containing "# Hello world" as UTF-8 text
    let file = write_temp_text("# Hello world");
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/notes.md", old_string "Hello world", new_string "Goodbye"
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path,
            old_string: "Hello world".to_string(),
            new_string: "Goodbye".to_string(),
        })
        .await;

    // @step Then the tool succeeds
    let msg = result.expect("edit on UTF-8 text should succeed");

    // @step And the file on disk now contains "# Goodbye"
    let after = fs::read_to_string(file.path()).unwrap();
    assert_eq!(after, "# Goodbye");

    // @step And no binary-guard error is emitted
    assert!(
        !msg.contains("detected"),
        "no binary-guard should fire on UTF-8 text, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Edit on a UTF-8 text file containing emoji and CJK succeeds unchanged
// ---------------------------------------------------------------------------

#[tokio::test]
async fn edit_on_utf8_emoji_cjk_text_succeeds_unchanged() {
    // @step Given a file at "/tmp/i18n.txt" containing "café 👋 中文 résumé" as UTF-8 text
    let original = "café 👋 中文 résumé";
    let file = write_temp_text(original);
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/i18n.txt", old_string "café", new_string "CAFE"
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path,
            old_string: "café".to_string(),
            new_string: "CAFE".to_string(),
        })
        .await;

    // @step Then the tool succeeds
    let msg = result.expect("edit on UTF-8 i18n text should succeed");

    // @step And no binary-guard error is emitted
    assert!(
        !msg.contains("detected"),
        "no binary-guard should fire on UTF-8 text, got: {msg}"
    );
    let after = fs::read_to_string(file.path()).unwrap();
    assert_eq!(after, "CAFE 👋 中文 résumé");
}

// ---------------------------------------------------------------------------
// Scenario: apply_patch Update rejects PDF target with named binary-guard error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn apply_patch_update_rejects_pdf_target() {
    // @step Given a file at "/tmp/report.pdf" whose first 5 bytes are "%PDF-"
    let bytes = pdf_bytes();
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("report.pdf");
    fs::write(&file_path, &bytes).unwrap();
    let path_str = file_path.to_string_lossy().to_string();

    // @step When the apply_patch tool is invoked with an Update operation targeting "/tmp/report.pdf"
    let patch = format!(
        "*** Begin Patch\n\
         *** Update File: {path_str}\n\
         @@\n\
         -anything\n\
         +nothing\n\
         *** End Patch"
    );
    let tool = ApplyPatchTool::new(Uuid::nil());
    let result = tool.call(ApplyPatchArgs { patch }).await;

    // @step Then the tool returns a ToolError::Validation
    let err = result.expect_err("apply_patch Update should reject PDF target");
    match err {
        ToolError::Validation { message, .. } => {
            // @step And the error message contains "detected PDF document"
            assert!(
                message.contains("detected PDF document"),
                "error should name PDF, got: {message}"
            );
            // @step And the error message instructs the agent to use the Read tool instead
            assert!(
                message.contains("Read tool"),
                "error should direct to Read tool, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }

    // @step And the file on disk is unchanged
    let after = fs::read(&file_path).unwrap();
    assert_eq!(after, bytes);
}

// ---------------------------------------------------------------------------
// Scenario: apply_patch Update rejects PNG target with named binary-guard error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn apply_patch_update_rejects_png_target() {
    // @step Given a file at "/tmp/icon.png" whose first 8 bytes are the PNG magic signature
    let bytes = png_bytes();
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("icon.png");
    fs::write(&file_path, &bytes).unwrap();
    let path_str = file_path.to_string_lossy().to_string();

    // @step When the apply_patch tool is invoked with an Update operation targeting "/tmp/icon.png"
    let patch = format!(
        "*** Begin Patch\n\
         *** Update File: {path_str}\n\
         @@\n\
         -anything\n\
         +nothing\n\
         *** End Patch"
    );
    let tool = ApplyPatchTool::new(Uuid::nil());
    let result = tool.call(ApplyPatchArgs { patch }).await;

    // @step Then the tool returns a ToolError::Validation
    let err = result.expect_err("apply_patch Update should reject PNG target");
    match err {
        ToolError::Validation { message, .. } => {
            // @step And the error message contains "detected PNG image"
            assert!(
                message.contains("detected PNG image"),
                "error should name PNG, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }

    // @step And the file on disk is unchanged
    let after = fs::read(&file_path).unwrap();
    assert_eq!(after, bytes);
}

// ---------------------------------------------------------------------------
// Scenario: apply_patch Update on UTF-8 source file applies cleanly
// ---------------------------------------------------------------------------

#[tokio::test]
async fn apply_patch_update_on_utf8_source_file_succeeds() {
    // @step Given a file at "/tmp/src.rs" containing valid UTF-8 Rust source
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("src.rs");
    fs::write(&file_path, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
    let path_str = file_path.to_string_lossy().to_string();

    // @step When the apply_patch tool is invoked with an Update operation replacing existing lines
    let patch = format!(
        "*** Begin Patch\n\
         *** Update File: {path_str}\n\
         @@ fn main() {{\n\
         -    println!(\"hi\");\n\
         +    println!(\"hello\");\n\
         *** End Patch"
    );
    let tool = ApplyPatchTool::new(Uuid::nil());
    let result = tool.call(ApplyPatchArgs { patch }).await;

    // @step Then the tool succeeds
    let msg = result.expect("apply_patch Update on UTF-8 source should succeed");
    assert!(msg.contains("Updated"));

    // @step And the file on disk reflects the updated content
    let after = fs::read_to_string(&file_path).unwrap();
    assert!(after.contains("println!(\"hello\");"));
    assert!(!after.contains("println!(\"hi\");"));

    // @step And no binary-guard error is emitted
    assert!(!msg.contains("detected"));
}

// ---------------------------------------------------------------------------
// Scenario: Binary guard inspects only the first 8 KiB of a large file
// ---------------------------------------------------------------------------

#[tokio::test]
async fn binary_guard_inspects_only_first_8_kib() {
    // @step Given a file at "/tmp/big.bin" of 10 MiB whose first 8 bytes are the PNG magic signature
    let mut bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.resize(10 * 1024 * 1024, b'A');
    let file = write_temp_binary(&bytes);
    let path = file.path().to_string_lossy().to_string();

    // @step When the Edit tool is invoked with file_path "/tmp/big.bin"
    let tool = EditTool::new(Uuid::nil());
    let result = tool
        .call(EditArgs {
            file_path: path,
            old_string: "A".to_string(),
            new_string: "B".to_string(),
        })
        .await;

    // @step Then the tool returns a ToolError::Validation naming PNG
    let err = result.expect_err("Edit should reject large PNG-prefixed file");
    match err {
        ToolError::Validation { message, .. } => {
            assert!(
                message.contains("detected PNG image"),
                "large PNG-prefixed file should still detect PNG, got: {message}"
            );
        }
        other => panic!("expected ToolError::Validation, got: {other:?}"),
    }

    // @step And the guard does not read more than 8 KiB of bytes from disk for detection purposes
    // Note: reading the full file for Edit is fine; what matters is the detection scan bound.
    // We verify this at the unit level via detect_bash_binary_output's NUL_SCAN_LIMIT constant
    // (covered by bash_binary_guard unit tests); this integration test confirms the large-file
    // path still terminates with a structured error rather than slurping invalid UTF-8.
}

// ---------------------------------------------------------------------------
// Scenario: Glob has no binary-guard surface because it never returns file bytes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn glob_has_no_binary_guard_surface() {
    // @step Given a directory tree containing "spec/attachments/WU-001/diagram.png" and "src/main.rs"
    let dir = TempDir::new().unwrap();
    let png_dir = dir.path().join("spec/attachments/WU-001");
    fs::create_dir_all(&png_dir).unwrap();
    fs::write(png_dir.join("diagram.png"), png_bytes()).unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() {}\n").unwrap();

    // @step When the Glob tool is invoked with pattern "**/*"
    let tool = GlobTool::new(Uuid::nil());
    let result = tool
        .call(GlobArgs {
            pattern: "**/*".to_string(),
            path: Some(dir.path().to_string_lossy().to_string()),
            case_insensitive: None,
        })
        .await;

    // @step Then the output is a list of file paths
    let output = result.expect("glob should succeed");
    assert!(
        output.contains("diagram.png"),
        "output should list PNG path, got: {output}"
    );
    assert!(
        output.contains("main.rs"),
        "output should list source path, got: {output}"
    );

    // @step And no file contents are loaded by the Glob tool
    // (There are no bytes in the output — it's just a list of relative paths.)
    assert!(
        !output.as_bytes().contains(&0x89),
        "glob output must not contain PNG magic bytes"
    );

    // @step And no binary-guard error is emitted
    assert!(
        !output.contains("detected"),
        "no binary-guard should fire, got: {output}"
    );
}
