//! Output formatting for bash command results.
//!
//! Separates data capture from formatting (Single Responsibility Principle).
//! Provides composable formatting methods for different output scenarios.

use super::error::ToolError;
use super::limits::OutputLimits;
use super::truncation::{format_truncation_warning, process_output_lines, truncate_output};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Marker for stderr content to enable red styling in UI
pub const STDERR_MARKER: &str = "⚠stderr⚠";

/// TOOL-022 P4: split a merged exec-session output string back into
/// (stdout, stderr) for the [`BashOutput`] formatter.
///
/// The unified exec store merges stdout and stderr into ONE buffer
/// (interleaved, read-order). `poll` drains it as one lossy-UTF-8
/// string; stderr lines are tagged with [`STDERR_MARKER`] at the
/// Bash layer. This inverse is line-based:
///
/// - a line STARTING with `STDERR_MARKER` → its remainder (the
///   original stderr line, verbatim) joins the stderr result;
/// - any other line joins the stdout result;
/// - relative ordering inside each stream is preserved (the merged
///   buffer only interleaves BETWEEN streams, never reorders within
///   one).
///
/// A command whose genuine stdout line literally begins with the
/// marker text is misclassified as stderr — accepted: the marker is
/// a non-ASCII sentinel (`⚠stderr⚠`) no shell command emits in
/// practice.
pub fn split_merged_output(merged: &str) -> (String, String) {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut first_out = true;
    let mut first_err = true;
    for line in merged.split('\n') {
        match line.strip_prefix(STDERR_MARKER) {
            Some(rest) => {
                if !first_err {
                    stderr.push('\n');
                }
                stderr.push_str(rest);
                first_err = false;
            }
            None => {
                if !first_out {
                    stdout.push('\n');
                }
                stdout.push_str(line);
                first_out = false;
            }
        }
    }
    (stdout, stderr)
}

/// TOOL-022 P4: split RAW merged exec-session output bytes back into
/// (stdout_bytes, stderr_bytes) for the [`BashOutput`] formatter and
/// the BUG-142 binary-output guard.
///
/// Byte-level (unlike [`split_merged_output`], which operates on a
/// lossy-decoded String): a binary stdout payload must reach the guard
/// UNTOUCHED — lossy UTF-8 decoding would turn magic bytes (PNG/JPEG/ELF)
/// into U+FFFD and the guard could no longer recognize the format.
///
/// Same line-based marker contract:
/// - a line STARTING with [`STDERR_MARKER`] (as bytes) → its remainder
///   joins the stderr half;
/// - any other line joins the stdout half;
/// - lines are rejoined with the exact `b'\n'` separator, so the
///   round-trip of the stdout half is byte-perfect (binary payloads
///   containing newlines survive intact).
pub fn split_merged_output_bytes(merged: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let marker = STDERR_MARKER.as_bytes();
    let mut stdout: Vec<u8> = Vec::new();
    let mut stderr: Vec<u8> = Vec::new();
    let mut first_out = true;
    let mut first_err = true;
    for line in merged.split(|&b| b == b'\n') {
        match line.strip_prefix(marker) {
            Some(rest) => {
                if !first_err {
                    stderr.push(b'\n');
                }
                stderr.extend_from_slice(rest);
                first_err = false;
            }
            None => {
                if !first_out {
                    stdout.push(b'\n');
                }
                stdout.extend_from_slice(line);
                first_out = false;
            }
        }
    }
    (stdout, stderr)
}

/// Holds the raw output from a bash command execution.
///
/// Separates data capture from formatting (Single Responsibility Principle).
/// Provides composable formatting methods for different output needs.
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

impl BashOutput {
    /// Create from process output and exit status
    pub fn from_execution(
        stdout: String,
        stderr: String,
        status: std::process::ExitStatus,
    ) -> Self {
        Self {
            stdout,
            stderr,
            exit_code: status.code(),
            success: status.success(),
        }
    }

    /// Format output for successful command execution.
    ///
    /// Returns stdout with truncation applied, and stderr appended if present.
    /// No labels like "Stderr:" are used - just clean content.
    pub fn format_success(&self) -> String {
        // Apply truncation to stdout
        let lines = process_output_lines(&self.stdout);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

        let mut output = truncate_result.output;
        let was_truncated = truncate_result.char_truncated || truncate_result.remaining_count > 0;

        if was_truncated {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "lines",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            output.push_str(&warning);
        }

        // Append stderr if present (warnings/diagnostics from successful commands)
        self.append_stderr_if_present(&mut output);

        output
    }

    /// Format output for failed command execution.
    ///
    /// Returns a clear error message with exit code, followed by combined output.
    /// No labels like "Stdout:" or "Stderr:" - just clean content.
    pub fn format_error(&self) -> String {
        let code = self.exit_code.unwrap_or(-1);

        // Combine stdout and stderr for error context
        let combined = self.combine_outputs();

        if combined.is_empty() {
            format!("Command failed with exit code {code}")
        } else {
            format!("Command failed with exit code {code}\n{combined}")
        }
    }

    /// Append stderr to output if present (helper for DRY)
    /// Marks stderr lines with STDERR_MARKER for red styling in UI
    pub fn append_stderr_if_present(&self, output: &mut String) {
        let stderr_trimmed = self.stderr.trim();
        if !stderr_trimmed.is_empty() {
            // Ensure there's a newline before stderr content
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            // Mark each stderr line with marker for red rendering
            for line in stderr_trimmed.lines() {
                output.push_str(STDERR_MARKER);
                output.push_str(line);
                output.push('\n');
            }
        }
    }

    /// Combine stdout and stderr into a single string (helper for DRY)
    /// Marks stderr lines with STDERR_MARKER for red styling in UI
    pub fn combine_outputs(&self) -> String {
        let stdout_trimmed = self.stdout.trim();
        let stderr_trimmed = self.stderr.trim();

        match (stdout_trimmed.is_empty(), stderr_trimmed.is_empty()) {
            (true, true) => String::new(),
            (false, true) => stdout_trimmed.to_string(),
            (true, false) => {
                // Mark each stderr line
                stderr_trimmed
                    .lines()
                    .map(|line| format!("{STDERR_MARKER}{line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            (false, false) => {
                // stdout unchanged, stderr marked
                let marked_stderr = stderr_trimmed
                    .lines()
                    .map(|line| format!("{STDERR_MARKER}{line}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{stdout_trimmed}\n{marked_stderr}")
            }
        }
    }

    /// Convert to Result based on success status
    pub fn into_result(self) -> Result<String, ToolError> {
        if self.success {
            Ok(self.format_success())
        } else {
            Err(ToolError::Execution {
                tool: "bash",
                message: self.format_error(),
            })
        }
    }
}

/// Manages stdout and stderr buffers for command execution.
///
/// Encapsulates buffer creation and content extraction (DRY principle).
///
/// stdout is stored as raw bytes (`Vec<u8>`) rather than `String` so that
/// binary payloads survive the capture path intact — the BUG-142 binary-output
/// guard inspects the raw bytes to decide whether to suppress the output.
/// stderr remains a String because stderr is by convention text diagnostics.
#[derive(Default)]
pub struct StreamBuffers {
    stdout: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<String>>,
}

impl StreamBuffers {
    /// Create new empty stream buffers.
    pub fn new() -> Self {
        Self {
            stdout: Arc::new(Mutex::new(Vec::new())),
            stderr: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Get a clone of the stdout buffer handle (raw bytes).
    pub fn stdout_handle(&self) -> Arc<Mutex<Vec<u8>>> {
        Arc::clone(&self.stdout)
    }

    /// Get a clone of the stderr buffer handle.
    pub fn stderr_handle(&self) -> Arc<Mutex<String>> {
        Arc::clone(&self.stderr)
    }

    /// Extract the buffered stdout (raw bytes) and stderr (decoded string) content.
    pub async fn extract(self) -> (Vec<u8>, String) {
        let stdout = self.stdout.lock().await.clone();
        let stderr = self.stderr.lock().await.clone();
        (stdout, stderr)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn empty_merged_output_splits_to_two_empty_strings() {
        let (out, err) = split_merged_output("");
        assert_eq!(out, "");
        assert_eq!(err, "");
    }

    #[test]
    fn stdout_only_lines_stay_in_stdout() {
        // Trailing-newline semantics: the final empty segment after the
        // last `\n` round-trips as a trailing newline (byte-perfect
        // stdout half — matches the pre-P4 raw-bytes contract).
        let (out, err) = split_merged_output("a\nb\n");
        assert_eq!(out, "a\nb\n");
        assert_eq!(err, "");
    }

    #[test]
    fn stderr_lines_strip_marker_and_landing_in_stderr() {
        let (out, err) = split_merged_output(&format!("{STDERR_MARKER}e1\n{STDERR_MARKER}e2\n"));
        assert_eq!(out, "");
        assert_eq!(err, "e1\ne2");
    }

    #[test]
    fn interleaved_streams_split_by_marker_prefix() {
        let (out, err) = split_merged_output(
            &format!("s1\n{STDERR_MARKER}e1\ns2\n{STDERR_MARKER}e2\ns3"),
        );
        assert_eq!(out, "s1\ns2\ns3");
        assert_eq!(err, "e1\ne2");
    }

    #[test]
    fn stderr_line_with_empty_payload_is_preserved() {
        // The marker line itself carries no payload; the trailing
        // newline round-trips in the stdout half (see
        // stdout_only_lines_stay_in_stdout).
        let (out, err) = split_merged_output(&format!("s\n{STDERR_MARKER}\n"));
        assert_eq!(out, "s\n");
        assert_eq!(err, "");
    }

    #[test]
    fn marker_only_line_with_trailing_content_kept_verbatim() {
        let (out, err) = split_merged_output(&format!("{STDERR_MARKER}fatal: bad thing"));
        assert_eq!(out, "");
        assert_eq!(err, "fatal: bad thing");
    }

    // ==================================================================
    // split_merged_output_bytes (TOOL-022 P4 — binary-safe split)
    // ==================================================================

    #[test]
    fn bytes_split_empty_input() {
        assert_eq!(split_merged_output_bytes(b""), (vec![], vec![]));
    }

    #[test]
    fn bytes_split_binary_stdout_round_trips() {
        // A PNG-ish payload: raw binary bytes incl. NUL must come back
        // BYTE-IDENTICAL (the BUG-142 guard needs the magic bytes).
        let png_magic = [0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01];
        let merged: Vec<u8> = png_magic.to_vec();
        let (out, err) = split_merged_output_bytes(&merged);
        assert_eq!(out, png_magic.to_vec());
        assert!(err.is_empty());
    }

    #[test]
    fn bytes_split_stderr_lines_strip_marker() {
        let merged = format!("out1\n{STDERR_MARKER}err1\nout2\n{STDERR_MARKER}err2");
        let merged = merged.as_bytes();
        let (out, err) = split_merged_output_bytes(merged);
        assert_eq!(out, "out1\nout2".as_bytes());
        assert_eq!(err, "err1\nerr2".as_bytes());
    }

    #[test]
    fn bytes_split_stdout_newlines_are_preserved() {
        // Newlines inside the stdout half must survive verbatim.
        let stdout_bytes = b"a\nb\nc";
        let (out, err) = split_merged_output_bytes(stdout_bytes);
        assert_eq!(out, stdout_bytes);
        assert!(err.is_empty());
    }
}
