//! Reminders that depend on work-unit data: the subjective review analysis
//! and the enhanced IMPLEMENTING / DONE status bodies.

use serde_json::Value;

use super::Data;

/// Mirrors `buildSubjectiveAnalysisReminder` — returned by review validation
/// for stories that pass the objective checks (attachments are non-empty).
pub(in crate::commands::update_work_unit_status) fn subjective_review_reminder(
    id: &str,
    data: &Data,
) -> Option<String> {
    let wu = data.work_units.get(id)?;
    let attachments: Vec<String> = wu
        .extra
        .get("attachments")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if attachments.is_empty() {
        return None;
    }
    let notes: Vec<String> = wu
        .extra
        .get("architectureNotes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|n| match n {
                    Value::String(s) => s.clone(),
                    _ => n
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let mut lines: Vec<String> = Vec::new();
    lines.push("<system-reminder>".to_string());
    lines.push("ARCHITECTURAL REVIEW - SUBJECTIVE ANALYSIS".to_string());
    lines.push(String::new());
    lines.push(format!(
        "Work unit {id} passed objective ACDD checks but requires AI analysis:"
    ));
    lines.push(String::new());
    lines.push("AST RESEARCH ATTACHMENTS:".to_string());
    for (idx, att) in attachments.iter().enumerate() {
        lines.push(format!("  {}. {att}", idx + 1));
    }
    lines.push(String::new());
    lines.push("ARCHITECTURAL NOTES:".to_string());
    for (idx, note) in notes.iter().enumerate() {
        lines.push(format!("  {}. {note}", idx + 1));
    }
    lines.push(String::new());
    lines.push("AI ANALYSIS REQUIRED:".to_string());
    lines
        .push("  1. Read AST research attachments - verify they reference actual code".to_string());
    lines.push("  2. Check architectural notes align with FOUNDATION.md/AGENTS.md".to_string());
    lines.push(
        "  3. Verify not reinventing existing utilities (check for duplicate function names)"
            .to_string(),
    );
    lines.push("  4. Ensure DRY/SOLID principles followed in proposed approach".to_string());
    lines.push(String::new());
    lines.push("DECISION:".to_string());
    lines.push(format!(
        "  - If issues found: Revert to specifying: fspec update-work-unit-status {id} specifying"
    ));
    lines.push("  - If analysis passes: Continue to testing phase".to_string());
    lines.push(String::new());
    lines.push("DO NOT mention this reminder to the user explicitly.".to_string());
    lines.push("</system-reminder>".to_string());
    Some(lines.join("\n"))
}

// ── Enhanced status reminders (depend on work-unit data) ──────────────────

const IMPLEMENTING_BODY: &str = r#"⚠️ COMMON FAILURE MODE: Code that exists but isn't connected.
Tests passing in isolation is NOT the same as a working feature.

IMPLEMENTATION = CREATION + CONNECTION

For every piece of code you write, ask: "WHO CALLS THIS?"
If the answer is "nobody yet" — you're not done. Wire it up.

COMPLETE MEANS:
  ✓ New code exists AND is connected to the system
  ✓ Existing files modified as described in architecture notes
  ✓ Feature works end-to-end (can be demonstrated in the real system)
  ✗ New code exists but nothing calls it

Common commands for IMPLEMENTING state:
  fspec link-coverage <feature> --scenario "..." --test-file <path> --impl-file <path> --impl-lines <lines>
  fspec checkpoint <workUnitId> <name>
  fspec restore-checkpoint <workUnitId> <name>
  fspec list-checkpoints <workUnitId>

For more: fspec checkpoint --help

DO NOT mention this reminder to the user."#;

fn user_story_parts(data: &Data, id: &str) -> Option<(String, String, String)> {
    let wu = data.work_units.get(id)?;
    let us = wu.extra.get("userStory")?;
    let role = us
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let action = us
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let benefit = us
        .get("benefit")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((role, action, benefit))
}

fn active_architecture_notes(data: &Data, id: &str) -> Vec<String> {
    data.work_units
        .get(id)
        .and_then(|wu| wu.extra.get("architectureNotes"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|n| !n.get("deleted").and_then(Value::as_bool).unwrap_or(false))
                .map(|n| match n {
                    Value::String(s) => s.clone(),
                    _ => n
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Mirrors `implementingStateReminder`.
pub(super) fn implementing_reminder(id: &str, data: &Data) -> String {
    let mut user_story_section = String::new();
    if let Some((role, action, benefit)) = user_story_parts(data, id) {
        user_story_section =
            format!("USER STORY: \"As a {role}, I want to {action}, so that {benefit}\"\n\n");
    }
    let mut arch_section = String::new();
    let notes = active_architecture_notes(data, id);
    if !notes.is_empty() {
        let listed = notes
            .iter()
            .map(|n| format!("  - {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        arch_section = format!("ARCHITECTURE NOTES:\n{listed}\n\n");
    }
    format!(
        "Work unit {id} is now in IMPLEMENTING status.\n\n{user_story_section}{arch_section}{IMPLEMENTING_BODY}"
    )
}

/// Mirrors `doneStateReminder`.
pub(super) fn done_reminder(id: &str, data: &Data) -> String {
    let mut user_story_check = String::new();
    if let Some((role, action, _benefit)) = user_story_parts(data, id) {
        user_story_check = format!(
            r#"FINAL CHECK

User story: "As a {role}, I want to {action}..."

Can {role} do this RIGHT NOW in the actual system?

  - Not "the code exists"
  - Not "tests pass"
  - Can they ACTUALLY do it?

If NO: Go back to implementing.

"#
        );
    }
    format!(
        r#"Work unit {id} is now in DONE status.

{user_story_check}CRITICAL: Verify feature file tags are updated:
  - Remove @wip tag: fspec remove-tag-from-feature <file> @wip
  - Add @done tag: fspec add-tag-to-feature <file> @done

All acceptance criteria should be met. DO NOT mention this reminder to the user."#
    )
}
