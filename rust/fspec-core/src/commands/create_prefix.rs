//! `create-prefix` — Rust port of `src/commands/create-prefix.ts` (RPC-213).
//!
//! Registers a new work-unit ID prefix (e.g. `AUTH`, `DASH`) in
//! `spec/prefixes.json`. Validates the prefix shape (2-6 ASCII uppercase
//! letters), refuses duplicates, and atomically writes the updated
//! `PrefixesData` to disk.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## Parity edge cases (vs TS)
//!
//! - Validation regex `^[A-Z]{2,6}$` (TS `PREFIX_REGEX`). On failure the
//!   command throws BEFORE any file IO, and the error message is wrapped
//!   by the TS catch arm into `"Failed to create prefix: Prefix must be
//!   2-6 uppercase letters (e.g., AUTH, DASH)"`. Rust mirrors the wrapped
//!   substring set so existing CLI/agent assertions still match.
//! - Reads `spec/prefixes.json` via [`ensure_prefixes_file`] which
//!   auto-creates the file when missing (TS `ensurePrefixesFile`). This is
//!   DIFFERENT from `list-prefixes` which uses the read-only twin
//!   `read_prefixes_or_empty`.
//! - Insertion order is preserved (TS object literal insertion order, Rust
//!   `IndexMap`). Appending `AUTH` then `UI` writes `{ AUTH, UI }`.
//! - Duplicate check happens BEFORE the write, so existing files stay
//!   byte-identical on the failure path. The bare `if (data.prefixes[X])`
//!   check at TS:39 is mirrored by [`IndexMap::contains_key`].
//! - On success we atomically write with [`write_json_atomic`] which uses
//!   write-temp + rename semantics (TS uses `fileManager.transaction`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_prefixes_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::prefix::Prefix;

/// CLI arguments accepted by `create-prefix`. Matches the TS Commander.js
/// surface at `src/commands/create-prefix.ts:66-86` which declares two
/// positional arguments and NO flags.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CreatePrefixArgs {
    /// Short prefix identifier (e.g. `"AUTH"`). MUST match `^[A-Z]{2,6}$`.
    #[serde(default)]
    prefix: Option<String>,
    /// Free-text description rendered by `list-prefixes`.
    #[serde(default)]
    description: Option<String>,
}

/// Dispatcher result shape. Returned as pretty-printed JSON from
/// [`run`]. `#[derive(Serialize)]` with explicit field declaration order
/// guarantees the JSON output order `success, prefix, description,
/// createdAt` — routing through `serde_json::json!` would alphabetize
/// (`serde_json::Map` is a `BTreeMap` underneath).
#[derive(Debug, Serialize)]
struct CreatePrefixResult {
    success: bool,
    prefix: String,
    description: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

/// Dispatcher entry point. Both the LLM-facing agent loop and the
/// shell-facing clap subcommand re-enter through this function.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CreatePrefixArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "create-prefix",
            reason: format!("failed to parse args: {e}"),
        })?;

    let prefix = args.prefix.unwrap_or_default();
    let description = args.description.unwrap_or_default();

    // ── Validation ─────────────────────────────────────────────────────
    // TS: `PREFIX_REGEX = /^[A-Z]{2,6}$/`, throws before any file IO at
    // `src/commands/create-prefix.ts:28-30` — OUTSIDE the outer
    // try/catch, so the `"Failed to create prefix: "` wrap is NOT
    // applied to this error path. Mirror that asymmetry here.
    if !is_valid_prefix_shape(&prefix) {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-prefix",
            reason: "Prefix must be 2-6 uppercase letters (e.g., AUTH, DASH)".to_string(),
        });
    }

    // ── Load existing state (auto-creates spec/prefixes.json) ──────────
    // TS calls ensurePrefixesFile(cwd) which auto-creates an empty
    // PrefixesData and ensures the spec/ directory exists.
    let mut data = ensure_prefixes_file(project_root)?;

    // ── Duplicate check (TS: src/commands/create-prefix.ts:39-41) ──────
    if data.prefixes.contains_key(&prefix) {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-prefix",
            reason: format!("Failed to create prefix: Prefix {prefix} already exists"),
        });
    }

    // ── Build & insert new Prefix ──────────────────────────────────────
    let created_at = iso8601_now();
    let new_prefix = Prefix {
        prefix: prefix.clone(),
        description: description.clone(),
        created_at: Some(created_at.clone()),
        extra: serde_json::Map::new(),
    };
    data.prefixes.insert(prefix.clone(), new_prefix);

    // ── Atomic write ───────────────────────────────────────────────────
    // The path is rebuilt here (rather than threaded from
    // ensure_prefixes_file) to avoid changing the shared helper's
    // signature. The canonical project resolution already happened during
    // ensure_prefixes_file, which created spec/ for us.
    let spec_path = project_root.join("spec").join("prefixes.json");
    write_json_atomic(&spec_path, &data)?;

    // ── Build dispatcher JSON ──────────────────────────────────────────
    let result = CreatePrefixResult {
        success: true,
        prefix,
        description,
        created_at,
    };
    serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "create-prefix",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// True when `s` consists of 2..=6 ASCII uppercase letters. Mirrors the
/// TS `PREFIX_REGEX = /^[A-Z]{2,6}$/` semantic exactly: ASCII-only, no
/// digits, no punctuation, no Unicode.
fn is_valid_prefix_shape(s: &str) -> bool {
    let len = s.len();
    (2..=6).contains(&len) && s.chars().all(|c| c.is_ascii_uppercase())
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
    fn valid_prefix_shapes() {
        assert!(is_valid_prefix_shape("AU"));
        assert!(is_valid_prefix_shape("AUTH"));
        assert!(is_valid_prefix_shape("DASH"));
        assert!(is_valid_prefix_shape("ABCDEF"));
    }

    #[test]
    fn invalid_prefix_shapes_too_short() {
        assert!(!is_valid_prefix_shape(""));
        assert!(!is_valid_prefix_shape("A"));
    }

    #[test]
    fn invalid_prefix_shapes_too_long() {
        assert!(!is_valid_prefix_shape("ABCDEFG"));
    }

    #[test]
    fn invalid_prefix_shapes_lowercase() {
        assert!(!is_valid_prefix_shape("auth"));
        assert!(!is_valid_prefix_shape("Auth"));
    }

    #[test]
    fn invalid_prefix_shapes_digits() {
        assert!(!is_valid_prefix_shape("AB1"));
        assert!(!is_valid_prefix_shape("A1"));
    }

    #[test]
    fn invalid_prefix_shapes_special_chars() {
        assert!(!is_valid_prefix_shape("AB-CD"));
        assert!(!is_valid_prefix_shape("AB_CD"));
        assert!(!is_valid_prefix_shape("AB CD"));
    }

    #[test]
    fn iso8601_now_matches_shape() {
        // The shared helper now emits millisecond precision (TS parity);
        // we keep this shape test to lock the 24-byte canonical form.
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
        let a: CreatePrefixArgs = serde_json::from_str("{}").unwrap();
        assert!(a.prefix.is_none());
        assert!(a.description.is_none());
    }

    #[test]
    fn args_parse_full_object() {
        let a: CreatePrefixArgs =
            serde_json::from_str(r#"{"prefix":"AUTH","description":"x"}"#).unwrap();
        assert_eq!(a.prefix.as_deref(), Some("AUTH"));
        assert_eq!(a.description.as_deref(), Some("x"));
    }
}
