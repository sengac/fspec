//! `add-attachment` — Rust port of `src/commands/add-attachment.ts` (RPC-170).
//!
//! Adds a file attachment to a work unit during Example Mapping. The file
//! is copied into `spec/attachments/<workUnitId>/` and its project-relative
//! path is appended to the work unit's `attachments` array. The
//! `attachments` field is read/written via [`crate::types::work_unit::WorkUnit::extra`]
//! (no struct change) — same approach as the `list-attachments` port
//! (RPC-241).
//!
//! ## Behavioural parity with `src/commands/add-attachment.ts`
//!
//! 1. Resolve work-units file via [`ensure_work_units_file`] — auto-creates
//!    `spec/work-units.json` on first run.
//! 2. Validate work unit exists: `Work unit '<id>' does not exist`.
//! 3. Validate the source file exists. The error uses the **ORIGINAL
//!    caller-supplied path** (TS line 41), not any canonicalised form:
//!    `Source file '<filePath>' does not exist`.
//! 4. `mkdir -p spec/attachments/<workUnitId>/`.
//! 5. Copy source → dest (`spec/attachments/<workUnitId>/<basename>`).
//! 6. **BUG-055 dedup**: if the source file's parent directory equals
//!    `<project_root>/spec/attachments`, delete the source after copy.
//! 7. Compute the project-relative path of the dest. Always rendered with
//!    forward slashes as `spec/attachments/<workUnitId>/<basename>`.
//! 8. Initialise the attachments array if absent; check for duplicate
//!    relativePath — error: `Attachment '<basename>' already exists for
//!    work unit '<workUnitId>'`. Note: matches TS, the duplicate check
//!    runs **AFTER** the copy. Subsequent calls with the same source will
//!    over-write the destination but leave the work-units.json unchanged.
//! 9. Push the relativePath onto attachments.
//! 10. Bump `updatedAt`, set `meta.lastUpdated` if `meta` exists.
//! 11. Single `write_json_atomic` write at the end.
//!
//! ## Scope simplifications (deliberate divergence from TS)
//!
//! - **No Mermaid syntax validation.** The TS path conditionally invokes
//!   `validateMermaidAttachment` for `.mmd` / `.mermaid` / `.md` files.
//!   Pulling jsdom + mermaid into `fspec-core` is inappropriate; the port
//!   omits the validation entirely. Documented in the work unit's
//!   architecture notes (RPC-170).
//! - **No Unicode-whitespace fuzzy path resolution (BUG-130).** The TS
//!   `resolveFilePath` helper rescans the parent directory for filenames
//!   whose code points differ only by U+202F vs U+0020 (a macOS-specific
//!   pasted-screenshot quirk). The Rust port treats `options.filePath`
//!   verbatim. Also documented in RPC-170's architecture notes.
//!
//! ## Rendered output (success)
//!
//! ```text
//! ✓ Attachment added successfully
//!   File: <relativePath>
//!   Description: <text>     (only when description is provided)
//! ```

use std::path::{Path, PathBuf};

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
struct AddAttachmentArgs {
    work_unit_id: String,
    file_path: String,
    description: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddAttachmentArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-attachment",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Auto-create + load work-units.json.
    let mut data = ensure_work_units_file(project_root)?;

    // Source-of-truth: work unit must exist BEFORE we touch the filesystem.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-attachment",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Resolve the source path. The Rust port omits the Unicode-whitespace
    // fuzzy resolution (BUG-130) — see module rustdoc. The caller-supplied
    // path is used verbatim.
    let source_input = PathBuf::from(&args.file_path);
    let source_abs = if source_input.is_absolute() {
        source_input
    } else {
        project_root.join(&source_input)
    };
    if !source_abs.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-attachment",
            // TS uses the ORIGINAL caller-supplied path here, not the
            // resolved one.
            reason: format!("Source file '{}' does not exist", args.file_path),
        });
    }

    // basename(resolvedPath)
    let file_name = source_abs
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "add-attachment",
            reason: format!("Source file '{}' has no basename", args.file_path),
        })?
        .to_string();

    // Create per-WU attachments directory and copy.
    let attachments_dir = project_root
        .join("spec")
        .join("attachments")
        .join(&args.work_unit_id);
    std::fs::create_dir_all(&attachments_dir).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-attachment",
        reason: format!(
            "failed to create attachments dir {}: {e}",
            attachments_dir.display()
        ),
    })?;
    let dest_path = attachments_dir.join(&file_name);
    std::fs::copy(&source_abs, &dest_path).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-attachment",
        reason: format!(
            "failed to copy {} → {}: {e}",
            source_abs.display(),
            dest_path.display()
        ),
    })?;

    // BUG-055: if the source lives DIRECTLY in spec/attachments/ (i.e. its
    // parent equals project_root/spec/attachments), unlink the source so
    // it isn't duplicated. Canonicalise both sides for a robust equality
    // check; fall back to the absolute parent when canonicalisation fails
    // (e.g. tests with symlink-quirky tempdirs).
    let attachments_root = project_root.join("spec").join("attachments");
    let src_parent_canon = source_abs
        .parent()
        .and_then(|p| std::fs::canonicalize(p).ok());
    let att_root_canon = std::fs::canonicalize(&attachments_root).ok();
    if src_parent_canon.is_some() && src_parent_canon == att_root_canon {
        let _ = std::fs::remove_file(&source_abs);
    }

    // Project-relative path with forward slashes, matching the TS output.
    let relative_path = format!("spec/attachments/{}/{}", args.work_unit_id, file_name);

    // Mutate the attachments array.
    {
        let wu = match data.work_units.get_mut(&args.work_unit_id) {
            Some(wu) => wu,
            // Unreachable: presence was checked at the top of `run`. Returning
            // an error here keeps clippy::expect_used clean and preserves the
            // contract that a missing work unit produces InvalidArgs.
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "add-attachment",
                    reason: format!("Work unit '{}' does not exist", args.work_unit_id),
                });
            }
        };
        let entry = wu
            .extra
            .entry("attachments".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        // Coerce a non-array shape to an empty array so we never panic on
        // garbage data; mirrors the TS `if (!workUnit.attachments)` guard.
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        let arr = match entry.as_array_mut() {
            Some(a) => a,
            None => unreachable!("just coerced to array"),
        };

        // Duplicate guard (after copy, per TS). On error we do NOT write
        // the JSON back — work-units.json is byte-equal to its pre-call
        // state. The destination file on disk MAY have been over-written;
        // that mirrors TS behaviour.
        if arr.iter().any(|v| v.as_str() == Some(relative_path.as_str())) {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-attachment",
                reason: format!(
                    "Attachment '{}' already exists for work unit '{}'",
                    file_name, args.work_unit_id
                ),
            });
        }

        arr.push(Value::String(relative_path.clone()));
        wu.updated_at = iso8601_now();
    }

    // meta.lastUpdated bump (TS lines 104-107).
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = iso8601_now();
    }

    // Single atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    // Render the canonical multi-line output.
    let mut out = String::new();
    out.push_str("✓ Attachment added successfully\n");
    out.push_str("  File: ");
    out.push_str(&relative_path);
    out.push('\n');
    if let Some(desc) = args.description.as_deref().filter(|s| !s.is_empty()) {
        out.push_str("  Description: ");
        out.push_str(desc);
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_camel_case() {
        let a: AddAttachmentArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","filePath":"./diagram.png"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.file_path, "./diagram.png");
        assert!(a.description.is_none());
    }

    #[test]
    fn args_parse_with_description() {
        let a: AddAttachmentArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","filePath":"a.png","description":"x"}"#,
        )
        .unwrap();
        assert_eq!(a.description.as_deref(), Some("x"));
    }
}
