//! `create-task` — Rust port of `src/commands/create-task.ts` (RPC-215).
//!
//! Creates a new `task`-typed work unit in `spec/work-units.json` with an
//! auto-generated `<PREFIX>-NNN` id, optional description/epic/parent links,
//! and a trailing minimal-requirements `<system-reminder>` block in the
//! dispatch response text.
//!
//! ## Validation order (mirrors create-task.ts)
//!
//! 1. Foundation must exist (`check_foundation_exists`) — else the verbatim
//!    foundation-missing userMessage + `<system-reminder>` is returned and
//!    NOTHING is written.
//! 2. Title must be non-empty (after trim) — else `Title is required`.
//! 3. Prefix must be registered in `spec/prefixes.json` — else
//!    `Prefix '<p>' is not registered. Run 'fspec create-prefix <p>
//!    "Description"' first.`.
//! 4. Parent (if given) must exist — else `Parent task '<p>' does not
//!    exist`; nesting depth must be < 3 — else `Maximum nesting depth (3)
//!    exceeded`.
//! 5. Epic (if given) must exist — else `Epic '<e>' does not exist`.
//!
//! ## ID generation (high-water-mark)
//!
//! `nextNumber = max(prefixCounters[prefix] || 0, max(existing <prefix>-NNN
//! suffixes)) + 1`, zero-padded to 3 digits. `prefixCounters[prefix]` is then
//! persisted to the new high-water-mark.
//!
//! ## On-disk task field order (TS object-literal insertion order)
//!
//! ```text
//! id, title, type, status, createdAt, updatedAt, [description], [epic], [parent | children]
//! ```
//!
//! `parent` and `children` are mutually exclusive: a parent-linked task
//! OMITS `children`; a root task carries `children: []`.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/create_task.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::{
    check_foundation_exists, ensure_epics_file, ensure_prefixes_file, ensure_work_units_file,
};
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

const MAX_NESTING_DEPTH: usize = 3;

/// CLI arguments accepted by `create-task`. Mirrors the TS
/// `CreateTaskOptions` interface at `src/commands/create-task.ts:17-24`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskArgs {
    prefix: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    epic: Option<String>,
    #[serde(default)]
    parent: Option<String>,
}

/// Dispatcher entry point. Returns the success block (✓ Created task... +
/// Title line + optional Epic/Parent/Description lines) followed by the
/// minimal-requirements `<system-reminder>` — mirroring the TS CLI output
/// PLUS the systemReminder that the TS `createTask` returns to the agent.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CreateTaskArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "create-task",
            reason: format!("failed to parse args: {e}"),
        })?;

    // 1. Foundation must exist. The original command string mirrors the TS
    //    `fspec create-task <prefix> "<title>"` at create-task.ts:38.
    let original_command = format!("fspec create-task {} \"{}\"", args.prefix, args.title);
    check_foundation_exists(project_root, &original_command)?;

    // 2. Validate title (non-empty after trim).
    if args.title.trim().is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-task",
            reason: "Title is required".to_string(),
        });
    }

    // 3. Validate prefix is registered.
    let prefixes_data = ensure_prefixes_file(project_root)?;
    if !prefixes_data.prefixes.contains_key(&args.prefix) {
        return Err(FspecCoreError::InvalidArgs {
            command: "create-task",
            reason: format!(
                "Prefix '{}' is not registered. Run 'fspec create-prefix {} \"Description\"' first.",
                args.prefix, args.prefix
            ),
        });
    }

    // Load work units for validation + mutation (auto-create if missing).
    let data = ensure_work_units_file(project_root)?;

    // 4. Validate parent (existence + nesting depth).
    if let Some(parent) = args.parent.as_deref() {
        if !data.work_units.contains_key(parent) {
            return Err(FspecCoreError::InvalidArgs {
                command: "create-task",
                reason: format!("Parent task '{parent}' does not exist"),
            });
        }
        let depth = nesting_depth(&data, parent, 1);
        if depth >= MAX_NESTING_DEPTH {
            return Err(FspecCoreError::InvalidArgs {
                command: "create-task",
                reason: format!("Maximum nesting depth ({MAX_NESTING_DEPTH}) exceeded"),
            });
        }
    }

    // 5. Validate epic (existence). Auto-creates spec/epics.json on ENOENT,
    //    parity with TS `ensureEpicsFile`.
    if let Some(epic) = args.epic.as_deref() {
        let epics_data = ensure_epics_file(project_root)?;
        if !epics_data.epics.contains_key(epic) {
            return Err(FspecCoreError::InvalidArgs {
                command: "create-task",
                reason: format!("Epic '{epic}' does not exist"),
            });
        }
    }

    // Generate next id (high-water-mark). Also computes the new counter.
    let (next_id, next_number) = generate_next_id(&data, &args.prefix);

    let now = iso8601_now();

    // Build the new task object in TS object-literal insertion order:
    // id, title, type, status, createdAt, updatedAt, [description],
    // [epic], [parent | children].
    let mut task = Map::new();
    task.insert("id".to_string(), Value::String(next_id.clone()));
    task.insert("title".to_string(), Value::String(args.title.clone()));
    task.insert("type".to_string(), Value::String("task".to_string()));
    task.insert("status".to_string(), Value::String("backlog".to_string()));
    task.insert("createdAt".to_string(), Value::String(now.clone()));
    task.insert("updatedAt".to_string(), Value::String(now));
    if let Some(desc) = args.description.as_deref() {
        task.insert("description".to_string(), Value::String(desc.to_string()));
    }
    if let Some(epic) = args.epic.as_deref() {
        task.insert("epic".to_string(), Value::String(epic.to_string()));
    }
    if let Some(parent) = args.parent.as_deref() {
        task.insert("parent".to_string(), Value::String(parent.to_string()));
    } else {
        task.insert("children".to_string(), Value::Array(Vec::new()));
    }

    // Round-trip the work-units file as a raw JSON object so existing
    // entries keep their exact on-disk key order and so we can write the
    // freshly-built task object verbatim (preserving the TS field order
    // computed above).
    let mut top: Map<String, Value> =
        read_raw_work_units_object(project_root).unwrap_or_else(|| {
            serde_json::to_value(&data)
                .ok()
                .and_then(|v| match v {
                    Value::Object(m) => Some(m),
                    _ => None,
                })
                .unwrap_or_default()
        });

    // Insert the new task into workUnits.
    {
        let work_units = top
            .entry("workUnits".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !work_units.is_object() {
            *work_units = Value::Object(Map::new());
        }
        if let Some(obj) = work_units.as_object_mut() {
            obj.insert(next_id.clone(), Value::Object(task));
        }
    }

    // Add to states.backlog.
    {
        let states = top
            .entry("states".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !states.is_object() {
            *states = Value::Object(Map::new());
        }
        if let Some(states_obj) = states.as_object_mut() {
            let backlog = states_obj
                .entry("backlog".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if !backlog.is_array() {
                *backlog = Value::Array(Vec::new());
            }
            if let Value::Array(arr) = backlog {
                arr.push(Value::String(next_id.clone()));
            }
        }
    }

    // Update parent's children array if parent exists.
    if let Some(parent) = args.parent.as_deref() {
        if let Some(work_units) = top.get_mut("workUnits").and_then(Value::as_object_mut) {
            if let Some(parent_obj) = work_units.get_mut(parent).and_then(Value::as_object_mut) {
                let children = parent_obj
                    .entry("children".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if !children.is_array() {
                    *children = Value::Array(Vec::new());
                }
                if let Value::Array(arr) = children {
                    arr.push(Value::String(next_id.clone()));
                }
            }
        }
    }

    // Persist prefixCounters high-water-mark.
    {
        let counters = top
            .entry("prefixCounters".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !counters.is_object() {
            *counters = Value::Object(Map::new());
        }
        if let Some(obj) = counters.as_object_mut() {
            obj.insert(args.prefix.clone(), Value::from(next_number));
        }
    }

    // Atomic write of work-units.json.
    let wu_path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&wu_path, &Value::Object(top))?;

    // Update epic's workUnits array if epic provided (separate file).
    if let Some(epic) = args.epic.as_deref() {
        let mut epics_top = read_raw_epics_object(project_root).unwrap_or_default();
        if let Some(epics_obj) = epics_top.get_mut("epics").and_then(Value::as_object_mut) {
            if let Some(epic_obj) = epics_obj.get_mut(epic).and_then(Value::as_object_mut) {
                let work_units = epic_obj
                    .entry("workUnits".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if !work_units.is_array() {
                    *work_units = Value::Array(Vec::new());
                }
                if let Value::Array(arr) = work_units {
                    arr.push(Value::String(next_id.clone()));
                }
            }
        }
        let epics_path = project_root.join("spec").join("epics.json");
        write_json_atomic(&epics_path, &Value::Object(epics_top))?;
    }

    Ok(render_success(&args, &next_id))
}

/// Render the dispatch response text: the ✓ success block followed by the
/// minimal-requirements `<system-reminder>`. Mirrors the TS CLI `output.log`
/// lines (create-task.ts:234-244) PLUS the `systemReminder` returned by
/// `createTask` (create-task.ts:143-163).
fn render_success(args: &CreateTaskArgs, next_id: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("✓ Created task {next_id}\n"));
    s.push_str(&format!("  Title: {}\n", args.title));
    if let Some(desc) = args.description.as_deref() {
        s.push_str(&format!("  Description: {desc}\n"));
    }
    if let Some(epic) = args.epic.as_deref() {
        s.push_str(&format!("  Epic: {epic}\n"));
    }
    if let Some(parent) = args.parent.as_deref() {
        s.push_str(&format!("  Parent: {parent}\n"));
    }
    s.push('\n');
    s.push_str(&system_reminder(next_id));
    s
}

/// Build the minimal-requirements `<system-reminder>` block — verbatim port
/// of create-task.ts:143-163.
fn system_reminder(next_id: &str) -> String {
    format!(
        "<system-reminder>\n\
Task {next_id} created successfully.\n\
\n\
Tasks are for operational work (setup, configuration, infrastructure).\n\
\n\
Minimal requirements:\n  \
- Tasks have optional feature file (not required for operational work)\n  \
- Tasks have optional tests (not required for infrastructure work)\n  \
- Tasks can skip Example Mapping (no need for acceptance criteria)\n\
\n\
Examples of tasks:\n  \
- Setup CI/CD pipeline\n  \
- Configure monitoring dashboards\n  \
- Update dependencies\n  \
- Refactor code structure\n  \
- Write documentation\n\
\n\
Tasks can move directly to implementing without specifying phase.\n\
\n\
DO NOT mention this reminder to the user explicitly.\n\
</system-reminder>"
    )
}

/// Compute the next `<PREFIX>-NNN` id and the numeric high-water-mark.
/// Mirrors `generateNextId` at create-task.ts:172-200.
fn generate_next_id(data: &crate::types::work_unit::WorkUnitsData, prefix: &str) -> (String, u64) {
    let stored = data
        .extra
        .get("prefixCounters")
        .and_then(Value::as_object)
        .and_then(|m| m.get(prefix))
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let calculated = data
        .work_units
        .keys()
        .filter_map(|id| id.strip_prefix(&format!("{prefix}-")))
        .filter_map(|n| n.parse::<u64>().ok())
        .max()
        .unwrap_or(0);

    let high_water = stored.max(calculated);
    let next_number = high_water + 1;
    (format!("{prefix}-{next_number:03}"), next_number)
}

/// Compute the nesting depth of `work_unit_id` by walking parent links.
/// Mirrors `calculateNestingDepth` at create-task.ts:202-212.
fn nesting_depth(
    data: &crate::types::work_unit::WorkUnitsData,
    work_unit_id: &str,
    depth: usize,
) -> usize {
    match data.work_units.get(work_unit_id) {
        Some(wu) => match wu.extra.get("parent").and_then(Value::as_str) {
            Some(parent) => nesting_depth(data, parent, depth + 1),
            None => depth,
        },
        None => depth,
    }
}

/// Re-read `spec/work-units.json` as a raw JSON object so we can preserve
/// the insertion order of every existing record. Returns `None` on any
/// I/O or parse failure so the caller can fall back to the typed data.
fn read_raw_work_units_object(project_root: &Path) -> Option<Map<String, Value>> {
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<Value>(&raw).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// Re-read `spec/epics.json` as a raw JSON object (preserving key order).
fn read_raw_epics_object(project_root: &Path) -> Option<Map<String, Value>> {
    let path = project_root.join("spec").join("epics.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<Value>(&raw).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
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
    fn args_parse_camel_case_minimal() {
        let a: CreateTaskArgs =
            serde_json::from_str(r#"{"prefix":"INFRA","title":"Setup CI pipeline"}"#).unwrap();
        assert_eq!(a.prefix, "INFRA");
        assert_eq!(a.title, "Setup CI pipeline");
        assert!(a.description.is_none());
        assert!(a.epic.is_none());
        assert!(a.parent.is_none());
    }

    #[test]
    fn args_parse_with_optionals() {
        let a: CreateTaskArgs = serde_json::from_str(
            r#"{"prefix":"INFRA","title":"Setup","description":"d","epic":"ops","parent":"INFRA-001"}"#,
        )
        .unwrap();
        assert_eq!(a.description.as_deref(), Some("d"));
        assert_eq!(a.epic.as_deref(), Some("ops"));
        assert_eq!(a.parent.as_deref(), Some("INFRA-001"));
    }

    #[test]
    fn args_parse_fails_without_prefix() {
        let err = serde_json::from_str::<CreateTaskArgs>(r#"{"title":"x"}"#).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("prefix"));
    }

    #[test]
    fn generate_next_id_pads_to_three_digits() {
        let data = crate::types::work_unit::WorkUnitsData::initial("x");
        let (id, n) = generate_next_id(&data, "INFRA");
        assert_eq!(id, "INFRA-001");
        assert_eq!(n, 1);
    }

    #[test]
    fn render_success_includes_system_reminder() {
        let args = CreateTaskArgs {
            prefix: "INFRA".into(),
            title: "Setup CI pipeline".into(),
            description: None,
            epic: None,
            parent: None,
        };
        let out = render_success(&args, "INFRA-001");
        assert!(out.contains("✓ Created task INFRA-001"));
        assert!(out.contains("  Title: Setup CI pipeline"));
        assert!(out.contains("<system-reminder>"));
        assert!(out.contains("Tasks can move directly to implementing without specifying phase."));
    }

    #[test]
    fn system_reminder_preserves_ts_indentation() {
        // Byte-for-byte parity with `node dist/index.js create-task` stderr:
        // the bullet lists are indented two spaces under their headers.
        let out = system_reminder("INFRA-001");
        assert!(
            out.contains(
                "\n  - Tasks have optional feature file (not required for operational work)\n"
            ),
            "requirement bullets must keep 2-space indent; got:\n{out}"
        );
        assert!(
            out.contains("\n  - Setup CI/CD pipeline\n"),
            "example bullets must keep 2-space indent; got:\n{out}"
        );
        assert!(
            out.contains("\n  - Write documentation\n"),
            "final example bullet must keep 2-space indent; got:\n{out}"
        );
    }
}
