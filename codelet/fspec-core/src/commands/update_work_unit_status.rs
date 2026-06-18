//! `update-work-unit-status` — Rust port of `src/commands/update-work-unit-status.ts`.
//!
//! Enforces the ACDD lifecycle. Runs under `poll_sync_future` (the future is
//! polled exactly once), so ALL I/O uses BLOCKING std APIs.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitStatus;

mod reminders;
mod step_docstrings;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateWorkUnitStatusArgs {
    work_unit_id: String,
    status: String,
    #[serde(default)]
    blocked_reason: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    skip_temporal_validation: Option<bool>,
}

const VALID_STATES: &[&str] = &[
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

fn allowed_transitions(from: &str) -> &'static [&'static str] {
    // Mirrors STATE_TRANSITIONS in src/commands/update-work-unit-status.ts.
    match from {
        "backlog" => &["specifying", "blocked"],
        "specifying" => &["testing", "blocked"],
        "testing" => &["implementing", "specifying", "blocked"],
        "implementing" => &["validating", "testing", "specifying", "blocked"],
        "validating" => &["done", "implementing", "testing", "specifying", "blocked"],
        "done" => &["specifying", "testing", "implementing", "validating", "blocked"],
        "blocked" => &["backlog", "specifying", "testing", "implementing", "validating"],
        _ => &[],
    }
}

fn status_from_str(s: &str) -> Option<WorkUnitStatus> {
    match s {
        "backlog" => Some(WorkUnitStatus::Backlog),
        "specifying" => Some(WorkUnitStatus::Specifying),
        "testing" => Some(WorkUnitStatus::Testing),
        "implementing" => Some(WorkUnitStatus::Implementing),
        "validating" => Some(WorkUnitStatus::Validating),
        "done" => Some(WorkUnitStatus::Done),
        "blocked" => Some(WorkUnitStatus::Blocked),
        _ => None,
    }
}

/// Plain message error that is NOT wrapped by the args-validation prefix.
fn msg(reason: impl Into<String>) -> FspecCoreError {
    FspecCoreError::Message(reason.into())
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateWorkUnitStatusArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-work-unit-status",
            reason: format!("failed to parse args: {e}"),
        })?;

    let id = args.work_unit_id;
    let new_status = args.status;
    let blocked_reason = args.blocked_reason;
    let reason = args.reason;
    let skip_temporal = args.skip_temporal_validation.unwrap_or(false);

    // Validate state is allowed.
    if !VALID_STATES.contains(&new_status.as_str()) {
        return Err(msg(format!(
            "Invalid status value: {}. Allowed values: {}",
            new_status,
            VALID_STATES.join(", ")
        )));
    }

    let mut data = ensure_work_units_file(project_root)?;

    // Check if work unit exists.
    if !data.work_units.contains_key(&id) {
        return Err(msg(format!("Work unit '{id}' does not exist")));
    }

    let (current_status, work_type) = {
        let wu = &data.work_units[&id];
        (
            wu.status.as_str().to_string(),
            wu.r#type.clone().unwrap_or_else(|| "story".to_string()),
        )
    };

    let is_task = work_type == "task";

    // GATE: tasks have no testing phase.
    if is_task && new_status == "testing" {
        return Err(msg(
            "Tasks do not have a testing phase. task workflow: backlog → specifying → implementing → validating → done.\n\
Tasks are for operational work without testable acceptance criteria.\n\
Use stories for user-facing features that require tests.",
        ));
    }

    // GATE: prevent moving back to backlog.
    if new_status == "backlog" && current_status != "backlog" {
        return Err(msg(
            "Cannot move work back to backlog. Use 'blocked' state if work cannot progress.",
        ));
    }

    // GATE: blocked state requires a reason.
    if new_status == "blocked" && blocked_reason.as_deref().unwrap_or("").is_empty() {
        return Err(msg(
            "Blocked reason is required when moving to blocked state. Use --blocked-reason='description of blocker'",
        ));
    }

    // GATE: validate state transitions (ACDD enforcement with type-specific rules).
    if current_status != new_status {
        // Special case for tasks: allow specifying → implementing (skip testing).
        let is_task_skipping_test =
            is_task && current_status == "specifying" && new_status == "implementing";
        if !is_task_skipping_test
            && !allowed_transitions(&current_status).contains(&new_status.as_str())
        {
            let mut parts = vec![format!(
                "Invalid state transition from '{current_status}' to '{new_status}'."
            )];
            if current_status == "backlog" && new_status == "testing" {
                parts.push("Must move to 'specifying' state first.".to_string());
                parts.push("ACDD requires specification before testing.".to_string());
            } else if current_status == "specifying" && new_status == "implementing" && !is_task {
                parts.push("Must move to 'testing' state first.".to_string());
                parts.push("ACDD requires tests before implementation.".to_string());
                parts.push(
                    "Note: Only tasks can skip testing. Use --type=task for operational work."
                        .to_string(),
                );
            }
            return Err(msg(parts.join(" ")));
        }
    }

    // GATE: prevent starting work blocked by incomplete dependencies.
    let active_states = ["specifying", "testing", "implementing", "validating"];
    if new_status != "blocked" && active_states.contains(&new_status.as_str()) {
        let blockers = collect_active_blockers(&data, &id);
        if !blockers.is_empty() {
            return Err(msg(format!(
                "Cannot start work on {id}: work unit is blocked by incomplete dependencies.\n\n\
Active blockers:\n  - {}\n\n\
Complete blocking work units or remove dependencies before starting work.",
                blockers.join("\n  - ")
            )));
        }
    }

    // GATE: prefill in linked feature files (blocks ALL forward transitions except to blocked).
    if new_status != "blocked" && current_status != new_status {
        check_prefill(project_root, &id)?;
    }

    // Warnings accumulated for the success output.
    let mut warnings: Vec<String> = Vec::new();

    // REMIND-014: subjective review reminder, captured during the
    // specifying→testing review validation and surfaced after consolidation.
    let mut review_reminder: Option<String> = None;

    // GATE: prerequisites for testing state (only when leaving specifying).
    if new_status == "testing" && current_status == "specifying" {
        // Review validation (Example Mapping + architectural notes + AST research).
        review_reminder = perform_review_validation(&data, &id, &work_type)?;

        // Bugs must link to an existing feature file.
        if work_type == "bug" {
            let has_linked = data
                .work_units
                .get(&id)
                .and_then(|wu| wu.extra.get("linkedFeatures"))
                .and_then(Value::as_array)
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            let has_scenarios = !linked_feature_files(project_root, &id).is_empty();
            if !has_linked && !has_scenarios {
                return Err(msg(format!(
                    "Bugs must link to existing feature file before moving to testing.\n\n\
Use: fspec link-feature {id} <feature-name>\n\
Or tag scenarios in feature files with @{id}\n\n\
If the feature has no spec, create a story instead of a bug."
                )));
            }
        }

        // Unanswered questions block the transition.
        check_unanswered_questions(&data, &id, &current_status, &new_status)?;

        // Warn if no examples captured.
        if !has_examples(&data, &id) {
            warnings.push(
                "No examples captured in Example Mapping. Consider adding examples with 'fspec add-example' before testing."
                    .to_string(),
            );
        }

        // Scenarios must exist (skip for parent work units with children).
        if !is_parent_work_unit(&data, &id) {
            check_scenarios_exist(project_root, &id)?;
        }

        // Temporal validation: feature file created after entering specifying.
        if !skip_temporal {
            check_temporal_ordering(project_root, &data, &id, &new_status)?;
        }

        // Warn if no estimate.
        if !has_estimate(&data, &id) {
            warnings.push(
                "No estimate assigned. Consider adding estimate with --estimate=<points>"
                    .to_string(),
            );
        }

        // Warn about soft dependencies (dependsOn) that aren't done.
        if let Some(w) = soft_dependency_warning(&data, &id) {
            warnings.push(w);
        }
    }

    // GATE: temporal validation for implementing state (tests created after entering testing).
    if new_status == "implementing"
        && current_status == "testing"
        && !skip_temporal
        && !is_task
    {
        check_temporal_ordering(project_root, &data, &id, &new_status)?;
    }

    // GATE (BUG-061/BUG-093): step-docstring validation for implementing and
    // validating transitions. Tasks are exempt. Ensures test files linked via
    // coverage carry a complete set of @step comments.
    if (new_status == "implementing" || new_status == "validating") && !is_task {
        step_docstrings::validate_test_step_docstrings(project_root, &id, &work_type)?;
    }

    // GATE: coverage completeness for the implementing→validating transition
    // (implementation mappings required). Tasks are exempt. The optional
    // "coverage tracking is optional" warning is discarded on this path,
    // mirroring the TS caller which only inspects `complete`.
    if new_status == "validating" && !is_task {
        check_coverage_completeness(project_root, &data, &id, true)?;
    }

    // GATE: done finalization — parent/child constraints first, then coverage.
    if new_status == "done" {
        let incomplete = incomplete_children(&data, &id);
        if !incomplete.is_empty() {
            return Err(msg(format!(
                "Cannot mark parent as done while children are incomplete: {}. Complete all children first.",
                incomplete.join(", ")
            )));
        }
        if let Some(warning) = check_coverage_completeness(project_root, &data, &id, false)? {
            warnings.push(warning);
        }
    }

    // GATE 8: pre-transition blocking virtual hooks.
    run_pre_hooks(project_root, &data, &id, &new_status)?;

    // GATE 9: auto-checkpoint before transition (dirty & not →backlog & not from backlog).
    if current_status != "backlog" && new_status != "backlog" {
        let checkpoint_name = format!("{id}-auto-{current_status}");
        maybe_auto_checkpoint(project_root, &id, &checkpoint_name)?;
    }

    // Apply the status change.
    let now = iso8601_now();
    {
        let wu = data
            .work_units
            .get_mut(&id)
            .ok_or_else(|| msg(format!("Work unit {id} does not exist")))?;

        if let Some(parsed) = status_from_str(&new_status) {
            wu.status = parsed;
        }
        wu.updated_at = now.clone();

        if new_status == "blocked" {
            if let Some(reason) = &blocked_reason {
                wu.extra.insert("blockedReason".to_string(), json!(reason));
            }
        } else {
            wu.extra.remove("blockedReason");
        }

        let mut entry = serde_json::Map::new();
        entry.insert("state".to_string(), Value::String(new_status.clone()));
        entry.insert("timestamp".to_string(), Value::String(now.clone()));
        // `reason` mirrors TS: options.reason wins unless moving to blocked,
        // in which case the blockedReason is recorded as the reason.
        if let Some(r) = &reason {
            entry.insert("reason".to_string(), Value::String(r.clone()));
        }
        if new_status == "blocked" {
            if let Some(r) = &blocked_reason {
                entry.insert("reason".to_string(), Value::String(r.clone()));
            }
        }
        let entry = Value::Object(entry);
        let history = wu
            .extra
            .entry("stateHistory".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !history.is_array() {
            *history = Value::Array(Vec::new());
        }
        if let Some(arr) = history.as_array_mut() {
            arr.push(entry);
        }
    }

    // Move the id between state arrays.
    move_state(&mut data, &id, &current_status, &new_status);

    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    // GATE 10: done finalization — inline compaction.
    if new_status == "done" {
        compact_done(&mut data, &id);
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    // Cleanup auto-checkpoints on →done, preserving manual ones.
    if new_status == "done" {
        cleanup_done_checkpoints(project_root, &id);
    }

    // Collect system reminders in TS order, then consolidate into one block.
    let mut reminder_parts: Vec<String> = Vec::new();
    if let Some(r) = reminders::status_change_reminder(&id, &new_status, &data, project_root) {
        reminder_parts.push(r);
    }
    if new_status == "specifying" {
        reminder_parts.push(reminders::event_storm_reminder(&id));
    }
    if current_status == "specifying" && new_status == "testing" {
        if let Some(r) = reminders::virtual_hooks_reminder(&id) {
            reminder_parts.push(r);
        }
    }
    if let Some(r) = review_reminder {
        reminder_parts.push(r);
    }
    if new_status == "done" {
        let hook_count = array_len(&data, &id, "virtualHooks");
        if let Some(r) = reminders::virtual_hooks_cleanup_reminder(&id, hook_count) {
            reminder_parts.push(r);
        }
        if let Some(r) = reminders::done_review_reminder(&id, &work_type) {
            reminder_parts.push(r);
        }
    }
    let consolidated = reminders::consolidate(&reminder_parts);

    // Build stdout to mirror the TS CLI exactly:
    //   1. (validating only) configure-tools test + quality check messages
    //   2. `✓ ...` confirmation line
    //   3. one `⚠ <warning>` line each
    //   4. blank line + consolidated `<system-reminder>` block
    let mut out = String::new();
    if new_status == "validating" {
        out.push_str(&reminders::check_test_command(project_root));
        out.push('\n');
        out.push_str(&reminders::check_quality_commands(project_root));
        out.push('\n');
    }
    out.push_str(&format!("✓ Work unit {id} status updated to {new_status}\n"));
    for w in &warnings {
        out.push_str(&format!("⚠ {w}\n"));
    }
    if let Some(reminder) = consolidated {
        out.push('\n');
        out.push_str(&reminder);
        out.push('\n');
    }
    Ok(out)
}

// --------------------------------------------------------------------------
// Work-unit field accessors (typed fields live in `extra`)
// --------------------------------------------------------------------------

type Data = crate::types::work_unit::WorkUnitsData;

/// Length of an array-valued `extra` field (mirrors `(wu.field || []).length`).
fn array_len(data: &Data, id: &str, key: &str) -> usize {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get(key))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

/// String items of an array-valued `extra` field.
fn string_array<'a>(data: &'a Data, id: &str, key: &str) -> Vec<&'a str> {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get(key))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn has_examples(data: &Data, id: &str) -> bool {
    array_len(data, id, "examples") > 0
}

fn has_estimate(data: &Data, id: &str) -> bool {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get("estimate"))
        .map(|v| !v.is_null())
        .unwrap_or(false)
}

fn is_parent_work_unit(data: &Data, id: &str) -> bool {
    array_len(data, id, "children") > 0
}

/// Active blockers: `blockedBy` entries whose target work unit is not `done`.
/// Each formatted as `ID (status: S)` to mirror the TS message.
fn collect_active_blockers(data: &Data, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    for blocker_id in string_array(data, id, "blockedBy") {
        if let Some(blocker) = data.work_units.get(blocker_id) {
            let status = blocker.status.as_str();
            if status != "done" {
                out.push(format!("{blocker_id} (status: {status})"));
            }
        }
    }
    out
}

/// Incomplete children: `children` entries whose work unit is not `done`.
fn incomplete_children(data: &Data, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    for child_id in string_array(data, id, "children") {
        if let Some(child) = data.work_units.get(child_id) {
            let status = child.status.as_str();
            if status != "done" {
                out.push(format!("{child_id} (status: {status})"));
            }
        }
    }
    out
}

/// Soft-dependency warning for `dependsOn` entries that are not `done`.
fn soft_dependency_warning(data: &Data, id: &str) -> Option<String> {
    let incomplete: Vec<String> = string_array(data, id, "dependsOn")
        .into_iter()
        .filter_map(|dep_id| {
            data.work_units.get(dep_id).and_then(|dep| {
                let status = dep.status.as_str();
                (status != "done").then(|| format!("{dep_id} (status: {status})"))
            })
        })
        .collect();
    if incomplete.is_empty() {
        return None;
    }
    Some(format!(
        "Work unit has soft dependencies that are not complete: {}. Consider completing dependencies first for better workflow.",
        incomplete.join(", ")
    ))
}

/// Block the transition if any non-deleted, non-selected questions remain.
fn check_unanswered_questions(
    data: &Data,
    id: &str,
    current: &str,
    new: &str,
) -> Result<(), FspecCoreError> {
    let Some(wu) = data.work_units.get(id) else {
        return Ok(());
    };
    let Some(questions) = wu.extra.get("questions").and_then(Value::as_array) else {
        return Ok(());
    };
    let mut lines = Vec::new();
    for (idx, q) in questions.iter().enumerate() {
        let deleted = q.get("deleted").and_then(Value::as_bool).unwrap_or(false);
        let selected = q.get("selected").and_then(Value::as_bool).unwrap_or(false);
        if !deleted && !selected {
            let text = q.get("text").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("  - [{idx}] {text}"));
        }
    }
    if !lines.is_empty() {
        return Err(msg(format!(
            "Unanswered questions prevent state transition from '{current}' to '{new}':\n{}\n\nAnswer questions with 'fspec answer-question {id} <index>' before moving to testing.",
            lines.join("\n")
        )));
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Feature-file discovery + review validation
// --------------------------------------------------------------------------

/// Read every `.feature` file under `spec/features` tagged with `@<id>`.
/// Returns `(path, contents)` pairs.
fn linked_feature_files(project_root: &Path, id: &str) -> Vec<(std::path::PathBuf, String)> {
    let dir = project_root.join("spec").join("features");
    let mut out = Vec::new();
    let tag = format!("@{id}");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("feature") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.contains(&tag) {
                    out.push((path, content));
                }
            }
        }
    }
    out
}

/// A detected prefill placeholder: 1-based line number + placeholder name.
struct PrefillMatch {
    line: usize,
    pattern: String,
}

/// Line-by-line prefill detection mirroring `detectPrefill` in
/// `src/utils/prefill-detection.ts` (case-insensitive substring patterns plus
/// the `TODO:` marker and the `@component` / `@feature-group` tag placeholders).
fn detect_prefill(content: &str) -> Vec<PrefillMatch> {
    const SUBSTRING_PATTERNS: &[&str] = &[
        "[role]",
        "[action]",
        "[benefit]",
        "[precondition]",
        "[expected outcome]",
        "[scenario name]",
        "TODO:",
    ];
    let mut matches = Vec::new();
    // Preserve TS ordering: all matches for one pattern before the next.
    for pat in SUBSTRING_PATTERNS {
        let needle = pat.to_lowercase();
        for (i, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                matches.push(PrefillMatch {
                    line: i + 1,
                    pattern: (*pat).to_string(),
                });
            }
        }
    }
    for (name, needle) in [("@component", "@component"), ("@feature-group", "@feature-group")] {
        for (i, line) in content.lines().enumerate() {
            if line.starts_with('@') && tag_placeholder_present(line, needle) {
                matches.push(PrefillMatch {
                    line: i + 1,
                    pattern: name.to_string(),
                });
            }
        }
    }
    matches
}

/// Whether `line` contains `needle` not immediately followed by a word
/// character (mirrors the `(?!\w)` lookahead in the TS tag regexes).
fn tag_placeholder_present(line: &str, needle: &str) -> bool {
    if let Some(pos) = line.find(needle) {
        let after = &line[pos + needle.len()..];
        match after.chars().next() {
            Some(c) => !(c.is_alphanumeric() || c == '_'),
            None => true,
        }
    } else {
        false
    }
}

/// Prefill gate: blocks ALL forward transitions when the `@id`-tagged feature
/// file contains prefill placeholders. Mirrors `checkWorkUnitFeatureForPrefill`
/// + the inline message construction in the TS command.
fn check_prefill(project_root: &Path, id: &str) -> Result<(), FspecCoreError> {
    let features = linked_feature_files(project_root, id);
    let Some((_, content)) = features.first() else {
        return Ok(());
    };
    let matches = detect_prefill(content);
    if matches.is_empty() {
        return Ok(());
    }
    let detail: String = matches
        .iter()
        .take(3)
        .map(|m| format!("  Line {}: {}", m.line, m.pattern))
        .collect::<Vec<_>>()
        .join("\n");
    let tail = if matches.len() > 3 {
        format!("  ... and {} more\n\n", matches.len() - 3)
    } else {
        "\n".to_string()
    };
    Err(msg(format!(
        r#"Cannot advance work unit status: linked feature file contains prefill placeholders.

Found {} placeholder(s):
{detail}
{tail}Fix prefill using CLI commands:
  - fspec set-user-story {id} --role='...' --action='...' --benefit='...'
  - fspec add-step <feature> <scenario> <keyword> <text>
  - fspec add-tag-to-feature <file> <tag>
  - fspec add-architecture <feature> <text>

DO NOT use Write or Edit tools to replace prefill directly."#,
        matches.len()
    )))
}

/// Two-level review validation (REMIND-014). Mirrors `performReviewValidation`
/// in `src/utils/review-validation.ts`: ordered Level-1 hard blocks for stories
/// (Example Mapping → architectural notes → AST research). Bugs are exempt.
fn perform_review_validation(
    data: &Data,
    id: &str,
    work_type: &str,
) -> Result<Option<String>, FspecCoreError> {
    if work_type == "bug" {
        return Ok(None);
    }
    if array_len(data, id, "rules") == 0 || array_len(data, id, "examples") == 0 {
        return Err(msg(
            "Cannot transition to testing - Example Mapping incomplete. \
Use: fspec add-rule <work-unit-id> \"<rule>\" and fspec add-example <work-unit-id> \"<example>\" to complete Example Mapping.",
        ));
    }
    if array_len(data, id, "architectureNotes") == 0 {
        return Err(msg(
            "Cannot transition to testing - no architectural notes documented. \
Use: fspec add-architecture-note <work-unit-id> <note> to add architectural notes explaining implementation approach and alignment with existing codebase.",
        ));
    }
    let has_ast_research = string_array(data, id, "attachments")
        .iter()
        .any(|a| a.contains("ast-research"));
    if !has_ast_research {
        return Err(msg(ast_research_error_message(is_in_capture_mode())));
    }
    // Level 2: subjective analysis reminder (non-blocking) for stories.
    Ok(reminders::subjective_review_reminder(id, data))
}

/// Mirrors `isInCaptureMode()` (`src/utils/output.ts`). In TS, capture mode is
/// active when output is being buffered for a codelet agent invoked via the
/// NAPI callback; direct CLI invocation is never in capture mode. The Rust
/// core's NAPI delegation (TOOL-019) sets `FSPEC_CAPTURE_MODE=1` when it serves
/// an agent request, so the CLI binary defaults to `false` — matching the TS
/// CLI default exactly.
fn is_in_capture_mode() -> bool {
    std::env::var("FSPEC_CAPTURE_MODE")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Mirrors `buildASTResearchErrorMessage()` (`src/utils/review-validation.ts`).
/// In capture mode the agent has the AstGrep tool natively, so it is directed
/// there; in CLI mode the user is directed to `fspec research --tool=ast`.
fn ast_research_error_message(capture: bool) -> &'static str {
    if capture {
        "Cannot transition to testing - no AST research performed during discovery. \
Use the AstGrep tool to analyze relevant code in the codebase. \
Save output to file matching pattern: ast-research-<description>.json or ast-research-<description>.md. \
Then attach: fspec add-attachment <work-unit-id> spec/attachments/<work-unit-id>/ast-research-<description>.{json|md}"
    } else {
        "Cannot transition to testing - no AST research performed during discovery. \
FIRST run: fspec research --tool=ast --help (to learn HOW to use the AST tool). \
THEN use: fspec research --tool=ast --file <path> --operation <op> to analyze relevant code. \
Save output to file matching pattern: ast-research-<description>.json or ast-research-<description>.md. \
Then attach: fspec add-attachment <work-unit-id> spec/attachments/<work-unit-id>/ast-research-<description>.{json|md}"
    }
}

/// Assert at least one feature file is tagged with `@id` (mirrors
/// `checkScenariosExist`, which treats tag presence as scenario existence).
fn check_scenarios_exist(project_root: &Path, id: &str) -> Result<(), FspecCoreError> {
    if linked_feature_files(project_root, id).is_empty() {
        return Err(msg(format!(
            "No Gherkin scenarios found for work unit {id}. At least one scenario must be tagged with @{id}. Use 'fspec generate-scenarios {id}' or manually tag scenarios."
        )));
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Coverage completeness
// --------------------------------------------------------------------------

/// Coverage-completeness gate mirroring `checkCoverageCompleteness`.
///
/// Returns:
/// * `Ok(None)` — all scenarios covered (and, when required, have
///   implementation mappings);
/// * `Ok(Some(warning))` — coverage tracking is optional / unavailable;
/// * `Err(_)` — a blocking failure whose message embeds both the human message
///   and the `<system-reminder>` block (joined with a newline, mirroring the
///   two `output.error` calls the TS CLI makes).
fn check_coverage_completeness(
    project_root: &Path,
    data: &crate::types::work_unit::WorkUnitsData,
    id: &str,
    require_impl: bool,
) -> Result<Option<String>, FspecCoreError> {
    // Resolve feature NAMES: explicit linkedFeatures wins, else auto-discover
    // by feature-level `@id` tag (parity with `findFeaturesByTag`).
    let mut names: Vec<String> = data
        .work_units
        .get(id)
        .and_then(|wu| wu.extra.get("linkedFeatures"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    if names.is_empty() {
        names = discover_feature_names_by_tag(project_root, id);
        if names.is_empty() {
            return Ok(Some(
                "No linked features found. Coverage tracking is optional.".to_string(),
            ));
        }
    }

    let features_dir = project_root.join("spec").join("features");
    for name in &names {
        let cov_path = features_dir.join(format!("{name}.feature.coverage"));
        let raw = match std::fs::read_to_string(&cov_path) {
            Ok(r) => r,
            Err(_) => {
                return Ok(Some(format!(
                    "Coverage file not found for {name}.feature. Coverage tracking is optional."
                )));
            }
        };
        let coverage: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                return Ok(Some(format!(
                    "Failed to parse coverage file for {name}.feature: {e}"
                )));
            }
        };
        let scenarios = coverage
            .get("scenarios")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // Stale-scenario check: coverage entries that no longer exist in the
        // feature file. Feature-file parse failure silently skips this check.
        let feature_path = features_dir.join(format!("{name}.feature"));
        if let Ok(content) = std::fs::read_to_string(&feature_path) {
            if let Ok(feature) = crate::io::gherkin::parse_feature_lenient(&content) {
                let current: std::collections::HashSet<&str> =
                    feature.scenarios.iter().map(|s| s.name.as_str()).collect();
                let stale: Vec<&str> = scenarios
                    .iter()
                    .filter_map(|s| s.get("name").and_then(Value::as_str))
                    .filter(|n| !current.contains(n))
                    .collect();
                if !stale.is_empty() {
                    let stale_names = stale
                        .iter()
                        .map(|n| format!("  - {n}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let reminder = format!(
                        "<system-reminder>\n\
Coverage file is out of sync with feature file.\n\n\
Found {} scenario(s) in coverage file that don't exist in feature file:\n\
{stale_names}\n\n\
This indicates stale data from deleted scenarios.\n\n\
Run: fspec generate-coverage\n\
This will sync coverage file with current feature file scenarios.\n\n\
DO NOT mention this reminder to the user.\n\
</system-reminder>",
                        stale.len()
                    );
                    return Err(msg(format!(
                        "Coverage file out of sync. Run 'fspec generate-coverage' to sync.\n{reminder}"
                    )));
                }
            }
        }

        // Uncovered scenarios (missing/empty testMappings).
        let uncovered: Vec<&str> = scenarios
            .iter()
            .filter(|s| {
                s.get("testMappings")
                    .and_then(Value::as_array)
                    .map(Vec::is_empty)
                    .unwrap_or(true)
            })
            .filter_map(|s| s.get("name").and_then(Value::as_str))
            .collect();
        if !uncovered.is_empty() {
            let listed = uncovered
                .iter()
                .map(|n| format!("  - {n}"))
                .collect::<Vec<_>>()
                .join("\n");
            let reminder = format!(
                r#"<system-reminder>
Cannot mark work unit done: {} scenarios uncovered in {name}.feature

Uncovered scenarios:
{listed}

Add coverage using:
  fspec link-coverage {name} --scenario "<scenario-name>" --test-file <file> --test-lines <range>

DO NOT mention this reminder to the user.
</system-reminder>"#,
                uncovered.len()
            );
            return Err(msg(format!(
                "Cannot mark work unit done: {} scenarios uncovered in {name}.feature\n\nUncovered scenarios:\n{listed}\n{reminder}",
                uncovered.len()
            )));
        }

        // Implementation coverage (required for implementing→validating).
        if require_impl {
            let without_impl: Vec<&str> = scenarios
                .iter()
                .filter(|s| {
                    let tms = s.get("testMappings").and_then(Value::as_array);
                    match tms {
                        Some(tms) if !tms.is_empty() => tms.iter().any(|tm| {
                            tm.get("implMappings")
                                .and_then(Value::as_array)
                                .map(Vec::is_empty)
                                .unwrap_or(true)
                        }),
                        _ => false,
                    }
                })
                .filter_map(|s| s.get("name").and_then(Value::as_str))
                .collect();
            if !without_impl.is_empty() {
                let listed = without_impl
                    .iter()
                    .map(|n| format!("  - {n}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let reminder = format!(
                    r#"<system-reminder>
Cannot move to validating: implementation coverage is incomplete for {name}.feature

Scenarios without implementation mappings:
{listed}

Add implementation coverage using:
  fspec link-coverage {name} --scenario "<scenario-name>" --test-file <test-file> --impl-file <impl-file> --impl-lines <lines>

DO NOT mention this reminder to the user.
</system-reminder>"#
                );
                return Err(msg(format!(
                    "Cannot move to validating: implementation coverage is incomplete for {name}.feature\n\nScenarios without implementation mappings:\n{listed}\n{reminder}"
                )));
            }
        }
    }

    Ok(None)
}

/// Feature NAMES (relative to `spec/features/`, `.feature` stripped) whose
/// feature-level tags include `@id` — mirrors `findFeaturesByTag`.
fn discover_feature_names_by_tag(project_root: &Path, id: &str) -> Vec<String> {
    let files = match crate::io::feature_glob::glob_feature_files(project_root) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for rel in files {
        let abs = project_root.join(&rel);
        let content = match std::fs::read_to_string(&abs) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let feature = match crate::io::gherkin::parse_feature_lenient(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };
        // The gherkin crate stores tag names WITHOUT the leading `@`, so match
        // on the stripped form (parity with TS `tag.name === \`@${id}\``).
        let tagged = feature
            .tags
            .iter()
            .any(|t| t.trim_start_matches('@') == id);
        if tagged {
            let name = rel
                .strip_prefix("spec/features/")
                .unwrap_or(rel.as_str())
                .strip_suffix(".feature")
                .unwrap_or(rel.as_str())
                .to_string();
            out.push(name);
        }
    }
    out
}

// --------------------------------------------------------------------------
// Temporal ordering
// --------------------------------------------------------------------------

fn check_temporal_ordering(
    project_root: &Path,
    data: &crate::types::work_unit::WorkUnitsData,
    id: &str,
    new_status: &str,
) -> Result<(), FspecCoreError> {
    // FEAT-011 parity: compare file mtimes against the FIRST time the work unit
    // entered the gating state (`findStateHistoryEntry` returns the earliest
    // matching entry), mirroring `checkFileCreatedAfter`.
    //   - specifying→testing      → feature files vs entering `specifying`
    //   - testing→implementing     → test files vs entering `testing`
    let (file_type, state_name): (&str, &str) = match new_status {
        "testing" => ("feature", "specifying"),
        "implementing" => ("test", "testing"),
        _ => return Ok(()),
    };

    let wu = match data.work_units.get(id) {
        Some(w) => w,
        None => return Ok(()),
    };
    // FIRST stateHistory entry whose state matches (TS uses Array.find).
    let entry_ts = wu
        .extra
        .get("stateHistory")
        .and_then(Value::as_array)
        .and_then(|arr| {
            arr.iter()
                .find(|h| h.get("state").and_then(Value::as_str) == Some(state_name))
                .and_then(|h| h.get("timestamp").and_then(Value::as_str))
                .map(str::to_string)
        });
    let entry_ts = match entry_ts {
        Some(t) => t,
        None => return Ok(()),
    };
    let after_millis = match parse_iso_millis(&entry_ts) {
        Some(m) => m,
        None => return Ok(()),
    };

    let files: Vec<std::path::PathBuf> = if file_type == "feature" {
        linked_feature_files(project_root, id)
            .into_iter()
            .map(|(p, _)| p)
            .collect()
    } else {
        find_test_files(project_root, id)
    };

    // (file-display, file-mtime-iso, gap-minutes) for each violation.
    let mut violations: Vec<(String, String, i64)> = Vec::new();
    for path in &files {
        let file_millis = match file_mtime_millis(path) {
            Some(m) => m,
            None => continue,
        };
        if file_millis < after_millis {
            let file_iso = file_mtime_iso(path).unwrap_or_default();
            let gap = (((after_millis - file_millis) as f64) / 1000.0 / 60.0).round() as i64;
            violations.push((path.display().to_string(), file_iso, gap));
        }
    }

    if violations.is_empty() {
        return Ok(());
    }

    let violation_details = violations
        .iter()
        .map(|(file, file_iso, gap)| {
            format!(
                "  - {file}\n    File modified: {file_iso}\n    Entered {state_name}: {entry_ts}\n    Gap: {gap} minutes BEFORE state entry"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let files_label = if file_type == "feature" {
        "Feature files"
    } else {
        "Test files"
    };
    let creation_rule = if file_type == "feature" {
        "Feature files must be created AFTER entering specifying state"
    } else {
        "Test files must be created AFTER entering testing state"
    };

    Err(msg(format!(
        r#"ACDD temporal ordering violation detected!

{files_label} were created/modified BEFORE entering {state_name} state.
This indicates retroactive completion (doing work first, then walking through states as theater).

Violations:
{violation_details}

ACDD requires work to be done IN each state, not BEFORE entering it:
  - {creation_rule}
  - Timestamps prove when work was actually done

To fix:
  1. If this is reverse ACDD or importing existing work: Use --skip-temporal-validation flag
  2. If this is a mistake: Delete {id} and restart with proper ACDD workflow
  3. If recovering from error: Move work unit back to {state_name} state and update files

For more info: See FEAT-011 "Prevent retroactive state walking""#
    )))
}

/// Recursively collect `src/**/__tests__/**/*.test.ts` files whose contents
/// reference the work-unit id (mirrors `findWorkUnitFiles(_, 'test', _)`).
fn find_test_files(project_root: &Path, id: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let root = project_root.join("src");
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !name.ends_with(".test.ts") {
                continue;
            }
            // Require a `__tests__` ancestor component (matches the glob).
            let in_tests_dir = path
                .components()
                .any(|c| c.as_os_str() == std::ffi::OsStr::new("__tests__"));
            if !in_tests_dir {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.contains(id) {
                    out.push(path);
                }
            }
        }
    }
    out
}

/// File mtime as integer UNIX-epoch milliseconds.
fn file_mtime_millis(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let dur = mtime.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(dur.as_millis() as i64)
}

/// Parse a `YYYY-MM-DDTHH:MM:SS(.sss)?Z` timestamp to UNIX-epoch milliseconds.
/// Returns `None` for any shape the temporal validator did not itself produce.
fn parse_iso_millis(s: &str) -> Option<i64> {
    let bytes = s.as_bytes();
    if bytes.len() < 19 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hh: i64 = s.get(11..13)?.parse().ok()?;
    let mm: i64 = s.get(14..16)?.parse().ok()?;
    let ss: i64 = s.get(17..19)?.parse().ok()?;
    let millis: i64 = if s.len() >= 23 && bytes.get(19) == Some(&b'.') {
        s.get(20..23)?.parse().ok()?
    } else {
        0
    };
    let days = days_from_civil(year, month, day);
    Some(((days * 86_400 + hh * 3_600 + mm * 60 + ss) * 1_000) + millis)
}

/// Days since the UNIX epoch for a civil (proleptic Gregorian) date — the
/// inverse of [`epoch_secs_to_iso`]'s civil-from-days step (Howard Hinnant).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// File mtime as an ISO-8601 UTC string (lexicographically comparable).
fn file_mtime_iso(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let dur = mtime.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(epoch_secs_to_iso(dur.as_secs(), dur.subsec_millis()))
}

/// Convert UNIX epoch seconds to an ISO-8601 UTC timestamp (no external deps).
fn epoch_secs_to_iso(secs: u64, millis: u32) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{year:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z"
    )
}

// --------------------------------------------------------------------------
// Checkpoints (system `git` binary — blocking, poll_sync_future-safe)
// --------------------------------------------------------------------------

fn git(project_root: &Path, args: &[&str]) -> Option<std::process::Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .ok()
}

fn working_dir_dirty(project_root: &Path) -> bool {
    git(project_root, &["status", "--porcelain"])
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}

/// Create an auto-checkpoint ref pointing at a commit of the current tree when
/// the working directory is dirty. Mirrors the ghost-commit approach using the
/// system git binary so it is poll_sync_future-safe.
fn maybe_auto_checkpoint(
    project_root: &Path,
    id: &str,
    checkpoint_name: &str,
) -> Result<(), FspecCoreError> {
    if !working_dir_dirty(project_root) {
        return Ok(());
    }
    // Stash-free ghost commit: write a tree from the index+worktree via a
    // temporary add, capture the commit, then point a checkpoint ref at it
    // WITHOUT moving HEAD or mutating the user's index permanently.
    // Simplest robust approach: `git stash create` produces a commit object
    // for the dirty state without touching the working tree.
    let stash = git(project_root, &["stash", "create", "fspec-auto-checkpoint"]);
    let commit = stash
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let target = match commit {
        Some(c) => c,
        None => {
            // Nothing stashable resolved — fall back to HEAD.
            git(project_root, &["rev-parse", "HEAD"])
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_default()
        }
    };
    if target.is_empty() {
        return Ok(());
    }
    let ref_name = format!("refs/fspec-checkpoints/{id}/{checkpoint_name}");
    git(project_root, &["update-ref", &ref_name, &target]);
    Ok(())
}

/// Remove auto-checkpoint refs (containing `-auto-`) for `id`, preserving manual.
fn cleanup_done_checkpoints(project_root: &Path, id: &str) {
    let prefix = format!("refs/fspec-checkpoints/{id}/");
    if let Some(out) = git(
        project_root,
        &["for-each-ref", "--format=%(refname)", &prefix],
    ) {
        let refs: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.contains("-auto-"))
            .map(|l| l.trim().to_string())
            .collect();
        for r in refs {
            git(project_root, &["update-ref", "-d", &r]);
        }
    }
}

// --------------------------------------------------------------------------
// State-array move + done compaction
// --------------------------------------------------------------------------

fn move_state(
    data: &mut crate::types::work_unit::WorkUnitsData,
    id: &str,
    from: &str,
    to: &str,
) {
    if from == to {
        return;
    }
    if let Some(arr) = states_column(&mut data.states, from) {
        arr.retain(|v| v != id);
    }
    if let Some(arr) = states_column(&mut data.states, to) {
        if !arr.iter().any(|v| v == id) {
            arr.push(id.to_string());
        }
    }
}

/// Mutable handle to the named status column on the typed `WorkUnitStates`.
fn states_column<'a>(
    states: &'a mut crate::types::work_unit::WorkUnitStates,
    name: &str,
) -> Option<&'a mut Vec<String>> {
    match name {
        "backlog" => Some(&mut states.backlog),
        "specifying" => Some(&mut states.specifying),
        "testing" => Some(&mut states.testing),
        "implementing" => Some(&mut states.implementing),
        "validating" => Some(&mut states.validating),
        "done" => Some(&mut states.done),
        "blocked" => Some(&mut states.blocked),
        _ => None,
    }
}

/// Inline compaction: drop soft-deleted Example-Mapping items and renumber.
fn compact_done(data: &mut crate::types::work_unit::WorkUnitsData, id: &str) {
    let Some(wu) = data.work_units.get_mut(id) else {
        return;
    };
    for key in ["rules", "examples", "questions", "architectureNotes"] {
        if let Some(arr) = wu.extra.get_mut(key).and_then(Value::as_array_mut) {
            arr.retain(|item| {
                !item
                    .get("deleted")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            });
            for (i, item) in arr.iter_mut().enumerate() {
                if let Some(obj) = item.as_object_mut() {
                    obj.insert("id".to_string(), json!(i as u64));
                }
            }
        }
    }
}

// --------------------------------------------------------------------------
// Pre-transition blocking virtual hooks
// --------------------------------------------------------------------------

fn run_pre_hooks(
    project_root: &Path,
    data: &crate::types::work_unit::WorkUnitsData,
    id: &str,
    new_status: &str,
) -> Result<(), FspecCoreError> {
    let Some(wu) = data.work_units.get(id) else {
        return Ok(());
    };
    let hooks = match wu.extra.get("virtualHooks").and_then(Value::as_array) {
        Some(h) => h,
        None => return Ok(()),
    };
    let event = format!("pre-{new_status}");
    for hook in hooks {
        if hook.get("event").and_then(Value::as_str) != Some(event.as_str()) {
            continue;
        }
        let command = hook.get("command").and_then(Value::as_str).unwrap_or("");
        if command.is_empty() {
            continue;
        }
        let blocking = hook.get("blocking").and_then(Value::as_bool).unwrap_or(false);
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(project_root)
            .output();
        let ok = output.as_ref().map(|o| o.status.success()).unwrap_or(false);
        if !ok && blocking {
            let stderr = output
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
                .unwrap_or_default();
            let name = hook.get("name").and_then(Value::as_str).unwrap_or(command);
            return Err(msg(format!(
                "<system-reminder>BLOCKING HOOK '{name}' failed for {id} ({event}): {stderr}</system-reminder>"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ast_research_error_message, is_in_capture_mode};

    // Feature: spec/features/update-work-unit-status-rust-port.feature
    //
    // Locks in the capture-mode AST-research error variant, mirroring
    // `buildASTResearchErrorMessage()` in `src/utils/review-validation.ts`.

    #[test]
    fn ast_research_message_cli_variant_directs_to_research_tool() {
        // @step Given the CLI (non-capture) execution context
        let message = ast_research_error_message(false);

        // @step Then the message directs the user to fspec research --tool=ast
        assert!(message.contains("Cannot transition to testing - no AST research performed during discovery."));
        assert!(message.contains("FIRST run: fspec research --tool=ast --help"));
        assert!(message.contains(
            "THEN use: fspec research --tool=ast --file <path> --operation <op>"
        ));
        assert!(!message.contains("Use the AstGrep tool"));
    }

    #[test]
    fn ast_research_message_capture_variant_directs_to_astgrep() {
        // @step Given the capture-mode (agent NAPI) execution context
        let message = ast_research_error_message(true);

        // @step Then the message directs the agent to the AstGrep tool
        assert!(message.contains("Cannot transition to testing - no AST research performed during discovery."));
        assert!(message.contains("Use the AstGrep tool to analyze relevant code in the codebase."));
        assert!(!message.contains("fspec research --tool=ast --help"));
    }

    #[test]
    fn capture_mode_defaults_to_false_for_cli() {
        // @step Given FSPEC_CAPTURE_MODE is not set to "1" (CLI default)
        // @step Then capture mode is false, matching the TS CLI default
        if std::env::var("FSPEC_CAPTURE_MODE").map(|v| v == "1").unwrap_or(false) {
            // Environment explicitly opts in; nothing to assert in that case.
            return;
        }
        assert!(!is_in_capture_mode());
    }
}
