//! Agent Lifecycle Hooks — Config Loader
//!
//! Two-level config loading (user-level + project-level), merging,
//! and compilation into the runtime `CompiledLifecycleHooks`.

use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

use super::compiled::{
    CompiledHookCommand, CompiledHookDefinition, CompiledHookGroup, CompiledLifecycleHooks,
    HookMatcher,
};
use super::config::{
    is_agent_lifecycle_event, is_tool_hook_event, FspecHooksConfig, HookDefinition, HookGroupConfig,
};

/// Default timeout in seconds when neither global nor per-hook timeout is set.
const DEFAULT_TIMEOUT: u64 = 60;

/// Load, merge, and compile lifecycle hooks from the two-level config hierarchy.
///
/// # Arguments
/// - `project_root`: Path to the project root (for `spec/fspec-hooks.json`). `None` to skip.
/// - `user_home`: Path to the user's home directory (for `.fspec/fspec-hooks.json`). `None` to skip.
///
/// # Returns
/// - `Ok(Some(compiled))` — hooks were found and compiled successfully
/// - `Ok(None)` — no agent lifecycle events configured (zero overhead)
/// - `Err(e)` — invalid config (e.g., bad regex pattern)
pub fn load_lifecycle_hooks(
    project_root: Option<&Path>,
    user_home: Option<&Path>,
) -> Result<Option<CompiledLifecycleHooks>> {
    // Load configs from both levels (both are optional)
    let user_config = match user_home {
        Some(home) => load_config_file(&home.join(".fspec").join("fspec-hooks.json"))?,
        None => None,
    };

    let project_config = match project_root {
        Some(root) => load_config_file(&root.join("spec").join("fspec-hooks.json"))?,
        None => None,
    };

    // If neither file exists, no hooks
    if user_config.is_none() && project_config.is_none() {
        return Ok(None);
    }

    // Determine global timeout and shell
    let global_timeout = resolve_global_timeout(&user_config, &project_config);
    let global_shell = resolve_global_shell(&user_config, &project_config);

    // Merge and compile each agent lifecycle event
    let session_start = merge_and_compile_definitions(
        "session_start",
        &user_config,
        &project_config,
        global_timeout,
    )?;
    let session_end = merge_and_compile_definitions(
        "session_end",
        &user_config,
        &project_config,
        global_timeout,
    )?;
    let user_prompt_submit = merge_and_compile_definitions(
        "user_prompt_submit",
        &user_config,
        &project_config,
        global_timeout,
    )?;
    let notification = merge_and_compile_definitions(
        "notification",
        &user_config,
        &project_config,
        global_timeout,
    )?;
    let pre_tool_use = merge_and_compile_groups(
        "pre_tool_use",
        &user_config,
        &project_config,
        global_timeout,
    )?;
    let post_tool_use = merge_and_compile_groups(
        "post_tool_use",
        &user_config,
        &project_config,
        global_timeout,
    )?;

    let compiled = CompiledLifecycleHooks {
        global_timeout,
        global_shell,
        session_start,
        session_end,
        user_prompt_submit,
        notification,
        pre_tool_use,
        post_tool_use,
    };

    // Return None if no agent lifecycle events were actually configured
    if compiled.is_empty() {
        return Ok(None);
    }

    Ok(Some(compiled))
}

/// Load a single config file, returning None if it doesn't exist.
/// Returns an error if the file exists but contains invalid JSON.
fn load_config_file(path: &Path) -> Result<Option<FspecHooksConfig>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read hooks config: {}", path.display()))?;
    let config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse hooks config as JSON: {}", path.display()))?;
    Ok(Some(config))
}

/// Resolve the global timeout from the config hierarchy.
/// Project-level global takes precedence over user-level.
fn resolve_global_timeout(
    user_config: &Option<FspecHooksConfig>,
    project_config: &Option<FspecHooksConfig>,
) -> u64 {
    // Project-level global wins if present
    if let Some(config) = project_config {
        if let Some(global) = &config.global {
            if let Some(timeout) = global.timeout {
                return timeout;
            }
        }
    }
    // Fall back to user-level global
    if let Some(config) = user_config {
        if let Some(global) = &config.global {
            if let Some(timeout) = global.timeout {
                return timeout;
            }
        }
    }
    DEFAULT_TIMEOUT
}

/// Resolve the global shell from the config hierarchy.
/// Project-level global takes precedence over user-level.
fn resolve_global_shell(
    user_config: &Option<FspecHooksConfig>,
    project_config: &Option<FspecHooksConfig>,
) -> Option<String> {
    // Project-level global wins if present
    if let Some(config) = project_config {
        if let Some(global) = &config.global {
            if global.shell.is_some() {
                return global.shell.clone();
            }
        }
    }
    // Fall back to user-level global
    if let Some(config) = user_config {
        if let Some(global) = &config.global {
            if global.shell.is_some() {
                return global.shell.clone();
            }
        }
    }
    None
}

/// Extract hook definitions for a non-tool event from a config.
fn extract_definitions(
    event: &str,
    config: &Option<FspecHooksConfig>,
) -> Result<Vec<HookDefinition>> {
    let config = match config {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let value = match config.hooks.get(event) {
        Some(v) => v,
        None => return Ok(vec![]),
    };

    // Only process agent lifecycle events
    if !is_agent_lifecycle_event(event) || is_tool_hook_event(event) {
        return Ok(vec![]);
    }

    // Parse as HookDefinition[]
    serde_json::from_value::<Vec<HookDefinition>>(value.clone())
        .with_context(|| format!("Invalid hook definition format for event '{event}': expected an array of hook definitions"))
}

/// Extract hook groups for a tool event from a config.
fn extract_groups(event: &str, config: &Option<FspecHooksConfig>) -> Result<Vec<HookGroupConfig>> {
    let config = match config {
        Some(c) => c,
        None => return Ok(vec![]),
    };

    let value = match config.hooks.get(event) {
        Some(v) => v,
        None => return Ok(vec![]),
    };

    // Only process tool hook events
    if !is_tool_hook_event(event) {
        return Ok(vec![]);
    }

    // Parse as HookGroup[]
    serde_json::from_value::<Vec<HookGroupConfig>>(value.clone())
        .with_context(|| format!("Invalid hook group format for event '{event}': expected an array of hook groups with optional matcher and hooks array"))
}

/// Merge and compile HookDefinition entries from both config levels.
/// User-level first, project-level appended (concatenation).
fn merge_and_compile_definitions(
    event: &str,
    user_config: &Option<FspecHooksConfig>,
    project_config: &Option<FspecHooksConfig>,
    global_timeout: u64,
) -> Result<Vec<CompiledHookDefinition>> {
    let mut user_defs = extract_definitions(event, user_config)?;
    let project_defs = extract_definitions(event, project_config)?;

    // Concatenate: user-level first, project-level appended
    user_defs.extend(project_defs);

    Ok(user_defs
        .into_iter()
        .map(|def| CompiledHookDefinition {
            name: def.name,
            command: def.command,
            blocking: def.blocking.unwrap_or(false),
            timeout: def.timeout.unwrap_or(global_timeout),
        })
        .collect())
}

/// Merge and compile HookGroup entries from both config levels.
/// User-level first, project-level appended (concatenation).
fn merge_and_compile_groups(
    event: &str,
    user_config: &Option<FspecHooksConfig>,
    project_config: &Option<FspecHooksConfig>,
    global_timeout: u64,
) -> Result<Vec<CompiledHookGroup>> {
    let mut user_groups = extract_groups(event, user_config)?;
    let project_groups = extract_groups(event, project_config)?;

    // Concatenate: user-level first, project-level appended
    user_groups.extend(project_groups);

    user_groups
        .into_iter()
        .map(|group| compile_hook_group(group, global_timeout))
        .collect()
}

/// Compile a single hook group, including regex matcher compilation.
fn compile_hook_group(group: HookGroupConfig, global_timeout: u64) -> Result<CompiledHookGroup> {
    let matcher = match &group.matcher {
        None => HookMatcher::Any,
        Some(pattern) if pattern.is_empty() => HookMatcher::Any,
        Some(pattern) => {
            // Anchor the regex with full-match semantics: ^(?:PATTERN)$
            let anchored = format!("^(?:{pattern})$");
            let regex = Regex::new(&anchored)
                .with_context(|| format!("Invalid regex matcher pattern: {pattern}"))?;
            HookMatcher::Pattern(regex)
        }
    };

    let commands = group
        .hooks
        .into_iter()
        .map(|cmd| CompiledHookCommand {
            command: cmd.command,
            timeout: cmd.timeout.unwrap_or(global_timeout),
        })
        .collect();

    Ok(CompiledHookGroup { matcher, commands })
}
