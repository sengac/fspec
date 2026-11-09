//! Running a command's pre-/post- hooks for both front doors (HOOK-008).
//!
//! The fspec CLI (`rust/fspec/src/cli_registration/entry.rs`) and agent tool
//! calls ([`crate::dispatch::dispatch_command`]) each run the pre phase before
//! the command and the post phase after it succeeded. A failing blocking
//! pre-hook stops the phase and the command does not run; a failing blocking
//! post-hook fails the command after it ran.

use std::path::Path;

use super::conditions::{hook_matches, HookWorkUnit};
use super::config::load_event_hooks;
use super::discovery::{discover_hooks, event_names};
use super::executor::{execute_hook, HookContext, HookResult};
use super::formatting::format_hook_output;
use crate::dispatch::DispatchResult;
use crate::error::FspecCoreError;

/// Which side of the command a hook phase runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPhase {
    Pre,
    Post,
}

/// The outcome of one hook phase.
#[derive(Debug, Clone, Default)]
pub struct PhaseOutcome {
    pub event: String,
    /// The hooks that ran, in order.
    pub results: Vec<HookResult>,
    /// Every result formatted with [`format_hook_output`], one per line block.
    pub output: String,
    /// A blocking hook failed: before the command it must not run, after it
    /// the command fails.
    pub blocked: bool,
}

/// Commands that run command hooks: the canonical fspec commands and the
/// Rust-only fspec commands the dispatcher serves, except the four hook
/// management commands, which manage and report spec/fspec-hooks.json (a
/// broken one included) as their own features specify.
pub fn is_hooked_command(name: &str) -> bool {
    let hook_management = matches!(
        name,
        "add-hook" | "remove-hook" | "list-hooks" | "validate-hooks"
    );
    !hook_management
        && (crate::canonical::lookup(name).is_some()
            || matches!(name, "foundation-status" | "validate-config"))
}

/// The `workUnitId` of a command's JSON args, when it has one.
pub fn work_unit_id_from_args(args_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args_json)
        .ok()?
        .get("workUnitId")?
        .as_str()
        .map(str::to_string)
}

/// Run the hooks of `event` whose condition matches the work unit.
///
/// The work unit is read from the board only when a hook has a condition. A
/// failing blocking hook marks the phase blocked; in the pre phase it also
/// stops the remaining hooks.
pub fn run_event_hooks(
    project_root: &Path,
    event: &str,
    work_unit_id: Option<&str>,
    phase: HookPhase,
) -> Result<PhaseOutcome, FspecCoreError> {
    let mut outcome = PhaseOutcome {
        event: event.to_string(),
        ..PhaseOutcome::default()
    };
    let config = load_event_hooks(project_root, event)?;
    let hooks = discover_hooks(&config, event);
    if hooks.is_empty() {
        return Ok(outcome);
    }
    let work_unit = match work_unit_id {
        Some(id) if hooks.iter().any(|h| h.condition.is_some()) => {
            HookWorkUnit::load(project_root, id)?
        }
        _ => None,
    };
    let context = HookContext::new(work_unit_id, event);
    for hook in hooks.iter().filter(|h| hook_matches(h, work_unit.as_ref())) {
        let result = execute_hook(project_root, &config, hook, &context);
        let failed_blocking = result.blocking && !result.success();
        outcome.results.push(result);
        if failed_blocking {
            outcome.blocked = true;
            if phase == HookPhase::Pre {
                break;
            }
        }
    }
    outcome.output = join(outcome.results.iter().map(format_hook_output));
    Ok(outcome)
}

/// Run `command`'s `pre-<command>` or `post-<command>` hooks.
pub fn run_command_hooks(
    project_root: &Path,
    command: &str,
    work_unit_id: Option<&str>,
    phase: HookPhase,
) -> Result<PhaseOutcome, FspecCoreError> {
    let names = event_names(command);
    let event = match phase {
        HookPhase::Pre => names.pre,
        HookPhase::Post => names.post,
    };
    run_event_hooks(project_root, &event, work_unit_id, phase)
}

/// The dispatcher front door: run `run` (the command) between its pre and
/// post hooks. Hook output reaches the agent in `system_reminder`; a blocked
/// phase or a hook configuration error fails the tool call.
pub(crate) fn with_command_hooks(
    project_root: &Path,
    command: &str,
    args_json: &str,
    run: impl FnOnce() -> DispatchResult,
) -> DispatchResult {
    if !is_hooked_command(command) {
        return run();
    }
    // BUG-523: hooks resolve the same project root the command does.
    let resolved = codelet_common::project_root::resolve_project_root(project_root);
    let root = if command == "init" {
        project_root
    } else {
        resolved.as_path()
    };
    let work_unit_id = work_unit_id_from_args(args_json);
    let pre = match run_command_hooks(root, command, work_unit_id.as_deref(), HookPhase::Pre) {
        Ok(pre) => pre,
        Err(err) => return failure(String::new(), err.to_string()),
    };
    if pre.blocked {
        return failure(String::new(), pre.output);
    }
    let mut result = run();
    if !result.success {
        if !pre.output.is_empty() {
            let error = result.error.take().unwrap_or_default();
            result.error = Some(join([pre.output, error]));
        }
        return result;
    }
    let post = match run_command_hooks(root, command, work_unit_id.as_deref(), HookPhase::Post) {
        Ok(post) => post,
        Err(err) => return failure(result.data, err.to_string()),
    };
    let hook_output = join([pre.output, post.output]);
    if post.blocked {
        return failure(result.data, hook_output);
    }
    if !hook_output.is_empty() {
        let existing = result.system_reminder.take().unwrap_or_default();
        result.system_reminder = Some(join([existing, hook_output]));
    }
    result
}

fn failure(data: String, error: String) -> DispatchResult {
    DispatchResult {
        success: false,
        data,
        error: Some(error),
        system_reminder: None,
    }
}

/// Join the non-empty parts with newlines.
fn join(parts: impl IntoIterator<Item = String>) -> String {
    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
