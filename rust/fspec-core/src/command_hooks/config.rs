//! Loading the command hooks of `spec/fspec-hooks.json` (HOOK-002).

use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::error::FspecCoreError;

/// Timeout of a hook with no `timeout` and no `global.timeout`.
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// When a command hook runs for a work unit (HOOK-005).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HookCondition {
    pub tags: Vec<String>,
    pub prefix: Vec<String>,
    pub epic: Option<String>,
    pub estimate_min: Option<f64>,
    pub estimate_max: Option<f64>,
}

/// One configured command hook with its defaults resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandHook {
    pub name: String,
    pub command: String,
    pub blocking: bool,
    pub timeout_secs: u64,
    pub condition: Option<HookCondition>,
}

/// The command hooks of a project, by event in config order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HookConfig {
    pub global_timeout_secs: Option<u64>,
    pub shell: Option<String>,
    pub events: IndexMap<String, Vec<CommandHook>>,
}

impl HookConfig {
    /// The hooks configured for `event`, in config order.
    pub fn hooks_for(&self, event: &str) -> &[CommandHook] {
        self.events.get(event).map_or(&[], Vec::as_slice)
    }
}

/// The config file, relative to the project root, as errors name it.
pub(crate) const CONFIG_FILE: &str = "spec/fspec-hooks.json";

/// Load the command hooks of `spec/fspec-hooks.json`.
///
/// A missing file means no command hooks. Invalid JSON, a malformed
/// `global`/`hooks` value or command-hook entry, or a script path that does
/// not exist is an error naming the file: never a silent fallback to no
/// hooks. Only `pre-<name>` / `post-<name>` keys are command-hook events; the
/// agent lifecycle keys (`session_start`, `pre_tool_use`, ...) belong to
/// codelet-core's lifecycle engine and keep their own shape.
pub fn load_hook_config(project_root: &Path) -> Result<HookConfig, FspecCoreError> {
    load(project_root, None)
}

/// Load only `event`'s hooks, for running a command: a hook of another event
/// (a script not written yet, say) does not affect the command. The file must
/// still be valid JSON with a `hooks` object.
pub(crate) fn load_event_hooks(
    project_root: &Path,
    event: &str,
) -> Result<HookConfig, FspecCoreError> {
    load(project_root, Some(event))
}

fn load(project_root: &Path, only_event: Option<&str>) -> Result<HookConfig, FspecCoreError> {
    let path = project_root.join(CONFIG_FILE);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HookConfig::default()),
        Err(e) => return Err(invalid(format!("cannot be read: {e}"))),
    };
    let value: Value = serde_json::from_str(&raw)
        .map_err(|e| FspecCoreError::Message(format!("Invalid JSON in {CONFIG_FILE}: {e}")))?;
    let root = value
        .as_object()
        .ok_or_else(|| invalid("the top level must be an object".to_string()))?;

    let mut config = HookConfig::default();
    if let Some(global) = root.get("global") {
        let global = global
            .as_object()
            .ok_or_else(|| invalid("\"global\" must be an object".to_string()))?;
        config.global_timeout_secs = optional_timeout(global.get("timeout"), "global.timeout")?;
        config.shell = optional_string(global.get("shell"), "global.shell")?;
    }
    let default_timeout = config.global_timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);

    let Some(hooks) = root.get("hooks") else {
        return Ok(config);
    };
    let hooks = hooks
        .as_object()
        .ok_or_else(|| invalid("\"hooks\" must be an object".to_string()))?;
    for (event, entries) in hooks {
        if !is_command_hook_event(event) || only_event.is_some_and(|only| only != event) {
            continue;
        }
        let entries = entries
            .as_array()
            .ok_or_else(|| invalid(format!("hooks.{event} must be an array")))?;
        let mut parsed = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            let hook = parse_hook(entry, &format!("hooks.{event}[{index}]"), default_timeout)?;
            if is_script_path(&hook.command, project_root)
                && !project_root.join(&hook.command).exists()
            {
                return Err(FspecCoreError::Message(format!(
                    "Hook command not found: {} (configured in {CONFIG_FILE})",
                    hook.command
                )));
            }
            parsed.push(hook);
        }
        config.events.insert(event.clone(), parsed);
    }
    Ok(config)
}

/// `pre-<name>` or `post-<name>` with a non-empty name.
fn is_command_hook_event(event: &str) -> bool {
    event
        .strip_prefix("pre-")
        .or_else(|| event.strip_prefix("post-"))
        .is_some_and(|name| !name.is_empty())
}

/// A whitespace-free command starting with `./`, `/` or `spec/`, or naming
/// an existing file under the project root, is a script path; anything else
/// (a script with arguments, a pipeline, a program on PATH) is a shell
/// command run from the project root.
pub(crate) fn is_script_path(command: &str, project_root: &Path) -> bool {
    if command.chars().any(char::is_whitespace) {
        return false;
    }
    command.starts_with("./")
        || command.starts_with('/')
        || command.starts_with("spec/")
        || project_root.join(command).is_file()
}

fn parse_hook(
    entry: &Value,
    at: &str,
    default_timeout: u64,
) -> Result<CommandHook, FspecCoreError> {
    let obj = entry
        .as_object()
        .ok_or_else(|| invalid(format!("{at} must be an object")))?;
    let name = required_string(obj.get("name"), &format!("{at}.name"))?;
    let command = required_string(obj.get("command"), &format!("{at}.command"))?;
    let blocking = match obj.get("blocking") {
        None => false,
        Some(v) => v
            .as_bool()
            .ok_or_else(|| invalid(format!("{at}.blocking must be true or false")))?,
    };
    let timeout_secs =
        optional_timeout(obj.get("timeout"), &format!("{at}.timeout"))?.unwrap_or(default_timeout);
    let condition = match obj.get("condition") {
        None => None,
        Some(v) => Some(parse_condition(v, &format!("{at}.condition"))?),
    };
    Ok(CommandHook {
        name,
        command,
        blocking,
        timeout_secs,
        condition,
    })
}

fn parse_condition(value: &Value, at: &str) -> Result<HookCondition, FspecCoreError> {
    let obj = value
        .as_object()
        .ok_or_else(|| invalid(format!("{at} must be an object")))?;
    Ok(HookCondition {
        tags: string_list(obj.get("tags"), &format!("{at}.tags"))?,
        prefix: string_list(obj.get("prefix"), &format!("{at}.prefix"))?,
        epic: optional_string(obj.get("epic"), &format!("{at}.epic"))?,
        estimate_min: optional_number(obj.get("estimateMin"), &format!("{at}.estimateMin"))?,
        estimate_max: optional_number(obj.get("estimateMax"), &format!("{at}.estimateMax"))?,
    })
}

fn required_string(value: Option<&Value>, at: &str) -> Result<String, FspecCoreError> {
    match value.and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => Ok(s.to_string()),
        _ => Err(invalid(format!("{at} must be a non-empty string"))),
    }
}

fn optional_string(value: Option<&Value>, at: &str) -> Result<Option<String>, FspecCoreError> {
    match value {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| invalid(format!("{at} must be a string"))),
    }
}

fn optional_number(value: Option<&Value>, at: &str) -> Result<Option<f64>, FspecCoreError> {
    match value {
        None => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| invalid(format!("{at} must be a number"))),
    }
}

fn optional_timeout(value: Option<&Value>, at: &str) -> Result<Option<u64>, FspecCoreError> {
    match value {
        None => Ok(None),
        Some(v) => match v.as_u64() {
            Some(secs) if secs > 0 => Ok(Some(secs)),
            _ => Err(invalid(format!(
                "{at} must be a whole number of seconds above 0"
            ))),
        },
    }
}

fn string_list(value: Option<&Value>, at: &str) -> Result<Vec<String>, FspecCoreError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let message = || invalid(format!("{at} must be an array of strings"));
    value
        .as_array()
        .ok_or_else(message)?
        .iter()
        .map(|v| v.as_str().map(str::to_string).ok_or_else(message))
        .collect()
}

fn invalid(detail: String) -> FspecCoreError {
    FspecCoreError::Message(format!(
        "Invalid hook configuration in {CONFIG_FILE}: {detail}"
    ))
}
