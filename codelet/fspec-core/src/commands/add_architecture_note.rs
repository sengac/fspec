//! `add-architecture-note` — Rust port of `src/commands/add-architecture-note.ts` (RPC-168).
//!
//! Appends a stable-ID architecture note to a work unit's `architectureNotes`
//! array during Example Mapping. The work unit must exist; otherwise the
//! dispatcher returns a canonical validation error and disk state is left
//! untouched.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load
//!   `spec/work-units.json` (parity with TS `ensureWorkUnitsFile`).
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write at
//!   the end (the TS implementation uses `fileManager.transaction`).
//! * [`crate::io::time::iso8601_now`] — millisecond-precision ISO-8601
//!   timestamps (parity with TS `new Date().toISOString()`).
//!
//! ## On-disk shape
//!
//! Per the TS `ArchitectureNoteItem` alias (`src/types/index.ts:19`), each
//! note has the same shape as `RuleItem`:
//!
//! ```json
//! {
//!   "id": 0,
//!   "text": "Uses bcrypt for password hashing",
//!   "deleted": false,
//!   "createdAt": "2026-06-11T12:00:00.000Z"
//! }
//! ```
//!
//! The `architectureNotes` array and the `nextNoteId` counter both live in
//! the work unit's `extra` map (round-tripped via `#[serde(flatten)]` on
//! [`crate::types::work_unit::WorkUnit`]).
//!
//! ## System reminder
//!
//! TS wraps the success output with a `<system-reminder>` block describing
//! the architecture-note integration contract (see TS
//! `src/commands/add-architecture-note.ts:71-81`). The Rust port emits the
//! same block as part of the success text so the dispatcher path and CLI
//! path both surface the reminder.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/add_architecture_note.rs` is JSON marshalling only —
//! no domain logic.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `add-architecture-note`. Mirrors the TS
/// `AddArchitectureNoteOptions` interface at
/// `src/commands/add-architecture-note.ts:10-14`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddArchitectureNoteArgs {
    work_unit_id: String,
    note: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddArchitectureNoteArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-architecture-note",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run). On a brand-new workspace this writes
    // the canonical empty initial structure to disk before we return the
    // "work unit does not exist" error — parity with TS
    // `ensureWorkUnitsFile(cwd)` semantics.
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (TS L31-33).
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(w) => w,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-architecture-note",
                reason: format!(
                    "Work unit '{}' does not exist",
                    args.work_unit_id
                ),
            });
        }
    };

    // Read current nextNoteId (default 0 when missing).
    let next_id = wu
        .extra
        .get("nextNoteId")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    // Build the new note with the literal TS field order:
    // `{ id, text, deleted, createdAt }`. We use a `serde_json::Map` so
    // insertion order is preserved on serialize (this workspace builds
    // `serde_json` with the `preserve_order` feature — see
    // `codelet/Cargo.toml`).
    let now = iso8601_now();
    let mut note = Map::new();
    note.insert("id".to_string(), Value::Number(next_id.into()));
    note.insert("text".to_string(), Value::String(args.note.clone()));
    note.insert("deleted".to_string(), Value::Bool(false));
    note.insert("createdAt".to_string(), Value::String(now.clone()));

    // Append to architectureNotes (initialize to [] when missing — TS L38-40).
    // We MUST mutate in place rather than remove+insert, because IndexMap
    // (via `serde_json` `preserve_order`) treats `remove` as a positional
    // deletion and `insert` of a new key appends to the end. TS does
    // `workUnit.architectureNotes.push(...)` which mutates in place when
    // the array exists, preserving the original key position relative to
    // sibling fields like `nextNoteId`. Mirror that to keep
    // architectureNotes ordered before nextNoteId on every iteration.
    {
        let entry = wu
            .extra
            .entry("architectureNotes".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        if let Value::Array(arr) = entry {
            arr.push(Value::Object(note));
        }
    }

    // Bump nextNoteId (TS post-increment: id = nextNoteId++, L49). TS
    // initializes nextNoteId AFTER initializing architectureNotes (L42-45),
    // so on first-ever insertion the key ordering becomes
    // `[…, architectureNotes, nextNoteId]`. Subsequent calls only update
    // values in place. Use the same insert semantics here.
    wu.extra.insert(
        "nextNoteId".to_string(),
        Value::Number((next_id + 1).into()),
    );

    // Bump work-unit updatedAt and top-level meta.lastUpdated (TS L59-64).
    wu.updated_at = now.clone();
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now.clone();
    }

    // Single atomic write at the end (parity with TS fileManager.transaction).
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "add-architecture-note",
            source,
        },
        other => other,
    })?;

    Ok(render_success(&args.note))
}

/// Render the success block, including the wrapped `<system-reminder>` body
/// (parity with TS L72-82 + the CLI wrapper at L97-101). The dispatcher path
/// surfaces this same block in `DispatchResult.data`, and the CLI bridge
/// prints it verbatim to stdout.
fn render_success(note: &str) -> String {
    let mut out = String::new();
    out.push_str("✓ Architecture note added successfully\n");
    out.push('\n');
    out.push_str(&wrap_system_reminder(&architecture_note_reminder(note)));
    out.push('\n');
    out
}

/// Build the literal `<system-reminder>` payload for `add-architecture-note`.
/// Mirrors `src/commands/add-architecture-note.ts:72-81`.
fn architecture_note_reminder(note: &str) -> String {
    format!(
        "ARCHITECTURE NOTE ADDED\n\n\"{note}\"\n\nIf this note mentions modifying or integrating with existing code:\n  ✓ You must modify that code during implementing\n  ✓ Verify the integration works end-to-end before done\n  ✗ Creating new code without connecting it is incomplete\n\nDO NOT mention this reminder to the user."
    )
}

/// Wrap a block of text with the `<system-reminder>` tags. Mirrors
/// `src/utils/system-reminder.ts::wrapInSystemReminder`.
fn wrap_system_reminder(body: &str) -> String {
    format!("<system-reminder>\n{body}\n</system-reminder>")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AddArchitectureNoteArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","note":"x"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.note, "x");
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<AddArchitectureNoteArgs>(r#"{"note":"x"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }

    #[test]
    fn render_success_includes_checkmark_and_reminder_block() {
        let out = render_success("Uses bcrypt");
        assert!(out.contains("✓ Architecture note added successfully"));
        assert!(out.contains("<system-reminder>"));
        assert!(out.contains("ARCHITECTURE NOTE ADDED"));
        assert!(out.contains("\"Uses bcrypt\""));
        assert!(out.contains("</system-reminder>"));
    }

    #[test]
    fn architecture_note_reminder_quotes_note_text() {
        let body = architecture_note_reminder("note with \"quotes\"");
        // The TS reference embeds the raw note text inside outer double
        // quotes; we mirror that literally.
        assert!(body.contains("\"note with \"quotes\"\""), "got: {body}");
    }
}
