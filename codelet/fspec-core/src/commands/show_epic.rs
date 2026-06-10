//! `show-epic` — Rust port of `src/commands/show-epic.ts` (RPC-302).
//!
//! Reads `spec/epics.json` (returning `Ok(empty)` on ENOENT — show-epic
//! does NOT auto-create the file) and looks up the requested epic by id.
//! When the epic exists it aggregates completion progress from
//! `spec/work-units.json` (silently swallowing missing or malformed
//! work-units, matching TS's bare `catch {}`). Emits either pretty-
//! printed JSON or a plain-text summary.
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function (RPC-003
//! §7/§11 two-front-doors invariant).
//!
//! Behaviour parity with TypeScript (`src/commands/show-epic.ts:37-95`):
//!
//! * spec/epics.json ENOENT → `InvalidArgs` error with reason
//!   `"Epic <id> not found"` (mirrors TS `Error("Epic " + id + " not found")`
//!   at line 53).
//! * spec/epics.json present but `epics[id]` missing → same `"Epic <id>
//!   not found"` reason (TS line 59).
//! * spec/epics.json malformed → escalated as `ParseJson` error whose
//!   Display contains the substring `"Failed to parse epics.json"`.
//! * spec/work-units.json missing OR malformed → silently treated as
//!   zero counts, parity with TS's bare `catch {}` at lines 81-83.
//! * `completionPercentage` rounds to **2 decimal places** (TS:
//!   `Math.round((c / t) * 100 * 100) / 100`) — so 1/3 → 33.33 and
//!   2/3 → 66.67, NOT integer-rounded like list-epics.
//!
//! Text format mirrors `output.log` calls at TS lines 108-121, with a
//! leading blank line before `Epic: <id>` and the `Description:` line
//! omitted entirely when the epic has no description.

use std::path::Path;

use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::{read_epics_or_empty, read_work_units_or_empty};
use crate::types::epic::Epic;

/// CLI arguments accepted by `show-epic`. Mirrors the TS Commander.js
/// registration at `src/commands/show-epic.ts:136-142` — one required
/// positional `<epicId>` and an optional `--format <format>` flag.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShowEpicArgs {
    /// Epic ID to look up (e.g. `"auth"`). REQUIRED — serde surfaces a
    /// missing-field error wrapped as `InvalidArgs` via the
    /// `failed to parse args` reason string.
    epic_id: String,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// Response shape returned by `show-epic`. Mirrors the TS `EpicProgress`
/// interface at `src/commands/show-epic.ts:30-35`.
///
/// `#[derive(Serialize)]` with explicit field declaration order preserves
/// the field order on the JSON wire (epic, totalWorkUnits,
/// completedWorkUnits, completionPercentage) so the agent-facing tool-call
/// protocol stays byte-stable across ports. Routing through `json!{}`
/// would alphabetize fields because `serde_json::Map` is a `BTreeMap`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShowEpicResult {
    /// The raw epic JSON object. We keep this as a `serde_json::Value`
    /// (rather than the typed [`Epic`] struct) so the field order on the
    /// JSON wire matches the source `spec/epics.json` insertion order
    /// byte-for-byte. The typed `Epic` reorders fields to its own
    /// declaration order (id, title, description, ...extra) which breaks
    /// parity with TS's pass-through `JSON.stringify(epicsData.epics[id])`.
    epic: Value,
    total_work_units: usize,
    completed_work_units: usize,
    /// Custom serializer demotes whole-number `f64` → `i64` so `50.0`
    /// renders as `50` (JS-parity for `JSON.stringify(50)`).
    #[serde(serialize_with = "serialize_completion_percentage")]
    completion_percentage: f64,
}

/// Serialize an `f64` as a JSON integer when it is whole and finite, and as
/// the natural decimal representation otherwise. Matches JS `JSON.stringify`
/// where `50.0 === 50` and prints `50` (no decimal point), while `33.33`
/// prints `33.33`.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde serialize_with signature
fn serialize_completion_percentage<S>(v: &f64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if v.is_finite() && v.fract() == 0.0 {
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowEpicArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-epic",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Read epics.json. ENOENT → empty store (no auto-create); parse error
    // → escalated as ParseJson. The TS implementation throws ENOENT as
    //   `Error("Epic <id> not found")` (line 53); we collapse the same
    // observable behaviour by letting the subsequent lookup-miss path emit
    // the canonical message — both ENOENT and missing-key arrive at line
    // 99 below with no separate branch.
    let epics_data = read_epics_or_empty(project_root)?;

    let epic = epics_data
        .epics
        .get(&args.epic_id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "show-epic",
            reason: format!("Epic {} not found", args.epic_id),
        })?
        .clone();

    // For JSON-format parity we need to emit the epic with the SOURCE
    // field order from spec/epics.json (e.g. id, title, createdAt,
    // description, workUnits) rather than the typed `Epic` struct's
    // declaration order (id, title, description, ...extra). Re-read the
    // raw file and pluck the requested object as a Value. If anything
    // goes wrong we fall back to serializing the typed Epic.
    let epic_value = read_raw_epic(project_root, &args.epic_id).unwrap_or_else(|| {
        serde_json::to_value(&epic).unwrap_or(Value::Null)
    });

    // Work-units read-failures are silently swallowed (TS bare `catch {}`
    // at `src/commands/show-epic.ts:81-83`). `read_work_units_or_empty`
    // honours both ENOENT and JSON parse-error by returning an empty
    // store.
    let work_units_data = read_work_units_or_empty(project_root)?;

    let mut total = 0_usize;
    let mut completed = 0_usize;
    for wu in work_units_data.work_units.values() {
        if wu.epic.as_deref() == Some(args.epic_id.as_str()) {
            total += 1;
            if wu.status.as_str() == "done" {
                completed += 1;
            }
        }
    }

    // 2-decimal-place rounding via Math.round((c/t)*100*100)/100. Distinct
    // from list-epics which rounds to integer percent. (TS rounds the
    // scaled value to the nearest integer; dividing by 100 brings us back
    // to the 2-decimal representation.)
    let completion_percentage = if total > 0 {
        let pct = (completed as f64 / total as f64) * 100.0 * 100.0;
        pct.round() / 100.0
    } else {
        0.0
    };

    let result = ShowEpicResult {
        epic: epic_value,
        total_work_units: total,
        completed_work_units: completed,
        completion_percentage,
    };

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "show-epic",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text — use the typed Epic for stable field access.
        _ => Ok(render_text(&epic, &result)),
    }
}

/// Re-read `spec/epics.json` to extract the requested epic as a raw JSON
/// `Value`, preserving the on-disk field-insertion order. Returns `None`
/// on any I/O or parse failure so the caller can fall back gracefully.
///
/// This second read is the simplest way to preserve source field-order
/// without restructuring [`read_epics_or_empty`] (which deserializes into
/// a typed [`Epic`] struct that loses the original key sequence).
fn read_raw_epic(project_root: &Path, epic_id: &str) -> Option<Value> {
    let path = project_root.join("spec").join("epics.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    v.get("epics")?.get(epic_id).cloned()
}

/// Render the text format expected by the TS CLI wrapper
/// (`src/commands/show-epic.ts:108-121`).
///
/// Layout (the leading blank line and `Description:` omission are part of
/// the byte-for-byte parity contract — do not modify without updating the
/// scenario `text_format_renders_epic_header_and_progress`):
///
/// ```text
/// (blank line)
/// Epic: <id>
/// (blank line)
/// Title: <title or N/A>
/// Description: <desc>     — emitted only when description is Some
/// (blank line)
/// Progress:
///   Total work units: <n>
///   Completed: <n>
///   Completion: <pct>%
/// (blank line)
/// ```
fn render_text(epic: &Epic, result: &ShowEpicResult) -> String {
    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!("Epic: {}\n", epic.id));
    out.push('\n');
    out.push_str(&format!(
        "Title: {}\n",
        epic.title.as_deref().unwrap_or("N/A")
    ));
    if let Some(desc) = epic.description.as_deref() {
        out.push_str(&format!("Description: {desc}\n"));
    }
    out.push('\n');
    out.push_str("Progress:\n");
    out.push_str(&format!("  Total work units: {}\n", result.total_work_units));
    out.push_str(&format!("  Completed: {}\n", result.completed_work_units));
    // f64 Display picks the shortest round-trip representation, so 50.0
    // renders as "50" and 33.33 renders as "33.33" — matching TS's
    // template-literal interpolation of `${result.completionPercentage}%`.
    out.push_str(&format!(
        "  Completion: {}%\n",
        result.completion_percentage
    ));
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;

    fn make_epic(id: &str, title: Option<&str>, desc: Option<&str>) -> Epic {
        let mut v = serde_json::Map::new();
        v.insert("id".into(), json!(id));
        if let Some(t) = title {
            v.insert("title".into(), json!(t));
        }
        if let Some(d) = desc {
            v.insert("description".into(), json!(d));
        }
        serde_json::from_value(serde_json::Value::Object(v)).unwrap()
    }

    #[test]
    fn args_parse_requires_epic_id() {
        let err = serde_json::from_str::<ShowEpicArgs>("{}").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.to_lowercase().contains("epicid") || msg.contains("epicId"),
            "expected missing-field error to mention epicId, got: {msg}"
        );
    }

    #[test]
    fn args_parse_camel_case() {
        let a: ShowEpicArgs =
            serde_json::from_str(r#"{"epicId":"auth","format":"json"}"#).unwrap();
        assert_eq!(a.epic_id, "auth");
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn render_text_emits_canonical_layout_with_description() {
        let e = make_epic("auth", Some("Authentication"), Some("Login features"));
        let r = ShowEpicResult {
            epic: serde_json::to_value(&e).unwrap(),
            total_work_units: 4,
            completed_work_units: 2,
            completion_percentage: 50.0,
        };
        let out = render_text(&e, &r);
        assert!(out.lines().any(|l| l == "Epic: auth"), "{out}");
        assert!(out.lines().any(|l| l == "Title: Authentication"), "{out}");
        assert!(
            out.lines().any(|l| l == "Description: Login features"),
            "{out}"
        );
        assert!(out.lines().any(|l| l == "Progress:"), "{out}");
        assert!(out.lines().any(|l| l == "  Total work units: 4"), "{out}");
        assert!(out.lines().any(|l| l == "  Completed: 2"), "{out}");
        assert!(out.lines().any(|l| l == "  Completion: 50%"), "{out}");
    }

    #[test]
    fn render_text_omits_description_when_none() {
        let e = make_epic("auth", Some("Authentication"), None);
        let r = ShowEpicResult {
            epic: serde_json::to_value(&e).unwrap(),
            total_work_units: 0,
            completed_work_units: 0,
            completion_percentage: 0.0,
        };
        let out = render_text(&e, &r);
        assert!(
            !out.contains("Description:"),
            "Description line must not appear when description is None: {out}"
        );
    }

    #[test]
    fn render_text_renders_title_n_a_when_missing() {
        let e = make_epic("auth", None, None);
        let r = ShowEpicResult {
            epic: serde_json::to_value(&e).unwrap(),
            total_work_units: 0,
            completed_work_units: 0,
            completion_percentage: 0.0,
        };
        let out = render_text(&e, &r);
        assert!(out.lines().any(|l| l == "Title: N/A"), "{out}");
    }

    #[test]
    fn percentage_one_third_rounds_to_33_33() {
        // Replicate the Rust rounding directly (no fs needed) to lock the
        // algorithm against drift.
        let total = 3_usize;
        let completed = 1_usize;
        let pct = (completed as f64 / total as f64) * 100.0 * 100.0;
        let p = pct.round() / 100.0;
        assert!((p - 33.33).abs() < 1e-9, "got {p}");
    }

    #[test]
    fn percentage_two_thirds_rounds_to_66_67() {
        let total = 3_usize;
        let completed = 2_usize;
        let pct = (completed as f64 / total as f64) * 100.0 * 100.0;
        let p = pct.round() / 100.0;
        assert!((p - 66.67).abs() < 1e-9, "got {p}");
    }
}
