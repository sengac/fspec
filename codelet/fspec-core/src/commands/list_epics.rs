//! `list-epics` — Rust port of `src/commands/list-epics.ts` (RPC-243).
//!
//! Reads `spec/epics.json` (returning `Ok(empty)` on ENOENT — list-epics
//! does NOT auto-create the file) and aggregates per-epic work-unit
//! completion progress from `spec/work-units.json` (silently swallowing
//! malformed work-units, matching TS's bare `catch {}`). Emits either
//! pretty-printed JSON or a plain-text summary.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::ensure::{read_epics_or_empty, read_work_units_or_empty};
use crate::types::epic::Epic;
use crate::types::work_unit::WorkUnit;

/// CLI arguments accepted by `list-epics`. Today only `format` is
/// recognised at the dispatcher surface — the TS Commander.js registration
/// at `src/commands/list-epics.ts:141-146` declares NO `.option(...)`
/// calls, so the shell-facing CLI bridge does not pass anything beyond an
/// empty object. `format` is exposed for the structured dispatcher path so
/// `{"format":"json"}` produces the same 2-space-indented JSON shape used
/// by the agent loop's tool-call protocol.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListEpicsArgs {
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListEpicsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-epics",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Parity with TS: read epics.json directly. ENOENT → empty list,
    // parse error → escalated. Work-units read-failures are silently
    // swallowed (TS bare `catch {}`).
    let epics_data = read_epics_or_empty(project_root)?;
    let work_units_data = read_work_units_or_empty(project_root)?;

    let summaries: Vec<EpicWithProgress> = epics_data
        .epics
        .values()
        .map(|epic| aggregate_progress(epic, work_units_data.work_units.values()))
        .collect();

    let result = json!({
        "epics": summaries,
    });

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "list-epics",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text.
        _ => Ok(render_text(&summaries)),
    }
}

/// In-memory shape returned by [`aggregate_progress`]. Mirrors the TS
/// `EpicWithProgress` interface at `src/commands/list-epics.ts:30-37`.
///
/// `#[derive(Serialize)]` with explicit field order preserves the
/// **declaration order** of the JSON fields, matching the TS
/// `JSON.stringify` output where object-literal insertion order is honoured.
/// (Routing through `json!{}` instead would alphabetize fields because
/// `serde_json::Map` is a `BTreeMap`.)
///
/// `title` and `description` are `Option<String>` with
/// `skip_serializing_if = "Option::is_none"` so the JSON payload omits
/// them entirely when the source Epic does not declare them — matching
/// TS `JSON.stringify` which drops `undefined` properties.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EpicWithProgress {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    total_work_units: usize,
    completed_work_units: usize,
    completion_percentage: u32,
}

/// Aggregate completion progress for a single epic.
///
/// Mirrors the body of the for-loop at `src/commands/list-epics.ts:71-99`:
/// counts work-units whose `epic === epic.id` (exact equality, NOT a
/// prefix-startsWith match — distinct from list-prefixes), of which
/// completed = `status === 'done'`, and rounds the percentage to the
/// nearest integer (Math.round semantics — 33.33 → 33, 66.67 → 67).
fn aggregate_progress<'a, I>(epic: &Epic, work_units: I) -> EpicWithProgress
where
    I: IntoIterator<Item = &'a WorkUnit>,
{
    let mut total = 0_usize;
    let mut completed = 0_usize;
    for wu in work_units {
        if wu.epic.as_deref() == Some(epic.id.as_str()) {
            total += 1;
            if wu.status.as_str() == "done" {
                completed += 1;
            }
        }
    }
    let completion_percentage = if total > 0 {
        // Math.round-equivalent: round half-away-from-zero. (TS's Math.round
        // rounds half UP for positive values; (x * 100 / total) is always
        // non-negative here, so add 0.5 and truncate.)
        let pct = (completed as f64 / total as f64) * 100.0;
        (pct + 0.5).floor() as u32
    } else {
        0
    };
    EpicWithProgress {
        id: epic.id.clone(),
        title: epic.title.clone(),
        description: epic.description.clone(),
        total_work_units: total,
        completed_work_units: completed,
        completion_percentage,
    }
}

/// Render the text format expected by the TS CLI wrapper
/// (`src/commands/list-epics.ts:108-128`).
///
/// Layout (one block per epic, blank-line separated):
///
///     <id>
///       <title>
///       <description>       — omitted when title or description is None
///       Work Units: c/t (p%) — omitted when total is 0
fn render_text(summaries: &[EpicWithProgress]) -> String {
    if summaries.is_empty() {
        return "No epics found".to_string();
    }

    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!("Epics ({})\n", summaries.len()));
    out.push('\n');

    for e in summaries {
        out.push_str(&e.id);
        out.push('\n');
        // TS unconditionally renders `${epic.title}` even when undefined
        // (producing the literal "  undefined"). Rust's parity choice is
        // to render the empty-title case as "  " (no trailing literal),
        // which is byte-compatible with the only realistic on-disk
        // shape (title is present per the TS interface declaration).
        // When None we still emit the indented prefix line so layout
        // doesn't collapse — matching TS's unconditional emit.
        out.push_str("  ");
        if let Some(t) = e.title.as_deref() {
            out.push_str(t);
        }
        out.push('\n');
        if let Some(d) = e.description.as_deref() {
            out.push_str("  ");
            out.push_str(d);
            out.push('\n');
        }
        if e.total_work_units > 0 {
            out.push_str(&format!(
                "  Work Units: {}/{} ({}%)\n",
                e.completed_work_units, e.total_work_units, e.completion_percentage
            ));
        }
        out.push('\n');
    }

    out
}

// ============================================================================
// Production: lines 1-191 (above this divider)
// Tests:      lines 195-322 (below this divider — gated by `#[cfg(test)]`)
//
// Inline `mod tests` is retained instead of extracted because the test cases
// exercise private items (`ListEpicsArgs`, `EpicWithProgress`,
// `aggregate_progress`, `render_text`) via `use super::*`. Promoting these to
// `pub(crate)` to support a sibling test file under `tests/` was deemed too
// invasive for the size win — the production surface above the divider stays
// under 200 lines.
// ============================================================================

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

    fn make_wu(id: &str, epic: &str, status: &str) -> WorkUnit {
        serde_json::from_value(json!({
            "id": id,
            "title": "t",
            "epic": epic,
            "status": status,
            "createdAt": "x",
            "updatedAt": "x"
        }))
        .unwrap()
    }

    #[test]
    fn args_parse_with_defaults() {
        let a: ListEpicsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListEpicsArgs = serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn aggregate_progress_filters_by_exact_epic_match() {
        let epic = make_epic("auth", Some("Authentication"), None);
        let wus = vec![
            make_wu("AUTH-001", "auth", "done"),
            make_wu("AUTH-002", "auth", "backlog"),
            // Different epic must NOT match.
            make_wu("DASH-001", "dash", "done"),
            // Unmatched epic must NOT match.
            make_wu("X-001", "nonexistent", "done"),
        ];
        let p = aggregate_progress(&epic, wus.iter());
        assert_eq!(p.total_work_units, 2);
        assert_eq!(p.completed_work_units, 1);
        assert_eq!(p.completion_percentage, 50);
    }

    #[test]
    fn aggregate_progress_one_third_rounds_to_33() {
        let epic = make_epic("auth", Some("x"), None);
        let wus = vec![
            make_wu("AUTH-001", "auth", "done"),
            make_wu("AUTH-002", "auth", "backlog"),
            make_wu("AUTH-003", "auth", "backlog"),
        ];
        let p = aggregate_progress(&epic, wus.iter());
        assert_eq!(p.completion_percentage, 33);
    }

    #[test]
    fn aggregate_progress_two_thirds_rounds_to_67() {
        let epic = make_epic("auth", Some("x"), None);
        let wus = vec![
            make_wu("AUTH-001", "auth", "done"),
            make_wu("AUTH-002", "auth", "done"),
            make_wu("AUTH-003", "auth", "backlog"),
        ];
        let p = aggregate_progress(&epic, wus.iter());
        assert_eq!(p.completion_percentage, 67);
    }

    #[test]
    fn aggregate_progress_zero_total_is_zero_percent() {
        let epic = make_epic("auth", Some("x"), None);
        let p = aggregate_progress(&epic, std::iter::empty());
        assert_eq!(p.total_work_units, 0);
        assert_eq!(p.completion_percentage, 0);
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel() {
        assert_eq!(render_text(&[]), "No epics found");
    }

    #[test]
    fn render_text_omits_description_when_missing() {
        let summaries = vec![EpicWithProgress {
            id: "auth".into(),
            title: Some("Authentication".into()),
            description: None,
            total_work_units: 0,
            completed_work_units: 0,
            completion_percentage: 0,
        }];
        let out = render_text(&summaries);
        assert!(out.contains("auth"), "missing id line: {out}");
        assert!(out.contains("  Authentication"), "missing title: {out}");
        assert!(
            !out.contains("  Login features"),
            "must not contain description when None: {out}"
        );
        assert!(!out.contains("Work Units:"), "must omit progress: {out}");
    }

    #[test]
    fn render_text_omits_work_units_line_when_total_zero() {
        let summaries = vec![EpicWithProgress {
            id: "auth".into(),
            title: Some("Authentication".into()),
            description: Some("Login features".into()),
            total_work_units: 0,
            completed_work_units: 0,
            completion_percentage: 0,
        }];
        let out = render_text(&summaries);
        assert!(out.contains("  Login features"));
        assert!(!out.contains("Work Units:"), "must omit progress: {out}");
    }
}
