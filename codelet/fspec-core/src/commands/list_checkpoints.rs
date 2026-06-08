//! `list-checkpoints` — Rust port of `src/commands/list-checkpoints.ts` (RPC-242).
//!
//! Lists every checkpoint stored under
//! `refs/fspec-checkpoints/<work_unit_id>/<name>` for a given work unit,
//! correlating each ref with the optional metadata index at
//! `.git/fspec-checkpoints-index/<work_unit_id>.json` to recover the
//! original creation timestamp. Classifies each checkpoint as automatic
//! (name contains [`codelet_git::ghost_commit::AUTO_CHECKPOINT_PATTERN`])
//! or manual, sorts newest-first by timestamp, and emits either pretty-
//! printed JSON or the chalk-stripped TS text rendering.
//!
//! Per RPC-003 §7/§11 (two-front-doors invariant) this single function is
//! invoked by BOTH the LLM-facing dispatcher AND the standalone fspec
//! binary's `fspec list-checkpoints` clap subcommand — no filter, sort,
//! or rendering logic is duplicated in the CLI bridge.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;

// codelet-git owns the ref-iteration + auto-checkpoint classification
// constants. Pulling them in keeps a single source of truth for the
// checkpoint ref-namespace (refs/fspec-checkpoints/*) and the `-auto-`
// substring pattern.
use codelet_git::ghost_commit::{list_ghost_checkpoints, AUTO_CHECKPOINT_PATTERN};

/// CLI arguments accepted by `list-checkpoints`.
///
/// Parity with TS Commander.js registration at
/// `src/commands/list-checkpoints.ts:83-88`: the only positional argument
/// is `<workUnitId>` and there are no `.option(...)` flags. We expose
/// `format` here ("text" | "json") for the structured dispatcher path
/// so agent-loop callers can request the canonical 2-space-indented
/// JSON shape used by every other RPC-241..253 ported `list-*` command.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListCheckpointsArgs {
    /// Required: work unit identifier (e.g. `"AUTH-001"`).
    #[serde(default)]
    work_unit_id: Option<String>,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// In-memory rendering shape — mirrors the TS `CheckpointDisplay` interface
/// at `src/commands/list-checkpoints.ts:15-20`.
///
/// `#[derive(Serialize)]` preserves declaration order in the JSON output,
/// matching the TS `JSON.stringify` behaviour where object-literal insertion
/// order is honoured (routing through `serde_json::Map` directly would
/// alphabetise keys).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckpointDisplay {
    name: String,
    timestamp: String,
    display_icon: String,
    is_automatic: bool,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions / working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListCheckpointsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-checkpoints",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Validation: workUnitId must be present AND non-empty after trim. The
    // TS layer is loose here (Commander.js just declares the positional)
    // but the dispatcher contract is strict — agent-loop callers must
    // surface a clear InvalidArgs message rather than a `No checkpoints
    // found for ` line with an empty id.
    let work_unit_id = match args.work_unit_id.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "list-checkpoints",
                reason: "missing or empty `workUnitId` field".to_string(),
            });
        }
    };

    // Enumerate checkpoint refs via codelet-git. Any error (no .git,
    // corrupt refs db, etc.) is treated as "no checkpoints" — matches the
    // TS implementation's silent ENOENT tolerance via the NAPI binding's
    // wrapper at `src/utils/git-checkpoint.ts:listGhostCheckpoints`
    // (errors in that path become an empty list in TS).
    let names: Vec<String> =
        list_ghost_checkpoints(project_root, &work_unit_id).unwrap_or_default();

    // Load the optional metadata index. ENOENT and malformed JSON both
    // degrade silently to an empty index — matching the TS bare-`catch`
    // at `src/commands/list-checkpoints.ts` (via the
    // `readCheckpointIndex` helper in `src/utils/git-checkpoint.ts:114-128`).
    let index = read_index(project_root, &work_unit_id);

    let mut displays: Vec<CheckpointDisplay> = names
        .into_iter()
        .map(|name: String| build_display(&name, &index))
        .collect();

    // Sort newest first by timestamp. ISO-8601 strings are lexicographically
    // ordered by chronology, so a string compare is sufficient — matches the
    // TS `new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()`
    // sort at `src/utils/git-checkpoint.ts:372-374`.
    displays.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    match args.format.as_deref() {
        Some("json") => render_json(&work_unit_id, &displays),
        // Default to text.
        _ => Ok(render_text(&work_unit_id, &displays)),
    }
}

/// Read `.git/fspec-checkpoints-index/<work_unit_id>.json`. Returns the
/// parsed `Value` on success, or `None` for both ENOENT and malformed
/// JSON (parity with the TS bare-`catch` swallow).
fn read_index(project_root: &Path, work_unit_id: &str) -> Option<Value> {
    let path = project_root
        .join(".git")
        .join("fspec-checkpoints-index")
        .join(format!("{work_unit_id}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

/// Translate a single checkpoint name into its displayable form,
/// looking up the metadata index for the original creation timestamp.
fn build_display(name: &str, index: &Option<Value>) -> CheckpointDisplay {
    let is_automatic = name.contains(AUTO_CHECKPOINT_PATTERN);
    let display_icon = if is_automatic {
        "\u{1F916}".to_string() // 🤖
    } else {
        "\u{1F4CC}".to_string() // 📌
    };

    let timestamp = index
        .as_ref()
        .and_then(|v| v.get("checkpoints"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().find(|cp| cp.get("name").and_then(|n| n.as_str()) == Some(name)))
        .and_then(|cp| cp.get("timestamp").and_then(|t| t.as_str()))
        .map(|s| s.to_string())
        .unwrap_or_else(fallback_timestamp);

    CheckpointDisplay {
        name: name.to_string(),
        timestamp,
        display_icon,
        is_automatic,
    }
}

/// Fallback ISO-8601 timestamp used when the index file is missing or
/// malformed. Mirrors the TS `new Date().toISOString()` shape (24 chars,
/// e.g. `"2026-06-07T12:43:00.123Z"`). We deliberately avoid pulling in
/// chrono just for `now()` formatting — a small civil-time calculation
/// over `SystemTime::UNIX_EPOCH` keeps fspec-core's dependency footprint
/// stable.
///
/// The test contract only asserts `len() >= 20`, but we produce a
/// well-formed ISO-8601 string regardless so the JSON payload stays
/// useful to downstream consumers.
fn fallback_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs_total = dur.as_secs() as i64;
    let millis = dur.subsec_millis();

    // Civil-time decomposition (proleptic Gregorian, UTC). Algorithm
    // mirrors Howard Hinnant's `days_from_civil` inverse — sufficient
    // for years 1970..9999. Avoids the chrono dep.
    let days = secs_total.div_euclid(86_400);
    let secs_of_day = secs_total.rem_euclid(86_400);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    // Convert days-since-1970-01-01 to (year, month, day).
    let z = days + 719_468; // shift to 0000-03-01 epoch
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hour, minute, second, millis
    )
}

/// Render the empty-result text sentinel or the populated checkpoint
/// listing. Mirrors `src/commands/list-checkpoints.ts:32-55`.
fn render_text(work_unit_id: &str, displays: &[CheckpointDisplay]) -> String {
    if displays.is_empty() {
        // No trailing newline — TS `output.log` adds one at print time;
        // CLI bridge appends a single `\n` for shell-pipeline friendliness.
        return format!("No checkpoints found for {work_unit_id}");
    }

    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!("Checkpoints for {work_unit_id}:\n"));
    out.push('\n');

    for cp in displays {
        let label = if cp.is_automatic {
            "(automatic)"
        } else {
            "(manual)"
        };
        // TS uses `${icon}  ${name} ${typeLabel}` — TWO spaces between
        // icon and name (the icon is wide but TS uses two ASCII spaces
        // regardless), ONE space before the label.
        out.push_str(&format!("{}  {} {}\n", cp.display_icon, cp.name, label));
        out.push_str(&format!("   Created: {}\n", cp.timestamp));
        out.push('\n');
    }

    out
}

/// Render the 2-space-indented JSON payload. Matches the dispatcher
/// JSON contract shared by every RPC-241..253 `list-*` port.
fn render_json(
    work_unit_id: &str,
    displays: &[CheckpointDisplay],
) -> Result<String, FspecCoreError> {
    let result = json!({
        "workUnitId": work_unit_id,
        "checkpoints": displays,
    });
    serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "list-checkpoints",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: ListCheckpointsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_camel_case() {
        let a: ListCheckpointsArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","format":"json"}"#).unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn fallback_timestamp_is_iso8601_24_chars() {
        let ts = fallback_timestamp();
        assert_eq!(ts.len(), 24, "want 24 chars for ISO-8601 millis-Z; got {ts:?}");
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[19..20], ".");
    }

    #[test]
    fn render_text_empty_returns_sentinel_with_no_newline() {
        let out = render_text("AUTH-001", &[]);
        assert_eq!(out, "No checkpoints found for AUTH-001");
    }

    #[test]
    fn render_text_manual_uses_pin_icon() {
        let cps = vec![CheckpointDisplay {
            name: "baseline".into(),
            timestamp: "2026-06-01T10:00:00.000Z".into(),
            display_icon: "\u{1F4CC}".into(),
            is_automatic: false,
        }];
        let out = render_text("AUTH-001", &cps);
        assert!(out.contains("Checkpoints for AUTH-001:"));
        assert!(out.contains("\u{1F4CC}  baseline (manual)"));
        assert!(out.contains("Created: 2026-06-01T10:00:00.000Z"));
    }

    #[test]
    fn render_text_automatic_uses_robot_icon() {
        let cps = vec![CheckpointDisplay {
            name: "AUTH-001-auto-testing".into(),
            timestamp: "2026-06-02T12:00:00.000Z".into(),
            display_icon: "\u{1F916}".into(),
            is_automatic: true,
        }];
        let out = render_text("AUTH-001", &cps);
        assert!(out.contains("\u{1F916}  AUTH-001-auto-testing (automatic)"));
    }

    #[test]
    fn build_display_classifies_auto_by_pattern() {
        let d = build_display("AUTH-001-auto-testing", &None);
        assert!(d.is_automatic);
        assert_eq!(d.display_icon, "\u{1F916}");
    }

    #[test]
    fn build_display_classifies_manual_when_no_pattern() {
        let d = build_display("baseline", &None);
        assert!(!d.is_automatic);
        assert_eq!(d.display_icon, "\u{1F4CC}");
    }

    #[test]
    fn build_display_pulls_timestamp_from_index_when_present() {
        let index = serde_json::json!({
            "checkpoints": [
                { "name": "baseline", "sha": "deadbeef", "timestamp": "2026-06-01T10:00:00.000Z" }
            ]
        });
        let d = build_display("baseline", &Some(index));
        assert_eq!(d.timestamp, "2026-06-01T10:00:00.000Z");
    }
}
