//! `list-prefixes` — Rust port of `src/commands/list-prefixes.ts` (RPC-248).
//!
//! Reads `spec/prefixes.json` (returning `Ok(empty)` on ENOENT — list-prefixes
//! does NOT auto-create the file) and aggregates per-prefix work-unit
//! completion progress from `spec/work-units.json` (silently swallowing
//! malformed work-units, matching TS's bare `catch {}`). Emits either pretty-
//! printed JSON or a plain-text summary.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::ensure::{read_prefixes_or_empty, read_work_units_or_empty};
use crate::types::prefix::Prefix;
use crate::types::work_unit::WorkUnit;

/// CLI arguments accepted by `list-prefixes`. Today only `format` is
/// recognised at the dispatcher surface — the TS Commander.js registration
/// at `src/commands/list-prefixes.ts:101-104` declares NO `.option(...)`
/// calls, so the shell-facing CLI bridge does not pass anything beyond an
/// empty object. `format` is exposed for the structured dispatcher path so
/// `{"format":"json"}` produces the same 2-space-indented JSON shape used
/// by the agent loop's tool-call protocol.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListPrefixesArgs {
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListPrefixesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-prefixes",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Parity with TS: read prefixes.json directly. ENOENT → empty list,
    // parse error → escalated. Work-units read-failures are silently
    // swallowed (TS bare `catch {}`).
    let prefixes_data = read_prefixes_or_empty(project_root)?;
    let work_units_data = read_work_units_or_empty(project_root)?;

    let summaries: Vec<PrefixWithProgress> = prefixes_data
        .prefixes
        .values()
        .map(|prefix| aggregate_progress(prefix, work_units_data.work_units.values()))
        .collect();

    let result = json!({
        "prefixes": summaries,
    });

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "list-prefixes",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text.
        _ => Ok(render_text(&summaries)),
    }
}

/// In-memory shape returned by [`aggregate_progress`]. Mirrors the TS
/// `PrefixWithProgress` interface at `src/commands/list-prefixes.ts:28-34`.
///
/// `#[derive(Serialize)]` with explicit `#[serde(rename = ...)]` annotations
/// preserves the **declaration order** of the JSON fields, matching the TS
/// `JSON.stringify` output where object-literal insertion order is honoured.
/// (Routing through `json!{}` instead would alphabetize fields because
/// `serde_json::Map` is a `BTreeMap`.)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrefixWithProgress {
    prefix: String,
    description: String,
    total_work_units: usize,
    completed_work_units: usize,
    completion_percentage: u32,
}

/// Aggregate completion progress for a single prefix.
///
/// Mirrors the body of the for-loop at `src/commands/list-prefixes.ts:68-95`:
/// counts work-units whose `id.startsWith(prefix.prefix + '-')`, of which
/// completed = `status === 'done'`, and rounds the percentage to the nearest
/// integer (Math.round semantics — 33.33 → 33, 66.67 → 67).
fn aggregate_progress<'a, I>(prefix: &Prefix, work_units: I) -> PrefixWithProgress
where
    I: IntoIterator<Item = &'a WorkUnit>,
{
    let needle = format!("{}-", prefix.prefix);
    let mut total = 0_usize;
    let mut completed = 0_usize;
    for wu in work_units {
        if wu.id.starts_with(&needle) {
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
    PrefixWithProgress {
        prefix: prefix.prefix.clone(),
        description: prefix.description.clone(),
        total_work_units: total,
        completed_work_units: completed,
        completion_percentage,
    }
}

/// Render the text format expected by the TS CLI wrapper
/// (`src/commands/list-prefixes.ts:107-123`).
fn render_text(summaries: &[PrefixWithProgress]) -> String {
    if summaries.is_empty() {
        return "No prefixes found".to_string();
    }

    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!("Prefixes ({})\n", summaries.len()));
    out.push('\n');

    for p in summaries {
        out.push_str(&p.prefix);
        out.push('\n');
        out.push_str("  ");
        out.push_str(&p.description);
        out.push('\n');
        if p.total_work_units > 0 {
            out.push_str(&format!(
                "  Work Units: {}/{} ({}%)\n",
                p.completed_work_units, p.total_work_units, p.completion_percentage
            ));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;
    use serde_json::json;

    fn make_prefix(name: &str, desc: &str) -> Prefix {
        serde_json::from_value(json!({
            "prefix": name,
            "description": desc,
            "createdAt": "x"
        }))
        .unwrap()
    }

    fn make_wu(id: &str, status: &str) -> WorkUnit {
        serde_json::from_value(json!({
            "id": id,
            "title": "t",
            "status": status,
            "createdAt": "x",
            "updatedAt": "x"
        }))
        .unwrap()
    }

    #[test]
    fn args_parse_with_defaults() {
        let a: ListPrefixesArgs = serde_json::from_str("{}").unwrap();
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_format_json() {
        let a: ListPrefixesArgs = serde_json::from_str(r#"{"format":"json"}"#).unwrap();
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn aggregate_progress_filters_by_prefix_dash() {
        let prefix = make_prefix("AUTH", "Auth features");
        let wus = vec![
            make_wu("AUTH-001", "done"),
            make_wu("AUTH-002", "backlog"),
            // AUTHX-001 must NOT match prefix=AUTH (we append '-' before startsWith).
            make_wu("AUTHX-001", "done"),
            // DASH-001 also excluded.
            make_wu("DASH-001", "done"),
        ];
        let p = aggregate_progress(&prefix, wus.iter());
        assert_eq!(p.total_work_units, 2);
        assert_eq!(p.completed_work_units, 1);
        assert_eq!(p.completion_percentage, 50);
    }

    #[test]
    fn aggregate_progress_one_third_rounds_to_33() {
        let prefix = make_prefix("AUTH", "x");
        let wus = vec![
            make_wu("AUTH-001", "done"),
            make_wu("AUTH-002", "backlog"),
            make_wu("AUTH-003", "backlog"),
        ];
        let p = aggregate_progress(&prefix, wus.iter());
        assert_eq!(p.completion_percentage, 33);
    }

    #[test]
    fn aggregate_progress_two_thirds_rounds_to_67() {
        let prefix = make_prefix("AUTH", "x");
        let wus = vec![
            make_wu("AUTH-001", "done"),
            make_wu("AUTH-002", "done"),
            make_wu("AUTH-003", "backlog"),
        ];
        let p = aggregate_progress(&prefix, wus.iter());
        assert_eq!(p.completion_percentage, 67);
    }

    #[test]
    fn aggregate_progress_zero_total_is_zero_percent() {
        let prefix = make_prefix("AUTH", "x");
        let p = aggregate_progress(&prefix, std::iter::empty());
        assert_eq!(p.total_work_units, 0);
        assert_eq!(p.completion_percentage, 0);
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel() {
        assert_eq!(render_text(&[]), "No prefixes found");
    }

    #[test]
    fn render_text_omits_work_units_line_when_total_zero() {
        let summaries = vec![PrefixWithProgress {
            prefix: "AUTH".into(),
            description: "Auth features".into(),
            total_work_units: 0,
            completed_work_units: 0,
            completion_percentage: 0,
        }];
        let out = render_text(&summaries);
        assert!(out.contains("AUTH"), "missing prefix line: {out}");
        assert!(out.contains("  Auth features"), "missing desc: {out}");
        assert!(!out.contains("Work Units:"), "must omit progress: {out}");
    }
}
