//! Codex freeform patch parser.
//!
//! Parses the `*** Begin Patch` / `*** End Patch` format into
//! structured `PatchOp` values (Add, Update, Delete).

// ============================================================================
// Patch data model
// ============================================================================

/// A single file operation parsed from a Codex patch.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PatchOp {
    /// Create a new file with the given content lines.
    Add { path: String, lines: Vec<String> },
    /// Apply hunks to an existing file.
    Update { path: String, hunks: Vec<Hunk> },
    /// Delete a file from disk.
    Delete { path: String },
}

/// One contiguous change block inside an Update operation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Hunk {
    /// Context lines that anchor this hunk (lines prefixed with space or no prefix).
    pub context_before: Vec<String>,
    /// Lines to remove (prefixed with `-` in the patch).
    pub removals: Vec<String>,
    /// Lines to add (prefixed with `+` in the patch).
    pub additions: Vec<String>,
    /// Context lines after the change block.
    pub context_after: Vec<String>,
}

// ============================================================================
// Parser
// ============================================================================

/// Parse the Codex freeform patch text into a list of `PatchOp`s.
pub(crate) fn parse_patch(text: &str) -> Result<Vec<PatchOp>, String> {
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

        if let Some(stripped) = line.strip_prefix('-') {
            in_change = true;
            removals.push(stripped.to_string());
            i += 1;
        } else if let Some(stripped) = line.strip_prefix('+') {
            in_change = true;
            additions.push(stripped.to_string());
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
            let ctx_text = if let Some(stripped) = line.strip_prefix(' ') {
                stripped.to_string()
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
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_add_file() {
        let patch = "\
*** Begin Patch
*** Add File: /tmp/test/new_file.rs
+fn main() {
+    println!(\"hello\");
+}
*** End Patch";

        let ops = parse_patch(patch).unwrap();
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

    #[test]
    fn test_parse_update_file() {
        let patch = "\
*** Begin Patch
*** Update File: /tmp/test/existing.rs
@@ fn main() {
-    println!(\"old\");
+    println!(\"new\");
*** End Patch";

        let ops = parse_patch(patch).unwrap();
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
    fn test_parse_delete_file() {
        let patch = "\
*** Begin Patch
*** Delete File: /tmp/test/to_delete.rs
*** End Patch";

        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            PatchOp::Delete { path } => {
                assert_eq!(path, "/tmp/test/to_delete.rs");
            }
            _ => panic!("Expected PatchOp::Delete"),
        }
    }

    #[test]
    fn test_parse_multi_file_patch() {
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
        assert_eq!(ops.len(), 3);
        assert!(matches!(&ops[0], PatchOp::Add { path, .. } if path == "/tmp/test/new.rs"));
        assert!(matches!(&ops[1], PatchOp::Update { path, .. } if path == "/tmp/test/update_me.rs"));
        assert!(matches!(&ops[2], PatchOp::Delete { path } if path == "/tmp/test/delete_me.rs"));
    }

    #[test]
    fn test_parse_malformed_no_begin() {
        let patch = "*** Add File: /foo\n+hello\n*** End Patch";
        let result = parse_patch(patch);
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

    #[test]
    fn test_parse_empty_patch() {
        let patch = "*** Begin Patch\n*** End Patch";
        let ops = parse_patch(patch).unwrap();
        assert!(ops.is_empty());
    }
}
