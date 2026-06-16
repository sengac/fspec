//! `query-work-units` — Rust port of `src/commands/query-work-units.ts` (RPC-263).
//!
//! Reads `spec/work-units.json` directly (does NOT auto-create it, unlike
//! `list-work-units`), applies the TS filter chain (status / prefix / epic /
//! type / tag / hasQuestions / questionsFor), supports sort/order, and emits
//! either a structured JSON envelope, a table envelope, or writes a CSV file.
//! All errors are wrapped with the TS-canonical prefix `Failed to query
//! work units:` so the dispatcher and CLI surfaces share that exact substring.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::io_error::format_io_error;
use crate::types::work_unit::{WorkUnit, WorkUnitsData};

/// CLI / dispatcher arguments accepted by `query-work-units`. Field names
/// mirror the camelCase JSON shape produced by the TS Commander wrapper
/// and the broader dispatcher payload (which carries the full superset of
/// function-level options, not just the six CLI flags).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct QueryWorkUnitsArgs {
    work_unit_id: Option<String>,
    status: Option<String>,
    epic: Option<String>,
    prefix: Option<String>,
    #[serde(rename = "type")]
    r#type: Option<String>,
    sort: Option<String>,
    order: Option<String>,
    /// `"json"`, `"csv"`, `"text"`, `"table"`, or absent. Default is
    /// `table` (mirrors TS `options.format || 'table'`). When `json` is
    /// `true` it overrides format.
    format: Option<String>,
    output: Option<String>,
    show_cycle_time: Option<bool>,
    has_questions: Option<bool>,
    questions_for: Option<String>,
    tag: Option<String>,
    json: Option<bool>,
}

/// Dispatcher entry point. Two-front-doors invariant: the CLI bridge and
/// the LLM dispatcher both call this function with a JSON-encoded args
/// payload and a project_root path.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: QueryWorkUnitsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-work-units",
            reason: wrap_failure(&format!("failed to parse args: {e}")),
        })?;

    let work_units_path = project_root.join("spec").join("work-units.json");

    // Read raw bytes — query-work-units does NOT auto-create the file (TS
    // parity: src/commands/query-work-units.ts:55-57 uses readFile directly
    // with no ensure helper, surfacing any IO error to the catch-all wrapper).
    let raw =
        std::fs::read_to_string(&work_units_path).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-work-units",
            reason: wrap_failure(&format_io_error(&e, &work_units_path.display().to_string())),
        })?;

    let data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::InvalidArgs {
            command: "query-work-units",
            reason: wrap_failure(&format!("Unexpected token in JSON: {e}")),
        })?;

    // Cycle-time mode: single work-unit, ignore filters.
    if let (Some(id), Some(true)) = (&args.work_unit_id, args.show_cycle_time) {
        return cycle_time_mode(&data, id);
    }

    // Get all work units as a Vec, preserving insertion order.
    let mut units: Vec<&WorkUnit> = data.work_units.values().collect();

    apply_filters(&mut units, &args);
    apply_sort(&mut units, &args);

    // CSV side-effect: write the file at args.output (TS only writes when
    // BOTH format=='csv' AND options.output is set).
    if args.format.as_deref() == Some("csv") {
        if let Some(out_path) = args.output.as_deref() {
            write_csv(&units, out_path)?;
        }
    }

    // Determine output envelope shape.
    let output_format = if args.json.unwrap_or(false) {
        "json"
    } else {
        args.format.as_deref().unwrap_or("table")
    };

    match output_format {
        "json" => Ok(render_json_envelope(&units)),
        // For text / csv / table the TS impl returns the "table" envelope.
        _ => Ok(render_table_envelope(&units)),
    }
}

/// Wrap any inner error message with the TS-canonical prefix used by both
/// the dispatcher error path and the CLI stderr path.
fn wrap_failure(inner: &str) -> String {
    format!("Failed to query work units: {inner}")
}

fn apply_filters(units: &mut Vec<&WorkUnit>, args: &QueryWorkUnitsArgs) {
    if let Some(status) = &args.status {
        units.retain(|wu| wu.status.as_str() == status);
    }
    if let Some(epic) = &args.epic {
        units.retain(|wu| wu.epic.as_deref() == Some(epic.as_str()));
    }
    if let Some(prefix) = &args.prefix {
        let needle = format!("{prefix}-");
        units.retain(|wu| wu.id.starts_with(&needle));
    }
    if let Some(want_type) = &args.r#type {
        units.retain(|wu| wu.type_str() == want_type.as_str());
    }
    if let Some(tag) = &args.tag {
        units.retain(|wu| {
            wu.extra
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().any(|t| t.as_str() == Some(tag.as_str())))
                .unwrap_or(false)
        });
    }
    if let Some(has) = args.has_questions {
        if has {
            units.retain(|wu| question_count(wu) > 0);
        } else {
            units.retain(|wu| question_count(wu) == 0);
        }
    }
    if let Some(qf) = &args.questions_for {
        let mention = if qf.starts_with('@') {
            qf.clone()
        } else {
            format!("@{qf}")
        };
        units.retain(|wu| {
            wu.extra
                .get("questions")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter().any(|q| {
                        // Question can be an object with `text` or a bare string.
                        if let Some(obj) = q.as_object() {
                            obj.get("text")
                                .and_then(Value::as_str)
                                .map(|t| t.contains(mention.as_str()))
                                .unwrap_or(false)
                        } else if let Some(s) = q.as_str() {
                            s.contains(mention.as_str())
                        } else {
                            false
                        }
                    })
                })
                .unwrap_or(false)
        });
    }
}

fn question_count(wu: &WorkUnit) -> usize {
    wu.extra
        .get("questions")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn apply_sort(units: &mut [&WorkUnit], args: &QueryWorkUnitsArgs) {
    let Some(sort_key) = &args.sort else {
        return;
    };
    let desc = matches!(args.order.as_deref(), Some("desc"));

    units.sort_by(|a, b| {
        let av = field_value(a, sort_key);
        let bv = field_value(b, sort_key);
        let cmp = compare_values(&av, &bv);
        if desc {
            cmp.reverse()
        } else {
            cmp
        }
    });
}

/// Look up a sortable field value on a [`WorkUnit`], pulling from the typed
/// fields first and falling back to the `extra` JSON map (which is where
/// `estimate`, `createdAt`, `updatedAt`, etc. live for round-trip safety).
fn field_value(wu: &WorkUnit, key: &str) -> Value {
    match key {
        "id" => Value::String(wu.id.clone()),
        "title" => Value::String(wu.title.clone()),
        "status" => Value::String(wu.status.as_str().to_string()),
        "epic" => match &wu.epic {
            Some(e) => Value::String(e.clone()),
            None => Value::Null,
        },
        other => wu.extra.get(other).cloned().unwrap_or(Value::Null),
    }
}

fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // TS `if (aVal === undefined || bVal === undefined) return 0;`
    if a.is_null() || b.is_null() {
        return Ordering::Equal;
    }
    if let (Some(sa), Some(sb)) = (a.as_str(), b.as_str()) {
        return sa.cmp(sb);
    }
    if let (Some(na), Some(nb)) = (a.as_f64(), b.as_f64()) {
        return na.partial_cmp(&nb).unwrap_or(Ordering::Equal);
    }
    Ordering::Equal
}

/// Mirror the TS cycle-time computation (queries-work-units.ts:60-89). For
/// every adjacent pair in `stateHistory` compute `next - current` in
/// rounded hours and store on the earlier state's key; sum into total.
fn cycle_time_mode(data: &WorkUnitsData, id: &str) -> Result<String, FspecCoreError> {
    let wu = data
        .work_units
        .get(id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "query-work-units",
            reason: wrap_failure(&format!("Work unit '{id}' does not exist")),
        })?;

    let empty: Vec<Value> = Vec::new();
    let history = wu
        .extra
        .get("stateHistory")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    let mut state_timings = serde_json::Map::new();
    let mut total_hours: i64 = 0;

    if history.len() >= 2 {
        for i in 0..history.len() - 1 {
            let current = &history[i];
            let next = &history[i + 1];
            let cur_ts = current
                .get("timestamp")
                .and_then(Value::as_str)
                .unwrap_or("");
            let next_ts = next.get("timestamp").and_then(Value::as_str).unwrap_or("");
            let cur_state = current.get("state").and_then(Value::as_str).unwrap_or("");

            let cur_ms = parse_iso_to_ms(cur_ts);
            let next_ms = parse_iso_to_ms(next_ts);
            let dur_ms = next_ms - cur_ms;
            // TS uses Math.round(durationMs / (1000 * 60 * 60)).
            let dur_hours = ((dur_ms as f64) / (1000.0 * 60.0 * 60.0)).round() as i64;
            let label = format_hours(dur_hours);
            state_timings.insert(cur_state.to_string(), Value::String(label));
            total_hours += dur_hours;
        }
    }

    let payload = json!({
        "stateTimings": Value::Object(state_timings),
        "totalCycleTime": format_hours(total_hours),
    });
    serde_json::to_string(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "query-work-units",
        reason: wrap_failure(&format!("serialize cycle-time payload: {e}")),
    })
}

fn format_hours(h: i64) -> String {
    if h == 1 || h == -1 {
        format!("{h} hour")
    } else {
        format!("{h} hours")
    }
}

/// Minimal RFC-3339 / ISO-8601 parser sufficient for our cycle-time
/// computations: accepts `YYYY-MM-DDThh:mm:ss[.fff]Z`. Falls back to 0 on
/// any unparseable input (matches TS `new Date('garbage').getTime()` → NaN
/// then coerced to 0 by the `Math.round` path in failed cases).
fn parse_iso_to_ms(s: &str) -> i64 {
    // Strict but tolerant: drop fractional seconds and the trailing Z.
    let trimmed = s.trim_end_matches('Z');
    let (date_part, time_part) = match trimmed.split_once('T') {
        Some((d, t)) => (d, t),
        None => return 0,
    };
    let date_iter: Vec<&str> = date_part.split('-').collect();
    if date_iter.len() != 3 {
        return 0;
    }
    let year: i64 = date_iter[0].parse().unwrap_or(0);
    let month: u32 = date_iter[1].parse().unwrap_or(0);
    let day: u32 = date_iter[2].parse().unwrap_or(0);
    let time_core = time_part.split('.').next().unwrap_or("");
    let t_iter: Vec<&str> = time_core.split(':').collect();
    if t_iter.len() != 3 {
        return 0;
    }
    let hh: i64 = t_iter[0].parse().unwrap_or(0);
    let mm: i64 = t_iter[1].parse().unwrap_or(0);
    let ss: i64 = t_iter[2].parse().unwrap_or(0);
    let frac_ms: i64 = time_part
        .split('.')
        .nth(1)
        .and_then(|f| f.parse::<i64>().ok())
        .unwrap_or(0);
    let days = days_from_civil(year as i32, month, day);
    let secs = days * 86_400 + hh * 3_600 + mm * 60 + ss;
    secs * 1_000 + frac_ms
}

/// Howard Hinnant's days_from_civil — civil date → days since 1970-01-01.
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y / 400 } else { (y - 399) / 400 };
    let yoe = y - era * 400;
    let m = m as i64;
    let d = d as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// CSV writer — header `id,title,status,createdAt,updatedAt`; commas in
/// title fields are stripped (TS quirk).
fn write_csv(units: &[&WorkUnit], path: &str) -> Result<(), FspecCoreError> {
    let mut lines: Vec<String> = vec!["id,title,status,createdAt,updatedAt".to_string()];
    for wu in units {
        let title = wu.title.replace(',', "");
        let status = wu.status.as_str();
        // createdAt / updatedAt are typed on WorkUnit.
        let created = wu.created_at.clone();
        let updated = wu.updated_at.clone();
        lines.push(format!(
            "{id},{title},{status},{created},{updated}",
            id = wu.id
        ));
    }
    let body = lines.join("\n");
    std::fs::write(path, body).map_err(|source| FspecCoreError::Io {
        command: "query-work-units",
        source,
    })
}

/// Build a JSON Value for a work unit using TS-canonical field order so the
/// pretty-printed JSON envelope is byte-identical to the TypeScript
/// implementation's `Object.values(...)` output for the standard fixtures.
///
/// Canonical order (matches the actual TS query-work-units output captured
/// against real fixtures): id, title, status, type, epic, createdAt,
/// updatedAt, estimate, tags, then any remaining `extra` keys in their
/// original insertion order. With workspace-wide `serde_json/preserve_order`,
/// the order we insert here is the order serde_json emits.
fn wu_to_full_value(wu: &WorkUnit) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("id".to_string(), Value::String(wu.id.clone()));
    obj.insert("title".to_string(), Value::String(wu.title.clone()));
    obj.insert(
        "status".to_string(),
        Value::String(wu.status.as_str().to_string()),
    );
    if let Some(t) = &wu.r#type {
        obj.insert("type".to_string(), Value::String(t.clone()));
    }
    if let Some(e) = &wu.epic {
        obj.insert("epic".to_string(), Value::String(e.clone()));
    }
    obj.insert(
        "createdAt".to_string(),
        Value::String(wu.created_at.clone()),
    );
    obj.insert(
        "updatedAt".to_string(),
        Value::String(wu.updated_at.clone()),
    );
    // Promote `estimate` from `extra` (if present) after timestamps so it
    // appears before `tags`, matching the canonical TS JSON field layout.
    if let Some(estimate) = wu.extra.get("estimate") {
        obj.insert("estimate".to_string(), estimate.clone());
    }
    // Promote `tags` from `extra` after estimate so the canonical order
    // `..., createdAt, updatedAt, estimate, tags, ...` is preserved.
    if let Some(tags) = wu.extra.get("tags") {
        obj.insert("tags".to_string(), tags.clone());
    }
    // Append remaining extras (excluding already-promoted `tags` / `estimate`)
    // in their original insertion order so anything outside the canonical set
    // still round-trips losslessly.
    for (k, v) in &wu.extra {
        if k == "tags" || k == "estimate" {
            continue;
        }
        obj.insert(k.clone(), v.clone());
    }
    Value::Object(obj)
}

fn data_entries(units: &[&WorkUnit]) -> Value {
    let arr: Vec<Value> = units
        .iter()
        .map(|wu| {
            let feature_file = wu
                .extra
                .get("featureFile")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            json!({
                "workUnitId": wu.id,
                "featureFilePath": feature_file,
            })
        })
        .collect();
    Value::Array(arr)
}

fn render_json_envelope(units: &[&WorkUnit]) -> String {
    let full: Vec<Value> = units.iter().map(|wu| wu_to_full_value(wu)).collect();
    let payload = json!({
        "workUnits": full,
        "format": "json",
        "data": data_entries(units),
    });
    // TS parity: `output.log(JSON.stringify(result, null, 2))` — pretty-print
    // with 2-space indent. With `serde_json/preserve_order` enabled
    // workspace-wide, `json!{}` preserves the order we inserted keys above.
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
}

fn render_table_envelope(units: &[&WorkUnit]) -> String {
    let full: Vec<Value> = units.iter().map(|wu| wu_to_full_value(wu)).collect();
    let rows: Vec<Value> = units
        .iter()
        .map(|wu| {
            let type_str = wu.type_str().to_string();
            let status = wu.status.as_str().to_string();
            let tags = wu
                .extra
                .get("tags")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));
            json!({
                "type": type_str,
                "status": status,
                "tags": tags,
            })
        })
        .collect();
    let payload = json!({
        "workUnits": full,
        "format": "table",
        "columns": ["workUnitId", "featureFilePath"],
        "rows": rows,
    });
    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}
