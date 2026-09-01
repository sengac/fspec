//! Foundation discovery shared guidance (DISC-003).
//!
//! Single source of truth for:
//! * the 8-field draft table (JSON path, update-foundation alias, 1-2 examples,
//!   reminder body),
//! * the full-field scan returning a status for EVERY draft field,
//! * the progress / trailer renderers,
//! * the unified field reminder (existing body + appended `Examples:` block),
//! * the agent meta-cognition resolver and the `[DETECTED:]` marker extractor.
//!
//! This module replaces the code that was previously duplicated verbatim in
//! `commands/update_foundation.rs` and `commands/discover_foundation.rs`
//! (work unit DISC-003, rules 0-2, 7).

use std::path::Path;

use serde_json::Value;

/// Per-field scan status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldStatus {
    /// Field present and filled with real content.
    Complete,
    /// Field present but still holds a `[QUESTION:` / `[DETECTED:` marker.
    Placeholder,
    /// JSON key absent from the document.
    Missing,
    /// Array field present but empty.
    EmptyArray,
    /// Array field present, non-empty, with at least one placeholder entry.
    PlaceholderEntries,
}

/// One row of the 8-field draft table plus its live status.
#[derive(Debug, Clone)]
pub struct FieldRow {
    pub path: &'static str,
    pub alias: &'static str,
    pub status: FieldStatus,
    /// Short preview of the current value (or marker kind for arrays).
    pub preview: String,
}

impl FieldRow {
    pub fn is_complete(&self) -> bool {
        self.status == FieldStatus::Complete
    }
    pub fn is_empty_array(&self) -> bool {
        self.status == FieldStatus::EmptyArray
    }
    pub fn has_placeholder_entries(&self) -> bool {
        self.status == FieldStatus::PlaceholderEntries
    }
    /// Human-readable status name (kebab-case, matches the json envelope).
    pub fn status_label(&self) -> &'static str {
        match self.status {
            FieldStatus::Complete => "complete",
            FieldStatus::Placeholder => "placeholder",
            FieldStatus::Missing => "missing",
            FieldStatus::EmptyArray => "empty-array",
            FieldStatus::PlaceholderEntries => "placeholder-entries",
        }
    }
}

/// Scan every draft field and return its status (rule 1).
///
/// Statuses: `Complete`, `Placeholder` (string value contains a marker),
/// `Missing` (key absent), `EmptyArray` (array `[]`), `PlaceholderEntries`
/// (array non-empty and ALL entries contain a marker). Array fields are
/// complete iff non-empty AND no placeholder entries.
pub fn scan_fields(draft: &Value) -> Vec<FieldRow> {
    let rows: [(Option<&Value>, &str, &str); 8] = [
        (
            draft.pointer("/project/name"),
            "project.name",
            "projectName",
        ),
        (
            draft.pointer("/project/vision"),
            "project.vision",
            "projectVision",
        ),
        (
            draft.pointer("/project/projectType"),
            "project.projectType",
            "projectType",
        ),
        (
            draft.pointer("/problemSpace/primaryProblem/title"),
            "problemSpace.primaryProblem.title",
            "problemTitle",
        ),
        (
            draft.pointer("/problemSpace/primaryProblem/description"),
            "problemSpace.primaryProblem.description",
            "problemDefinition",
        ),
        (
            draft.pointer("/solutionSpace/overview"),
            "solutionSpace.overview",
            "solutionOverview",
        ),
        (
            draft.pointer("/solutionSpace/capabilities"),
            "solutionSpace.capabilities",
            "capabilities",
        ),
        (draft.pointer("/personas"), "personas", "personas"),
    ];
    rows.into_iter()
        .map(|(value, path, alias)| FieldRow {
            path,
            alias,
            status: field_status(value),
            preview: field_preview(value),
        })
        .collect()
}

fn contains_marker(v: &Value) -> bool {
    match v {
        Value::String(s) => s.contains("[QUESTION:") || s.contains("[DETECTED:"),
        other => {
            let s = other.to_string();
            s.contains("[QUESTION:") || s.contains("[DETECTED:")
        }
    }
}

fn field_status(value: Option<&Value>) -> FieldStatus {
    let Some(value) = value else {
        return FieldStatus::Missing;
    };
    if let Some(arr) = value.as_array() {
        if arr.is_empty() {
            return FieldStatus::EmptyArray;
        }
        if arr.iter().any(contains_marker) {
            return FieldStatus::PlaceholderEntries;
        }
        return FieldStatus::Complete;
    }
    if contains_marker(value) {
        return FieldStatus::Placeholder;
    }
    FieldStatus::Complete
}

fn field_preview(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "missing".to_string();
    };
    if let Some(arr) = value.as_array() {
        return format!("[{} entries]", arr.len());
    }
    value.to_string()
}

/// Count of fields whose status is [`FieldStatus::Complete`].
pub fn progress(rows: &[FieldRow]) -> usize {
    rows.iter().filter(|r| r.is_complete()).count()
}

/// Total number of draft fields (the `/8` denominator).
pub const TOTAL_FIELDS: usize = 8;

/// Compact two-line status trailer for mutation success envelopes
/// (envelope field `nextSteps`, rule 4/5).
pub fn render_trailer(rows: &[FieldRow]) -> String {
    let complete = progress(rows);
    let remaining: Vec<&str> = rows
        .iter()
        .filter(|r| !r.is_complete())
        .map(|r| r.alias)
        .collect();

    if remaining.is_empty() {
        return format!(
            "progress: {complete}/{TOTAL_FIELDS} fields complete | remaining: none\n\
             next: fspec discover-foundation --finalize"
        );
    }

    // First incomplete field in table order is the next action.
    let Some(next) = rows.iter().find(|r| !r.is_complete()) else {
        return format!(
            "progress: {complete}/{TOTAL_FIELDS} fields complete | remaining: none\n\
             next: fspec discover-foundation --finalize"
        );
    };
    format!(
        "progress: {complete}/{TOTAL_FIELDS} fields complete | remaining: {}\n\
         next: {}",
        remaining.join(", "),
        fix_command(next)
    )
}

/// Draft-phase success trailer computed from the just-written foundation
/// document (used by add/remove-capability, add/remove-persona, and the
/// update-foundation draft path).
pub fn draft_trailer(foundation: &Value) -> String {
    render_trailer(&scan_fields(foundation))
}

/// Event-storm-phase success trailer (used by the six event-storm
/// add/remove commands). `context` names the bounded context the mutation
/// touched (or the first remaining one after a removal).
pub fn event_storm_trailer(foundation: &Value, context: &str) -> String {
    let items: Vec<&Value> = match foundation.pointer("/eventStorm/items") {
        Some(Value::Array(arr)) => arr.iter().collect(),
        _ => Vec::new(),
    };
    let live: Vec<&Value> = items
        .into_iter()
        .filter(|i| i.get("deleted").and_then(Value::as_bool) != Some(true))
        .collect();
    let count = |ty: &str| {
        live.iter()
            .filter(|i| i.get("type").and_then(Value::as_str) == Some(ty))
            .count()
    };
    let contexts = count("bounded_context");
    let aggregates = count("aggregate");
    let events = count("event");
    let commands = count("command");

    let next = if contexts == 0 {
        "fspec add-foundation-bounded-context \"<Context>\""
    } else if aggregates == 0 {
        &format!("fspec add-aggregate-to-foundation \"{context}\" \"<Aggregate>\"")
    } else if events == 0 {
        &format!("fspec add-domain-event-to-foundation \"{context}\" \"<Event>\"")
    } else if commands == 0 {
        &format!("fspec add-command-to-foundation \"{context}\" \"<Command>\"")
    } else {
        "Foundation event storm complete. Run: fspec generate-tags-md"
    };

    format!(
        "eventStorm: {contexts} contexts, {aggregates} aggregates, {events} events, {commands} commands\n\
         next: {next}"
    )
}

/// First non-deleted bounded context name in the foundation event storm
/// (or an empty string when none remain). Used after a bounded-context
/// removal to anchor the `next:` suggestion.
pub fn first_bounded_context(foundation: &Value) -> String {
    foundation
        .pointer("/eventStorm/items")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|i| {
                    i.get("type").and_then(Value::as_str) == Some("bounded_context")
                        && i.get("deleted").and_then(Value::as_bool) != Some(true)
                })
                .and_then(|i| i.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default()
}

/// Fix command for a field (alias-based; array fields use the add-* command).
pub fn fix_command(row: &FieldRow) -> String {
    match row.alias {
        "capabilities" => "fspec add-capability \"<name>\" \"<description>\"".to_string(),
        "personas" => {
            "fspec add-persona \"<name>\" \"<description>\" --goal \"<goal>\"".to_string()
        }
        alias => format!("fspec update-foundation {alias} \"<value>\""),
    }
}

/// Example snippets per field, appended after the existing reminder body
/// (rule 2: new content is appended, never rewords existing text).
pub fn examples_for(path: &str) -> String {
    match path {
        "project.name" => "\"fspec\" (the project's short identifier)".to_string(),
        "project.vision" => "\"A CLI tool that keeps Gherkin specs in sync with code\"".to_string(),
        "project.projectType" => {
            "\"cli-tool\", \"web-app\", \"library\" (any short descriptor)".to_string()
        }
        "problemSpace.primaryProblem.title" => "\"Spec drift\"".to_string(),
        "problemSpace.primaryProblem.description" => {
            "\"Developers manage Gherkin feature files by hand; specs drift out of sync with code\""
                .to_string()
        }
        "solutionSpace.overview" => {
            "\"A CLI tool agents use to manage Gherkin specs and work units\"".to_string()
        }
        "solutionSpace.capabilities" => {
            "\"Spec Validation\" (WHAT), NOT \"Uses Cucumber parser\" (HOW)".to_string()
        }
        "personas" => {
            "\"Developer\" — description \"Builds features\", goals [\"Ship quality code faster\"]"
                .to_string()
        }
        other => format!("(no canned example for {other})"),
    }
}

/// Build the field-specific reminder body (without the `<system-reminder>`
/// wrapper) and append the `Examples:` block. Verbatim port of
/// `generateFieldReminder` (`src/commands/discover-foundation.ts:98-176`)
/// with examples appended (DISC-003 rule 2).
pub fn field_reminder_body(
    field_path: &str,
    field_num: usize,
    total_fields: usize,
    supports_meta: bool,
    detected_value: Option<&str>,
) -> String {
    let base = match field_path {
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
    };

    // Append the examples block AFTER the existing text (rule 2).
    format!("{base}\n\nExamples:\n  {}", examples_for(field_path))
}

/// Scan the draft for the FIRST field still holding a `[QUESTION:` /
/// `[DETECTED:` placeholder and, when found, build the field-specific
/// `<system-reminder>`. Verbatim port of `scanDraftForNextField` +
/// `generateFieldReminder` (`src/commands/discover-foundation.ts:37-177`).
///
/// Returns `None` when all fields are complete (TS returns an empty
/// systemReminder which the CLI skips).
pub fn next_field_reminder(draft: &Value, project_root: &Path) -> Option<String> {
    let rows = scan_fields(draft);

    // The legacy scan only considered PRESENT fields whose serialized value
    // contained a marker (missing keys skipped, empty arrays skipped, arrays
    // with at least one placeholder entry included). Reproduce that exactly:
    // an incomplete row counts when it is a placeholder string or carries
    // placeholder entries.
    for (i, row) in rows.iter().enumerate() {
        if row.status == FieldStatus::Placeholder || row.has_placeholder_entries() {
            let detected_value = if row.path == "project.projectType" {
                draft
                    .pointer("/project/projectType")
                    .and_then(Value::as_str)
                    .and_then(extract_detected_value)
            } else {
                None
            };
            let supports_meta = agent_supports_meta_cognition(project_root);
            let body = field_reminder_body(
                row.path,
                i + 1,
                TOTAL_FIELDS,
                supports_meta,
                detected_value.as_deref(),
            );
            return Some(format!("<system-reminder>\n{body}\n</system-reminder>"));
        }
    }
    None
}

/// Extract the inner value of a `[DETECTED: <value>]` marker, trimmed.
/// Mirrors the TS regex `/\[DETECTED:\s*([^\]]+)\]/`.
pub fn extract_detected_value(s: &str) -> Option<String> {
    let start = s.find("[DETECTED:")?;
    let after = &s[start + "[DETECTED:".len()..];
    let end = after.find(']')?;
    Some(after[..end].trim().to_string())
}

/// Resolve the active agent's `supportsMetaCognition` capability, mirroring
/// `getAgentConfig` (`src/utils/agentRuntimeConfig.ts:20-60`):
///   1. `FSPEC_AGENT` env var
///   2. `spec/fspec-config.json` `agent` field
///   3. safe default (`supportsMetaCognition = false`)
///
/// Only `claude` and `antigravity` enable meta-cognition in the agent
/// registry (`src/utils/agentRegistry.ts`).
pub fn agent_supports_meta_cognition(project_root: &Path) -> bool {
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

/// Known agent ids from `src/utils/agentRegistry.ts`. An id not in this set
/// is treated as unrecognised (`getAgentById` returns undefined), so the
/// resolver falls through to the next priority — matching the TS behaviour.
pub fn is_known_agent(id: &str) -> bool {
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
