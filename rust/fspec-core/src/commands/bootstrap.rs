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
//!   - Shell argv → clap → rust/fspec/src/bootstrap.rs → bootstrap::run

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

    // CLI-015: fill the two mode-aware AST-research placeholders before the
    // config replacements (both use `replace`, so order does not matter).
    let in_capture = crate::utils::mode::in_capture_mode();
    content = content.replace("__AST_RESEARCH_BLOCK__", ast_research_block(in_capture));
    content = content
        .replace("__AST_RESEARCH_NOTES_BLOCK__", ast_research_notes(in_capture));

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

/// CLI-015: the Research-First Workflow block of the embedded bootstrap doc,
/// rendered per mode — harness (capture) mode names the native AstGrep tool,
/// CLI mode names the `fspec astgrep` subcommand.
fn ast_research_block(in_capture: bool) -> &'static str {
    if in_capture {
        "**CRITICAL**: Before deciding whether to use Feature Event Storm, FIRST research the codebase using the AstGrep tool to understand the domain structure.\n\n```\n# Step 1: Research relevant code using the AstGrep tool\n# Find all functions in the domain area:\nAstGrep(language=\"typescript\", pattern=\"function $NAME($$$ARGS) { $$$BODY }\", path=\"src/auth/\")\n\n# Find classes to understand domain entities:\nAstGrep(language=\"typescript\", pattern=\"class $NAME { $$$FIELDS }\", path=\"src/auth/\")\n\n# Find interfaces to understand data structures:\nAstGrep(language=\"typescript\", pattern=\"interface $NAME { $$$FIELDS }\", path=\"src/auth/\")\n\n# Find async functions (often indicate external integrations or events):\nAstGrep(language=\"typescript\", pattern=\"async function $NAME($$$ARGS) { $$$BODY }\", path=\"src/auth/\")\n\n# Step 2: Analyze findings to understand domain\n# - What domain events exist in the code?\n# - What commands trigger those events?\n# - What business rules/policies are present?\n\n# Step 3: If uncertain after research, ASK USER\n# Share your findings and let the user decide:\n# \"I found 3 domain events: UserRegistered, LoginAttempted, SessionExpired.\n#  Should we do Feature Event Storm to map the full authentication flow?\"\n\n# Step 4: Proceed with chosen approach\n# - Feature Event Storm (if complex/unfamiliar)\n# - Example Mapping (if simple/clear)\n```"
    } else {
        "**CRITICAL**: Before deciding whether to use Feature Event Storm, FIRST research the codebase using the `fspec astgrep` command to understand the domain structure.\n\n```\n# Step 1: Research relevant code using fspec astgrep\n# Find all functions in the domain area:\nfspec astgrep --pattern 'function $NAME($$$ARGS) { $$$BODY }' --lang typescript --path src/auth/\n\n# Find classes to understand domain entities:\nfspec astgrep --pattern 'class $NAME { $$$FIELDS }' --lang typescript --path src/auth/\n\n# Find interfaces to understand data structures:\nfspec astgrep --pattern 'interface $NAME { $$$FIELDS }' --lang typescript --path src/auth/\n\n# Find async functions (often indicate external integrations or events):\nfspec astgrep --pattern 'async function $NAME($$$ARGS) { $$$BODY }' --lang typescript --path src/auth/\n\n# Step 2: Analyze findings to understand domain\n# - What domain events exist in the code?\n# - What commands trigger those events?\n# - What business rules/policies are present?\n\n# Step 3: If uncertain after research, ASK USER\n# Share your findings and let the user decide:\n# \"I found 3 domain events: UserRegistered, LoginAttempted, SessionExpired.\n#  Should we do Feature Event Storm to map the full authentication flow?\"\n\n# Step 4: Proceed with chosen approach\n# - Feature Event Storm (if complex/unfamiliar)\n# - Example Mapping (if simple/clear)\n```"
    }
}

/// CLI-015: the Notes-line variant for the Research Tools section of the
/// embedded bootstrap doc, rendered per mode.
fn ast_research_notes(in_capture: bool) -> &'static str {
    if in_capture {
        "    - For AST code search and refactoring, use the AstGrep / AstGrepRefactor tools"
    } else {
        "    - For AST code search and refactoring, use the `fspec astgrep` command"
    }
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::{ast_research_block, ast_research_notes, run};

    // Feature: spec/features/specifying-to-testing-error-directs-per-mode.feature (CLI-015 bootstrap doc mode-awareness)

    #[test]
    fn ast_research_block_cli_names_fspec_astgrep() {
        // @step Given the CLI (non-capture) execution context
        // @step Then the block names `fspec astgrep` and never the AstGrep tool call
        let block = ast_research_block(false);
        assert!(block.contains("fspec astgrep --pattern"));
        assert!(!block.contains("AstGrep(language="));
    }

    #[test]
    fn ast_research_block_capture_names_astgrep_tool() {
        // @step Given the capture-mode (harness) execution context
        // @step Then the block names the AstGrep tool and never `fspec astgrep`
        let block = ast_research_block(true);
        assert!(block.contains("AstGrep(language="));
        assert!(!block.contains("fspec astgrep"));
    }

    #[test]
    fn ast_research_notes_variants_are_mode_specific() {
        // @step Given both capture and CLI execution contexts
        // @step Then each note names only its own tool
        assert!(ast_research_notes(false).contains("`fspec astgrep`"));
        assert!(!ast_research_notes(false).contains("AstGrep / AstGrepRefactor"));
        assert!(ast_research_notes(true).contains("AstGrep / AstGrepRefactor"));
        assert!(!ast_research_notes(true).contains("`fspec astgrep`"));
    }

    #[test]
    fn run_replaces_placeholders_in_cli_mode() {
        // @step Given FSPEC_CAPTURE_MODE is not "1" (CLI default)
        // @step When I run bootstrap in an empty project directory
        let tmp = tempfile::TempDir::new().unwrap();
        if crate::utils::mode::in_capture_mode() {
            return; // environment explicitly opts in; nothing to assert
        }
        let out = single_poll(run("{}", tmp.path()));
        let doc = out.expect("bootstrap run succeeds");
        assert!(!doc.contains("__AST_RESEARCH_BLOCK__"));
        assert!(!doc.contains("__AST_RESEARCH_NOTES_BLOCK__"));
        assert!(doc.contains("fspec astgrep --pattern"));
        assert!(!doc.contains("AstGrep(language="));
    }

    /// Minimal single-poll driver mirroring the dispatcher's
    /// `poll_sync_future` (bootstrap::run never genuinely awaits).
    fn single_poll(
        fut: impl std::future::Future<Output = Result<String, crate::error::FspecCoreError>>,
    ) -> Result<String, crate::error::FspecCoreError> {
        use std::pin::pin;
        use std::task::{Context, Poll, Waker};
        let mut fut = pin!(fut);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("bootstrap::run unexpectedly pending on first poll"),
        }
    }
}
