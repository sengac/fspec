//! Hook discovery and event naming (HOOK-004).

use super::config::{CommandHook, HookConfig};

/// A command's two hook events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventNames {
    pub pre: String,
    pub post: String,
}

/// `pre-<command>` and `post-<command>`.
pub fn event_names(command: &str) -> EventNames {
    EventNames {
        pre: format!("pre-{command}"),
        post: format!("post-{command}"),
    }
}

/// The hooks of `event` (case-sensitive) in config order; empty when none.
pub fn discover_hooks<'a>(config: &'a HookConfig, event: &str) -> &'a [CommandHook] {
    config.hooks_for(event)
}
