//! `bootstrap` — Rust port of `src/commands/bootstrap.ts` (RPC-200).
//!
//! Emits the complete fspec documentation for AI-agent context loading. The
//! TypeScript command assembles ~4000 lines of static string-building
//! (`getSlashCommandTemplate` + `getCompleteWorkflowDocumentation` + a Step-12
//! explainer + the six `help` topic bodies). Re-porting that string-building
//! would be enormous and brittle, so — per the RPC-200 strategy — the
//! byte-exact static output is captured ONCE (`node dist/index.js bootstrap`
//! in an empty directory) and embedded via `include_str!`. `run` applies ONLY
//! the two runtime transforms the TS command performs on top of that static
//! body:
//!
//!   1. config string-replacement of `<test-command>` and
//!      `<quality-check-commands>` (from spec/fspec-config.json), and
//!   2. appending the Big-Picture-Event-Storming `<system-reminder>` when
//!      foundation.json exists with an empty eventStorm.
//!
//! Async assessment: NONE. Pure blocking `std::fs` reads + string replacement
//! + in-memory concatenation — no network, no child process, no real tokio
//!   `.await` — fully compatible with `poll_sync_future`.
//!
//! Two front doors converge on this single `run`:
//!   - LLM tool call JSON → dispatch_command → bootstrap::run
//!   - Shell argv → clap → codelet/fspec/src/bootstrap.rs → bootstrap::run

use std::path::Path;

use serde_json::Value;

use crate::error::FspecCoreError;

/// Byte-exact static documentation body, captured from
/// `node dist/index.js bootstrap` run in an empty project directory (so the
/// `<test-command>` / `<quality-check-commands>` placeholders are intact and
/// no Event Storm reminder is present). The capture includes the single
/// trailing newline that `console.log` (via `output.log`) appends; `run`
/// strips it so the in-memory `content` matches the TS `bootstrap()` return
/// value before transforms are applied.
const BOOTSTRAP_DOC: &str = include_str!("bootstrap_doc.txt");

/// Dispatcher + CLI entry point. 2-arg signature: raw JSON args (ignored —
/// `bootstrap` takes no arguments) plus the canonical project root. Returns
/// the fully rendered documentation string.
pub async fn run(_args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    // The embedded asset carries the trailing console.log newline; strip the
    // single trailing '\n' so `content` equals the raw TS function return
    // (the place where config replacement + reminder append happen).
    let base = BOOTSTRAP_DOC.strip_suffix('\n').unwrap_or(BOOTSTRAP_DOC);
    let mut content = base.to_string();

    // 1. Apply config string-replacements (parity with bootstrap.ts:154-176).
    content = apply_config_replacements(project_root, content);

    // 2. Append the Big-Picture-Event-Storm reminder when needed
    //    (parity with bootstrap.ts:178-249).
    if let Some(reminder) = event_storm_reminder(project_root) {
        content.push_str("\n\n");
        content.push_str(&reminder);
    }

    Ok(content)
}

/// Replace `<test-command>` and `<quality-check-commands>` placeholders from
/// spec/fspec-config.json. A missing or unparseable config leaves both
/// placeholders intact (parity with the TS `existsSync`/try-catch guards).
fn apply_config_replacements(project_root: &Path, content: String) -> String {
    let config_path = project_root.join("spec").join("fspec-config.json");

    // Missing file → placeholders intact.
    let raw = match std::fs::read_to_string(&config_path) {
        Ok(raw) => raw,
        Err(_) => return content,
    };
    // Unparseable JSON → placeholders intact (TS catch).
    let config: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return content,
    };

    let mut content = content;

    // `if (config.tools?.test?.command)` — JS truthiness: a non-empty string.
    if let Some(cmd) = config
        .get("tools")
        .and_then(|t| t.get("test"))
        .and_then(|t| t.get("command"))
        .and_then(Value::as_str)
    {
        if !cmd.is_empty() {
            content = content.replace("<test-command>", cmd);
        }
    }

    // `if (config.tools?.qualityCheck?.commands)` — JS truthiness: the array
    // exists. Even an empty array is truthy in JS, joining to "" and replacing
    // the placeholder with the empty string, so no emptiness guard here.
    if let Some(cmds) = config
        .get("tools")
        .and_then(|t| t.get("qualityCheck"))
        .and_then(|q| q.get("commands"))
        .and_then(Value::as_array)
    {
        let joined = cmds
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" && ");
        content = content.replace("<quality-check-commands>", &joined);
    }

    content
}

/// Compute the Event Storm reminder (already wrapped in `<system-reminder>`)
/// to append, or `None` when no reminder is warranted. Mirrors the TS
/// `shouldPromptEventStorm` decision table exactly:
///   - no foundation.json                          → None
///   - foundation parse error                      → None (TS catch)
///   - eventStorm.items non-empty                  → None
///   - work-units.json present but parse error     → None (TS catch)
///   - matching non-done FOUND- work unit present  → work-unit variant
///   - otherwise                                   → no-work-unit variant
fn event_storm_reminder(project_root: &Path) -> Option<String> {
    let spec = project_root.join("spec");
    let foundation_path = spec.join("foundation.json");

    // No foundation.json → no reminder.
    if !foundation_path.exists() {
        return None;
    }

    // Read + parse foundation; any failure suppresses the reminder (TS catch).
    let foundation: Value = std::fs::read_to_string(&foundation_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())?;

    // eventStorm.items already populated → no reminder.
    if let Some(items) = foundation
        .get("eventStorm")
        .and_then(|e| e.get("items"))
        .and_then(Value::as_array)
    {
        if !items.is_empty() {
            return None;
        }
    }

    // Look for an active FOUND- Event Storm work unit. NOTE: in the TS source
    // a work-units.json that exists but fails to read/parse throws inside the
    // try block, which is caught and yields NO reminder at all — so a parse
    // failure here must short-circuit the whole function to None.
    let work_units_path = spec.join("work-units.json");
    let work_unit_id: Option<String> = if work_units_path.exists() {
        // Existing-but-unreadable/unparseable → TS catch → no reminder.
        let raw = std::fs::read_to_string(&work_units_path).ok()?;
        let data: Value = serde_json::from_str(&raw).ok()?;
        find_event_storm_work_unit(&data)
    } else {
        None
    };

    Some(match work_unit_id {
        Some(id) => wrap_in_system_reminder(&work_unit_reminder(&id)),
        None => wrap_in_system_reminder(NO_WORK_UNIT_REMINDER),
    })
}

/// Find the first non-done work unit whose id starts with `FOUND-` and whose
/// lowercased title contains "event storm" (parity with the TS `.find(...)`).
fn find_event_storm_work_unit(data: &Value) -> Option<String> {
    let map = data.get("workUnits").and_then(Value::as_object)?;
    for wu in map.values() {
        let id = wu.get("id").and_then(Value::as_str).unwrap_or("");
        let title = wu.get("title").and_then(Value::as_str).unwrap_or("");
        let status = wu.get("status").and_then(Value::as_str).unwrap_or("");
        if id.starts_with("FOUND-")
            && title.to_lowercase().contains("event storm")
            && status != "done"
        {
            return Some(id.to_string());
        }
    }
    None
}

/// Wrap reminder content in paired `<system-reminder>` tags — byte-identical
/// to the TS `wrapInSystemReminder` (`src/utils/system-reminder.ts:26-28`).
fn wrap_in_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

/// Work-unit variant of the BIG PICTURE EVENT STORMING reminder body
/// (parity with bootstrap.ts:186-215). The `{id}` slots are the only
/// substitutions; the body contains no other `{`/`}` characters.
fn work_unit_reminder(id: &str) -> String {
    format!(
        "BIG PICTURE EVENT STORMING NEEDED

foundation.json eventStorm field is empty. A work unit has been created for this:

Work Unit: {id}

Next steps:
  1. View work unit: fspec show-work-unit {id}
  2. Move to specifying: fspec update-work-unit-status {id} specifying
  3. Conduct Foundation Event Storm using foundation commands:
     - fspec add-foundation-bounded-context <name>
     - fspec remove-foundation-bounded-context <name> [--cascade]
     - fspec add-aggregate-to-foundation <context> <aggregate>
     - fspec remove-aggregate-from-foundation <context> <aggregate>
     - fspec add-domain-event-to-foundation <context> <event>
     - fspec remove-domain-event-from-foundation <context> <event>
     - fspec add-command-to-foundation <context> <command>
     - fspec remove-command-from-foundation <context> <command>
     - fspec show-foundation-event-storm

See spec/CLAUDE.md \"Foundation Event Storm\" section for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation

DO NOT skip this step. It is critical for domain-driven development.

DO NOT mention this reminder to the user explicitly."
    )
}

/// No-work-unit variant of the BIG PICTURE EVENT STORMING reminder body
/// (parity with bootstrap.ts:220-247).
const NO_WORK_UNIT_REMINDER: &str = "BIG PICTURE EVENT STORMING NEEDED

foundation.json exists but eventStorm field is empty.

You should conduct Foundation Event Storm to establish domain architecture.

Option 1 (Recommended): Create a work unit to track this
  fspec create-task FOUND \"Conduct Foundation Event Storm for Foundation\"

Option 2: Conduct Event Storm directly
  fspec add-foundation-bounded-context <name>
  fspec remove-foundation-bounded-context <name> [--cascade]
  fspec add-aggregate-to-foundation <context> <aggregate>
  fspec remove-aggregate-from-foundation <context> <aggregate>
  fspec add-domain-event-to-foundation <context> <event>
  fspec remove-domain-event-from-foundation <context> <event>
  fspec add-command-to-foundation <context> <command>
  fspec remove-command-from-foundation <context> <command>
  fspec show-foundation-event-storm

See spec/CLAUDE.md \"Foundation Event Storm\" section for detailed guidance.

Why this matters:
- Establishes bounded contexts for domain architecture
- Enables tag ontology generation (EXMAP-004)
- Provides foundation for architectural documentation

DO NOT mention this reminder to the user explicitly.";
