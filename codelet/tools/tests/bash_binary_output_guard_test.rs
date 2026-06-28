//! Integration tests for Bash tool binary-output guard (BUG-142).
//!
//! Feature: spec/features/bash-tool-binary-output-guard.feature
//!
//! These tests spawn real bash commands via `BashTool` and assert that
//! binary payloads on stdout are replaced with a structured guard error
//! rather than forwarded to the model.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codelet_tools::bash::BashTool;
use codelet_tools::bash::{BashArgs, StreamCallback};
use rig::tool::Tool;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;
use uuid::Uuid;

/// Helper: write `bytes` to a temp file and return (file, path).
fn write_temp_file(bytes: &[u8]) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(bytes).unwrap();
    f.flush().unwrap();
    f
}

/// Fixture: minimal valid PNG file (8-byte signature + minimal IHDR/IEND).
fn png_bytes() -> Vec<u8> {
    let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    // IHDR chunk
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]); // length
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // width
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // height
    data.extend_from_slice(&[0x08, 0x06, 0x00, 0x00, 0x00]);
    data.extend_from_slice(&[0x1F, 0x15, 0xC4, 0x89]); // CRC
                                                       // IEND chunk
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    data.extend_from_slice(b"IEND");
    data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);
    data
}

fn jpeg_bytes() -> Vec<u8> {
    vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00,
        0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xD9,
    ]
}

fn pdf_bytes() -> Vec<u8> {
    let mut v = b"%PDF-1.4\n".to_vec();
    v.extend_from_slice(&[0x25, 0xE2, 0xE3, 0xCF, 0xD3, 0x0A]);
    v.extend_from_slice(b"1 0 obj\n<<>>\nendobj\n%%EOF\n");
    v
}

fn elf_bytes() -> Vec<u8> {
    let mut v = vec![0x7F, 0x45, 0x4C, 0x46];
    v.extend_from_slice(&[0x02, 0x01, 0x01, 0x00]);
    v.extend_from_slice(&[0u8; 56]);
    v
}

fn gzip_bytes() -> Vec<u8> {
    let mut v = vec![0x1F, 0x8B, 0x08, 0x00];
    v.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    v.extend_from_slice(&[0x03, 0x4B, 0x4C, 0x02, 0x00]);
    v.extend_from_slice(&[0x4D, 0x7E, 0x2E, 0x9C, 0x01, 0x00, 0x00, 0x00]);
    v
}

async fn run_bash(command: &str) -> Result<String, codelet_tools::error::ToolError> {
    let tool = BashTool::new(Uuid::nil());
    tool.call(BashArgs {
        command: command.to_string(),
        cwd: None,
    })
    .await
}

// ----------------------------------------------------------------------------
// Scenario: PNG bytes on stdout trigger the image-aware binary guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn png_bytes_on_stdout_trigger_image_guard() {
    // @step Given a bash command prints PNG magic bytes (0x89 0x50 0x4E 0x47) followed by a PNG payload to stdout
    let file = write_temp_file(&png_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("cat '{path}'")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("binary output must produce an error");
    let msg = err.to_string();

    // @step And the error message contains "detected PNG image"
    assert!(
        msg.contains("detected PNG image"),
        "msg did not name PNG: {msg}"
    );
    // @step And the error message contains "Use the Read tool"
    assert!(msg.contains("Use the Read tool"), "msg = {msg}");
    // @step And the error message does NOT contain any of the raw PNG bytes
    assert!(!msg.contains('\u{0}'), "msg leaked NUL byte: {msg}");
    assert!(!msg.contains("IHDR"), "msg leaked PNG chunk name: {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: JPEG bytes on stdout trigger the image-aware binary guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn jpeg_bytes_on_stdout_trigger_image_guard() {
    // @step Given a bash command prints JPEG magic bytes (0xFF 0xD8 0xFF) followed by a JPEG payload to stdout
    let file = write_temp_file(&jpeg_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("cat '{path}'")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("binary output must produce an error");
    let msg = err.to_string();

    // @step And the error message contains "detected JPEG image"
    assert!(msg.contains("detected JPEG image"), "msg = {msg}");
    // @step And the error message contains "Use the Read tool"
    assert!(msg.contains("Use the Read tool"), "msg = {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: PDF bytes on stdout trigger the document-aware binary guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn pdf_bytes_on_stdout_trigger_pdf_guard() {
    // @step Given a bash command prints PDF magic bytes ("%PDF-1.4") to stdout
    let file = write_temp_file(&pdf_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("cat '{path}'")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("binary output must produce an error");
    let msg = err.to_string();

    // @step And the error message contains "detected PDF document"
    assert!(msg.contains("detected PDF document"), "msg = {msg}");
    // @step And the error message contains "Use the Read tool"
    assert!(msg.contains("Use the Read tool"), "msg = {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: ELF binary on stdout triggers the generic binary guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn elf_bytes_on_stdout_trigger_generic_guard() {
    // @step Given a bash command prints ELF magic bytes (0x7F 0x45 0x4C 0x46) to stdout
    let file = write_temp_file(&elf_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("cat '{path}'")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("binary output must produce an error");
    let msg = err.to_string();

    // @step And the error message contains "detected binary content"
    assert!(msg.contains("detected binary content"), "msg = {msg}");
    // @step And the error message contains "Use the Read tool"
    assert!(msg.contains("Use the Read tool"), "msg = {msg}");
    // @step And the error message does NOT contain the word "PNG"
    assert!(!msg.contains("PNG"), "msg = {msg}");
    // @step And the error message does NOT contain the word "PDF"
    assert!(!msg.contains("PDF"), "msg = {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: Gzip-compressed stdout triggers the generic binary guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn gzip_bytes_on_stdout_trigger_generic_guard() {
    // @step Given a bash command prints gzip magic bytes (0x1F 0x8B) followed by compressed payload to stdout
    let file = write_temp_file(&gzip_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("cat '{path}'")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("binary output must produce an error");
    let msg = err.to_string();

    // @step And the error message contains "detected binary content"
    assert!(msg.contains("detected binary content"), "msg = {msg}");
    // @step And the error message contains "Use the Read tool"
    assert!(msg.contains("Use the Read tool"), "msg = {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: Raw NUL bytes in stdout trigger the generic binary guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn raw_nul_bytes_on_stdout_trigger_generic_guard() {
    // @step Given a bash command prints "\x00\x01\x02\x03hello" to stdout (bytes with an embedded NUL)
    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(r#"printf '\x00\x01\x02\x03hello'"#).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("binary output must produce an error");
    let msg = err.to_string();

    // @step And the error message contains "detected binary content"
    assert!(msg.contains("detected binary content"), "msg = {msg}");
    // @step And the error message does NOT contain the raw bytes
    assert!(!msg.contains('\u{0}'), "msg leaked NUL: {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: Plain text with emoji and high-bit UTF-8 is NOT flagged as binary
// ----------------------------------------------------------------------------
#[tokio::test]
async fn emoji_and_utf8_text_is_not_flagged() {
    // @step Given a bash command prints "hello 👋 world — café résumé" to stdout
    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash("printf 'hello 👋 world — café résumé'").await;

    // @step Then the caller receives Ok containing the original text
    let out = result.expect("UTF-8 text must pass through");
    // @step And the returned string equals the command's stdout unchanged
    assert!(out.contains("hello 👋 world — café résumé"), "out = {out}");
    assert!(!out.contains("detected"), "false-positive guard: {out}");
}

// ----------------------------------------------------------------------------
// Scenario: hexdump output of a binary file is text and passes through unchanged
// ----------------------------------------------------------------------------
#[tokio::test]
async fn hexdump_of_binary_file_is_text() {
    // @step Given a bash command pipeline produces canonical hexdump text
    let file = write_temp_file(&png_bytes());
    let path = file.path().to_string_lossy().to_string();

    // Use `od` which is POSIX and always available; produces ASCII-only output.
    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("od -An -c '{path}' | head -n 2")).await;

    // @step Then the caller receives Ok containing the hexdump lines unchanged
    let out = result.expect("hexdump output is text");
    // @step And no binary guard is triggered
    assert!(!out.contains("detected"), "guard fired on text: {out}");
    assert!(!out.contains("Use the Read tool"), "out = {out}");
}

// ----------------------------------------------------------------------------
// Scenario: Missing-file failure preserves stderr diagnostic and does NOT trigger the guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn missing_file_error_is_not_intercepted_by_guard() {
    // @step Given a bash command fails with exit code 1
    // @step And stdout is empty
    // @step And stderr contains "cat: /tmp/missing.png: No such file or directory"
    // @step When the Bash tool returns
    let result = run_bash("cat /tmp/this_really_should_not_exist_for_BUG142.png").await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("missing file must fail");
    let msg = err.to_string();

    // @step And the error message contains "exit code 1"
    assert!(msg.contains("exit code 1"), "msg = {msg}");
    // @step And the error message contains the stderr diagnostic
    assert!(
        msg.contains("No such file") || msg.contains("cannot open"),
        "missing stderr: {msg}"
    );
    // @step And the error message does NOT contain "detected binary content"
    assert!(!msg.contains("detected binary content"), "msg = {msg}");
    // @step And the error message does NOT contain "Use the Read tool"
    assert!(!msg.contains("Use the Read tool"), "msg = {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: Text prefix followed by PNG payload is intercepted by the guard
// ----------------------------------------------------------------------------
#[tokio::test]
async fn mixed_text_and_png_payload_is_intercepted() {
    // @step Given a bash command prints "header\n" then a PNG payload to stdout
    let file = write_temp_file(&png_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When the Bash tool returns
    let result = run_bash(&format!("{{ printf 'header\\n'; cat '{path}'; }}")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("mixed binary payload must fail");
    let msg = err.to_string();

    // @step And the error message contains "detected PNG image"
    assert!(msg.contains("detected PNG image"), "msg = {msg}");
    // @step And the error message does NOT contain the text prefix "header"
    // (the guard message never includes captured output)
    assert!(!msg.contains("header\n"), "msg leaked prefix: {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: call_with_streaming replaces the final buffered return value with the guard error
// ----------------------------------------------------------------------------
#[tokio::test]
async fn call_with_streaming_still_returns_guard_error() {
    // @step Given a stream_callback is provided to call_with_streaming
    let captured: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&captured);
    let cb: StreamCallback = Arc::new(move |chunk: &str| {
        sink.lock().unwrap().push_str(chunk);
    });

    // @step And a bash command prints PNG bytes to stdout
    let file = write_temp_file(&png_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step And the command exits with status 0
    // @step When call_with_streaming returns
    let tool = BashTool::new(Uuid::nil());
    let result = tool
        .call_with_streaming(
            BashArgs {
                command: format!("cat '{path}'"),
                cwd: None,
            },
            Some(cb),
        )
        .await;

    // @step Then the buffered Result returned to the caller is a ToolError::Execution
    let err = result.expect_err("streaming path must also guard");
    let msg = err.to_string();

    // @step And the error message contains "detected PNG image"
    assert!(msg.contains("detected PNG image"), "msg = {msg}");
    // @step And the error message contains "Use the Read tool"
    assert!(msg.contains("Use the Read tool"), "msg = {msg}");
}

// ----------------------------------------------------------------------------
// Scenario: Binary payload combined with a non-zero exit status still returns the guard error
// ----------------------------------------------------------------------------
#[tokio::test]
async fn binary_payload_with_nonzero_exit_prefers_guard_error() {
    // @step Given a bash command prints PNG bytes to stdout
    // @step And the command exits with status 2
    let file = write_temp_file(&png_bytes());
    let path = file.path().to_string_lossy().to_string();

    // @step When the Bash tool returns
    let result = run_bash(&format!("cat '{path}'; exit 2")).await;

    // @step Then the caller receives a ToolError::Execution
    let err = result.expect_err("failure path must still guard");
    let msg = err.to_string();

    // @step And the error message contains "detected PNG image"
    assert!(msg.contains("detected PNG image"), "msg = {msg}");
    // @step And the error message does NOT contain "exit code 2"
    // (the guard preempts the exit-code reporting so the model isn't confused)
    assert!(!msg.contains("exit code 2"), "msg = {msg}");
}
