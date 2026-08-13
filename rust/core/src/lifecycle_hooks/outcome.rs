//! Agent Lifecycle Hooks — Outcome Types
//!
//! Typed outcome structs returned by the execution engine.
//! Each event type returns its own outcome struct.

/// Decision from a pre_tool_use hook evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreToolHookDecision {
    /// No opinion — continue to next hook group or default policy
    Continue,
    /// Explicitly allow the tool call (short-circuits remaining groups)
    Allow,
    /// Deny the tool call (short-circuits remaining groups)
    Deny,
    /// Ask the user for interactive permission
    Ask,
}

/// Severity level for hook messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookMessageLevel {
    Info,
    Warning,
    Error,
}

/// A message produced during hook execution.
#[derive(Debug, Clone)]
pub struct HookMessage {
    pub level: HookMessageLevel,
    pub content: String,
}

/// Outcome of running session_start hooks.
#[derive(Debug)]
pub struct SessionStartOutcome {
    pub messages: Vec<HookMessage>,
    pub additional_context: Vec<String>,
}

/// Outcome of running session_end hooks.
#[derive(Debug)]
pub struct SessionEndOutcome {
    pub messages: Vec<HookMessage>,
}

/// Outcome of running user_prompt_submit hooks.
#[derive(Debug)]
pub struct UserPromptOutcome {
    pub allow_prompt: bool,
    pub block_reason: Option<String>,
    pub additional_context: Vec<String>,
    pub messages: Vec<HookMessage>,
}

/// Outcome of running pre_tool_use hooks (single group).
#[derive(Debug)]
pub struct PreToolOutcome {
    pub decision: PreToolHookDecision,
    pub reason: Option<String>,
    pub messages: Vec<HookMessage>,
}

/// Outcome of running post_tool_use hooks (single group).
#[derive(Debug)]
pub struct PostToolOutcome {
    pub additional_context: Vec<String>,
    pub messages: Vec<HookMessage>,
}

/// Outcome of running notification hooks.
#[derive(Debug)]
pub struct NotificationOutcome {
    pub messages: Vec<HookMessage>,
}
