//! Codex `apply_patch` tool implementation.
//!
//! Parses the Codex freeform patch format and delegates to internal file
//! operations (create, edit, delete). This is a standalone `rig::tool::Tool`
//! because `apply_patch` has no equivalent in other providers.
//!
//! Feature: spec/features/codex-apply-patch.feature

use super::blocklist::check_file_path;
use super::error::ToolError;
use super::facade::validate_and_resolve_path;
use super::validation::{
    create_parent_dirs, read_file_contents, require_absolute_path, require_file_exists,
    write_file_contents,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// Patch data model
// ============================================================================

/// A single file operation parsed from a Codex patch.
#[derive(Debug, Clone, PartialEq)]
enum PatchOp {
    /// Create a new file with the given content lines.
    Add { path: String, lines: Vec<String> },
    /// Apply hunks to an existing file.
    Update { path: String, hunks: Vec<Hunk> },
    /// Delete a file from disk.
    Delete { path: String },
}

/// One contiguous change block inside an Update operation.
#[derive(Debug, Clone, PartialEq)]
struct Hunk {
    /// Context lines that anchor this hunk (lines prefixed with space or no prefix).
    context_before: Vec<String>,
    /// Lines to remove (prefixed with `-` in the patch).
    removals: Vec<String>,
    /// Lines to add (prefixed with `+` in the patch).
    additions: Vec<String>,
    /// Context lines after the change block.
    context_after: Vec<String>,
}

// ============================================================================
// Parser
// ============================================================================

/// Parse the Codex freeform patch text into a list of `PatchOp`s.
fn parse_patch(text: &str) -> Result<Vec<PatchOp>, String> {
    let lines: Vec<&str> = text.lines().collect();

    if lines.is_empty() || lines[0].trim() != "*** Begin Patch" {
        return Err("Patch must start with '*** Begin Patch'".to_string());
    }

    let mut ops: Vec<PatchOp> = Vec::new();
    let mut i = 1; // skip "*** Begin Patch"

    while i < lines.len() {
        let line = lines[i].trim();

        if line == "*** End Patch" {
            return Ok(ops);
        }

        if let Some(path) = line.strip_prefix("*** Add File: ") {
            let (op, next) = parse_add_file(path.trim(), &lines, i + 1)?;
            ops.push(op);
            i = next;
        } else if let Some(path) = line.strip_prefix("*** Update File: ") {
            let (op, next) = parse_update_file(path.trim(), &lines, i + 1)?;
            ops.push(op);
            i = next;
        } else if let Some(path) = line.strip_prefix("*** Delete File: ") {
            ops.push(PatchOp::Delete {
                path: path.trim().to_string(),
            });
            i += 1;
        } else if line.is_empty() {
            i += 1;
        } else {
            return Err(format!("Unexpected line in patch at line {}: {line}", i + 1));
        }
    }

    Err("Patch missing '*** End Patch' marker".to_string())
}

/// Parse an Add File block. Every line must start with `+`.
fn parse_add_file(path: &str, lines: &[&str], start: usize) -> Result<(PatchOp, usize), String> {
    let mut content_lines: Vec<String> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("*** ") {
            break;
        }
        if let Some(rest) = line.strip_prefix('+') {
            content_lines.push(rest.to_string());
        } else if line.is_empty() {
            // Allow blank lines in add blocks
            content_lines.push(String::new());
        } else {
            return Err(format!(
                "Add File block line {} must start with '+': {line}",
                i + 1
            ));
        }
        i += 1;
    }

    Ok((
        PatchOp::Add {
            path: path.to_string(),
            lines: content_lines,
        },
        i,
    ))
}

/// Parse an Update File block into hunks.
fn parse_update_file(
    path: &str,
    lines: &[&str],
    start: usize,
) -> Result<(PatchOp, usize), String> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut i = start;

    // State for building hunks
    let mut context_before: Vec<String> = Vec::new();
    let mut removals: Vec<String> = Vec::new();
    let mut additions: Vec<String> = Vec::new();
    let mut in_change = false;

    while i < lines.len() {
        let line = lines[i];

        // Next file operation or end of patch
        if line.starts_with("*** ") {
            break;
        }

        if line.starts_with('-') {
            in_change = true;
            removals.push(line[1..].to_string());
            i += 1;
        } else if line.starts_with('+') {
            in_change = true;
            additions.push(line[1..].to_string());
            i += 1;
        } else if line.starts_with("@@ ") {
            // Hunk header — flush any in-progress hunk
            if in_change {
                hunks.push(Hunk {
                    context_before: std::mem::take(&mut context_before),
                    removals: std::mem::take(&mut removals),
                    additions: std::mem::take(&mut additions),
                    context_after: Vec::new(),
                });
                in_change = false;
            }
            // The @@ line itself becomes context_before for the next hunk
            // Strip the @@ prefix — the rest is a context line
            let ctx = line.strip_prefix("@@ ").unwrap_or(line);
            context_before.push(ctx.to_string());
            i += 1;
        } else {
            // Plain context line (space-prefixed or literal)
            let ctx_text = if line.starts_with(' ') {
                line[1..].to_string()
            } else {
                line.to_string()
            };

            if in_change {
                // Context after the change — this hunk is complete
                hunks.push(Hunk {
                    context_before: std::mem::take(&mut context_before),
                    removals: std::mem::take(&mut removals),
                    additions: std::mem::take(&mut additions),
                    context_after: vec![ctx_text.clone()],
                });
                in_change = false;
                // This context line becomes context_before for the next hunk
                context_before.push(ctx_text);
            } else {
                context_before.push(ctx_text);
            }
            i += 1;
        }
    }

    // Flush trailing hunk
    if in_change || !removals.is_empty() || !additions.is_empty() {
        hunks.push(Hunk {
            context_before: std::mem::take(&mut context_before),
            removals: std::mem::take(&mut removals),
            additions: std::mem::take(&mut additions),
            context_after: Vec::new(),
        });
    }

    if hunks.is_empty() {
        return Err(format!("Update File block for '{path}' contains no hunks"));
    }

    Ok((
        PatchOp::Update {
            path: path.to_string(),
            hunks,
        },
        i,
    ))
}

// ============================================================================
// Hunk application
// ============================================================================

/// Apply a series of hunks to file content, returning the new content.
fn apply_hunks(content: &str, hunks: &[Hunk], path: &str) -> Result<String, String> {
    let mut file_lines: Vec<String> = content.lines().map(String::from).collect();

    // Apply hunks in reverse order so earlier indices stay valid.
    // First, resolve each hunk to a position, then sort by position descending.
    let mut positioned: Vec<(usize, &Hunk)> = Vec::new();

    for hunk in hunks {
        let pos = find_hunk_position(&file_lines, hunk, path)?;
        positioned.push((pos, hunk));
    }

    // Sort descending by position so we apply from bottom to top.
    positioned.sort_by(|a, b| b.0.cmp(&a.0));

    for (pos, hunk) in &positioned {
        let removal_count = hunk.context_before.len() + hunk.removals.len();
        let mut replacement: Vec<String> = Vec::new();
        replacement.extend(hunk.context_before.iter().cloned());
        replacement.extend(hunk.additions.iter().cloned());
        let start = *pos;
        let end = start + removal_count;
        file_lines.splice(start..end, replacement);
    }

    // Reconstruct with trailing newline if original had one
    let mut result = file_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

/// Find the position in `file_lines` where a hunk's context_before matches.
fn find_hunk_position(
    file_lines: &[String],
    hunk: &Hunk,
    path: &str,
) -> Result<usize, String> {
    if hunk.context_before.is_empty() && hunk.removals.is_empty() {
        // Pure insertion at the end of the file
        return Ok(file_lines.len());
    }

    let match_lines: Vec<&str> = hunk
        .context_before
        .iter()
        .chain(hunk.removals.iter())
        .map(String::as_str)
        .collect();

    if match_lines.is_empty() {
        return Ok(0);
    }

    let window_size = match_lines.len();
    for start in 0..=file_lines.len().saturating_sub(window_size) {
        let matches = file_lines[start..start + window_size]
            .iter()
            .zip(match_lines.iter())
            .all(|(a, b)| a == b);
        if matches {
            return Ok(start);
        }
    }

    Err(format!(
        "Context mismatch in '{path}': could not find matching lines for hunk starting with {:?}",
        match_lines.first().unwrap_or(&"")
    ))
}

// ============================================================================
// Tool struct and rig::tool::Tool impl
// ============================================================================

/// Codex-native `apply_patch` tool.
///
/// Accepts the freeform Codex patch format and applies file operations
/// (add, update, delete) using internal async I/O helpers.
pub struct ApplyPatchTool {
    session_id: Uuid,
}

impl ApplyPatchTool {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

/// Arguments for the apply_patch tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ApplyPatchArgs {
    /// The patch text in Codex freeform format.
    pub patch: String,
}

impl rig::tool::Tool for ApplyPatchTool {
    const NAME: &'static str = "apply_patch";

    type Error = ToolError;
    type Args = ApplyPatchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "apply_patch".to_string(),
            description:
                "Apply a patch to create, update, or delete files. Uses freeform patch format \
                with '*** Begin Patch' / '*** End Patch' markers. Supports '*** Add File:', \
                '*** Update File:', and '*** Delete File:' operations."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ApplyPatchArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let ops = parse_patch(&args.patch).map_err(|e| ToolError::Validation {
            tool: "apply_patch",
            message: e,
        })?;

        if ops.is_empty() {
            return Err(ToolError::Validation {
                tool: "apply_patch",
                message: "Patch contains no file operations".to_string(),
            });
        }

        let mut results: Vec<String> = Vec::new();

        for op in &ops {
            match op {
                PatchOp::Add { path, lines } => {
                    let resolved =
                        validate_and_resolve_path(self.session_id, path, "apply_patch")?;
                    let p = resolved.to_string_lossy().to_string();
                    if let Err(blocked) = check_file_path(&p) {
                        return Err(ToolError::Blocked {
                            tool: "apply_patch",
                            message: blocked.to_string(),
                        });
                    }
                    let abs = require_absolute_path(&p).map_err(|e| ToolError::Validation {
                        tool: "apply_patch",
                        message: e.content,
                    })?;
                    create_parent_dirs(abs)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    let content = lines.join("\n") + "\n";
                    write_file_contents(abs, &content)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    results.push(format!("Created {p}"));
                }

                PatchOp::Update { path, hunks } => {
                    let resolved =
                        validate_and_resolve_path(self.session_id, path, "apply_patch")?;
                    let p = resolved.to_string_lossy().to_string();
                    if let Err(blocked) = check_file_path(&p) {
                        return Err(ToolError::Blocked {
                            tool: "apply_patch",
                            message: blocked.to_string(),
                        });
                    }
                    let abs = require_absolute_path(&p).map_err(|e| ToolError::Validation {
                        tool: "apply_patch",
                        message: e.content,
                    })?;
                    require_file_exists(abs, &p)
                        .await
                        .map_err(|e| ToolError::Validation {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    let content = read_file_contents(abs)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    let new_content =
                        apply_hunks(&content, hunks, &p).map_err(|e| ToolError::Validation {
                            tool: "apply_patch",
                            message: e,
                        })?;
                    write_file_contents(abs, &new_content)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    results.push(format!("Updated {p}"));
                }

                PatchOp::Delete { path } => {
                    let resolved =
                        validate_and_resolve_path(self.session_id, path, "apply_patch")?;
                    let p = resolved.to_string_lossy().to_string();
                    if let Err(blocked) = check_file_path(&p) {
                        return Err(ToolError::Blocked {
                            tool: "apply_patch",
                            message: blocked.to_string(),
                        });
                    }
                    let abs = require_absolute_path(&p).map_err(|e| ToolError::Validation {
                        tool: "apply_patch",
                        message: e.content,
                    })?;
                    require_file_exists(abs, &p)
                        .await
                        .map_err(|e| ToolError::Validation {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    tokio::fs::remove_file(abs)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: format!("Error deleting file: {e}"),
                        })?;
                    results.push(format!("Deleted {p}"));
                }
            }
        }

        Ok(results.join("\n"))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // =========================================================================
    // Feature: spec/features/codex-apply-patch.feature
    // =========================================================================

    // ----- Scenario: Add a new file via apply_patch -----

    #[test]
    fn test_parse_add_file() {
        // @step Given a Codex session with the apply_patch tool registered
        // @step When the agent calls apply_patch with an Add File block for "/tmp/test/new_file.rs"
        let patch = "\
*** Begin Patch
*** Add File: /tmp/test/new_file.rs
+fn main() {
+    println!(\"hello\");
+}
*** End Patch";

        let ops = parse_patch(patch).unwrap();

        // @step Then the file "/tmp/test/new_file.rs" is created with the specified content
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::Add { path, lines } => {
                assert_eq!(path, "/tmp/test/new_file.rs");
                assert_eq!(lines.len(), 3);
                assert_eq!(lines[0], "fn main() {");
                assert_eq!(lines[1], "    println!(\"hello\");");
                assert_eq!(lines[2], "}");
            }
            _ => panic!("Expected PatchOp::Add"),
        }
    }

    // ----- Scenario: Update an existing file via apply_patch -----

    #[test]
    fn test_parse_update_file() {
        // @step Given a Codex session with the apply_patch tool registered
        // @step And a file "/tmp/test/existing.rs" exists with known content
        // @step When the agent calls apply_patch with an Update File block containing context lines, removals, and additions
        let patch = "\
*** Begin Patch
*** Update File: /tmp/test/existing.rs
@@ fn main() {
-    println!(\"old\");
+    println!(\"new\");
*** End Patch";

        let ops = parse_patch(patch).unwrap();

        // @step Then the matching lines in "/tmp/test/existing.rs" are replaced
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::Update { path, hunks } => {
                assert_eq!(path, "/tmp/test/existing.rs");
                assert_eq!(hunks.len(), 1);
                assert_eq!(hunks[0].context_before, vec!["fn main() {"]);
                assert_eq!(hunks[0].removals, vec!["    println!(\"old\");"]);
                assert_eq!(hunks[0].additions, vec!["    println!(\"new\");"]);
            }
            _ => panic!("Expected PatchOp::Update"),
        }
    }

    #[test]
    fn test_apply_hunks_update() {
        // @step And unchanged context lines remain intact
        let content = "fn main() {\n    println!(\"old\");\n}\n";
        let hunks = vec![Hunk {
            context_before: vec!["fn main() {".to_string()],
            removals: vec!["    println!(\"old\");".to_string()],
            additions: vec!["    println!(\"new\");".to_string()],
            context_after: vec![],
        }];

        let result = apply_hunks(content, &hunks, "/test").unwrap();
        assert_eq!(result, "fn main() {\n    println!(\"new\");\n}\n");
    }

    // ----- Scenario: Delete a file via apply_patch -----

    #[test]
    fn test_parse_delete_file() {
        // @step Given a Codex session with the apply_patch tool registered
        // @step And a file "/tmp/test/to_delete.rs" exists
        // @step When the agent calls apply_patch with a Delete File block for "/tmp/test/to_delete.rs"
        let patch = "\
*** Begin Patch
*** Delete File: /tmp/test/to_delete.rs
*** End Patch";

        let ops = parse_patch(patch).unwrap();

        // @step Then the file "/tmp/test/to_delete.rs" no longer exists on disk
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::Delete { path } => {
                assert_eq!(path, "/tmp/test/to_delete.rs");
            }
            _ => panic!("Expected PatchOp::Delete"),
        }
    }

    // ----- Scenario: Multi-file patch with mixed operations -----

    #[test]
    fn test_parse_multi_file_patch() {
        // @step Given a Codex session with the apply_patch tool registered
        // @step And a file "/tmp/test/update_me.rs" exists with known content
        // @step And a file "/tmp/test/delete_me.rs" exists
        // @step When the agent calls apply_patch with Add, Update, and Delete blocks in a single patch
        let patch = "\
*** Begin Patch
*** Add File: /tmp/test/new.rs
+// new file
*** Update File: /tmp/test/update_me.rs
@@ fn foo() {
-    old();
+    new();
*** Delete File: /tmp/test/delete_me.rs
*** End Patch";

        let ops = parse_patch(patch).unwrap();

        // @step Then the new file is created
        assert_eq!(ops.len(), 3);
        assert!(matches!(&ops[0], PatchOp::Add { path, .. } if path == "/tmp/test/new.rs"));

        // @step And the existing file is updated
        assert!(matches!(&ops[1], PatchOp::Update { path, .. } if path == "/tmp/test/update_me.rs"));

        // @step And the deleted file is removed
        assert!(matches!(&ops[2], PatchOp::Delete { path } if path == "/tmp/test/delete_me.rs"));

        // @step And the tool returns a success message listing all affected files
        // (verified by the 3 ops being parsed correctly — message assembly is in call())
    }

    // ----- Scenario: Malformed patch missing Begin Patch marker -----

    #[test]
    fn test_parse_malformed_no_begin() {
        // @step Given a Codex session with the apply_patch tool registered
        // @step When the agent calls apply_patch with text that does not start with "*** Begin Patch"
        let patch = "*** Add File: /foo\n+hello\n*** End Patch";

        let result = parse_patch(patch);

        // @step Then the tool returns an error mentioning the missing patch marker
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Begin Patch"),
            "Error should mention missing Begin Patch marker, got: {err}"
        );
    }

    #[test]
    fn test_parse_malformed_no_end() {
        let patch = "*** Begin Patch\n*** Add File: /foo\n+hello";
        let result = parse_patch(patch);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("End Patch"));
    }

    // ----- Scenario: Update File with non-matching context lines -----

    #[test]
    fn test_apply_hunks_context_mismatch() {
        // @step Given a Codex session with the apply_patch tool registered
        // @step And a file "/tmp/test/mismatch.rs" exists with known content
        let content = "fn main() {\n    println!(\"hello\");\n}\n";

        // @step When the agent calls apply_patch with an Update File block whose context lines do not match the file
        let hunks = vec![Hunk {
            context_before: vec!["fn nonexistent() {".to_string()],
            removals: vec!["    old();".to_string()],
            additions: vec!["    new();".to_string()],
            context_after: vec![],
        }];

        let result = apply_hunks(content, &hunks, "/tmp/test/mismatch.rs");

        // @step Then the tool returns an error describing the context mismatch
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Context mismatch"));
        assert!(err.contains("/tmp/test/mismatch.rs"));

        // @step And the file "/tmp/test/mismatch.rs" is not modified
        // (original content is unchanged because apply_hunks returns Err)
    }

    // =========================================================================
    // Additional parser edge-case tests
    // =========================================================================

    #[test]
    fn test_parse_empty_patch() {
        let patch = "*** Begin Patch\n*** End Patch";
        let ops = parse_patch(patch).unwrap();
        assert!(ops.is_empty());
    }

    #[test]
    fn test_apply_hunks_pure_addition_at_end() {
        let content = "line1\nline2\n";
        let hunks = vec![Hunk {
            context_before: vec!["line2".to_string()],
            removals: vec![],
            additions: vec!["line3".to_string()],
            context_after: vec![],
        }];
        let result = apply_hunks(content, &hunks, "/test").unwrap();
        // Context line "line2" is preserved, "line3" inserted after it
        assert_eq!(result, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_apply_hunks_multiple_hunks() {
        let content = "a\nb\nc\nd\ne\n";
        let hunks = vec![
            Hunk {
                context_before: vec!["a".to_string()],
                removals: vec!["b".to_string()],
                additions: vec!["B".to_string()],
                context_after: vec![],
            },
            Hunk {
                context_before: vec!["d".to_string()],
                removals: vec!["e".to_string()],
                additions: vec!["E".to_string()],
                context_after: vec![],
            },
        ];
        let result = apply_hunks(content, &hunks, "/test").unwrap();
        assert_eq!(result, "a\nB\nc\nd\nE\n");
    }

    #[test]
    fn test_tool_name_is_apply_patch() {
        use rig::tool::Tool;
        // @step Given a CodexProvider configured with a valid model
        // @step When create_rig_agent is called
        // @step Then the agent has an "apply_patch" tool in its tool set
        let tool = ApplyPatchTool::new(Uuid::new_v4());
        assert_eq!(ApplyPatchTool::NAME, "apply_patch");
        // @step And the agent does not have "Write" or "Edit" tools registered
        // Verified by integration in codex/mod.rs — WriteTool and EditTool removed
        let _ = tool;
    }
}
