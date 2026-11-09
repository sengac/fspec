//! Command hooks (HOOK-018): the `pre-<command>` / `post-<command>` hooks
//! configured in `spec/fspec-hooks.json`.
//!
//! Feature specs: hook-configuration-schema-and-validation (HOOK-002),
//! hook-execution-engine (HOOK-003), hook-discovery-and-event-naming
//! (HOOK-004), hook-condition-evaluation (HOOK-005),
//! system-reminder-formatting-for-ai (HOOK-006) and
//! integrate-hooks-into-all-commands (HOOK-008); work-unit virtual hooks on
//! status moves: virtual-hook-git-context-and-post-status-hooks (HOOK-019);
//! global pre-/post-<status> hooks: global-status-hooks-run-for-every-work-unit
//! (HOOK-020); a broken spec/fspec-hooks.json is backed up before a hook
//! command writes a new one: a-broken-hooks-file-is-backed-up-before-a-new-one-is-written
//! (BUG-611).
//!
//! Commands run under `poll_sync_future`, so everything here is synchronous
//! `std::process`. The agent lifecycle events in the same file
//! (`session_start`, `pre_tool_use`, ...) belong to codelet-core's
//! lifecycle engine and are not read here.

mod conditions;
mod config;
pub(crate) mod config_file;
mod discovery;
mod events;
mod executor;
mod formatting;
mod phases;
mod virtual_hooks;

pub use conditions::{hook_matches, HookWorkUnit};
pub use config::{load_hook_config, CommandHook, HookCondition, HookConfig, DEFAULT_TIMEOUT_SECS};
pub use discovery::{discover_hooks, event_names, EventNames};
pub use events::check_hook_event;
pub use executor::{execute_hook, execute_hooks, HookContext, HookResult};
pub use formatting::format_hook_output;
pub(crate) use phases::with_command_hooks;
pub use phases::{
    is_hooked_command, run_command_hooks, run_event_hooks, work_unit_id_from_args, HookPhase,
    PhaseOutcome,
};
pub use virtual_hooks::run_status_hooks;
