//! `list-work-units` — Rust port of `src/commands/list-work-units.ts` (RPC-253).
//!
//! Loads `spec/work-units.json` (auto-creating it on first run), applies the
//! TS filter chain (status / prefix / epic / type with the `story` default
//! for missing types), then emits either pretty-printed JSON or a plain-text
//! summary. All filesystem work is delegated to shared modules under
//! [`crate::io`] and [`crate::types`] so the same primitives are reused by
//! every subsequent ported command.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::{ensure_prefixes_file, ensure_work_units_file};
use crate::types::work_unit::{WorkUnit, WorkUnitType};

/// CLI arguments accepted by `list-work-units`. Field names mirror the
/// kebab-case flags exposed by the TS Commander registration; serde uses
/// camelCase aliases for parity with what the dispatcher receives over the
/// existing JSON tool-call protocol.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ListWorkUnitsArgs {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    epic: Option<String>,
    #[serde(default, rename = "type")]
    r#type: Option<WorkUnitType>,
    /// `"text"` (default) or `"json"`.
    #[serde(default)]
    format: Option<String>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ListWorkUnitsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "list-work-units",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Auto-create work-units.json AND prefixes.json on first run (parity with
    // the TS command which calls both ensure helpers regardless of filters).
    let data = ensure_work_units_file(project_root)?;
    let _ = ensure_prefixes_file(project_root)?;

    let summaries = filter_and_summarize(&data.work_units, &args);
    let result = json!({ "workUnits": summaries });

    match args.format.as_deref() {
        Some("json") => serde_json::to_string_pretty(&result).map_err(|e| {
            FspecCoreError::InvalidArgs {
                command: "list-work-units",
                reason: format!("failed to serialize result: {e}"),
            }
        }),
        // Default to text.
        _ => Ok(render_text(&summaries)),
    }
}

/// Apply the filter chain in the same order as the TS implementation
/// (`src/commands/list-work-units.ts:43-69`) and map to the `WorkUnitSummary`
/// shape (id, title, status, optional epic).
fn filter_and_summarize(
    work_units: &indexmap::IndexMap<String, WorkUnit>,
    args: &ListWorkUnitsArgs,
) -> Vec<Value> {
    let mut iter: Vec<&WorkUnit> = work_units.values().collect();

    if let Some(status) = &args.status {
        iter.retain(|wu| wu.status.as_str() == status);
    }

    if let Some(prefix) = &args.prefix {
        let needle = format!("{prefix}-");
        iter.retain(|wu| wu.id.starts_with(&needle));
    }

    if let Some(epic) = &args.epic {
        iter.retain(|wu| wu.epic.as_deref() == Some(epic.as_str()));
    }

    if let Some(want) = args.r#type {
        // Compare via string equality (parity with the TS expression
        // `(wu.type || 'story') === options.type` at
        // `src/commands/list-work-units.ts:56-61`). This preserves the
        // semantics where a `type="feature"` unit does NOT match
        // `--type=story` — the TS-runtime never coerces unknown variants
        // back to the default — while a missing/empty type DOES match
        // `--type=story` via `type_str()`'s short-circuit.
        let want_str = want.as_str();
        iter.retain(|wu| wu.type_str() == want_str);
    }

    iter.into_iter()
        .map(|wu| {
            let mut obj = serde_json::Map::new();
            obj.insert("id".to_string(), Value::String(wu.id.clone()));
            obj.insert("title".to_string(), Value::String(wu.title.clone()));
            obj.insert(
                "status".to_string(),
                Value::String(wu.status.as_str().to_string()),
            );
            if let Some(epic) = &wu.epic {
                if !epic.is_empty() {
                    obj.insert("epic".to_string(), Value::String(epic.clone()));
                }
            }
            Value::Object(obj)
        })
        .collect()
}

/// Render the text format expected by the TS CLI wrapper
/// (`list-work-units.ts:98-113`).
fn render_text(summaries: &[Value]) -> String {
    if summaries.is_empty() {
        return "No work units found".to_string();
    }

    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!("Work Units ({})\n", summaries.len()));
    out.push('\n');

    for entry in summaries {
        let id = entry["id"].as_str().unwrap_or("");
        let status = entry["status"].as_str().unwrap_or("");
        let title = entry["title"].as_str().unwrap_or("");
        out.push_str(&format!("{id} [{status}]\n"));
        out.push_str(&format!("  {title}\n"));
        if let Some(epic) = entry.get("epic").and_then(Value::as_str) {
            out.push_str(&format!("  Epic: {epic}\n"));
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_wu(v: Value) -> WorkUnit {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn args_parse_with_defaults() {
        let a: ListWorkUnitsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.status.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_camel_case_type() {
        let a: ListWorkUnitsArgs =
            serde_json::from_str(r#"{"status":"backlog","type":"task"}"#).unwrap();
        assert_eq!(a.status.as_deref(), Some("backlog"));
        assert_eq!(a.r#type, Some(WorkUnitType::Task));
    }

    #[test]
    fn filter_by_prefix_requires_trailing_hyphen() {
        let mut m = indexmap::IndexMap::new();
        m.insert(
            "AUTH-001".into(),
            parse_wu(json!({
                "id": "AUTH-001", "title": "x", "status": "backlog",
                "createdAt": "x", "updatedAt": "x"
            })),
        );
        m.insert(
            "AUTHX-001".into(),
            parse_wu(json!({
                "id": "AUTHX-001", "title": "y", "status": "backlog",
                "createdAt": "x", "updatedAt": "x"
            })),
        );
        let args = ListWorkUnitsArgs {
            prefix: Some("AUTH".into()),
            ..Default::default()
        };
        let out = filter_and_summarize(&m, &args);
        let ids: Vec<&str> = out.iter().map(|e| e["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["AUTH-001"]);
    }

    #[test]
    fn render_text_empty_returns_canonical_sentinel() {
        assert_eq!(render_text(&[]), "No work units found");
    }
}
