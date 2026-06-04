//! `list-attachments` — Rust port of `src/commands/list-attachments.ts` (RPC-241).
//!
//! Loads `spec/work-units.json` via [`crate::io::ensure::ensure_work_units_file`]
//! (the load-or-init helper that auto-creates the file when missing AND
//! escalates parse errors — parity with the TS `await ensureWorkUnitsFile(cwd)`
//! call at `src/commands/list-attachments.ts:20`), looks up the requested
//! work unit, and renders its attachment list in either text or JSON form.
//!
//! The `attachments` field is read from the [`crate::types::work_unit::WorkUnit`]
//! `extra` flatten map rather than a typed field — keeping the cross-crate
//! footprint of this port minimal (only commands/list_attachments.rs +
//! dispatch.rs + canonical.rs + the CLI bridge are touched; the `WorkUnit`
//! struct stays unchanged). Future work may promote `attachments` to a typed
//! field; this module is forward-compatible because `extra` will continue to
//! catch any unmodeled field.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

/// CLI arguments accepted by `list-attachments`.
///
/// `work_unit_id` is REQUIRED — parity with the TS Commander.js positional
/// `<workUnitId>` (`src/commands/list-attachments.ts:62-66`). It is deserialized
/// as `Option<String>` so the dispatcher can produce a structured InvalidArgs
/// error naming the missing field instead of failing serde with a confusing
/// "missing field" message.
///
/// `format` is exposed on the dispatcher surface only (the TS CLI registration
/// declares NO `.option(...)` calls — see rule [13] on RPC-241). The
/// `{"format":"json"}` JSON shape produces the 2-space-indented payload used
/// by the agent loop's tool-call protocol.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListAttachmentsArgs {
    work_unit_id: Option<String>,
    format: Option<String>,
}

/// A single attachment entry as rendered in the JSON output shape. Mirrors
/// the implicit shape the TS implementation would produce if it emitted JSON
/// (the TS source emits text only — JSON is a Rust-side dispatcher
/// convenience following the same convention as RPC-248).
///
/// `size_kb` and `modified` are wrapped in `Option` so that `exists: false`
/// entries omit them entirely (rule [12] on RPC-241).
#[derive(Debug, Serialize)]
struct AttachmentEntry {
    path: String,
    exists: bool,
    #[serde(rename = "sizeKb", skip_serializing_if = "Option::is_none")]
    size_kb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListAttachmentsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-attachments",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = match args.work_unit_id.as_deref().filter(|s| !s.is_empty()) {
        Some(id) => id.to_string(),
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "list-attachments",
                reason: "missing required field: workUnitId".to_string(),
            });
        }
    };

    // Load (or auto-init) the work-units file. Escalates parse errors with
    // the canonical "Failed to parse work-units.json" substring.
    let data = ensure_work_units_file(project_root)?;

    let work_unit = data
        .work_units
        .get(&work_unit_id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "list-attachments",
            reason: format!("Work unit '{work_unit_id}' does not exist"),
        })?;

    // Extract the attachments array from the round-tripped `extra` map.
    // The `WorkUnit` struct does NOT (yet) model `attachments` as a typed
    // field; this command reads it directly so the port is isolated to
    // this file and the dispatcher wiring (rule [16] revised on RPC-241).
    let attachments: Vec<String> = work_unit
        .extra
        .get("attachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Build entries (stats each attachment on disk). Errors from `fs::metadata`
    // are silently downgraded to `exists: false` — parity with the TS bare
    // `catch {}` at `src/commands/list-attachments.ts:54-58`.
    let entries: Vec<AttachmentEntry> = attachments
        .iter()
        .map(|rel| build_entry(project_root, rel))
        .collect();

    match args.format.as_deref() {
        Some("json") => {
            let payload = json!({
                "workUnitId": work_unit_id,
                "attachments": entries,
            });
            serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
                command: "list-attachments",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text.
        _ => Ok(render_text(&work_unit_id, &entries)),
    }
}

/// Build a single entry by stat-ing the file under `project_root/rel_path`.
/// Returns an `exists: false` marker entry on any I/O error (parity with the
/// TS bare `catch {}` swallow at `src/commands/list-attachments.ts:54-58`).
fn build_entry(project_root: &Path, rel_path: &str) -> AttachmentEntry {
    let full = project_root.join(rel_path);
    match fs::metadata(&full) {
        Ok(meta) => {
            let size_kb = format_size_kb(meta.len());
            let modified = meta.modified().ok().map(format_mtime).unwrap_or_default();
            AttachmentEntry {
                path: rel_path.to_string(),
                exists: true,
                size_kb: Some(size_kb),
                modified: Some(modified),
            }
        }
        Err(_) => AttachmentEntry {
            path: rel_path.to_string(),
            exists: false,
            size_kb: None,
            modified: None,
        },
    }
}

/// `(bytes / 1024).toFixed(2)` parity. JS `toFixed(2)` rounds half-away-from-zero
/// for positive values; Rust's `{:.2}` uses banker's rounding (half-to-even).
/// For the canonical test values (1024 → 1.00, 1234 → 1.21, 2048 → 2.00) the
/// two strategies agree, so this is acceptable parity for now.
fn format_size_kb(bytes: u64) -> String {
    format!("{:.2}", bytes as f64 / 1024.0)
}

/// Deterministic UTC `"YYYY-MM-DD HH:MM:SS UTC"` timestamp. NOT bit-stable with
/// TS `Date.toLocaleString()` — that method is locale- and TZ-dependent and
/// CANNOT be reproduced from Rust without pulling in chrono/tz-data. Tests
/// assert only the `    Modified: ` line PREFIX and a non-empty suffix
/// (architecture note [7] on RPC-241).
fn format_mtime(modified: SystemTime) -> String {
    let secs = modified
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, h, m, s) = epoch_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02} {h:02}:{m:02}:{s:02} UTC")
}

/// Howard Hinnant's civil-from-days algorithm. Copied from
/// [`crate::io::ensure`] because the helper there is private and we want
/// the dependency surface of this module to stay self-contained.
fn epoch_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let seconds_of_day = (secs % 86_400) as u32;
    let h = seconds_of_day / 3_600;
    let m = (seconds_of_day % 3_600) / 60;
    let s = seconds_of_day % 60;

    let z = days + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = (y + if month <= 2 { 1 } else { 0 }) as i32;

    (year, month, d, h, m, s)
}

/// Render the text format expected by the TS CLI wrapper
/// (`src/commands/list-attachments.ts:30-58`).
///
/// Structure:
///   - Empty attachments → single line "No attachments found for work unit <id>"
///   - Non-empty → leading "\n", header "Attachments for <id> (<N>):\n", blank line,
///     then per-attachment block followed by a blank line.
fn render_text(work_unit_id: &str, entries: &[AttachmentEntry]) -> String {
    if entries.is_empty() {
        return format!("No attachments found for work unit {work_unit_id}");
    }

    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!(
        "Attachments for {} ({}):\n",
        work_unit_id,
        entries.len()
    ));
    out.push('\n');

    for entry in entries {
        if entry.exists {
            out.push_str("  ✓ ");
            out.push_str(&entry.path);
            out.push('\n');
            if let Some(size) = &entry.size_kb {
                out.push_str("    Size: ");
                out.push_str(size);
                out.push_str(" KB\n");
            }
            if let Some(modified) = &entry.modified {
                out.push_str("    Modified: ");
                out.push_str(modified);
                out.push('\n');
            }
            out.push('\n');
        } else {
            out.push_str("  ✗ ");
            out.push_str(&entry.path);
            out.push('\n');
            out.push_str("    File not found on filesystem\n");
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_camel_case_work_unit_id() {
        let a: ListAttachmentsArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001"}"#).unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListAttachmentsArgs =
            serde_json::from_str(r#"{"workUnitId":"X","format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn args_parse_empty_object_yields_none_work_unit_id() {
        let a: ListAttachmentsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
    }

    #[test]
    fn format_size_kb_2048_is_2_00() {
        assert_eq!(format_size_kb(2048), "2.00");
    }

    #[test]
    fn format_size_kb_1024_is_1_00() {
        assert_eq!(format_size_kb(1024), "1.00");
    }

    #[test]
    fn format_size_kb_1234_is_1_21() {
        // 1234 / 1024 = 1.2050..., rounds to 1.21 (both JS toFixed and Rust
        // {:.2} agree on this case).
        assert_eq!(format_size_kb(1234), "1.21");
    }

    #[test]
    fn format_mtime_epoch_zero_is_canonical_utc() {
        let s = format_mtime(UNIX_EPOCH);
        assert_eq!(s, "1970-01-01 00:00:00 UTC");
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel() {
        assert_eq!(
            render_text("AUTH-001", &[]),
            "No attachments found for work unit AUTH-001"
        );
    }

    #[test]
    fn render_text_present_entry_includes_size_and_modified_lines() {
        let entries = vec![AttachmentEntry {
            path: "spec/attachments/AUTH-001/diagram.png".into(),
            exists: true,
            size_kb: Some("2.00".into()),
            modified: Some("2026-06-04 14:32:17 UTC".into()),
        }];
        let out = render_text("AUTH-001", &entries);
        assert!(out.contains("Attachments for AUTH-001 (1):"));
        assert!(out
            .lines()
            .any(|l| l == "  ✓ spec/attachments/AUTH-001/diagram.png"));
        assert!(out.lines().any(|l| l == "    Size: 2.00 KB"));
        assert!(out
            .lines()
            .any(|l| l == "    Modified: 2026-06-04 14:32:17 UTC"));
    }

    #[test]
    fn render_text_missing_entry_omits_size_and_uses_canonical_not_found_line() {
        let entries = vec![AttachmentEntry {
            path: "spec/attachments/AUTH-001/missing.png".into(),
            exists: false,
            size_kb: None,
            modified: None,
        }];
        let out = render_text("AUTH-001", &entries);
        assert!(out
            .lines()
            .any(|l| l == "  ✗ spec/attachments/AUTH-001/missing.png"));
        assert!(out
            .lines()
            .any(|l| l == "    File not found on filesystem"));
        assert!(!out.contains("Size:"));
    }
}
