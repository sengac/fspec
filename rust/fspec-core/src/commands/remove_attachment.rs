//! `remove-attachment` — Rust port of `src/commands/remove-attachment.ts` (RPC-268).
//!
//! Removes a file attachment from a work unit's `attachments` array and,
//! unless `keepFile=true`, unlinks the underlying file from disk. The
//! `attachments` field is read/written via [`crate::types::work_unit::WorkUnit::extra`]
//! (no struct change) — same approach as the `list-attachments` /
//! `add-attachment` ports.
//!
//! ## Behavioural parity with `src/commands/remove-attachment.ts`
//!
//! 1. Resolve work-units file via [`ensure_work_units_file`] — auto-creates
//!    `spec/work-units.json` on first run.
//! 2. Validate work unit exists: `Work unit '<id>' does not exist`.
//! 3. Validate `attachments` array is present AND non-empty:
//!    `Work unit '<id>' has no attachments to remove`.
//! 4. Locate the FIRST entry whose stored path **ends with** the supplied
//!    `fileName` (TS `findIndex(p => p.endsWith(options.fileName))`).
//!    Not found → `Attachment '<fileName>' not found for work unit '<id>'`.
//! 5. Splice the entry out of the array.
//! 6. If `keepFile == true`: render `✓ Attachment removed from work unit
//!    (file kept)`. Otherwise attempt `std::fs::remove_file(fullPath)`.
//!    Any `Err` (NotFound, permission, etc.) downgrades to `⚠ Attachment
//!    removed from work unit (file was already missing)` — matching the
//!    TS bare `catch {}` semantics. Success renders `✓ Attachment removed
//!    from work unit and file deleted`.
//! 7. Append `  File: <attachmentPath>` as the second output line.
//! 8. Bump `updatedAt`, set `meta.lastUpdated` if `meta` exists.
//! 9. Single `write_json_atomic` write at the end.
//!
//! ## Critical divergences from `add-attachment`
//!
//! - Status of the work unit is NEVER consulted. (TS does not gate.)
//! - The full path used for `remove_file` is `project_root.join(stored_path)`
//!   — the array stores forward-slash project-relative paths, which
//!   `PathBuf::join` handles transparently on both unix and windows.
//! - An unknown filename does NOT mutate the file at all and produces a
//!   byte-equal `work-units.json`.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoveAttachmentArgs {
    work_unit_id: String,
    file_name: String,
    keep_file: Option<bool>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveAttachmentArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-attachment",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    // Work-unit gate.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-attachment",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Locate the index + capture the attachment path. Borrow-scope-bounded
    // so we can re-borrow `data` mutably to splice + bump.
    let attachment_path: String = {
        let wu = match data.work_units.get(&args.work_unit_id) {
            Some(wu) => wu,
            // Unreachable: presence was checked above. Returning an error
            // here keeps clippy::expect_used clean while preserving the
            // missing-work-unit contract.
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "remove-attachment",
                    reason: format!("Work unit '{}' does not exist", args.work_unit_id),
                });
            }
        };
        let arr = match wu.extra.get("attachments").and_then(Value::as_array) {
            Some(a) if !a.is_empty() => a,
            _ => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "remove-attachment",
                    reason: format!(
                        "Work unit '{}' has no attachments to remove",
                        args.work_unit_id
                    ),
                });
            }
        };

        let found = arr.iter().find_map(|v| {
            v.as_str()
                .filter(|p| p.ends_with(args.file_name.as_str()))
                .map(str::to_string)
        });
        match found {
            Some(p) => p,
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "remove-attachment",
                    reason: format!(
                        "Attachment '{}' not found for work unit '{}'",
                        args.file_name, args.work_unit_id
                    ),
                });
            }
        }
    };

    // Splice the entry out of the array.
    {
        let wu = match data.work_units.get_mut(&args.work_unit_id) {
            Some(wu) => wu,
            // Unreachable: presence verified above. Bail with InvalidArgs
            // rather than panic to satisfy clippy::expect_used.
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "remove-attachment",
                    reason: format!("Work unit '{}' does not exist", args.work_unit_id),
                });
            }
        };
        if let Some(arr) = wu
            .extra
            .get_mut("attachments")
            .and_then(Value::as_array_mut)
        {
            if let Some(pos) = arr
                .iter()
                .position(|v| v.as_str() == Some(attachment_path.as_str()))
            {
                arr.remove(pos);
            }
        }
        wu.updated_at = iso8601_now();
    }

    // Unlink / keep-file branch produces the first output line.
    let keep_file = args.keep_file.unwrap_or(false);
    let mut out = String::new();
    if keep_file {
        out.push_str("✓ Attachment removed from work unit (file kept)\n");
    } else {
        let full_path = project_root.join(&attachment_path);
        match std::fs::remove_file(&full_path) {
            Ok(()) => {
                out.push_str("✓ Attachment removed from work unit and file deleted\n");
            }
            Err(_) => {
                out.push_str("⚠ Attachment removed from work unit (file was already missing)\n");
            }
        }
    }
    out.push_str("  File: ");
    out.push_str(&attachment_path);
    out.push('\n');

    // meta.lastUpdated bump.
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = iso8601_now();
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_with_camel_case() {
        let a: RemoveAttachmentArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","fileName":"d.png"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.file_name, "d.png");
        assert!(a.keep_file.is_none());
    }

    #[test]
    fn args_parse_with_keep_file() {
        let a: RemoveAttachmentArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","fileName":"d.png","keepFile":true}"#)
                .unwrap();
        assert_eq!(a.keep_file, Some(true));
    }
}
