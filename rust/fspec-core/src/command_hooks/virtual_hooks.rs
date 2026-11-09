//! Status hooks on status moves: the work unit's virtual hooks (HOOK-019)
//! and the global pre-/post-<status> hooks of spec/fspec-hooks.json
//! (HOOK-020).
//!
//! For a move from S to T the order is: global post-S hooks, the work
//! unit's post-S virtual hooks, global pre-T hooks, its pre-T virtual
//! hooks. Global hooks use their configured timeout and condition.
//!
//! A move from S to T runs the work unit's `post-S` hooks (leaving S) and
//! then its `pre-T` hooks (entering T), all before the move is saved. They
//! run through the command-hook executor; a hook with `gitContext` also
//! receives the staged and unstaged paths. A failing blocking hook stops the
//! remaining hooks and blocks the move.
//!
//! A virtual hook record (stored in `WorkUnit.extra["virtualHooks"]` by
//! add-virtual-hook) has the shape
//! `{ "name": "eslint", "event": "post-implementing", "command": "eslint src/",
//!    "blocking": true, "gitContext": true }`.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::conditions::{hook_matches, HookWorkUnit};
use super::config::{load_event_hooks, CommandHook, HookConfig, DEFAULT_TIMEOUT_SECS};
use super::executor::{execute_hook, HookContext, HookResult};
use super::formatting::format_hook_output;
use crate::error::FspecCoreError;
use crate::types::work_unit::WorkUnitsData;

/// One stored virtual hook.
struct VirtualHook {
    hook: CommandHook,
    event: String,
    git_context: bool,
}

/// Run the status hooks of work unit `id` for a move from `current` to
/// `new`: per event (post-`current`, then pre-`new`) the global hooks of
/// spec/fspec-hooks.json, then the work unit's virtual hooks.
///
/// Returns the formatted output of the hooks that ran. A move to the same
/// status or to `blocked` runs none. A failing blocking hook is an error
/// whose `<system-reminder>` names the hook, the work unit and the event;
/// an unreadable hooks file is an error, never "no hooks".
pub fn run_status_hooks(
    project_root: &Path,
    data: &WorkUnitsData,
    id: &str,
    current: &str,
    new: &str,
) -> Result<String, FspecCoreError> {
    if current == new || new == "blocked" {
        return Ok(String::new());
    }
    let hooks = stored_hooks(data, id)?;
    let virtual_config = HookConfig::default();
    let mut work_unit: Option<Option<HookWorkUnit>> = None;
    let mut git_files: Option<(Vec<String>, Vec<String>)> = None;
    let mut output = Vec::new();
    for event in [format!("post-{current}"), format!("pre-{new}")] {
        let global = load_event_hooks(project_root, &event)?;
        for hook in global.hooks_for(&event) {
            if hook.condition.is_some() && work_unit.is_none() {
                work_unit = Some(HookWorkUnit::load(project_root, id)?);
            }
            if !hook_matches(hook, work_unit.as_ref().and_then(Option::as_ref)) {
                continue;
            }
            let context = HookContext::status(id, &event, current, new);
            let result = execute_hook(project_root, &global, hook, &context);
            record(&mut output, "global hook", id, &event, &result)?;
        }
        for stored in hooks.iter().filter(|h| h.event == event) {
            let mut context = HookContext::status(id, &event, current, new);
            if stored.git_context {
                if git_files.is_none() {
                    git_files = Some(git_changed_files(project_root)?);
                }
                if let Some((staged, unstaged)) = &git_files {
                    context.staged_files = Some(staged.clone());
                    context.unstaged_files = Some(unstaged.clone());
                }
            }
            let result = execute_hook(project_root, &virtual_config, &stored.hook, &context);
            record(&mut output, "virtual hook", id, &event, &result)?;
        }
    }
    Ok(output.join("\n"))
}

/// Keep a hook's output, or stop with its blocking failure.
fn record(
    output: &mut Vec<String>,
    kind: &str,
    id: &str,
    event: &str,
    result: &HookResult,
) -> Result<(), FspecCoreError> {
    if result.blocking && !result.success() {
        return Err(FspecCoreError::Message(blocking_failure(
            kind, id, event, result,
        )));
    }
    let formatted = format_hook_output(result);
    if !formatted.is_empty() {
        output.push(formatted);
    }
    Ok(())
}

fn blocking_failure(kind: &str, id: &str, event: &str, result: &HookResult) -> String {
    let exit = result
        .exit_code
        .map_or_else(|| "none".to_string(), |code| code.to_string());
    let mut lines = vec![
        "<system-reminder>".to_string(),
        format!(
            "BLOCKING HOOK FAILURE: {kind} '{}' for {id} failed at {event}.",
            result.hook_name
        ),
        format!("Exit code: {exit}"),
    ];
    if result.timed_out {
        lines.push(format!("Timed out after {}s", result.timeout_secs));
    }
    let stdout = result.stdout.trim_end();
    if !stdout.is_empty() {
        lines.push(String::new());
        lines.push(stdout.to_string());
    }
    lines.push(String::new());
    let stderr = result.stderr.trim_end();
    lines.push(if stderr.is_empty() {
        "(Hook failed with no error output)".to_string()
    } else {
        stderr.to_string()
    });
    lines.push(String::new());
    lines.push("This is a BLOCKING hook. Fix the errors before proceeding.".to_string());
    lines.push("</system-reminder>".to_string());
    lines.join("\n")
}

/// The work unit's stored virtual hooks, in order. A malformed record is an
/// error, never skipped.
fn stored_hooks(data: &WorkUnitsData, id: &str) -> Result<Vec<VirtualHook>, FspecCoreError> {
    let Some(wu) = data.work_units.get(id) else {
        return Ok(Vec::new());
    };
    let Some(value) = wu.extra.get("virtualHooks") else {
        return Ok(Vec::new());
    };
    let malformed = |detail: &str| {
        FspecCoreError::Message(format!(
            "Work unit {id} has a malformed virtualHooks entry: {detail}"
        ))
    };
    let entries = value
        .as_array()
        .ok_or_else(|| malformed("virtualHooks must be an array"))?;
    entries
        .iter()
        .map(|entry| {
            let text = |key: &str| match entry.get(key).and_then(Value::as_str) {
                Some(s) if !s.trim().is_empty() => Ok(s.to_string()),
                _ => Err(malformed(&format!("{key} must be a non-empty string"))),
            };
            let flag = |key: &str| match entry.get(key) {
                None => Ok(false),
                Some(v) => v
                    .as_bool()
                    .ok_or_else(|| malformed(&format!("{key} must be true or false"))),
            };
            Ok(VirtualHook {
                hook: CommandHook {
                    name: text("name")?,
                    command: text("command")?,
                    blocking: flag("blocking")?,
                    timeout_secs: DEFAULT_TIMEOUT_SECS,
                    condition: None,
                },
                event: text("event")?,
                git_context: flag("gitContext")?,
            })
        })
        .collect()
}

/// Staged and unstaged paths (relative to the repository root), from the
/// system git binary as the auto-checkpoints use it. Outside a git work tree
/// both lists are empty; a git that cannot run or fails inside a work tree is
/// an error.
fn git_changed_files(project_root: &Path) -> Result<(Vec<String>, Vec<String>), FspecCoreError> {
    let inside = git(project_root, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() || String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Ok((Vec::new(), Vec::new()));
    }
    let staged = paths(project_root, &["diff", "--cached", "--name-only", "-z"])?;
    let unstaged = paths(project_root, &["diff", "--name-only", "-z"])?;
    Ok((staged, unstaged))
}

fn paths(project_root: &Path, args: &[&str]) -> Result<Vec<String>, FspecCoreError> {
    let out = git(project_root, args)?;
    if !out.status.success() {
        return Err(FspecCoreError::Message(format!(
            "git {} failed for a gitContext virtual hook: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect())
}

fn git(project_root: &Path, args: &[&str]) -> Result<std::process::Output, FspecCoreError> {
    Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|e| {
            FspecCoreError::Message(format!(
                "git could not run for a gitContext virtual hook: {e}"
            ))
        })
}
