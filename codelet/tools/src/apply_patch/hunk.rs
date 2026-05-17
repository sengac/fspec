//! Hunk application logic for Update File operations.
//!
//! Applies parsed hunks to file content using context-line matching
//! to find the correct position for each change.

use super::parser::Hunk;

/// Apply a series of hunks to file content, returning the new content.
pub(crate) fn apply_hunks(content: &str, hunks: &[Hunk], path: &str) -> Result<String, String> {
    let mut file_lines: Vec<String> = content.lines().map(String::from).collect();

    // Apply hunks in reverse order so earlier indices stay valid.
    // First, resolve each hunk to a position, then sort by position descending.
    let mut positioned: Vec<(usize, &Hunk)> = Vec::new();

    for hunk in hunks {
        let pos = find_hunk_position(&file_lines, hunk, path)?;
        positioned.push((pos, hunk));
    }

    // Sort descending by position so we apply from bottom to top.
    positioned.sort_by_key(|b| std::cmp::Reverse(b.0));

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
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_hunks_update() {
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

    #[test]
    fn test_apply_hunks_context_mismatch() {
        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        let hunks = vec![Hunk {
            context_before: vec!["fn nonexistent() {".to_string()],
            removals: vec!["    old();".to_string()],
            additions: vec!["    new();".to_string()],
            context_after: vec![],
        }];

        let result = apply_hunks(content, &hunks, "/tmp/test/mismatch.rs");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Context mismatch"));
        assert!(err.contains("/tmp/test/mismatch.rs"));
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
}
