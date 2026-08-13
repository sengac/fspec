//! Agent Lifecycle Hooks — Config Data Model
//!
//! Serde types for deserializing fspec-hooks.json, specifically the agent
//! lifecycle event entries alongside existing fspec CLI command events.

use serde::Deserialize;
use std::collections::HashMap;

/// Top-level fspec-hooks.json config structure.
///
/// Contains optional global settings and a map of event name → hook entries.
/// Values are `serde_json::Value` because the format is polymorphic:
/// - Agent lifecycle non-tool events: `HookDefinition[]`
/// - Agent lifecycle tool events: `HookGroup[]`
/// - fspec CLI events: also `HookDefinition[]` (ignored by Rust engine)
#[derive(Debug, Clone, Deserialize)]
pub struct FspecHooksConfig {
    /// Global settings (timeout, shell)
    pub global: Option<GlobalConfig>,
    /// Event name → array of hook entries (polymorphic JSON)
    #[serde(default)]
    pub hooks: HashMap<String, serde_json::Value>,
}

/// Global configuration shared across all hooks.
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalConfig {
    /// Default timeout in seconds (defaults to 60 if absent)
    pub timeout: Option<u64>,
    /// Shell to use for command execution (defaults to "sh -c")
    pub shell: Option<String>,
}

/// A single hook definition (used for session_start, session_end,
/// user_prompt_submit, notification events, and fspec CLI events).
#[derive(Debug, Clone, Deserialize)]
pub struct HookDefinition {
    /// Human-readable name for the hook
    #[serde(default = "default_hook_name")]
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Whether failure should block the operation (default: false)
    #[serde(default)]
    pub blocking: Option<bool>,
    /// Per-hook timeout override in seconds
    pub timeout: Option<u64>,
}

fn default_hook_name() -> String {
    "unnamed".to_string()
}

/// A hook group for pre_tool_use/post_tool_use events.
/// Contains an optional matcher regex and a list of sequential commands.
#[derive(Debug, Clone, Deserialize)]
pub struct HookGroupConfig {
    /// Optional regex pattern to match tool names (absent = match all)
    pub matcher: Option<String>,
    /// Sequential commands within this group
    pub hooks: Vec<HookCommandConfig>,
}

/// A single command within a hook group (simpler than HookDefinition).
#[derive(Debug, Clone, Deserialize)]
pub struct HookCommandConfig {
    /// Shell command to execute
    pub command: String,
    /// Per-command timeout override in seconds
    pub timeout: Option<u64>,
}

/// The 6 agent lifecycle events recognized by the Rust engine.
pub const AGENT_LIFECYCLE_EVENTS: &[&str] = &[
    "session_start",
    "session_end",
    "user_prompt_submit",
    "notification",
    "pre_tool_use",
    "post_tool_use",
];

/// Tool-specific events that use HookGroup[] format (with matcher).
pub const TOOL_HOOK_EVENTS: &[&str] = &["pre_tool_use", "post_tool_use"];

/// Check if an event key is an agent lifecycle event (vs fspec CLI event).
///
/// Agent lifecycle events use underscores (e.g., `session_start`).
/// fspec CLI events use hyphens (e.g., `pre-update-work-unit-status`).
pub fn is_agent_lifecycle_event(key: &str) -> bool {
    AGENT_LIFECYCLE_EVENTS.contains(&key)
}

/// Check if an agent lifecycle event uses the HookGroup[] format.
pub fn is_tool_hook_event(key: &str) -> bool {
    TOOL_HOOK_EVENTS.contains(&key)
}
