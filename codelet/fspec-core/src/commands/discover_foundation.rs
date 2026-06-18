//! `discover-foundation` — Rust port of `src/commands/discover-foundation.ts`
//! (RPC-226).
//!
//! Orchestrates the draft-driven foundation-discovery workflow:
//!
//! 1. **Draft creation** (default mode) — writes
//!    `spec/foundation.json.draft` with v2.0.0 `[QUESTION:]`/`[DETECTED:]`
//!    placeholders and returns a field-by-field `<system-reminder>` for the
//!    FIRST unfilled placeholder (Field 1/8: project.name).
//! 2. **Guard rails** — without `--force` an existing draft OR an existing
//!    `spec/foundation.json` blocks creation (valid=false + wrapped
//!    `<system-reminder>`); `--force` regenerates the draft with a warning.
//! 3. **Finalize** (`--finalize`) — reads+parses the draft, scans for unfilled
//!    placeholders (blocks with `Cannot finalize: ...`), validates against the
//!    generic-foundation schema (blocks with `Schema validation failed.`),
//!    then writes `spec/foundation.json`, deletes the draft, best-effort
//!    auto-creates an idempotent FOUND- task work unit (Foundation Event
//!    Storm), and (when `autoGenerateMd`) regenerates `spec/FOUNDATION.md`.
//!
//! ## Return envelope
//!
//! Mirrors `update_foundation.rs`: `run` returns a JSON string the CLI bridge
//! decodes. The envelope carries `valid` plus the optional
//! `systemReminder` / `validationErrors` / `completionMessage` strings the TS
//! `discoverFoundation` result object exposes. The dispatcher surfaces the
//! same JSON to the agent loop.
//!
//! ## Field-by-field reminder logic
//!
//! `scanDraftForNextField` + `generateFieldReminder` + `extract_detected_value`
//! + `agent_supports_meta_cognition` + `is_known_agent` are COPIED here
//!   (parallel-safe, isolated) from `update_foundation.rs` — the same verbatim
//!   port of `discover-foundation.ts:37-177`.
//!
//! ## FOUND auto-unit
//!
//! Inline-built mirroring the centralized TS `createWorkUnit`
//! (`work-unit.ts:134-230`): idempotent FOUND- id check (reuse existing id
//! when present), reuse `create_prefix::run` (swallow already-exists), build
//! the task object (id, title, status, createdAt, updatedAt, stateHistory,
//! description, type) + `states.backlog` push. The TS helper does NOT write a
//! `children: []` array for the task NOR touch `prefixCounters`. The WHOLE
//! block is best-effort (every error swallowed) per the TS try/catch.
//!
//! ## poll_sync_future safety
//!
//! All file IO is BLOCKING `std::fs`; `create_prefix::run`,
//! `generate_foundation_md::regenerate`, and `validate_foundation` are sync.
//! No real `.await`, no child process, no network — safe under the
//! single-poll dispatcher.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `discover-foundation`. The CLI surface exposes
/// only `{finalize, output, draftPath, autoGenerateMd, force}` — the
/// TS-internal `scanOnly`/`detectManualEdit`/`lastKnownState` modes (used by
/// update-foundation chaining) are OUT of port scope.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoverFoundationArgs {
    #[serde(default)]
    finalize: bool,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    draft_path: Option<String>,
    /// Defaults to TRUE (mirrors the TS `--auto-generate-md` default).
    #[serde(default = "default_true")]
    auto_generate_md: bool,
    #[serde(default)]
    force: bool,
}

fn default_true() -> bool {
    true
}

/// Dispatcher entry point. Returns a JSON envelope string.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DiscoverFoundationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "discover-foundation",
            reason: format!("failed to parse args: {e}"),
        })?;

    let draft_path = match args.draft_path.as_deref() {
        Some(p) => project_root.join(p),
        None => project_root.join("spec").join("foundation.json.draft"),
    };
    let final_path = match args.output.as_deref() {
        Some(p) => project_root.join(p),
        None => project_root.join("spec").join("foundation.json"),
    };

    if args.finalize {
        return finalize(project_root, &draft_path, &final_path, args.auto_generate_md);
    }

    create_draft(project_root, &draft_path, &final_path, args.force)
}

/// Serialize an envelope `Value` to its JSON string form.
fn envelope(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "discover-foundation",
        reason: format!("failed to serialize result: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Draft creation mode
// ─────────────────────────────────────────────────────────────────────────

/// Canonical draft template `discover-foundation` writes on creation. Mirrors
/// the TS `draftFoundation` literal at `discover-foundation.ts:681-706`.
fn placeholder_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "[QUESTION: What is the project name?]",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "[DETECTED: cli-tool]"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "[QUESTION: What problem does this solve?]",
                "description": "[QUESTION: What problem does this solve?]",
                "impact": "high"
            }
        },
        "solutionSpace": {
            "overview": "[QUESTION: What can users DO?]",
            "capabilities": []
        },
        "personas": [
            {
                "name": "[QUESTION: Who uses this?]",
                "description": "[QUESTION: Who uses this?]",
                "goals": ["[QUESTION: What are their goals?]"]
            }
        ]
    })
}

/// Create (or `--force`-overwrite) the draft. Returns the JSON envelope.
fn create_draft(
    project_root: &Path,
    draft_path: &Path,
    final_path: &Path,
    force: bool,
) -> Result<String, FspecCoreError> {
    // Guard rails (only when NOT --force).
    if !force {
        if draft_path.exists() {
            // Draft already exists — block with the wrapped reminder.
            let reminder = wrap_in_system_reminder(DRAFT_EXISTS_ERROR);
            return envelope(json!({
                "valid": false,
                "systemReminder": reminder,
            }));
        }
        if final_path.exists() {
            // foundation.json already exists — block with the wrapped reminder.
            let reminder = wrap_in_system_reminder(FOUNDATION_EXISTS_ERROR);
            return envelope(json!({
                "valid": false,
                "systemReminder": reminder,
            }));
        }
    }

    // `overwriting` (force AND a draft actually existed) gates the STDERR
    // `output.warn` line. The STDOUT banner below is gated on `force` ALONE —
    // see the `force_overwrite_warning` note.
    let overwriting = force && draft_path.exists();

    let draft = placeholder_draft();

    // Ensure parent dir exists, then write the draft (2-space indent, no
    // trailing newline — JSON.stringify(...,null,2) parity).
    if let Some(parent) = draft_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "discover-foundation",
            source,
        })?;
    }
    write_json_atomic(draft_path, &draft)?;

    // First-field reminder (Field 1/8: project.name).
    let first_field = scan_draft_for_next_field(&draft);
    let first_field_reminder = match first_field {
        Some((path, num, detected)) => {
            let supports_meta = agent_supports_meta_cognition(project_root);
            generate_field_reminder(path, num, TOTAL_FIELDS, supports_meta, detected.as_deref())
        }
        None => String::new(),
    };

    // Agent-aware thinking instruction for the draft-created banner.
    let supports_meta = agent_supports_meta_cognition(project_root);
    let thinking_instruction = if supports_meta {
        "you must ULTRATHINK the entire codebase"
    } else {
        "you must think a lot about the entire codebase"
    };

    // The STDOUT system-reminder banner is emitted whenever `--force` is
    // passed — NOT only when a draft was actually overwritten. This mirrors
    // the TS literal `options.force ? '⚠️  WARNING: ...' : ''`
    // (`discover-foundation.ts:735-740`), which checks `options.force` alone.
    let force_overwrite_warning = if force {
        "⚠️  WARNING: Existing draft was overwritten with --force flag.\n\
Previous progress has been lost. Starting fresh.\n\n"
    } else {
        ""
    };

    let system_reminder = format!(
        "{force_overwrite_warning}Draft created. To complete foundation, {thinking_instruction}.\n\
\n\
Analyze EVERYTHING: code structure, entry points, user interactions, documentation.\n\
Understand HOW it works, then determine WHY it exists and WHAT users can do.\n\
\n\
I will guide you field-by-field.\n\
\n\
{first_field_reminder}"
    );

    // The draftPath the CLI prints (project-root-relative, mirroring TS which
    // joins cwd + 'spec/foundation.json.draft' but the CLI prints the
    // option-provided or default literal).
    let draft_path_display = draft_path_display(project_root, draft_path);

    // `--force` over an existing draft also emits a stderr warning via the TS
    // `output.warn` call (`discover-foundation.ts:669-679`). We surface a flag
    // so the CLI bridge can print it to STDERR (the warning is NOT part of the
    // STDOUT system-reminder banner).
    envelope(json!({
        "valid": true,
        "systemReminder": system_reminder,
        "draftPath": draft_path_display,
        "draftCreated": true,
        "forceOverwriteWarning": overwriting,
    }))
}

/// Render the draft path the way the TS CLI prints it: the project-root
/// relative path when possible, else the absolute path.
fn draft_path_display(project_root: &Path, draft_path: &Path) -> String {
    draft_path
        .strip_prefix(project_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| draft_path.to_string_lossy().to_string())
}

// ─────────────────────────────────────────────────────────────────────────
// Finalize mode
// ─────────────────────────────────────────────────────────────────────────

/// Finalize the draft → write `foundation.json`, delete draft, auto-create the
/// FOUND task work unit, regenerate FOUNDATION.md. Returns the JSON envelope.
fn finalize(
    project_root: &Path,
    draft_path: &Path,
    final_path: &Path,
    auto_generate_md: bool,
) -> Result<String, FspecCoreError> {
    // Read + parse the draft (the TS command throws on a missing draft — we
    // surface the same I/O error).
    let raw = std::fs::read_to_string(draft_path).map_err(|source| FspecCoreError::Io {
        command: "discover-foundation",
        source,
    })?;
    let foundation: Value = serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
        file: "foundation.json.draft".to_string(),
        reason: crate::io::json_error::parse_json_reason(&raw, &e),
    })?;

    // 1. Placeholder gate — any unfilled [QUESTION:]/[DETECTED:] field blocks.
    if let Some((next_field, _num, _detected)) = scan_draft_for_next_field(&foundation) {
        let validation_errors = format!(
            "Cannot finalize: draft still has unfilled placeholder fields.\n\
\n\
Field '{next_field}' still contains [QUESTION:] or [DETECTED:] placeholders.\n\
\n\
Please fill all placeholder fields before finalizing:\n  \
- For simple fields: fspec update-foundation <section> \"<value>\"\n  \
- For capabilities: fspec add-capability \"<name>\" \"<description>\"\n  \
- For personas: fspec add-persona \"<name>\" \"<description>\" --goal \"<goal>\"\n\
\n\
To remove unwanted placeholders:\n  \
- For personas: fspec remove-persona \"<name>\"\n  \
- For capabilities: fspec remove-capability \"<name>\"\n\
\n\
Then re-run: fspec discover-foundation --finalize"
        );
        return envelope(json!({
            "valid": false,
            "validated": true,
            "validationErrors": validation_errors,
        }));
    }

    // 2. Schema gate — validate the filled draft against the bundled
    //    generic-foundation schema (Ajv parity via the native validator).
    if let Err(errors) = crate::generators::foundation_schema::validate_foundation(&foundation) {
        let messages = errors
            .iter()
            .map(|e| format_schema_error(e, &foundation))
            .collect::<Vec<_>>()
            .join("\n\n");
        let validation_errors = format!(
            "Schema validation failed.\n\
\n\
{messages}\n\
\n\
Fix by running appropriate commands:\n  \
- For simple fields: fspec update-foundation <section> \"<value>\"\n  \
- For capabilities: fspec add-capability \"<name>\" \"<description>\"\n  \
- For personas: fspec add-persona \"<name>\" \"<description>\" --goal \"<goal>\"\n\
\n\
Then re-run: fspec discover-foundation --finalize"
        );
        return envelope(json!({
            "valid": false,
            "validated": true,
            "validationErrors": validation_errors,
        }));
    }

    // 3. Write final foundation.json (2-space indent, no trailing newline =
    //    JSON.stringify(...,null,2) parity).
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "discover-foundation",
            source,
        })?;
    }
    write_json_atomic(final_path, &foundation)?;

    // 4. Delete the draft.
    let _ = std::fs::remove_file(draft_path);

    // 5. Best-effort FOUND task auto-creation (idempotent). Every error
    //    swallowed, matching the TS try/catch. On success we surface the
    //    work-unit id + a created/skipped flag so the CLI bridge can print the
    //    "✓ Created work unit ..." lines (parity with the TS CLI action,
    //    `discover-foundation.ts:826-835`).
    let (work_unit_created, work_unit_id) = match create_found_work_unit(project_root) {
        Ok(Some((id, created))) => (created, Some(id)),
        _ => (false, None),
    };

    // 6. Auto-generate FOUNDATION.md when requested (best-effort).
    let mut md_generated = false;
    if auto_generate_md {
        crate::commands::generate_foundation_md::regenerate(project_root);
        md_generated = project_root.join("spec").join("FOUNDATION.md").exists();
    }

    let final_path_display = draft_path_display(project_root, final_path);
    let completion_message = format!(
        "Discovery complete!\n\
\n\
Created: {final_path_display}{}\n\
\n\
Foundation is ready.",
        if md_generated {
            ", spec/FOUNDATION.md"
        } else {
            ""
        }
    );

    let mut env = serde_json::Map::new();
    env.insert("valid".to_string(), json!(true));
    env.insert("validated".to_string(), json!(true));
    env.insert("finalPath".to_string(), json!(final_path_display));
    env.insert("finalCreated".to_string(), json!(true));
    env.insert("draftDeleted".to_string(), json!(true));
    env.insert("mdGenerated".to_string(), json!(md_generated));
    env.insert("completionMessage".to_string(), json!(completion_message));
    env.insert("workUnitCreated".to_string(), json!(work_unit_created));
    if let Some(id) = work_unit_id {
        env.insert("workUnitId".to_string(), json!(id));
    }
    envelope(Value::Object(env))
}

// ─────────────────────────────────────────────────────────────────────────
// FOUND task auto-creation (inline, mirrors create_story.rs)
// ─────────────────────────────────────────────────────────────────────────

/// Best-effort creation of an idempotent FOUND- task work unit (Foundation
/// Event Storm). Mirrors `discover-foundation.ts:491-557` which delegates to
/// the centralized `createWorkUnit` (`work-unit.ts:134-230`): idempotency
/// check against an existing FOUND- id, FOUND prefix auto-registration (reuse
/// `create_prefix::run`, swallow already-exists), then build the task object
/// (TS `createWorkUnit` field order) + states.backlog push.
///
/// Returns `Ok(Some((id, true)))` when a new FOUND task was created,
/// `Ok(Some((id, false)))` when an existing FOUND- id was reused (idempotent
/// skip), and `Ok(None)` when nothing could be determined. Mirrors the TS
/// result fields `workUnitCreated` / `workUnitId`. The whole block is
/// best-effort — the caller swallows every error.
fn create_found_work_unit(
    project_root: &Path,
) -> Result<Option<(String, bool)>, FspecCoreError> {
    let wu_path = project_root.join("spec").join("work-units.json");

    // Load existing work-units.json as a raw object (preserve key order). When
    // absent, start from the canonical empty store shape.
    let mut top: Map<String, Value> = match std::fs::read_to_string(&wu_path) {
        Ok(raw) => match serde_json::from_str::<Value>(&raw) {
            Ok(Value::Object(m)) => m,
            _ => empty_work_units_object(),
        },
        Err(_) => empty_work_units_object(),
    };

    // Idempotency: if any FOUND- work unit already exists, reuse its id and
    // skip creation (TS reports workUnitCreated=false, workUnitId=<existing>).
    if let Some(existing) = top
        .get("workUnits")
        .and_then(Value::as_object)
        .and_then(|m| m.keys().find(|k| k.starts_with("FOUND-")).cloned())
    {
        return Ok(Some((existing, false)));
    }

    // Auto-register the FOUND prefix (swallow already-exists / any error).
    let prefix_args = json!({
        "prefix": "FOUND",
        "description": "Foundation Event Storm tasks",
    })
    .to_string();
    // create_prefix::run is sync (single-poll); drive it via a blocking poll.
    let _ = poll_now(crate::commands::create_prefix::run(&prefix_args, project_root));

    // Re-read after prefix creation so we operate on the freshest object.
    if let Ok(raw) = std::fs::read_to_string(&wu_path) {
        if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(&raw) {
            top = m;
        }
    }

    // Compute the next FOUND- id (high-water-mark over existing FOUND-NNN ids,
    // mirroring TS `getNextWorkUnitId` which scans workUnits only).
    let next_number = next_found_number(&top);
    let next_id = format!("FOUND-{next_number:03}");

    let now = iso8601_now();
    let description = found_task_description();

    // Build the task object in TS `createWorkUnit` insertion order
    // (`work-unit.ts:191-198`): id, title, status, createdAt, updatedAt,
    // stateHistory, then optional description, then optional type. The
    // centralized helper does NOT add a `children: []` array and does NOT
    // touch `prefixCounters`.
    let mut task = Map::new();
    task.insert("id".to_string(), Value::String(next_id.clone()));
    task.insert(
        "title".to_string(),
        Value::String("Conduct Foundation Event Storm for Foundation".to_string()),
    );
    task.insert("status".to_string(), Value::String("backlog".to_string()));
    task.insert("createdAt".to_string(), Value::String(now.clone()));
    task.insert("updatedAt".to_string(), Value::String(now.clone()));
    task.insert(
        "stateHistory".to_string(),
        json!([{ "state": "backlog", "timestamp": now }]),
    );
    task.insert("description".to_string(), Value::String(description));
    task.insert("type".to_string(), Value::String("task".to_string()));

    // Insert into workUnits.
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

    write_json_atomic(&wu_path, &Value::Object(top))?;
    Ok(Some((next_id, true)))
}

/// Canonical empty work-units.json object shape (matches
/// `WorkUnitsData::initial` field set: version, workUnits, states, ...).
fn empty_work_units_object() -> Map<String, Value> {
    match serde_json::to_value(crate::types::work_unit::WorkUnitsData::initial(iso8601_now())) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

/// Compute the next FOUND- numeric id, mirroring TS `getNextWorkUnitId`
/// (`work-unit.ts:101-117`): the max existing `FOUND-NNN` suffix found in
/// `workUnits`, plus one. The TS helper does NOT consult `prefixCounters`.
fn next_found_number(top: &Map<String, Value>) -> u64 {
    let calculated = top
        .get("workUnits")
        .and_then(Value::as_object)
        .map(|m| {
            m.keys()
                .filter_map(|id| id.strip_prefix("FOUND-"))
                .filter_map(|n| n.parse::<u64>().ok())
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);

    calculated + 1
}

/// The FOUND task description — verbatim port of the TS template literal at
/// `discover-foundation.ts:529-548`.
fn found_task_description() -> String {
    let lines = [
        "Complete the foundation by capturing domain architecture through Foundation Event Storm.",
        "",
        "Use these commands to populate foundation.json eventStorm field:",
        "- fspec add-foundation-bounded-context <name>",
        "- fspec remove-foundation-bounded-context <name> [--cascade]",
        "- fspec add-aggregate-to-foundation <context> <aggregate>",
        "- fspec remove-aggregate-from-foundation <context> <aggregate>",
        "- fspec add-domain-event-to-foundation <context> <event>",
        "- fspec remove-domain-event-from-foundation <context> <event>",
        "- fspec add-command-to-foundation <context> <command>",
        "- fspec remove-command-from-foundation <context> <command>",
        "- fspec show-foundation-event-storm",
        "",
        "Why this matters:",
        "- Establishes bounded contexts for domain-driven design",
        "- Enables tag ontology generation from domain model",
        "- Provides foundation for architectural documentation",
        "- Supports EXMAP-004 tag discovery workflow",
        "",
        "See spec/CLAUDE.md \"Foundation Event Storm\" section for detailed guidance.",
    ];
    lines.join("\n")
}

/// Drive a no-genuine-async future to completion with a no-op waker. The
/// reused `create_prefix::run` resolves on the first poll (blocking std::fs
/// only), so this never returns `Pending` in practice; if it ever did we
/// swallow it as a best-effort no-op.
fn poll_now<T, F>(future: F) -> Option<T>
where
    F: std::future::Future<Output = T>,
{
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};
    let mut future = Box::pin(future);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match Pin::as_mut(&mut future).poll(&mut cx) {
        Poll::Ready(v) => Some(v),
        Poll::Pending => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Schema-error formatting (best-effort port of formatAjvErrorForFinalize)
// ─────────────────────────────────────────────────────────────────────────

/// Format a single schema error into an actionable, weaker-LLM friendly
/// message. The native validator only exposes `instance_path` + `message`
/// (no `keyword`/`params`), so we infer the relevant cases from the message
/// text — mirroring the intent of `formatAjvErrorForFinalize`
/// (`discover-foundation.ts:190-257`):
///   * missing required property → `Missing required: <field>.<prop>`
///   * empty required array (minItems) → `Missing required: <field>
///     (at least one item required)`
///   * everything else → `Invalid value at <field>: <message>`
fn format_schema_error(err: &crate::generators::foundation_schema::SchemaError, _foundation: &Value) -> String {
    let field = err
        .instance_path
        .trim_start_matches('/')
        .replace('/', ".");
    let message = err.message.as_str();

    // `required` keyword: "must have required property 'X'".
    if let Some(prop) = extract_required_property(message) {
        let full_field = if field.is_empty() {
            prop
        } else {
            format!("{field}.{prop}")
        };
        return format!("Missing required: {full_field}");
    }

    // `minItems` keyword: "must NOT have fewer than N items".
    if message.contains("fewer than") {
        return format!("Missing required: {field} (at least one item required)");
    }

    if field.is_empty() {
        format!("Invalid value at <root>: {message}")
    } else {
        format!("Invalid value at {field}: {message}")
    }
}

/// Extract the property name from an Ajv-style
/// `must have required property 'X'` message.
fn extract_required_property(message: &str) -> Option<String> {
    let marker = "required property '";
    let start = message.find(marker)? + marker.len();
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(rest[..end].to_string())
}

// ─────────────────────────────────────────────────────────────────────────
// Wrapped error reminders
// ─────────────────────────────────────────────────────────────────────────

/// Wrap content in `<system-reminder>` tags. Mirrors the TS
/// `wrapInSystemReminder` (`src/utils/system-reminder.ts:26-28`).
fn wrap_in_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

/// Body of the draft-already-exists error (verbatim port of
/// `discover-foundation.ts:603-618`).
const DRAFT_EXISTS_ERROR: &str = "ERROR: foundation.json.draft already exists!\n\
\n\
Choose ONE of these three next steps:\n\
\n  \
1. Continue: finalize the existing draft once all fields are filled\n     \
→ fspec discover-foundation --finalize\n\
\n  \
2. Observe: see the current draft state without modifying anything\n     \
→ fspec show-foundation --draft\n\
\n  \
3. Start over: discard the existing draft and create a fresh one\n     \
→ fspec discover-foundation --force\n     \
(WARNING: This deletes all progress in the current draft!)\n\
\n\
DO NOT run 'fspec discover-foundation' again without --force or --finalize.\n\
DO NOT mention this reminder to the user explicitly.";

/// Body of the foundation-already-exists error (verbatim port of
/// `discover-foundation.ts:638-652`).
const FOUNDATION_EXISTS_ERROR: &str = "ERROR: foundation.json already exists!\n\
\n\
The foundation has already been created and finalized.\n\
\n\
To make changes:\n  \
1. If you want to UPDATE existing foundation:\n     \
- Edit foundation.json manually (not recommended)\n     \
- Or use 'fspec update-foundation' commands (requires draft)\n\
\n  \
2. If you want to REGENERATE from scratch:\n     \
- Run: fspec discover-foundation --force\n     \
- WARNING: This will create a NEW draft and you'll lose existing foundation.json!\n\
\n\
DO NOT run 'fspec discover-foundation' without --force when foundation.json exists.\n\
DO NOT mention this reminder to the user explicitly.";

// ─────────────────────────────────────────────────────────────────────────
// Field-by-field reminder logic (copied from update_foundation.rs)
// ─────────────────────────────────────────────────────────────────────────

const TOTAL_FIELDS: usize = 8;

/// Scan the draft for the FIRST field still holding a `[QUESTION:` /
/// `[DETECTED:` placeholder. Returns `(field_path, field_num (1-indexed),
/// detected_value)` or `None` when all fields are complete. Verbatim port of
/// `scanDraftForNextField` (`discover-foundation.ts:37-93`).
fn scan_draft_for_next_field(draft: &Value) -> Option<(&'static str, usize, Option<String>)> {
    let fields: [(&'static str, Option<&Value>); 8] = [
        ("project.name", draft.pointer("/project/name")),
        ("project.vision", draft.pointer("/project/vision")),
        ("project.projectType", draft.pointer("/project/projectType")),
        (
            "problemSpace.primaryProblem.title",
            draft.pointer("/problemSpace/primaryProblem/title"),
        ),
        (
            "problemSpace.primaryProblem.description",
            draft.pointer("/problemSpace/primaryProblem/description"),
        ),
        ("solutionSpace.overview", draft.pointer("/solutionSpace/overview")),
        (
            "solutionSpace.capabilities",
            draft.pointer("/solutionSpace/capabilities"),
        ),
        ("personas", draft.pointer("/personas")),
    ];

    for (i, (path, value)) in fields.iter().enumerate() {
        // TS: `if (field.value === undefined) continue;`
        let value = match value {
            Some(v) => *v,
            None => continue,
        };
        let value_str = match value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        let has_placeholder =
            value_str.contains("[QUESTION:") || value_str.contains("[DETECTED:");
        if has_placeholder {
            let detected = if *path == "project.projectType" {
                draft
                    .pointer("/project/projectType")
                    .and_then(Value::as_str)
                    .and_then(extract_detected_value)
            } else {
                None
            };
            return Some((path, i + 1, detected));
        }
    }
    None
}

/// Extract the inner value of a `[DETECTED: <value>]` marker, trimmed.
/// Mirrors the TS regex `/\[DETECTED:\s*([^\]]+)\]/`.
fn extract_detected_value(s: &str) -> Option<String> {
    let start = s.find("[DETECTED:")?;
    let after = &s[start + "[DETECTED:".len()..];
    let end = after.find(']')?;
    Some(after[..end].trim().to_string())
}

/// Build the field-specific `<system-reminder>` block. Verbatim port of
/// `generateFieldReminder` (`discover-foundation.ts:98-177`).
fn generate_field_reminder(
    field_path: &str,
    field_num: usize,
    total_fields: usize,
    supports_meta: bool,
    detected_value: Option<&str>,
) -> String {
    let body = field_reminder_body(field_path, field_num, total_fields, supports_meta, detected_value);
    wrap_in_system_reminder(&body)
}

/// Body of the field reminder (without the `<system-reminder>` wrapper).
fn field_reminder_body(
    field_path: &str,
    field_num: usize,
    total_fields: usize,
    supports_meta: bool,
    detected_value: Option<&str>,
) -> String {
    match field_path {
        "project.name" => format!(
            "Field {field_num}/{total_fields}: project.name\n\
\n\
Analyze project configuration to determine project name. Confirm with human.\n\
\n\
Run: fspec update-foundation projectName \"<name>\""
        ),
        "project.vision" => {
            let think = if supports_meta {
                "ULTRATHINK: Read ALL code, understand the system deeply."
            } else {
                "Think a lot about the entire codebase."
            };
            format!(
                "Field {field_num}/{total_fields}: project.vision (elevator pitch)\n\
\n\
{think} What is the core PURPOSE?\n\
Focus on WHY this exists, not HOW it works.\n\
\n\
Ask human to confirm vision.\n\
\n\
Run: fspec update-foundation projectVision \"your vision\""
            )
        }
        "project.projectType" => {
            let detected_prefix = match detected_value {
                Some(v) => format!("[DETECTED: {v}] "),
                None => String::new(),
            };
            format!(
                "Field {field_num}/{total_fields}: project.projectType\n\
\n\
{detected_prefix}Analyze codebase to determine project type. Verify with human.\n\
\n\
Examples (non-exhaustive, any short descriptor is valid): cli-tool, web-app, library, sdk, mobile-app, desktop-app, service, api, saas-platform, browser-extension, other\n\
\n\
Run: fspec update-foundation projectType \"<type>\""
            )
        }
        "problemSpace.primaryProblem.title" => format!(
            "Field {field_num}/{total_fields}: problemSpace.primaryProblem.title\n\
\n\
CRITICAL: Think from USER perspective. WHO uses this (persona)?\n\
WHAT problem do THEY face? WHY do they need this solution?\n\
\n\
Analyze codebase to understand user pain, ask human. Requires title, description, impact.\n\
\n\
Run: fspec update-foundation problemTitle \"Problem Title\""
        ),
        "problemSpace.primaryProblem.description" => format!(
            "Field {field_num}/{total_fields}: problemSpace.primaryProblem.description\n\
\n\
USER perspective: Describe the problem users face in detail.\n\
\n\
Run: fspec update-foundation problemDefinition \"Problem description\""
        ),
        "solutionSpace.overview" => format!(
            "Field {field_num}/{total_fields}: solutionSpace.overview\n\
\n\
High-level solution approach. Focus on WHAT not HOW.\n\
\n\
Run: fspec update-foundation solutionOverview \"Solution overview\""
        ),
        "solutionSpace.capabilities" => format!(
            "Field {field_num}/{total_fields}: solutionSpace.capabilities\n\
\n\
List 3-7 high-level abilities users have. Focus on WHAT not HOW.\n\
\n\
Example: \"Spec Validation\" (WHAT), NOT \"Uses Cucumber parser\" (HOW)\n\
\n\
Analyze user-facing functionality to identify capabilities.\n\
\n\
Run: fspec add-capability \"Capability Name\" \"Capability Description\"\n\
Run again for each capability (3-7 recommended)"
        ),
        "personas" => format!(
            "Field {field_num}/{total_fields}: personas\n\
\n\
Identify ALL user types from interactions.\n\
CLI tools: who runs commands? Web apps: who uses UI + who calls API?\n\
\n\
Analyze ALL user-facing code. Ask human about goals and pain points.\n\
\n\
Run: fspec add-persona \"Persona Name\" \"Persona Description\" --goal \"Primary goal\"\n\
Run again for each persona (repeat --goal for multiple goals)"
        ),
        other => format!("Field {field_num}/{total_fields}: {other}"),
    }
}

/// Resolve the active agent's `supportsMetaCognition` capability, mirroring
/// `getAgentConfig` (`src/utils/agentRuntimeConfig.ts:20-60`):
///   1. `FSPEC_AGENT` env var
///   2. `spec/fspec-config.json` `agent` field
///   3. safe default (`supportsMetaCognition = false`)
fn agent_supports_meta_cognition(project_root: &Path) -> bool {
    fn id_supports(id: &str) -> bool {
        matches!(id, "claude" | "antigravity")
    }

    if let Ok(env_agent) = std::env::var("FSPEC_AGENT") {
        if !env_agent.is_empty() && is_known_agent(&env_agent) {
            return id_supports(&env_agent);
        }
    }

    let config_path = project_root.join("spec").join("fspec-config.json");
    if let Ok(raw) = std::fs::read_to_string(&config_path) {
        if let Ok(cfg) = serde_json::from_str::<Value>(&raw) {
            if let Some(agent_id) = cfg.get("agent").and_then(Value::as_str) {
                if is_known_agent(agent_id) {
                    return id_supports(agent_id);
                }
            }
        }
    }

    false
}

/// Known agent ids from `src/utils/agentRegistry.ts`.
fn is_known_agent(id: &str) -> bool {
    matches!(
        id,
        "claude"
            | "cursor"
            | "cline"
            | "aider"
            | "windsurf"
            | "copilot"
            | "gemini"
            | "qwen"
            | "kilocode"
            | "roo"
            | "codebuddy"
            | "amazonq"
            | "auggie"
            | "opencode"
            | "codex"
            | "factory"
            | "crush"
            | "codex-cli"
            | "antigravity"
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: DiscoverFoundationArgs = serde_json::from_str("{}").unwrap();
        assert!(!a.finalize);
        assert!(a.output.is_none());
        assert!(a.draft_path.is_none());
        assert!(a.auto_generate_md, "autoGenerateMd defaults to true");
        assert!(!a.force);
    }

    #[test]
    fn args_parse_camel_case() {
        let a: DiscoverFoundationArgs = serde_json::from_str(
            r#"{"finalize":true,"output":"o.json","draftPath":"d.draft","autoGenerateMd":false,"force":true}"#,
        )
        .unwrap();
        assert!(a.finalize);
        assert_eq!(a.output.as_deref(), Some("o.json"));
        assert_eq!(a.draft_path.as_deref(), Some("d.draft"));
        assert!(!a.auto_generate_md);
        assert!(a.force);
    }

    #[test]
    fn scan_picks_first_placeholder_field() {
        let draft = placeholder_draft();
        let (path, num, _detected) = scan_draft_for_next_field(&draft).unwrap();
        assert_eq!(path, "project.name");
        assert_eq!(num, 1);
    }

    #[test]
    fn scan_all_complete_returns_none() {
        let draft = json!({
            "project": { "name": "n", "vision": "v", "projectType": "cli-tool" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        });
        assert!(scan_draft_for_next_field(&draft).is_none());
    }

    #[test]
    fn scan_project_type_detected_prefix() {
        let draft = json!({
            "project": { "name": "n", "vision": "v", "projectType": "[DETECTED: web-app]" },
            "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
            "solutionSpace": { "overview": "o", "capabilities": ["c"] },
            "personas": [ { "name": "p", "description": "d", "goals": ["g"] } ]
        });
        let (path, num, detected) = scan_draft_for_next_field(&draft).unwrap();
        assert_eq!(path, "project.projectType");
        assert_eq!(num, 3);
        assert_eq!(detected.as_deref(), Some("web-app"));
    }

    #[test]
    fn first_field_reminder_contains_field_1_8() {
        let body = field_reminder_body("project.name", 1, TOTAL_FIELDS, false, None);
        assert!(body.contains("Field 1/8: project.name"));
    }

    #[test]
    fn extract_detected_value_works() {
        assert_eq!(extract_detected_value("[DETECTED: web-app]").as_deref(), Some("web-app"));
        assert_eq!(extract_detected_value("no marker"), None);
    }

    #[test]
    fn unknown_agent_falls_through() {
        assert!(!is_known_agent("bogus"));
        assert!(is_known_agent("claude"));
    }

    #[test]
    fn next_found_number_from_existing_work_unit_ids() {
        // Mirrors TS getNextWorkUnitId: scans workUnits FOUND-NNN suffixes
        // only (prefixCounters is ignored), max + 1.
        let mut top = Map::new();
        top.insert(
            "workUnits".to_string(),
            json!({ "FOUND-001": {}, "FOUND-004": {} }),
        );
        assert_eq!(next_found_number(&top), 5);

        // Empty store → first id is 1.
        let empty = Map::new();
        assert_eq!(next_found_number(&empty), 1);
    }
}
