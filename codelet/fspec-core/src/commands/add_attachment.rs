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
//! 4. Compute the project-relative dest path. Always rendered with forward
//!    slashes as `spec/attachments/<workUnitId>/<basename>`.
//! 5. **BUG-151 duplicate guard (BEFORE any filesystem mutation)**: if the
//!    relativePath is already registered, error: `Attachment '<basename>'
//!    already exists for work unit '<workUnitId>'` — without creating the
//!    directory, copying, or unlinking anything. The pre-existing file and
//!    work-units.json are left byte-identical. (Matches the fixed TS
//!    ordering; the original TS/Rust ports checked AFTER the copy, which
//!    destroyed the destination on duplicate calls.)
//! 6. `mkdir -p spec/attachments/<workUnitId>/`.
//! 7. **BUG-151 self-copy guard**: canonicalise source and destination
//!    (`std::fs::canonicalize`, defeating symlink aliasing) and compare.
//!    The fallback is both-or-neither (TS parity): if EITHER side fails to
//!    canonicalise, the lexically-resolved forms of BOTH are compared —
//!    a canonical form is never compared to a lexical one. When the source
//!    IS the destination (it already lives in the per-work-unit dir, or is
//!    a symlink alias of it), the copy is skipped entirely — register-only,
//!    no copy, no unlink. `std::fs::copy` truncates the destination on
//!    open, so an unguarded self-copy destroys the file (0 bytes).
//! 8. Otherwise copy source → dest, then **BUG-055 dedup**: if the source
//!    file's parent directory equals `<project_root>/spec/attachments`,
//!    delete the source after copy. An unlink failure is propagated as an
//!    error (TS parity — `await unlink(...)` rejects). This unlink can
//!    never fire on the register-only path (step 7 returns before it).
//! 9. Push the relativePath onto attachments.
//! 10. Bump `updatedAt`, set `meta.lastUpdated` if `meta` exists.
//! 11. Single `write_json_atomic` write at the end.
//!
//! ## Mermaid validation (TS parity)
//!
//! When the source file extension is `.mmd` / `.mermaid` / `.md`, the file's
//! Mermaid content is validated via the shared
//! [`crate::utils::mermaid_validation`] (real `merman` parser) BEFORE any copy
//! or work-units.json mutation, mirroring the TS `validateMermaidAttachment`
//! call. A `.md` file with no ` ```mermaid ` fences is accepted as plain
//! markdown.
//!
//! ## Scope simplification (deliberate divergence from TS)
//!
//! - **No Unicode-whitespace fuzzy path resolution (BUG-130).** The TS
//!   `resolveFilePath` helper rescans the parent directory for filenames
//!   whose code points differ only by U+202F vs U+0020 (a macOS-specific
//!   pasted-screenshot quirk). The Rust port treats `options.filePath`
//!   verbatim. Documented in RPC-170's architecture notes.
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

    // Mermaid validation (TS parity): for .mmd/.mermaid/.md sources, validate
    // the diagram(s) via the real merman parser BEFORE any copy or mutation,
    // so an invalid diagram aborts with no filesystem/JSON side effects.
    if crate::utils::mermaid_validation::should_validate_mermaid(&source_abs) {
        if let Err(reason) =
            crate::utils::mermaid_validation::validate_mermaid_attachment(&source_abs)
        {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-attachment",
                reason: format!("Invalid Mermaid in '{}': {reason}", args.file_path),
            });
        }
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

    // Project-relative path with forward slashes, matching the TS output.
    let relative_path = format!("spec/attachments/{}/{}", args.work_unit_id, file_name);

    // BUG-151: duplicate-registration check BEFORE any filesystem mutation,
    // so a duplicate add-attachment errors without touching the file. The
    // pre-fix ordering (check after copy) over-wrote — and on a self-copy
    // TRUNCATED — the destination before erroring.
    let already_registered = data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("attachments"))
        .and_then(Value::as_array)
        .is_some_and(|arr| {
            arr.iter()
                .any(|v| v.as_str() == Some(relative_path.as_str()))
        });
    if already_registered {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-attachment",
            reason: format!(
                "Attachment '{}' already exists for work unit '{}'",
                file_name, args.work_unit_id
            ),
        });
    }

    // Create per-WU attachments directory.
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

    // BUG-151: guard source == destination. Canonicalise both sides
    // (std::fs::canonicalize resolves symlink aliases). The fallback is
    // both-or-neither (TS parity, add-attachment.ts): if EITHER side fails
    // to canonicalise, compare the lexically-resolved absolute forms of
    // BOTH — never a canonical form against a lexical one, which could
    // yield a false "not self-copy" and re-introduce the truncation. When
    // the source IS the destination, register-only: no copy, no unlink —
    // std::fs::copy truncates the destination on open, so an unguarded
    // self-copy destroys the file.
    let (canonical_source, canonical_dest) = match (
        std::fs::canonicalize(&source_abs),
        std::fs::canonicalize(&attachments_dir),
    ) {
        (Ok(src), Ok(dest_dir)) => (src, dest_dir.join(&file_name)),
        _ => (source_abs.clone(), dest_path.clone()),
    };
    let is_self_copy = canonical_source == canonical_dest;

    if !is_self_copy {
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
        // (e.g. tests with symlink-quirky tempdirs). Never runs on the
        // register-only (self-copy) path above.
        let attachments_root = project_root.join("spec").join("attachments");
        let src_parent_canon = source_abs
            .parent()
            .and_then(|p| std::fs::canonicalize(p).ok());
        let att_root_canon = std::fs::canonicalize(&attachments_root).ok();
        if src_parent_canon.is_some() && src_parent_canon == att_root_canon {
            // TS parity: `await unlink(resolvedPath)` rejects on failure, so
            // the Rust port propagates the error rather than swallowing it.
            std::fs::remove_file(&source_abs).map_err(|e| FspecCoreError::InvalidArgs {
                command: "add-attachment",
                reason: format!(
                    "failed to remove source {} after BUG-055 dedup move: {e}",
                    source_abs.display()
                ),
            })?;
        }
    }

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

        // BUG-151: the duplicate guard now runs BEFORE any filesystem
        // mutation (see above) — by the time we get here the relativePath
        // is guaranteed not to be registered yet.
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
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_with_camel_case() {
        let a: AddAttachmentArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","filePath":"./diagram.png"}"#)
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
