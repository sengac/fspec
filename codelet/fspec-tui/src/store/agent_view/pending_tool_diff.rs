//! RPC-391 — Edit/Write diff capture + production.
//!
//! Feature: spec/features/agentview-edit-diff-rendering.feature
//!
//! Mirrors the TS `pendingToolDiffsRef` flow (`AgentView.tsx:2067-2113`,
//! `:2173-2208`): at tool-call time the Edit/Write input is parsed and
//! stashed keyed by tool-call id with a precomputed `start_line`; on the
//! matching ToolResult it is consumed to build the marker-encoded diff
//! (collapsed inline body + full body for the modal).
//!
//! Lives in its own module so `chunk_processor.rs` / `session_context.rs`
//! stay under the 300-LoC ceiling pinned by `rpc024-source-shape.feature`.

use serde_json::Value;

use super::diff_format::{
    calculate_start_line, format_diff_for_display, format_edit_diff, format_write_diff,
    DIFF_COLLAPSED_LINES,
};

/// Which side of the Edit/Write family produced this pending entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingDiffKind {
    Edit {
        old_string: String,
        new_string: String,
    },
    Write {
        content: String,
    },
}

/// Captured Edit/Write tool input awaiting its matching ToolResult.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingToolDiff {
    pub kind: PendingDiffKind,
    /// 1-based line of the edit within the file (1 for Write / unknown).
    pub start_line: usize,
}

/// Parse a tool-call `input_json` for the Edit/Write family. Returns
/// `None` for non-diff tools or malformed/missing fields (the caller then
/// keeps the raw tool behaviour — no panic). Mirrors the TS lowercase
/// classification (`edit`/`replace` → Edit, `write`/`write_file` → Write).
pub fn capture_pending_diff(tool_name: &str, input_json: &str) -> Option<PendingToolDiff> {
    let parsed: Value = serde_json::from_str(input_json).ok()?;
    let obj = parsed.as_object()?;
    let lower = tool_name.to_lowercase();

    match lower.as_str() {
        "edit" | "replace" => {
            let old_string = obj.get("old_string")?.as_str()?.to_string();
            let new_string = obj.get("new_string")?.as_str()?.to_string();
            let file_path = obj.get("file_path").and_then(|v| v.as_str());
            let start_line = calculate_start_line(file_path, Some(&old_string), Some(&new_string));
            Some(PendingToolDiff {
                kind: PendingDiffKind::Edit {
                    old_string,
                    new_string,
                },
                start_line,
            })
        }
        "write" | "write_file" => {
            let content = obj.get("content")?.as_str()?.to_string();
            Some(PendingToolDiff {
                kind: PendingDiffKind::Write { content },
                start_line: 1,
            })
        }
        _ => None,
    }
}

/// Build the `(collapsed_inline, full)` marker-encoded diff strings from a
/// captured pending entry. `collapsed_inline` is collapsed at
/// `DIFF_COLLAPSED_LINES` (25); `full` keeps every display line for the
/// turn-content modal (parity with TS `toolResultContent` /
/// `toolResultFullContent`).
pub fn produce_diff_strings(pending: &PendingToolDiff) -> (String, String) {
    let lines = match &pending.kind {
        PendingDiffKind::Edit {
            old_string,
            new_string,
        } => format_edit_diff(old_string, new_string),
        PendingDiffKind::Write { content } => format_write_diff(content),
    };
    let collapsed = format_diff_for_display(&lines, DIFF_COLLAPSED_LINES, pending.start_line);
    let full = format_diff_for_display(&lines, lines.len().max(1), pending.start_line);
    (collapsed, full)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn captures_edit_old_and_new() {
        let p = capture_pending_diff("Edit", r#"{"old_string":"a","new_string":"b"}"#).unwrap();
        assert_eq!(
            p.kind,
            PendingDiffKind::Edit {
                old_string: "a".into(),
                new_string: "b".into()
            }
        );
    }

    #[test]
    fn captures_write_content() {
        let p = capture_pending_diff("Write", r#"{"content":"x\ny"}"#).unwrap();
        assert_eq!(
            p.kind,
            PendingDiffKind::Write {
                content: "x\ny".into()
            }
        );
    }

    #[test]
    fn malformed_json_yields_none() {
        assert!(capture_pending_diff("Edit", "not-json").is_none());
    }

    #[test]
    fn non_diff_tool_yields_none() {
        assert!(capture_pending_diff("Bash", r#"{"command":"ls"}"#).is_none());
    }

    #[test]
    fn full_diff_is_not_collapsed() {
        let old: String = (1..=60).map(|i| format!("o{i}\n")).collect();
        let new: String = (1..=60).map(|i| format!("n{i}\n")).collect();
        let p = PendingToolDiff {
            kind: PendingDiffKind::Edit {
                old_string: old,
                new_string: new,
            },
            start_line: 1,
        };
        let (collapsed, full) = produce_diff_strings(&p);
        assert!(collapsed.contains("(select turn to /expand)"));
        assert!(!full.contains("(select turn to /expand)"));
    }
}
