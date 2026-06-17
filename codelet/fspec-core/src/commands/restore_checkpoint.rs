//! `restore-checkpoint` — Rust port of `src/commands/restore-checkpoint.ts`
//! (RPC-288).
//!
//! Restores a ghost-commit checkpoint back onto the working tree, with
//! conflict detection and an AI-assisted resolution surface. Delegates the
//! actual tree restoration to [`codelet_git::ghost_commit::restore_ghost_commit`]
//! and the diff/dirty pre-checks to
//! [`codelet_git::ghost_commit::get_checkpoint_diff_files`] +
//! `codelet_git::status::{get_staged_files,get_unstaged_files,get_untracked_files}`
//! (pure gitoxide — no `git` CLI).
//!
//! Parity notes (predecessor findings, baked into the spec):
//!   - Restoration delegates to codelet-git; this command owns NO index write
//!     (unlike `checkpoint`).
//!   - Missing checkpoint ref → `success:false` + a sentinel `systemReminder`
//!     (`Checkpoint "<name>" not found for work unit <wu>`) — NOT an
//!     `InvalidArgs` error. Mirrors the TS bare-catch at
//!     `src/utils/git-checkpoint.ts:270-278`.
//!   - An empty `checkpointName` IS an `InvalidArgs` error (dispatcher
//!     contract is strict where TS Commander.js is loose).
//!   - Dirty working tree with no `userChoice` and no `force` → early return
//!     with `requiresUserChoice:true` and three numbered risk options; NO
//!     files are restored (mirrors `src/commands/restore-checkpoint.ts:54-110`).
//!   - Conflicts (dirty + differing files, no force) → `conflictsDetected:true`
//!     with the `CHECKPOINT RESTORATION CONFLICT DETECTED` system-reminder
//!     (`src/utils/git-checkpoint.ts:231-251`).
//!   - `sendIPCMessage` is a documented NO-OP in the Rust standalone binary.
//!
//! Two-front-doors (RPC-003 §7/§11): invoked by BOTH the dispatcher and the
//! `fspec restore-checkpoint` clap subcommand — no dirty-check, conflict
//! detection, restore, or rendering logic is duplicated in the CLI bridge.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;

use codelet_git::ghost_commit::{get_checkpoint_diff_files, restore_ghost_commit};
use codelet_git::status::{get_staged_files, get_unstaged_files, get_untracked_files};

/// The exact `userChoice` value that maps to the high-risk overwrite path
/// (`src/commands/restore-checkpoint.ts:119`).
const OVERWRITE_CHOICE: &str = "Overwrite files (discard changes)";

/// CLI arguments accepted by `restore-checkpoint`.
///
/// Parity with TS Commander.js registration
/// (`src/commands/restore-checkpoint.ts:193-199`): two positional arguments
/// `<work-unit-id>` and `<checkpoint-name>`, no `.option(...)` flags. The
/// dispatcher path additionally honours `force`, `userChoice`, and
/// `workingDirectoryDirty` (mirroring `RestoreCheckpointOptions`) plus
/// `format` ("text" | "json").
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RestoreArgs {
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    checkpoint_name: Option<String>,
    #[serde(default)]
    force: bool,
    #[serde(default)]
    user_choice: Option<String>,
    #[serde(default)]
    working_directory_dirty: Option<bool>,
    #[serde(default)]
    format: Option<String>,
}

/// The structured restore payload. `#[derive(Serialize)]` preserves field
/// declaration order so the JSON keys are emitted as
/// `success, conflictsDetected, conflictedFiles, systemReminder,
/// requiresTestValidation` — the order the dispatcher test pins. Mirrors the
/// TS `RestoreCheckpointResult` core fields
/// (`src/commands/restore-checkpoint.ts:22-35`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreResultPayload {
    success: bool,
    conflicts_detected: bool,
    conflicted_files: Vec<String>,
    system_reminder: String,
    requires_test_validation: bool,
}

/// Internal result of the lower-level restore utility (mirrors the TS
/// `restoreCheckpointUtil` return at `src/utils/git-checkpoint.ts:207-279`).
struct UtilResult {
    success: bool,
    conflicts_detected: bool,
    conflicted_files: Vec<String>,
    system_reminder: String,
    requires_test_validation: bool,
}

/// One displayable risk option (mirrors the TS `promptOptions` entries at
/// `src/commands/restore-checkpoint.ts:64-83`).
struct PromptOption {
    name: &'static str,
    risk_level: &'static str,
    description: &'static str,
}

const PROMPT_OPTIONS: &[PromptOption] = &[
    PromptOption {
        name: "Commit changes first",
        risk_level: "Low",
        description: "Safest option. Commits current changes before restoration.",
    },
    PromptOption {
        name: "Stash changes and restore",
        risk_level: "Medium",
        description: "Temporarily saves changes. Can restore later if needed.",
    },
    PromptOption {
        name: OVERWRITE_CHOICE,
        risk_level: "High",
        description: "Overwrites working directory with checkpoint. Current changes will be LOST FOREVER unless committed or stashed.",
    },
];

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RestoreArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "restore-checkpoint",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = match args.work_unit_id.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-checkpoint",
                reason: "missing or empty `workUnitId` field".to_string(),
            });
        }
    };

    // An empty checkpointName is rejected here (dispatcher contract is strict).
    let checkpoint_name = match args.checkpoint_name.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-checkpoint",
                reason: "missing or empty `checkpointName` field".to_string(),
            });
        }
    };

    let is_text = matches!(args.format.as_deref(), Some("text"));

    // Determine dirty state (caller override mirrors the TS
    // `workingDirectoryDirty ?? (await isWorkingDirectoryDirty(cwd))`).
    let is_dirty = args
        .working_directory_dirty
        .unwrap_or_else(|| is_working_directory_dirty(project_root));

    // Dirty + no choice + no force → show risk options and require a choice.
    if is_dirty && args.user_choice.is_none() && !args.force {
        let conflict_check = restore_util(project_root, &work_unit_id, &checkpoint_name, false);
        let system_reminder = if conflict_check.system_reminder.is_empty() {
            "User choice required".to_string()
        } else {
            conflict_check.system_reminder
        };

        if is_text {
            return Ok(render_prompt_text());
        }
        return render_user_choice_json(
            conflict_check.conflicts_detected,
            &conflict_check.conflicted_files,
            &system_reminder,
            conflict_check.requires_test_validation,
        );
    }

    // Normal restore. force is escalated when the user explicitly chose the
    // high-risk overwrite option.
    let effective_force =
        args.force || (is_dirty && args.user_choice.as_deref() == Some(OVERWRITE_CHOICE));

    let result = restore_util(project_root, &work_unit_id, &checkpoint_name, effective_force);

    let payload = RestoreResultPayload {
        success: result.success,
        conflicts_detected: result.conflicts_detected,
        conflicted_files: result.conflicted_files,
        system_reminder: result.system_reminder,
        requires_test_validation: result.requires_test_validation,
    };

    if is_text {
        return Ok(render_text(&work_unit_id, &checkpoint_name, &payload));
    }
    render_json(&payload)
}

/// Lower-level restore utility — mirrors `restoreCheckpointUtil`.
///
/// When not forced and the working tree is dirty with files differing from
/// the checkpoint, returns a conflict result WITHOUT touching the tree.
/// Otherwise restores via codelet-git. Any codelet-git error (e.g. a missing
/// ref) degrades to the not-found sentinel result (mirrors the TS bare-catch).
fn restore_util(
    project_root: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
    force: bool,
) -> UtilResult {
    if !force && is_working_directory_dirty(project_root) {
        match get_checkpoint_diff_files(project_root, work_unit_id, checkpoint_name) {
            Ok(diff_files) if !diff_files.is_empty() => {
                return UtilResult {
                    success: false,
                    conflicts_detected: true,
                    conflicted_files: diff_files.clone(),
                    system_reminder: conflict_reminder(checkpoint_name, work_unit_id, &diff_files),
                    requires_test_validation: true,
                };
            }
            Ok(_) => {}
            // Ref missing / not a checkpoint → fall through to the not-found
            // path produced by restore_ghost_commit below.
            Err(_) => {
                return not_found_result(checkpoint_name, work_unit_id);
            }
        }
    }

    match restore_ghost_commit(project_root, work_unit_id, checkpoint_name, force) {
        Ok(r) => UtilResult {
            success: r.success,
            conflicts_detected: false,
            conflicted_files: Vec::new(),
            system_reminder: String::new(),
            requires_test_validation: false,
        },
        Err(_) => not_found_result(checkpoint_name, work_unit_id),
    }
}

/// The not-found sentinel result (byte-identical `systemReminder` to the TS
/// catch text at `src/utils/git-checkpoint.ts:275`).
fn not_found_result(checkpoint_name: &str, work_unit_id: &str) -> UtilResult {
    UtilResult {
        success: false,
        conflicts_detected: false,
        conflicted_files: Vec::new(),
        system_reminder: format!(
            "Checkpoint \"{checkpoint_name}\" not found for work unit {work_unit_id}"
        ),
        requires_test_validation: false,
    }
}

/// Build the `CHECKPOINT RESTORATION CONFLICT DETECTED` system-reminder
/// byte-for-byte from the TS template (`src/utils/git-checkpoint.ts:236-248`).
fn conflict_reminder(checkpoint_name: &str, work_unit_id: &str, diff_files: &[String]) -> String {
    let file_list = diff_files
        .iter()
        .map(|f| format!("  - {f}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<system-reminder>\n\
CHECKPOINT RESTORATION CONFLICT DETECTED\n\
\n\
The following {count} file(s) have been modified since checkpoint \"{checkpoint_name}\" was created:\n\
{file_list}\n\
\n\
Working directory changes will be LOST if you restore this checkpoint!\n\
\n\
RECOMMENDED: Create new checkpoint first to preserve work:\n  \
fspec checkpoint {work_unit_id} before-restore\n\
\n\
DO NOT mention this reminder to the user explicitly.\n\
</system-reminder>",
        count = diff_files.len(),
    )
}

/// Compute working-tree dirtiness from the three status helpers (errors
/// degrade to "clean", mirroring the TS bare-catch at
/// `src/utils/git-checkpoint.ts:133-143`).
fn is_working_directory_dirty(project_root: &Path) -> bool {
    let staged = get_staged_files(project_root).unwrap_or_default();
    let unstaged = get_unstaged_files(project_root).unwrap_or_default();
    let untracked = get_untracked_files(project_root).unwrap_or_default();
    !staged.is_empty() || !unstaged.is_empty() || !untracked.is_empty()
}

/// Render the dirty-tree risk prompt text (mirrors the `output.log` lines at
/// `src/commands/restore-checkpoint.ts:85-98`).
fn render_prompt_text() -> String {
    let mut out = String::new();
    out.push_str("\u{26A0}\u{FE0F}  Working directory has uncommitted changes\n");
    out.push_str("\nChoose how to proceed:\n");
    for (idx, opt) in PROMPT_OPTIONS.iter().enumerate() {
        out.push_str(&format!(
            "  {}. {} [{} risk]\n",
            idx + 1,
            opt.name,
            opt.risk_level
        ));
        out.push_str(&format!("     {}\n", opt.description));
    }
    out
}

/// Render the rich `requiresUserChoice` JSON payload (default + json formats).
///
/// This payload deliberately differs from the 5-key normal-path payload: it
/// carries the prompt `message`, the structured `options`, and the
/// `requiresUserChoice` flag. The dispatcher tests only exercise this path via
/// the default format, so it embeds the human prompt in `message` while
/// keeping the structured fields available for agent-loop callers.
fn render_user_choice_json(
    conflicts_detected: bool,
    conflicted_files: &[String],
    system_reminder: &str,
    requires_test_validation: bool,
) -> Result<String, FspecCoreError> {
    let options: Vec<Value> = PROMPT_OPTIONS
        .iter()
        .map(|opt| {
            json!({
                "name": opt.name,
                "riskLevel": opt.risk_level,
                "description": opt.description,
            })
        })
        .collect();

    let payload = json!({
        "success": false,
        "requiresUserChoice": true,
        "conflictsDetected": conflicts_detected,
        "conflictedFiles": conflicted_files,
        "systemReminder": system_reminder,
        "requiresTestValidation": requires_test_validation,
        "promptShown": true,
        "options": options,
        "message": render_prompt_text(),
    });

    serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "restore-checkpoint",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Render the human-facing text for the normal restore path. Mirrors the
/// TS branches at `src/commands/restore-checkpoint.ts:122-144`.
fn render_text(
    work_unit_id: &str,
    checkpoint_name: &str,
    payload: &RestoreResultPayload,
) -> String {
    if payload.conflicts_detected {
        let mut out = String::new();
        out.push_str("\u{2717} Merge conflicts detected during restoration\n");
        out.push_str("\nConflicted files:\n");
        for f in &payload.conflicted_files {
            out.push_str(&format!("  - {f}\n"));
        }
        out.push_str("\n\u{1F4A1} Resolve conflicts using Read and Edit tools, then run tests\n");
        if !payload.system_reminder.is_empty() {
            out.push_str(&payload.system_reminder);
        }
        return out;
    }

    // Parity with `src/commands/restore-checkpoint.ts:138-144`: any non-conflict
    // path (including the not-found failure where `success:false`) falls into
    // the TS `else` branch and emits the ✓ banner. The structured `success`
    // flag — not the banner — drives the exit code, so a not-found restore
    // prints the banner yet still exits 1.
    format!("\u{2713} Restored checkpoint \"{checkpoint_name}\" for {work_unit_id}")
}

/// Render the structured 2-space-indented JSON payload (5 keys, fixed order).
fn render_json(payload: &RestoreResultPayload) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "restore-checkpoint",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: RestoreArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","checkpointName":"baseline","force":true,"userChoice":"2"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.checkpoint_name.as_deref(), Some("baseline"));
        assert!(a.force);
        assert_eq!(a.user_choice.as_deref(), Some("2"));
    }

    #[test]
    fn not_found_reminder_text_is_exact() {
        let r = not_found_result("ghost", "AUTH-001");
        assert_eq!(
            r.system_reminder,
            "Checkpoint \"ghost\" not found for work unit AUTH-001"
        );
        assert!(!r.success);
    }

    #[test]
    fn conflict_reminder_has_header_and_closing_tag() {
        let reminder = conflict_reminder("baseline", "AUTH-001", &["a.txt".into(), "b.txt".into()]);
        assert!(reminder.contains("CHECKPOINT RESTORATION CONFLICT DETECTED"));
        assert!(reminder.trim_end().ends_with("</system-reminder>"));
        assert!(reminder.contains("  - a.txt"));
        assert!(reminder.contains("The following 2 file(s)"));
    }

    #[test]
    fn prompt_text_lists_three_options_with_risks() {
        let out = render_prompt_text();
        assert!(out.contains("Working directory has uncommitted changes"));
        for n in ["1.", "2.", "3."] {
            assert!(out.contains(n), "missing {n}");
        }
        for risk in ["Low", "Medium", "High"] {
            assert!(out.contains(risk), "missing {risk}");
        }
    }

    #[test]
    fn json_key_order_is_preserved() {
        let payload = RestoreResultPayload {
            success: true,
            conflicts_detected: false,
            conflicted_files: vec![],
            system_reminder: String::new(),
            requires_test_validation: false,
        };
        let data: Value = serde_json::from_str(&render_json(&payload).unwrap()).unwrap();
        let keys: Vec<&str> = data.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            vec![
                "success",
                "conflictsDetected",
                "conflictedFiles",
                "systemReminder",
                "requiresTestValidation"
            ]
        );
    }

    #[test]
    fn restore_banner_text() {
        let payload = RestoreResultPayload {
            success: true,
            conflicts_detected: false,
            conflicted_files: vec![],
            system_reminder: String::new(),
            requires_test_validation: false,
        };
        let out = render_text("AUTH-001", "baseline", &payload);
        assert_eq!(out, "\u{2713} Restored checkpoint \"baseline\" for AUTH-001");
    }

    #[test]
    fn restore_not_found_still_renders_banner() {
        // Parity: a not-found restore (success:false, no conflicts) must still
        // emit the ✓ banner — the TS `else` branch fires regardless of success.
        let payload = RestoreResultPayload {
            success: false,
            conflicts_detected: false,
            conflicted_files: vec![],
            system_reminder: "Checkpoint \"zzz\" not found for work unit AUTH-001".into(),
            requires_test_validation: false,
        };
        let out = render_text("AUTH-001", "zzz", &payload);
        assert_eq!(out, "\u{2713} Restored checkpoint \"zzz\" for AUTH-001");
    }
}
