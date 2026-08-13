//! `update-prefix` — Rust port of `src/commands/update-prefix.ts` (RPC-313).
//!
//! Updates an existing work-unit prefix entry in `spec/prefixes.json`.
//! The dispatcher path accepts `prefix` (required), and optional
//! `description` and `epicId`. The CLI surface omits `epicId` for parity
//! with the TS Commander.js surface, which exposes only `-d/--description`.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## Parity edge cases (vs TS)
//!
//! - Reads `spec/prefixes.json` via [`ensure_prefixes_file`] which
//!   auto-creates the file when missing (TS `ensurePrefixesFile`). This is
//!   DIFFERENT from `list-prefixes` which uses the read-only twin
//!   `read_prefixes_or_empty`.
//! - When `epicId` is provided, verifies the epic exists by reading
//!   `spec/epics.json` via [`read_epics_or_empty`]. **Parity gap**: TS
//!   `ensureEpicsFile` would auto-create an empty `spec/epics.json` before
//!   reporting "Epic <Y> not found"; the Rust implementation does not
//!   touch disk in this branch. User-visible error message is identical.
//! - `updatedAt` is ALWAYS bumped on every successful run, even when no
//!   other fields change. This matches TS `data.prefixes[X].updatedAt =
//!   new Date().toISOString()` at `src/commands/update-prefix.ts:59`.
//! - Insertion order is preserved (`IndexMap::get_mut` mutates the
//!   existing slot without moving it).
//! - On success we atomically write with [`write_json_atomic`] which uses
//!   write-temp + rename semantics (TS uses `fileManager.transaction`).
//! - All errors are wrapped by the same `Failed to update prefix:` prefix
//!   the TS catch arm at `src/commands/update-prefix.ts:67-72` applies.
//!
//! ## Why `epicId` / `updatedAt` are mutated via `extra`
//!
//! The shared `Prefix` struct (`rust/fspec-core/src/types/prefix.rs`) only
//! exposes `prefix`, `description`, `created_at`, plus a
//! `#[serde(flatten)] extra: serde_json::Map<String, Value>` catch-all for
//! forward-compatible fields. Existing on-disk `epicId` and `updatedAt`
//! values therefore land in `extra` after deserialization, and re-serialize
//! correctly via the flatten attribute. To avoid touching the shared
//! type (worker file-ownership constraint), this command mutates both
//! fields through the `extra` map by string key. If/when a follow-up RPC
//! promotes these to native fields, the call sites here can be updated
//! without a behaviour change.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::ensure::{ensure_prefixes_file, read_epics_or_empty};
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `update-prefix`. Matches the TS
/// `updatePrefix({prefix, description?, epicId?, cwd?})` shape at
/// `src/commands/update-prefix.ts:24-29`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct UpdatePrefixArgs {
    /// Prefix identifier to update. MUST already exist in
    /// `spec/prefixes.json`.
    #[serde(default)]
    prefix: Option<String>,
    /// New description (optional — preserved verbatim when absent).
    #[serde(default)]
    description: Option<String>,
    /// New epic id (optional — preserved verbatim when absent). When
    /// present, must reference an existing epic in `spec/epics.json`.
    #[serde(default)]
    epic_id: Option<String>,
}

/// Dispatcher result shape. Returned as pretty-printed JSON from
/// [`run`]. Matches TS `{ success: true }` at
/// `src/commands/update-prefix.ts:66`.
#[derive(Debug, Serialize)]
struct UpdatePrefixResult {
    success: bool,
}

/// Dispatcher entry point. Both the LLM-facing agent loop and the
/// shell-facing clap subcommand re-enter through this function.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdatePrefixArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-prefix",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Match TS line 30: `cwd = options.cwd || process.cwd()`. The
    // dispatcher always supplies project_root via DispatchRequest, so we
    // need no fallback here.
    let prefix = args.prefix.unwrap_or_default();

    // ── Load existing state (auto-creates spec/prefixes.json) ──────────
    // TS calls ensurePrefixesFile(cwd) which auto-creates an empty
    // PrefixesData and ensures the spec/ directory exists.
    let mut data = ensure_prefixes_file(project_root)?;

    // ── Existence check (TS: src/commands/update-prefix.ts:38-40) ──────
    // TS: `if (!data.prefixes[options.prefix]) throw ...`. The bare
    // truthiness check also rejects the empty string, so we mirror with
    // `contains_key` against the (possibly empty) `prefix` string.
    if !data.prefixes.contains_key(&prefix) {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-prefix",
            reason: format!("Failed to update prefix: Prefix {prefix} not found"),
        });
    }

    // ── Epic verification (TS: src/commands/update-prefix.ts:43-48) ────
    // Only runs when epicId is provided. Reads epics.json via the
    // read-only twin to avoid auto-creating it (the user-visible error
    // message matches TS exactly).
    if let Some(epic_id) = &args.epic_id {
        let epics_data = read_epics_or_empty(project_root)?;
        if !epics_data.epics.contains_key(epic_id) {
            return Err(FspecCoreError::InvalidArgs {
                command: "update-prefix",
                reason: format!("Failed to update prefix: Epic {epic_id} not found"),
            });
        }
    }

    // ── Mutate the in-place entry ──────────────────────────────────────
    // `IndexMap::get_mut` preserves insertion position. We mutate native
    // fields (description) directly and dispatcher-only fields (epicId)
    // via the `extra` map (see module-level docs for why).
    let entry = data
        .prefixes
        .get_mut(&prefix)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "update-prefix",
            reason: format!("Failed to update prefix: Prefix {prefix} not found"),
        })?;

    if let Some(description) = args.description {
        entry.description = description;
    }

    if let Some(epic_id) = args.epic_id {
        entry
            .extra
            .insert("epicId".to_string(), serde_json::Value::String(epic_id));
    }

    // Always bump updatedAt — TS line 59 sets this on every successful
    // run, even no-op calls. Stored via extra to round-trip alongside any
    // pre-existing top-level field.
    let updated_at = iso8601_now();
    entry.extra.insert(
        "updatedAt".to_string(),
        serde_json::Value::String(updated_at),
    );

    // ── Atomic write ───────────────────────────────────────────────────
    let spec_path = project_root.join("spec").join("prefixes.json");
    write_json_atomic(&spec_path, &data)?;

    // ── Build dispatcher JSON ──────────────────────────────────────────
    let result = UpdatePrefixResult { success: true };
    serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-prefix",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Best-effort ISO-8601 UTC timestamp `YYYY-MM-DDTHH:MM:SS.mmmZ`. Now
/// delegated to the shared [`crate::io::time::iso8601_now`] helper —
/// see that module for the millisecond-precision rationale.
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
    fn iso8601_now_matches_shape() {
        let s = iso8601_now();
        assert_eq!(s.len(), 24, "iso8601_now must be 24 bytes; got: {s}");
        assert!(s.ends_with('Z'), "iso8601_now must end with Z; got: {s}");
        assert_eq!(s.as_bytes()[4], b'-');
        assert_eq!(s.as_bytes()[7], b'-');
        assert_eq!(s.as_bytes()[10], b'T');
        assert_eq!(s.as_bytes()[13], b':');
        assert_eq!(s.as_bytes()[16], b':');
        assert_eq!(s.as_bytes()[19], b'.');
    }

    #[test]
    fn args_parse_empty_object_yields_none_fields() {
        let a: UpdatePrefixArgs = serde_json::from_str("{}").unwrap();
        assert!(a.prefix.is_none());
        assert!(a.description.is_none());
        assert!(a.epic_id.is_none());
    }

    #[test]
    fn args_parse_full_object_with_camel_case_epic_id() {
        let a: UpdatePrefixArgs =
            serde_json::from_str(r#"{"prefix":"AUTH","description":"new","epicId":"auth-epic"}"#)
                .unwrap();
        assert_eq!(a.prefix.as_deref(), Some("AUTH"));
        assert_eq!(a.description.as_deref(), Some("new"));
        assert_eq!(a.epic_id.as_deref(), Some("auth-epic"));
    }

    #[test]
    fn args_parse_prefix_only() {
        let a: UpdatePrefixArgs = serde_json::from_str(r#"{"prefix":"AUTH"}"#).unwrap();
        assert_eq!(a.prefix.as_deref(), Some("AUTH"));
        assert!(a.description.is_none());
        assert!(a.epic_id.is_none());
    }
}
