//! `show-work-unit` — Rust port of `src/commands/show-work-unit.ts` (RPC-308).
//!
//! Displays a complete dump of a work unit (Example Mapping data,
//! dependencies, linked feature files, and contextual system reminders).
//!
//! Both invocation paths — the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand — call this single function
//! (RPC-003 §7/§11 two-front-doors invariant).
//!
//! ## Behaviour parity with TypeScript (`src/commands/show-work-unit.ts`)
//!
//! * Reads `spec/work-units.json` via bare `readFile` (TS line 71): ENOENT
//!   escalates as a structured I/O error and the Rust port DOES NOT
//!   auto-create the file (deviates from `show-deleted` which uses the
//!   load-or-init helper).
//! * Missing work unit → `"Work unit '<id>' does not exist"` (TS line 76).
//! * Soft-delete filtering on rules/examples/architectureNotes (`deleted ===
//!   true` excluded), plus the `selected` filter on questions.
//! * Bare-string question entries → `"Invalid question format. Questions
//!   must be QuestionItem objects."` (TS lines 188-192).
//! * `linkedFeatures` mirrors `extract_work_unit_tags` from `show_feature.rs`
//!   but ALWAYS returns an empty array on any error (missing
//!   `spec/features/`, gherkin parse failure, or I/O) — never escalates.
//! * Five system reminders are appended when applicable:
//!   missing-estimate, empty-example-mapping (specifying status),
//!   long-duration (>= 24h), large-estimate (> 13 pts, story/bug),
//!   soft-delete count notice. The `FSPEC_DISABLE_REMINDERS=1` env gate
//!   suppresses ALL reminders.
//! * `consolidateReminders` strips inner `<system-reminder>` wrappers and
//!   re-wraps the joined body in a single block (TS line 1057-1076).
//!
//! ## Field ordering on the JSON wire
//!
//! Fields are emitted in the TS declaration order
//! `id, title, type, status, description?, estimate?, epic?, parent?,
//! children?, blocks?, blockedBy?, dependsOn?, relatesTo?, rules?,
//! deletedRules?, examples?, questions?, assumptions?, architectureNotes?,
//! attachments?, virtualHooks?, createdAt, updatedAt, linkedFeatures,
//! systemReminders?, systemReminder?` because we use `#[derive(Serialize)]`
//! with explicit field-declaration order (not `serde_json::Map`, which is
//! alphabetical).

use std::path::Path;

use gherkin::Feature;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;

/// CLI / dispatcher arguments accepted by `show-work-unit`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowWorkUnitArgs {
    /// Required work-unit identifier (e.g. `AUTH-001`).
    work_unit_id: Option<String>,
    /// `"text"` (default) or `"json"`.
    format: Option<String>,
    /// Verbose mode (currently affects `deletedRules` rendering only).
    #[serde(default)]
    verbose: Option<bool>,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions/working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowWorkUnitArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-work-unit",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args.work_unit_id.ok_or(FspecCoreError::InvalidArgs {
        command: "show-work-unit",
        reason: "missing required argument: workUnitId".to_string(),
    })?;

    let verbose = args.verbose.unwrap_or(false);

    // Read spec/work-units.json directly — TS uses bare readFile (line 71)
    // which escalates ENOENT as a hard error and DOES NOT auto-create.
    let path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "show-work-unit",
        source,
    })?;
    let root: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "work-units.json".to_string(),
        reason: e.to_string(),
    })?;

    let wu = root
        .get("workUnits")
        .and_then(Value::as_object)
        .and_then(|m| m.get(&work_unit_id))
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "show-work-unit",
            reason: format!("Work unit '{work_unit_id}' does not exist"),
        })?;

    let projection = project_work_unit(&work_unit_id, wu, verbose)?;

    // Scan feature files for linked scenarios. Any failure is silently
    // swallowed — TS wraps the whole block in a bare `try {} catch {}`
    // and returns an empty array if `spec/features/` doesn't exist
    // (line 84-130).
    let linked = scan_linked_features(project_root, &work_unit_id);

    // Build reminders (env-gated).
    let reminders = build_system_reminders(&work_unit_id, wu, &linked);
    let consolidated = consolidate_reminders(&reminders);

    let result = ShowWorkUnitResult {
        projection,
        linked_features: linked,
        system_reminders: if reminders.is_empty() {
            None
        } else {
            Some(reminders)
        },
        system_reminder: consolidated,
    };

    match args.format.as_deref() {
        Some("json") => {
            serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
                command: "show-work-unit",
                reason: format!("failed to serialize result: {e}"),
            })
        }
        // Default to text.
        _ => Ok(render_text(&result)),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Projection structures (TS WorkUnitDetails)
// ─────────────────────────────────────────────────────────────────────────

/// Serialized payload for a single linked scenario.
#[derive(Debug, Clone, Serialize)]
struct LinkedScenarioRef {
    name: String,
    line: usize,
    file: String,
}

/// Serialized payload for one linked feature file.
#[derive(Debug, Clone, Serialize)]
struct LinkedFeature {
    file: String,
    scenarios: Vec<LinkedScenarioRef>,
}

/// Per-work-unit fields lifted out of the source `workUnits[id]` object,
/// projected with TS soft-delete filtering applied.
#[derive(Debug, Clone, Serialize)]
struct WorkUnitProjection {
    id: String,
    title: String,
    #[serde(rename = "type")]
    r#type: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_estimate_opt"
    )]
    estimate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    epic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blocks: Option<Vec<String>>,
    #[serde(rename = "blockedBy", skip_serializing_if = "Option::is_none")]
    blocked_by: Option<Vec<String>>,
    #[serde(rename = "dependsOn", skip_serializing_if = "Option::is_none")]
    depends_on: Option<Vec<String>>,
    #[serde(rename = "relatesTo", skip_serializing_if = "Option::is_none")]
    relates_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rules: Option<Vec<String>>,
    #[serde(rename = "deletedRules", skip_serializing_if = "Option::is_none")]
    deleted_rules: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    examples: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    questions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assumptions: Option<Vec<String>>,
    #[serde(rename = "architectureNotes", skip_serializing_if = "Option::is_none")]
    architecture_notes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<String>>,
    #[serde(rename = "virtualHooks", skip_serializing_if = "Option::is_none")]
    virtual_hooks: Option<Value>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

/// Serialize an optional `f64` estimate as a JSON integer when whole and
/// finite. Mirrors JS `JSON.stringify(21)` → `21` (not `21.0`).
/// Only invoked when `Option::is_some`; the `skip_serializing_if` guard
/// handles the `None` case before this function runs.
fn serialize_estimate_opt<S>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        Some(n) if n.is_finite() && n.fract() == 0.0 => s.serialize_i64(*n as i64),
        Some(n) => s.serialize_f64(*n),
        None => s.serialize_none(),
    }
}

/// Combined dispatcher output. The custom `Serialize` impl spreads
/// `projection` so the resulting JSON is a single flat object that
/// follows TS field-declaration order rather than nesting under
/// `projection: {...}`.
struct ShowWorkUnitResult {
    projection: WorkUnitProjection,
    linked_features: Vec<LinkedFeature>,
    system_reminders: Option<Vec<String>>,
    system_reminder: Option<String>,
}

impl Serialize for ShowWorkUnitResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        // Round-trip the projection through a Value so we can re-emit its
        // (already-ordered) keys and then append linkedFeatures /
        // reminders LAST — matching TS declaration order.
        let proj_value =
            serde_json::to_value(&self.projection).map_err(serde::ser::Error::custom)?;
        let proj_obj = proj_value
            .as_object()
            .ok_or_else(|| serde::ser::Error::custom("projection must serialize to object"))?;
        // 3 trailing fields + ordered projection length.
        let mut map = serializer.serialize_map(Some(proj_obj.len() + 3))?;
        for (k, v) in proj_obj {
            map.serialize_entry(k, v)?;
        }
        map.serialize_entry("linkedFeatures", &self.linked_features)?;
        if let Some(reminders) = &self.system_reminders {
            map.serialize_entry("systemReminders", reminders)?;
        }
        if let Some(block) = &self.system_reminder {
            map.serialize_entry("systemReminder", block)?;
        }
        map.end()
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Projection — mirrors the TS soft-delete filtering and field gating
// ─────────────────────────────────────────────────────────────────────────

fn as_str_opt(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

fn as_str_array(v: Option<&Value>) -> Option<Vec<String>> {
    let arr = v?.as_array()?;
    let collected: Vec<String> = arr
        .iter()
        .filter_map(|x| x.as_str().map(str::to_string))
        .collect();
    if collected.is_empty() {
        // Mirror TS spread `...workUnit.children &&` — TS treats an empty
        // array as truthy so the field IS emitted. Preserve the source.
        Some(Vec::new())
    } else {
        Some(collected)
    }
}

/// `[id] text` projection for an array of `{id, text, deleted, ...}` items.
/// Excludes items whose `deleted === true`.
fn project_active_items(arr: &[Value]) -> Vec<String> {
    arr.iter()
        .filter(|v| v.get("deleted").and_then(Value::as_bool) != Some(true))
        .filter_map(|v| {
            let id = v.get("id")?.as_u64()?;
            let text = v.get("text")?.as_str()?;
            Some(format!("[{id}] {text}"))
        })
        .collect()
}

/// `[id] text (deletedAt: ...)` projection for `deleted === true` items
/// (verbose mode only).
fn project_deleted_items_verbose(arr: &[Value]) -> Vec<String> {
    arr.iter()
        .filter(|v| v.get("deleted").and_then(Value::as_bool) == Some(true))
        .filter_map(|v| {
            let id = v.get("id")?.as_u64()?;
            let text = v.get("text")?.as_str()?;
            let suffix = match v.get("deletedAt").and_then(Value::as_str) {
                Some(ts) => format!(" (deletedAt: {ts})"),
                None => String::new(),
            };
            Some(format!("[{id}] {text}{suffix}"))
        })
        .collect()
}

fn project_work_unit(
    work_unit_id: &str,
    wu: &Value,
    verbose: bool,
) -> Result<WorkUnitProjection, FspecCoreError> {
    // Helper: optional array of strings from a JSON field.
    let id = as_str_opt(wu.get("id")).unwrap_or_else(|| work_unit_id.to_string());
    let title = as_str_opt(wu.get("title")).unwrap_or_default();
    // Default to "story" for missing/empty type (TS `wu.type || 'story'`).
    let r#type = wu
        .get("type")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("story")
        .to_string();
    let status = as_str_opt(wu.get("status")).unwrap_or_default();

    // Rules + (verbose) deletedRules.
    let mut rules_proj: Option<Vec<String>> = None;
    let mut deleted_rules_proj: Option<Vec<String>> = None;
    if let Some(rules) = wu.get("rules").and_then(Value::as_array) {
        if !rules.is_empty() {
            let active = project_active_items(rules);
            if !active.is_empty() {
                rules_proj = Some(active);
            }
            if verbose {
                let del = project_deleted_items_verbose(rules);
                if !del.is_empty() {
                    deleted_rules_proj = Some(del);
                }
            }
        }
    }

    let examples_proj = wu
        .get("examples")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .map(|a| project_active_items(a))
        .filter(|v| !v.is_empty());

    // Questions: enforce object-form, then filter deleted + selected.
    let mut questions_proj: Option<Vec<String>> = None;
    if let Some(qs) = wu.get("questions").and_then(Value::as_array) {
        if !qs.is_empty() {
            let mut out: Vec<String> = Vec::new();
            for q in qs {
                if q.is_string() {
                    return Err(FspecCoreError::InvalidArgs {
                        command: "show-work-unit",
                        reason: "Invalid question format. Questions must be QuestionItem objects."
                            .to_string(),
                    });
                }
                let obj = match q.as_object() {
                    Some(o) => o,
                    None => continue,
                };
                if obj.get("deleted").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                if obj.get("selected").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                let id = match obj.get("id").and_then(Value::as_u64) {
                    Some(v) => v,
                    None => continue,
                };
                let text = match obj.get("text").and_then(Value::as_str) {
                    Some(v) => v,
                    None => continue,
                };
                out.push(format!("[{id}] {text}"));
            }
            if !out.is_empty() {
                questions_proj = Some(out);
            }
        }
    }

    let arch_proj = wu
        .get("architectureNotes")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .map(|a| project_active_items(a))
        .filter(|v| !v.is_empty());

    let virtual_hooks = wu.get("virtualHooks").cloned();

    Ok(WorkUnitProjection {
        id,
        title,
        r#type,
        status,
        description: as_str_opt(wu.get("description")),
        estimate: wu.get("estimate").and_then(Value::as_f64),
        epic: as_str_opt(wu.get("epic")),
        parent: as_str_opt(wu.get("parent")),
        children: as_str_array(wu.get("children")),
        blocks: as_str_array(wu.get("blocks")),
        blocked_by: as_str_array(wu.get("blockedBy")),
        depends_on: as_str_array(wu.get("dependsOn")),
        relates_to: as_str_array(wu.get("relatesTo")),
        rules: rules_proj,
        deleted_rules: deleted_rules_proj,
        examples: examples_proj,
        questions: questions_proj,
        assumptions: as_str_array(wu.get("assumptions")),
        architecture_notes: arch_proj,
        attachments: as_str_array(wu.get("attachments")),
        virtual_hooks,
        created_at: as_str_opt(wu.get("createdAt")).unwrap_or_default(),
        updated_at: as_str_opt(wu.get("updatedAt")).unwrap_or_default(),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Linked-features scan (silently degrades on any error)
// ─────────────────────────────────────────────────────────────────────────

/// Walk `spec/features/`, parse every feature file with the gherkin crate,
/// and aggregate scenarios that reference the requested work-unit ID via
/// feature- or scenario-level tags. Returns an EMPTY vec on ANY error
/// (missing directory, parse failure, I/O) — never escalates.
fn scan_linked_features(project_root: &Path, work_unit_id: &str) -> Vec<LinkedFeature> {
    let files = match glob_feature_files(project_root) {
        Ok(v) => v,
        // Missing spec/features/ OR any I/O failure → empty list (TS bare catch).
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<LinkedFeature> = Vec::new();
    for rel in files {
        let abs = project_root.join(&rel);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let feature = match crate::io::gherkin::parse_feature_lenient(&content) {
            Ok(f) => f,
            // Skip invalid feature files (TS does `continue`).
            Err(_) => continue,
        };

        if let Some(scenarios) = extract_scenarios_for_work_unit(&feature, work_unit_id) {
            if !scenarios.is_empty() {
                let scenario_refs: Vec<LinkedScenarioRef> = scenarios
                    .into_iter()
                    .map(|(name, line)| LinkedScenarioRef {
                        name,
                        line,
                        file: rel.clone(),
                    })
                    .collect();
                out.push(LinkedFeature {
                    file: rel,
                    scenarios: scenario_refs,
                });
            }
        }
    }
    out
}

/// Mirrors `extract_work_unit_tags` from `show_feature.rs` but specialised
/// to a single requested ID — returns `Some(scenarios)` when the feature
/// carries (or its scenarios reference) the requested ID, `None` otherwise.
///
/// Feature-level tags claim scenarios that DO NOT have their own work-unit
/// override; scenario-level tags claim only their own scenario.
fn extract_scenarios_for_work_unit(
    feature: &Feature,
    work_unit_id: &str,
) -> Option<Vec<(String, usize)>> {
    let feature_ids: Vec<String> = feature
        .tags
        .iter()
        .filter_map(|t| extract_work_unit_id(t))
        .collect();

    let mut result: Vec<(String, usize)> = Vec::new();
    let mut matched_any = false;

    // Feature-level inheritance: attach scenarios with NO override.
    if feature_ids.iter().any(|id| id == work_unit_id) {
        matched_any = true;
        for s in &feature.scenarios {
            let has_own_override = s.tags.iter().any(|t| extract_work_unit_id(t).is_some());
            if !has_own_override {
                result.push((s.name.clone(), s.position.line));
            }
        }
    }

    // Scenario-level direct references.
    for s in &feature.scenarios {
        for tag in &s.tags {
            if let Some(id) = extract_work_unit_id(tag) {
                if id == work_unit_id {
                    matched_any = true;
                    // Avoid double-listing a scenario the feature-level
                    // pass already captured (only happens when a scenario
                    // is explicitly retagged with the same WU id; harmless
                    // dedup).
                    let key = (s.name.clone(), s.position.line);
                    if !result.iter().any(|r| r == &key) {
                        result.push(key);
                    }
                }
            }
        }
    }

    if matched_any {
        Some(result)
    } else {
        None
    }
}

/// Extract a `PREFIX-NNN` ID from a tag (case-sensitive uppercase prefix,
/// 2–6 chars). Accepts both `@PREFIX-NNN` and bare `PREFIX-NNN`.
fn extract_work_unit_id(tag: &str) -> Option<String> {
    let stripped = tag.strip_prefix('@').unwrap_or(tag);
    let (prefix, num) = stripped.split_once('-')?;
    if prefix.is_empty() || prefix.len() < 2 || prefix.len() > 6 {
        return None;
    }
    if !prefix.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(stripped.to_string())
}

// ─────────────────────────────────────────────────────────────────────────
// System reminders — mirrors src/utils/system-reminder.ts
// ─────────────────────────────────────────────────────────────────────────

fn reminders_enabled() -> bool {
    std::env::var("FSPEC_DISABLE_REMINDERS").as_deref() != Ok("1")
}

fn wrap_in_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

/// Build the list of contextual reminders for the work unit. Returns an
/// empty Vec when reminders are disabled OR no reminder conditions
/// trigger. Mirrors the five `get*Reminder` helpers in
/// `src/utils/system-reminder.ts`.
fn build_system_reminders(work_unit_id: &str, wu: &Value, linked: &[LinkedFeature]) -> Vec<String> {
    if !reminders_enabled() {
        return Vec::new();
    }

    let mut out: Vec<String> = Vec::new();
    let status = wu.get("status").and_then(Value::as_str).unwrap_or("");
    let estimate = wu.get("estimate").and_then(Value::as_f64);
    let wu_type = wu
        .get("type")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("story")
        .to_string();
    let has_estimate = estimate.is_some();

    // 1. Missing estimate (skip when status == backlog).
    if let Some(r) = missing_estimate_reminder(work_unit_id, has_estimate, status) {
        out.push(r);
    }

    // 2. Empty Example Mapping (specifying status only).
    if status == "specifying" {
        let has_rules = wu
            .get("rules")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .any(|r| r.get("deleted").and_then(Value::as_bool) != Some(true))
            })
            .unwrap_or(false);
        let has_examples = wu
            .get("examples")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .any(|r| r.get("deleted").and_then(Value::as_bool) != Some(true))
            })
            .unwrap_or(false);
        if let Some(r) = empty_example_mapping_reminder(work_unit_id, has_rules, has_examples) {
            out.push(r);
        }
    }

    // 3. Long-duration reminder (>= 24h in current phase).
    if let Some(history) = wu.get("stateHistory").and_then(Value::as_array) {
        if let Some(last) = history.last() {
            if let Some(ts) = last.get("timestamp").and_then(Value::as_str) {
                let duration_hours = compute_hours_since_iso(ts);
                if let Some(r) = long_duration_reminder(work_unit_id, status, duration_hours) {
                    out.push(r);
                }
            }
        }
    }

    // 4. Large estimate (> 13 pts, story/bug, non-done).
    let has_feature_file = !linked.is_empty();
    if let Some(r) =
        large_estimate_reminder(work_unit_id, estimate, &wu_type, status, has_feature_file)
    {
        out.push(r);
    }

    // 5. Soft-delete count notice (rules only — TS lines 276-284).
    if let Some(rules) = wu.get("rules").and_then(Value::as_array) {
        if !rules.is_empty() {
            let active_count = rules
                .iter()
                .filter(|r| r.get("deleted").and_then(Value::as_bool) != Some(true))
                .count();
            let deleted_count = rules
                .iter()
                .filter(|r| r.get("deleted").and_then(Value::as_bool) == Some(true))
                .count();
            if deleted_count > 0 {
                out.push(format!(
                    "{active_count} active items ({deleted_count} deleted)"
                ));
            }
        }
    }

    out
}

fn missing_estimate_reminder(
    work_unit_id: &str,
    has_estimate: bool,
    status: &str,
) -> Option<String> {
    if has_estimate || status == "backlog" {
        return None;
    }
    let body = format!(
        "Work unit {work_unit_id} has no estimate.\nAfter generating scenarios from Example Mapping, estimate based on feature file complexity.\nFibonacci scale: 1 (trivial), 2 (simple), 3 (moderate), 5 (complex), 8 (very complex), 13+ (too large - break down)\nRun: fspec update-work-unit-estimate {work_unit_id} <points>\nDO NOT mention this reminder to the user."
    );
    Some(wrap_in_system_reminder(&body))
}

fn empty_example_mapping_reminder(
    work_unit_id: &str,
    has_rules: bool,
    has_examples: bool,
) -> Option<String> {
    if has_rules && has_examples {
        return None;
    }
    let body = format!(
        "Work unit {work_unit_id} has no Example Mapping data (rules, examples, questions).\n\nCRITICAL: Complete Example Mapping BEFORE generating scenarios:\n  1. Capture business rules: fspec add-rule {work_unit_id} \"[rule]\"\n  2. Gather concrete examples: fspec add-example {work_unit_id} \"[example]\"\n  3. Ask clarifying questions: fspec add-question {work_unit_id} \"@human: [question]\"\n\nDiscovery prevents building the wrong feature. DO NOT mention this reminder to the user."
    );
    Some(wrap_in_system_reminder(&body))
}

fn long_duration_reminder(work_unit_id: &str, status: &str, duration_hours: f64) -> Option<String> {
    if duration_hours < 24.0 {
        return None;
    }
    let advice = match status {
        "backlog" => "Consider prioritizing or breaking down this work unit",
        "specifying" => "Unclear requirements - need more Example Mapping or clarification",
        "testing" => "Complex test setup - consider breaking down work unit",
        "implementing" => "Scope too large - consider splitting work unit",
        "validating" => "Quality issues or blocked on review - address blockers",
        "blocked" => "Blocker needs resolution or escalation",
        _ => "",
    };
    let hours = duration_hours.floor() as i64;
    let body = format!(
        "Work unit {work_unit_id} has been in {status} status for {hours} hours.\n\nThis may indicate: {advice}\n\nReview progress and consider next steps. DO NOT mention this reminder to the user."
    );
    Some(wrap_in_system_reminder(&body))
}

fn large_estimate_reminder(
    work_unit_id: &str,
    estimate: Option<f64>,
    wu_type: &str,
    status: &str,
    has_feature_file: bool,
) -> Option<String> {
    if wu_type != "story" && wu_type != "bug" {
        return None;
    }
    let est = estimate?;
    if est <= 13.0 {
        return None;
    }
    if status == "done" {
        return None;
    }

    let guidance = if has_feature_file {
        "\n1. REVIEW FEATURE FILE for natural boundaries:\n   - Look for scenario groupings that could be separate stories\n   - Each group should deliver incremental value\n   - Identify clear acceptance criteria boundaries".to_string()
    } else {
        format!(
            "\n1. CREATE FEATURE FILE FIRST before breaking down:\n   - Run: fspec generate-scenarios {work_unit_id}\n   - Complete the feature file with all scenarios\n   - Then identify natural boundaries for splitting"
        )
    };

    // f64 → display: prefer integer form for whole numbers (matches TS
    // template literal `${estimate}`).
    let est_str = if est.fract() == 0.0 {
        format!("{}", est as i64)
    } else {
        format!("{est}")
    };

    let body = format!(
        "LARGE ESTIMATE WARNING: Work unit {work_unit_id} estimate is greater than 13 points.\n\n{est_str} points is too large for a single {wu_type}. Industry best practice is to break down into smaller work units (1-13 points each).\n\nWHY BREAK DOWN:\n  - Reduces risk and complexity\n  - Enables incremental delivery\n  - Improves estimation accuracy\n  - Makes progress more visible\n\nSTEP-BY-STEP WORKFLOW:\n{guidance}\n\n2. IDENTIFY BOUNDARIES:\n   - Group related scenarios that deliver value together\n   - Each child work unit should be estimable at 1-13 points\n\n3. CREATE CHILD WORK UNITS:\n   - Run: fspec create-story <PREFIX> \"<Title>\" (for features/refactoring)\n   - Run: fspec create-bug <PREFIX> \"<Title>\" (for bug fixes)\n   - Run: fspec create-task <PREFIX> \"<Title>\" (for operational tasks)\n   - Create one child work unit for each logical grouping\n\n4. LINK DEPENDENCIES:\n   - Run: fspec add-dependency <CHILD-ID> --depends-on {work_unit_id}\n   - This establishes parent-child relationships\n\n5. ESTIMATE EACH CHILD:\n   - Run: fspec update-work-unit-estimate <CHILD-ID> <points>\n   - Each child should be 1-13 points\n\n6. HANDLE PARENT:\n   - Option A: Delete original work unit (if no longer needed)\n   - Option B: Convert to epic to group children\n     Run: fspec create-epic \"<Epic Name>\" <PREFIX> \"<Description>\"\n\nDO NOT mention this reminder to the user explicitly."
    );
    Some(wrap_in_system_reminder(&body))
}

/// Consolidate any number of reminder strings into a single
/// `<system-reminder>` block, stripping inner wrappers first and
/// joining the bodies with a blank line. Mirrors TS
/// `consolidateReminders` (`src/utils/system-reminder.ts:1057-1076`).
fn consolidate_reminders(reminders: &[String]) -> Option<String> {
    if reminders.is_empty() {
        return None;
    }
    let mut bodies: Vec<String> = Vec::new();
    for r in reminders {
        let stripped = r
            .replace("<system-reminder>\n", "")
            .replace("<system-reminder>", "")
            .replace("</system-reminder>\n", "")
            .replace("</system-reminder>", "");
        let trimmed = stripped.trim().to_string();
        if !trimmed.is_empty() {
            bodies.push(trimmed);
        }
    }
    if bodies.is_empty() {
        return None;
    }
    Some(wrap_in_system_reminder(&bodies.join("\n\n")))
}

/// Best-effort parsing of an ISO-8601 timestamp into hours-since-now.
/// Returns 0.0 on any parse error so the long-duration reminder does
/// not fire spuriously.
fn compute_hours_since_iso(ts: &str) -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let then_secs = match iso8601_to_epoch_secs(ts) {
        Some(v) => v,
        None => return 0.0,
    };
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let delta = now_secs.saturating_sub(then_secs);
    (delta as f64) / 3600.0
}

/// Parse `"YYYY-MM-DDTHH:MM:SS[.fff]Z"` into Unix epoch seconds (UTC).
/// Returns `None` on any malformed input.
fn iso8601_to_epoch_secs(ts: &str) -> Option<i64> {
    // Strip trailing Z (assume UTC) and optional fractional seconds.
    let s = ts.strip_suffix('Z').unwrap_or(ts);
    let (date, time_with_frac) = s.split_once('T')?;
    let time = time_with_frac.split('.').next().unwrap_or(time_with_frac);
    let mut date_parts = date.split('-');
    let y: i64 = date_parts.next()?.parse().ok()?;
    let mo: u32 = date_parts.next()?.parse().ok()?;
    let d: u32 = date_parts.next()?.parse().ok()?;
    let mut time_parts = time.split(':');
    let h: u32 = time_parts.next()?.parse().ok()?;
    let m: u32 = time_parts.next()?.parse().ok()?;
    let sec: u32 = time_parts.next()?.parse().ok()?;
    // Howard Hinnant's civil → days-since-epoch.
    let (yy, mm) = if mo <= 2 {
        (y - 1, mo + 9)
    } else {
        (y, mo - 3)
    };
    let era = if yy >= 0 { yy / 400 } else { (yy - 399) / 400 };
    let yoe = yy - era * 400; // [0, 399]
    let doy = (153 * mm as i64 + 2) / 5 + d as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + (h as i64) * 3600 + (m as i64) * 60 + sec as i64;
    Some(secs)
}

// ─────────────────────────────────────────────────────────────────────────
// Text rendering (TS showWorkUnitCommand text branch)
// ─────────────────────────────────────────────────────────────────────────

/// Render the text-format dump that mirrors the TS Commander.js
/// CLI rendering at `src/commands/show-work-unit.ts:328-455`.
/// ANSI colour wrappers from the TS `chalk.*` calls are dropped — the
/// byte-parity contract is defined against non-TTY captured output.
///
/// We deliberately emit the raw ISO timestamps for `Created:` / `Updated:`
/// (TS uses `.toLocaleString()`, which is locale-specific and unstable
/// across platforms — see show-deleted / show-epic for the same deviation).
fn render_text(result: &ShowWorkUnitResult) -> String {
    let p = &result.projection;
    let mut out = String::new();
    out.push('\n');
    out.push_str(&format!("{}\n", p.id));
    out.push_str(&format!("Type: {}\n", p.r#type));
    out.push_str(&format!("Status: {}\n", p.status));
    out.push('\n');
    out.push_str(&format!("{}\n", p.title));
    if let Some(desc) = &p.description {
        out.push_str(&format!("{desc}\n"));
    }
    out.push('\n');

    if let Some(epic) = &p.epic {
        out.push_str(&format!("Epic: {epic}\n"));
    }
    if let Some(parent) = &p.parent {
        out.push_str(&format!("Parent: {parent}\n"));
    }
    if let Some(children) = &p.children {
        if !children.is_empty() {
            out.push_str(&format!("Children: {}\n", children.join(", ")));
        }
    }

    if let Some(blocks) = &p.blocks {
        if !blocks.is_empty() {
            out.push_str(&format!("Blocks: {}\n", blocks.join(", ")));
        }
    }
    if let Some(bb) = &p.blocked_by {
        if !bb.is_empty() {
            out.push_str(&format!("Blocked By: {}\n", bb.join(", ")));
        }
    }
    if let Some(d) = &p.depends_on {
        if !d.is_empty() {
            out.push_str(&format!("Depends On: {}\n", d.join(", ")));
        }
    }
    if let Some(r) = &p.relates_to {
        if !r.is_empty() {
            out.push_str(&format!("Related To: {}\n", r.join(", ")));
        }
    }

    if let Some(rules) = &p.rules {
        if !rules.is_empty() {
            out.push_str("\nRules:\n");
            for r in rules {
                out.push_str(&format!("  {r}\n"));
            }
        }
    }
    if let Some(examples) = &p.examples {
        if !examples.is_empty() {
            out.push_str("\nExamples:\n");
            for e in examples {
                out.push_str(&format!("  {e}\n"));
            }
        }
    }
    if let Some(questions) = &p.questions {
        if !questions.is_empty() {
            out.push_str("\nQuestions:\n");
            for q in questions {
                out.push_str(&format!("  {q}\n"));
            }
        }
    }
    if let Some(assumptions) = &p.assumptions {
        if !assumptions.is_empty() {
            out.push_str("\nAssumptions:\n");
            for (idx, a) in assumptions.iter().enumerate() {
                out.push_str(&format!("  {}. {a}\n", idx + 1));
            }
        }
    }
    if let Some(notes) = &p.architecture_notes {
        if !notes.is_empty() {
            out.push_str("\nArchitecture Notes:\n");
            for n in notes {
                out.push_str(&format!("  {n}\n"));
            }
        }
    }
    if let Some(attachments) = &p.attachments {
        if !attachments.is_empty() {
            out.push_str("\nAttachments:\n");
            for (idx, a) in attachments.iter().enumerate() {
                out.push_str(&format!("  {}. {a}\n", idx + 1));
            }
        }
    }

    if let Some(vh) = &p.virtual_hooks {
        if let Some(arr) = vh.as_array() {
            if !arr.is_empty() {
                out.push_str("\nVirtual Hooks:\n");
                // Group hooks by event field (TS hooksByEvent).
                let mut by_event: Vec<(String, Vec<&Value>)> = Vec::new();
                for h in arr {
                    let event = h
                        .get("event")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    if let Some(idx) = by_event.iter().position(|(e, _)| e == &event) {
                        by_event[idx].1.push(h);
                    } else {
                        by_event.push((event, vec![h]));
                    }
                }
                for (event, hooks) in by_event {
                    out.push_str(&format!("  {event}:\n"));
                    for hk in hooks {
                        let name = hk.get("name").and_then(Value::as_str).unwrap_or("");
                        let blocking = hk.get("blocking").and_then(Value::as_bool).unwrap_or(false);
                        let git_ctx = hk
                            .get("gitContext")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        let bb = if blocking {
                            "(blocking)"
                        } else {
                            "(non-blocking)"
                        };
                        let gc = if git_ctx { " [git-context]" } else { "" };
                        out.push_str(&format!("    • {name} {bb}{gc}\n"));
                        let cmd = hk.get("command").and_then(Value::as_str).unwrap_or("");
                        out.push_str(&format!("      {cmd}\n"));
                    }
                }
            }
        }
    }

    if !result.linked_features.is_empty() {
        out.push_str("\nLinked Features:\n");
        for f in &result.linked_features {
            out.push_str(&format!("\n  {}\n", f.file));
            for s in &f.scenarios {
                out.push_str(&format!("    {}:{} - {}\n", s.file, s.line, s.name));
            }
        }
    }

    out.push('\n');
    out.push_str(&format!("Created: {}\n", p.created_at));
    out.push_str(&format!("Updated: {}\n", p.updated_at));
    out.push('\n');

    if let Some(block) = &result.system_reminder {
        out.push_str(block);
        out.push('\n');
        out.push('\n');
    }

    out
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
    use serde_json::json;

    #[test]
    fn args_parse_with_defaults() {
        let a: ShowWorkUnitArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
        assert!(a.format.is_none());
    }

    #[test]
    fn args_parse_camel_case() {
        let a: ShowWorkUnitArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","format":"json"}"#).unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.format.as_deref(), Some("json"));
    }

    #[test]
    fn extract_work_unit_id_accepts_canonical_form() {
        assert_eq!(
            extract_work_unit_id("AUTH-001"),
            Some("AUTH-001".to_string())
        );
        assert_eq!(
            extract_work_unit_id("@AUTH-001"),
            Some("AUTH-001".to_string())
        );
    }

    #[test]
    fn extract_work_unit_id_rejects_non_canonical() {
        assert!(extract_work_unit_id("auth-001").is_none());
        assert!(extract_work_unit_id("@critical").is_none());
        assert!(extract_work_unit_id("A-1").is_none());
    }

    #[test]
    fn project_active_items_filters_deleted_true() {
        let arr = vec![
            json!({"id":0,"text":"keep","deleted":false}),
            json!({"id":1,"text":"drop","deleted":true}),
            json!({"id":2,"text":"keep2"}),
        ];
        let out = project_active_items(&arr);
        assert_eq!(out, vec!["[0] keep", "[2] keep2"]);
    }

    #[test]
    fn consolidate_reminders_strips_and_rewraps() {
        let r1 = "<system-reminder>\nfoo\n</system-reminder>".to_string();
        let r2 = "bar".to_string();
        let combined = consolidate_reminders(&[r1, r2]).expect("Some");
        assert!(combined.starts_with("<system-reminder>"));
        assert!(combined.ends_with("</system-reminder>"));
        assert_eq!(combined.matches("<system-reminder>").count(), 1);
        assert!(combined.contains("foo\n\nbar"));
    }

    #[test]
    fn iso8601_round_trip_known_dates() {
        assert_eq!(iso8601_to_epoch_secs("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            iso8601_to_epoch_secs("2000-01-01T00:00:00.000Z"),
            Some(946_684_800)
        );
    }

    #[test]
    fn missing_estimate_reminder_suppressed_in_backlog() {
        assert!(missing_estimate_reminder("AUTH-001", false, "backlog").is_none());
        assert!(missing_estimate_reminder("AUTH-001", true, "specifying").is_none());
        assert!(missing_estimate_reminder("AUTH-001", false, "specifying").is_some());
    }

    #[test]
    fn large_estimate_reminder_only_fires_for_story_and_bug() {
        assert!(
            large_estimate_reminder("AUTH-001", Some(21.0), "task", "implementing", false)
                .is_none()
        );
        assert!(large_estimate_reminder("AUTH-001", Some(21.0), "story", "done", false).is_none());
        assert!(
            large_estimate_reminder("AUTH-001", Some(13.0), "story", "implementing", false)
                .is_none()
        );
        let r = large_estimate_reminder("AUTH-001", Some(21.0), "story", "implementing", false)
            .expect("Some");
        assert!(r.contains("LARGE ESTIMATE WARNING"));
        assert!(r.contains("CREATE FEATURE FILE FIRST"));
    }
}
