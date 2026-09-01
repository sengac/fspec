//! `foundation-status` — Rust-only extension command (DISC-003).
//!
//! Read-only progress report for the foundation discovery workflow. Reports
//! the phase (`none` | `draft` | `final`), an 8-row per-field progress table
//! with previews, the remaining fields with their fix commands and examples,
//! and the finalize next-action. With `json: true` it emits the machine-
//! readable envelope instead of the text report.
//!
//! This command is NOT part of the 162-command canonical TS list — it is a
//! Rust-only extension, so it ships no canonical entry; dispatch routes it via
//! a dedicated arm before the canonical lookup (work unit DISC-003, rule 3).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::foundation::guidance::{self, FieldStatus};

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct FoundationStatusArgs {
    json: bool,
}

/// Dispatcher entry point. Returns a text report or a JSON envelope string.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: FoundationStatusArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "foundation-status",
            reason: format!("failed to parse args: {e}"),
        })?;

    let draft_path = project_root.join("spec").join("foundation.json.draft");
    let final_path = project_root.join("spec").join("foundation.json");

    let (phase, doc) = if draft_path.exists() {
        ("draft", read_json(&draft_path)?)
    } else if final_path.exists() {
        ("final", read_json(&final_path)?)
    } else {
        ("none", Value::Null)
    };

    if args.json {
        let rows = guidance::scan_fields(&doc);
        let remaining: Vec<Value> = rows
            .iter()
            .filter(|r| !r.is_complete())
            .map(|r| {
                json!({
                    "path": r.path,
                    "alias": r.alias,
                    "status": status_name(r.status),
                    "fixCommand": guidance::fix_command(r),
                    "example": guidance::examples_for(r.path),
                })
            })
            .collect();
        return Ok(serde_json::to_string(&json!({
            "phase": phase,
            "progress": {
                "complete": guidance::progress(&rows),
                "total": guidance::TOTAL_FIELDS,
            },
            "fields": rows.iter().map(field_json).collect::<Vec<_>>(),
            "remaining": remaining,
            "nextAction": if remaining.is_empty() {
                "Foundation complete. Run: fspec discover-foundation --finalize"
            } else {
                "Continue discovery. Run: fspec foundation-status"
            },
        }))
        .unwrap_or_default());
    }

    // Text report.
    let mut out = String::new();
    match phase {
        "none" => {
            out.push_str("Foundation: MISSING\n\n");
            out.push_str("No spec/foundation.json or spec/foundation.json.draft found.\n");
            out.push_str("Start discovery: fspec discover-foundation\n");
            return Ok(out);
        }
        "draft" => out.push_str("Foundation: DRAFT (spec/foundation.json.draft)\n\n"),
        "final" => out.push_str("Foundation: FINAL (spec/foundation.json)\n\n"),
        _ => unreachable!(),
    }

    let rows = guidance::scan_fields(&doc);
    let complete = guidance::progress(&rows);
    out.push_str(&format!(
        "Progress: {complete}/{} fields complete\n\n",
        guidance::TOTAL_FIELDS
    ));

    for row in &rows {
        let mark = if row.is_complete() { "✓" } else { "✗" };
        out.push_str(&format!(
            "{mark} {} ({}) — {}\n",
            row.path,
            status_name(row.status),
            row.preview
        ));
    }

    let remaining: Vec<_> = rows.iter().filter(|r| !r.is_complete()).collect();
    if !remaining.is_empty() {
        out.push('\n');
        out.push_str("Remaining (in any order):\n");
        for row in remaining {
            out.push_str(&format!(
                "  - {} ({}) → {}\n",
                row.alias,
                status_name(row.status),
                guidance::fix_command(row)
            ));
            out.push_str(&format!(
                "    Example: {}\n",
                guidance::examples_for(row.path)
            ));
        }
    }

    out.push('\n');
    out.push_str("When complete: fspec discover-foundation --finalize");
    Ok(out)
}

fn read_json(path: &Path) -> Result<Value, FspecCoreError> {
    let raw = std::fs::read_to_string(path).map_err(|source| FspecCoreError::Io {
        command: "foundation-status",
        source,
    })?;
    serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })
}

fn field_json(row: &guidance::FieldRow) -> Value {
    json!({
        "path": row.path,
        "alias": row.alias,
        "status": status_name(row.status),
        "preview": row.preview,
    })
}

fn status_name(status: FieldStatus) -> &'static str {
    match status {
        FieldStatus::Complete => "complete",
        FieldStatus::Placeholder => "placeholder",
        FieldStatus::Missing => "missing",
        FieldStatus::EmptyArray => "empty-array",
        FieldStatus::PlaceholderEntries => "placeholder-entries",
    }
}
