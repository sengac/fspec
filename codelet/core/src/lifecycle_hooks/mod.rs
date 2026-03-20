//! Agent Lifecycle Hooks
//!
//! Configuration data model, two-level loading, merging, compilation,
//! JSON payloads, outcome types, and execution engine for agent lifecycle
//! events (session_start, session_end, user_prompt_submit,
//! pre_tool_use, post_tool_use, notification).

pub mod compiled;
pub mod config;
pub mod engine;
pub(crate) mod executor;
pub(crate) mod helpers;
pub mod loader;
pub mod outcome;
pub mod payloads;
pub(crate) mod response;
pub mod tool_engine;

pub use compiled::{
    CompiledHookCommand, CompiledHookDefinition, CompiledHookGroup, CompiledLifecycleHooks,
    HookMatcher,
};
pub use config::{
    FspecHooksConfig, GlobalConfig, HookCommandConfig, HookDefinition, HookGroupConfig,
    is_agent_lifecycle_event, is_tool_hook_event, AGENT_LIFECYCLE_EVENTS, TOOL_HOOK_EVENTS,
};
pub use engine::{
    HookContext, run_notification, run_session_end, run_session_start, run_user_prompt,
};
pub use loader::load_lifecycle_hooks;
pub use outcome::{
    HookMessage, HookMessageLevel, NotificationOutcome, PostToolOutcome, PreToolHookDecision,
    PreToolOutcome, SessionEndOutcome, SessionStartOutcome, UserPromptOutcome,
};
pub use payloads::{
    NotificationPayload, PostToolUsePayload, PreToolUsePayload, SessionEndPayload,
    SessionStartPayload, UserPromptSubmitPayload,
};
pub use tool_engine::{run_post_tool, run_pre_tool};
