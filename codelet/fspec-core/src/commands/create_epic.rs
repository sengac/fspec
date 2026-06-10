//! `create-epic` — Rust port of `src/commands/create-epic.ts` (RPC-211).
//!
//! Reads `spec/epics.json` (tolerating both ENOENT and parse errors as
//! "empty store", matching the TS bare `try/catch` at
//! `src/commands/create-epic.ts:43-48`), validates the epic id against the
//! canonical lowercase-with-hyphens regex, rejects duplicates, then writes
//! the merged store atomically. Errors are wrapped with the literal
//! prefix `"Failed to create epic: "` so the dispatcher path exposes the
//! same observable error text the TypeScript outer-catch produces at
//! `src/commands/create-epic.ts:74-78`.
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function
//! (RPC-003 §7/§11 two-front-doors invariant).
//!
//! Side effects:
//! * Auto-creates `spec/` if missing (parity with `mkdir(..., {recursive: true})`
//!   at `src/commands/create-epic.ts:39`).
//! * Writes `spec/epics.json` via [`crate::io::locked_file::write_json_atomic`]
//!   (write-then-rename with `fs2` exclusive lock; mirrors `LockedFileManager.transaction`).
//!
//! On-disk field order for each epic record is `id, title, createdAt, [description]`
//! — matching TS `JSON.stringify` insertion order which builds the
//! object literal as `{ id, title, createdAt }` and THEN appends
//! `description` via `newEpic.description = options.description` at
//! `src/commands/create-epic.ts:56-64`. Rust achieves this by using
//! `serde_json::Map` (which preserves insertion order in this
//! workspace — `serde_json` is built with the `preserve_order` feature,
//! see `codelet/Cargo.toml`).

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::read_epics_or_empty;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::EpicsData;

/// CLI arguments accepted by `create-epic`. Mirrors the TS Commander.js
/// registration at `src/commands/create-epic.ts:115-127`:
/// - positional `<epicId>` (required, kebab-case marshalled to camelCase)
/// - positional `<title>` (required)
/// - optional `-d, --description <description>`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateEpicArgs {
    /// Epic ID — must match the canonical lowercase-with-hyphens regex.
    epic_id: String,
    /// Epic title.
    title: String,
    /// Optional description.
    #[serde(default)]
    description: Option<String>,
}

/// Regex source for the epic-id validation. Hand-rolled validator (we
/// don't want a `regex` dep in the runtime path) — anchored, the rule is:
///
/// * First char: ASCII lowercase letter.
/// * Remaining: ASCII lowercase letters, digits, OR hyphens — with the
///   constraint that every hyphen is internal and followed by at least
///   one alphanumeric (no trailing hyphen, no double hyphens, no
///   leading hyphen).
///
/// Equivalent to the TS regex `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`.
fn is_valid_epic_id(s: &str) -> bool {
    let mut iter = s.chars();
    // First char must be a lowercase letter.
    match iter.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    // After a hyphen we require at least one alphanumeric before the
    // string can end or another hyphen can appear.
    let mut just_saw_hyphen = false;
    let mut empty_after_hyphen = false;
    for c in iter {
        if c == '-' {
            if just_saw_hyphen {
                // Double hyphen — invalid.
                return false;
            }
            just_saw_hyphen = true;
            empty_after_hyphen = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            just_saw_hyphen = false;
            empty_after_hyphen = false;
        } else {
            return false;
        }
    }
    // Cannot end on a hyphen.
    !empty_after_hyphen
}

/// Dispatcher entry point. The dispatcher passes the canonical project
/// root alongside the raw JSON args; we never call `std::env::current_dir()`
/// so the same binary can serve multiple sessions safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CreateEpicArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "create-epic",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Validate epic id format. The TS source throws this error from
    // OUTSIDE the outer try/catch at `src/commands/create-epic.ts:31-35`,
    // so the observable message is the raw validator string — NOT
    // wrapped in `"Failed to create epic: ..."` (only inner-try errors
    // get that wrap). Mirror that asymmetry here.
    if !is_valid_epic_id(&args.epic_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-epic",
            reason: "Epic ID must be lowercase-with-hyphens format \
                     (e.g., epic-user-management)"
                .to_string(),
        });
    }

    // Read existing epics. TS uses a bare try/catch at lines 42-48 that
    // swallows ANY read error (ENOENT or JSON parse failure) and
    // initializes to `{ epics: {} }`. `read_epics_or_empty` returns
    // ENOENT-as-empty for us; we wrap any parse error in the same way.
    let epics_data = match read_epics_or_empty(project_root) {
        Ok(d) => d,
        Err(_) => EpicsData::initial(),
    };

    // Duplicate check.
    if epics_data.epics.contains_key(&args.epic_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-epic",
            reason: format!("Failed to create epic: Epic {} already exists", args.epic_id),
        });
    }

    // Assemble the new epic record with explicit field order
    // (id, title, createdAt, [description]) so on-disk JSON matches
    // TS `JSON.stringify` insertion order. TS builds the object
    // literal `{ id, title, createdAt }` first at
    // `src/commands/create-epic.ts:56-60` and THEN appends
    // `description` only if it's provided (lines 62-64) — so
    // `description` lands AFTER `createdAt`. We use
    // `serde_json::Map` (insertion-order-preserving in this
    // workspace) and insert in the same order.
    let mut new_epic = Map::new();
    new_epic.insert("id".to_string(), Value::String(args.epic_id.clone()));
    new_epic.insert("title".to_string(), Value::String(args.title.clone()));
    new_epic.insert("createdAt".to_string(), Value::String(iso8601_now()));
    if let Some(desc) = args.description.as_deref() {
        new_epic.insert("description".to_string(), Value::String(desc.to_string()));
    }

    // Merge with existing epics, preserving insertion order. Re-read the
    // raw file (best-effort) so we round-trip any unknown top-level keys
    // and preserve every existing epic's exact field order. On any
    // failure we fall back to the typed `epics_data`.
    let mut top_level: Map<String, Value> = read_raw_epics_object(project_root).unwrap_or_else(
        || {
            // Build from typed EpicsData.
            let mut m = Map::new();
            let mut epics_obj = Map::new();
            for (k, v) in &epics_data.epics {
                if let Ok(val) = serde_json::to_value(v) {
                    epics_obj.insert(k.clone(), val);
                }
            }
            m.insert("epics".to_string(), Value::Object(epics_obj));
            m
        },
    );

    // Ensure "epics" key exists and is an object.
    let epics_value = top_level
        .entry("epics".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !epics_value.is_object() {
        *epics_value = Value::Object(Map::new());
    }
    if let Some(obj) = epics_value.as_object_mut() {
        obj.insert(args.epic_id.clone(), Value::Object(new_epic));
    }

    // Atomic write. Wrap any I/O error in the canonical "Failed to
    // create epic: ..." prefix so the dispatcher exposes the same text
    // the TS outer-catch produces.
    let path = project_root.join("spec").join("epics.json");
    write_json_atomic(&path, &Value::Object(top_level)).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "create-epic",
            reason: format!("Failed to create epic: {e}"),
        }
    })?;

    Ok(render_success(&args))
}

/// Render the success block written to `DispatchResult.data` — mirrors
/// the `output.log` lines at `src/commands/create-epic.ts:95-99`.
fn render_success(args: &CreateEpicArgs) -> String {
    let mut s = String::new();
    s.push_str(&format!("✓ Created epic {}\n", args.epic_id));
    s.push_str(&format!("  Title: {}\n", args.title));
    if let Some(desc) = args.description.as_deref() {
        s.push_str(&format!("  Description: {desc}\n"));
    }
    s
}

/// Re-read `spec/epics.json` as a raw JSON object so we can preserve the
/// insertion order of every existing epic record (including unknown
/// fields). Returns `None` on any I/O or parse failure so the caller can
/// fall back to the typed `EpicsData`.
fn read_raw_epics_object(project_root: &Path) -> Option<Map<String, Value>> {
    let path = project_root.join("spec").join("epics.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    match v {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn epic_id_regex_accepts_canonical_values() {
        assert!(is_valid_epic_id("auth"));
        assert!(is_valid_epic_id("user-management"));
        assert!(is_valid_epic_id("a"));
        assert!(is_valid_epic_id("a1"));
        assert!(is_valid_epic_id("a-1"));
        assert!(is_valid_epic_id("epic-user-management-v2"));
    }

    #[test]
    fn epic_id_regex_rejects_invalid_values() {
        assert!(!is_valid_epic_id(""));
        assert!(!is_valid_epic_id("AUTH"));
        assert!(!is_valid_epic_id("Auth"));
        assert!(!is_valid_epic_id("1auth"));
        assert!(!is_valid_epic_id("-auth"));
        assert!(!is_valid_epic_id("auth-"));
        assert!(!is_valid_epic_id("auth--user"));
        assert!(!is_valid_epic_id("auth_user"));
        assert!(!is_valid_epic_id("auth user"));
    }

    #[test]
    fn args_parse_camel_case() {
        let a: CreateEpicArgs =
            serde_json::from_str(r#"{"epicId":"auth","title":"Authentication"}"#).unwrap();
        assert_eq!(a.epic_id, "auth");
        assert_eq!(a.title, "Authentication");
        assert!(a.description.is_none());
    }

    #[test]
    fn args_parse_with_description() {
        let a: CreateEpicArgs = serde_json::from_str(
            r#"{"epicId":"auth","title":"Authentication","description":"Login"}"#,
        )
        .unwrap();
        assert_eq!(a.description.as_deref(), Some("Login"));
    }

    #[test]
    fn args_parse_fails_without_epic_id() {
        let err = serde_json::from_str::<CreateEpicArgs>(r#"{"title":"x"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("epicid"),
            "missing-field error must mention epicId; got: {msg}"
        );
    }

    #[test]
    fn render_success_includes_title_line() {
        let args = CreateEpicArgs {
            epic_id: "auth".into(),
            title: "Authentication".into(),
            description: None,
        };
        let out = render_success(&args);
        assert!(out.contains("✓ Created epic auth"));
        assert!(out.contains("  Title: Authentication"));
        assert!(!out.contains("Description:"));
    }

    #[test]
    fn render_success_includes_description_when_present() {
        let args = CreateEpicArgs {
            epic_id: "auth".into(),
            title: "Authentication".into(),
            description: Some("Login features".into()),
        };
        let out = render_success(&args);
        assert!(out.contains("  Description: Login features"));
    }
}
