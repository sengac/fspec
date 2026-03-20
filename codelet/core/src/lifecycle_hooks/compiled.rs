//! Agent Lifecycle Hooks — Compiled Types
//!
//! Compiled representations of lifecycle hook config, with pre-compiled regex
//! matchers for efficient runtime matching.

use regex::Regex;

/// Compiled lifecycle hooks ready for execution.
///
/// This is the result of loading + merging + compiling the two-level config.
/// Stored as `Option<CompiledLifecycleHooks>` on the session — `None` means
/// no agent lifecycle events are configured (zero overhead).
#[derive(Debug)]
pub struct CompiledLifecycleHooks {
    /// Global default timeout in seconds (from config or 60)
    pub global_timeout: u64,

    /// Shell to use for command execution (e.g., "bash -c"). `None` = default "sh -c".
    pub global_shell: Option<String>,

    // Non-tool events (HookDefinition[] format)
    pub session_start: Vec<CompiledHookDefinition>,
    pub session_end: Vec<CompiledHookDefinition>,
    pub user_prompt_submit: Vec<CompiledHookDefinition>,
    pub notification: Vec<CompiledHookDefinition>,

    // Tool events (HookGroup[] format with matchers)
    pub pre_tool_use: Vec<CompiledHookGroup>,
    pub post_tool_use: Vec<CompiledHookGroup>,
}

impl CompiledLifecycleHooks {
    /// Check if there are any hooks configured at all.
    pub fn is_empty(&self) -> bool {
        self.session_start.is_empty()
            && self.session_end.is_empty()
            && self.user_prompt_submit.is_empty()
            && self.notification.is_empty()
            && self.pre_tool_use.is_empty()
            && self.post_tool_use.is_empty()
    }
}

/// A compiled hook definition for non-tool events.
#[derive(Debug)]
pub struct CompiledHookDefinition {
    /// Human-readable name
    pub name: String,
    /// Shell command to execute
    pub command: String,
    /// Whether failure blocks the operation
    pub blocking: bool,
    /// Timeout in seconds (resolved: per-hook override or global default)
    pub timeout: u64,
}

/// A compiled hook group for pre_tool_use/post_tool_use events.
#[derive(Debug)]
pub struct CompiledHookGroup {
    /// Matcher for tool names
    pub matcher: HookMatcher,
    /// Sequential commands within this group
    pub commands: Vec<CompiledHookCommand>,
}

/// A compiled command within a hook group.
#[derive(Debug)]
pub struct CompiledHookCommand {
    /// Shell command to execute
    pub command: String,
    /// Timeout in seconds (resolved: per-command override or global default)
    pub timeout: u64,
}

/// Matcher for filtering tool names.
#[derive(Debug)]
pub enum HookMatcher {
    /// Matches any tool name (empty/absent matcher)
    Any,
    /// Matches tool names against a compiled regex (anchored: `^(?:PATTERN)$`)
    Pattern(Regex),
}

impl HookMatcher {
    /// Check if a tool name matches this matcher.
    pub fn matches(&self, tool_name: &str) -> bool {
        match self {
            HookMatcher::Any => true,
            HookMatcher::Pattern(regex) => regex.is_match(tool_name),
        }
    }
}
