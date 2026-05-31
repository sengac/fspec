//! Scheduler module — SCHED-003 / RPC-058 lift.
//!
//! Core scheduler engine: a tokio task that evaluates cron schedules
//! from `spec/schedules.json` every 30 seconds and triggers jobs.
//!
//! RPC-058 lifts these modules out of `codelet/napi/src/scheduler/` into
//! `codelet/core/src/scheduler/` (NAPI-free). The hooks trait
//! [`SchedulerHooks`] replaces the previously-direct
//! NAPI SessionManager singleton calls so the engine never
//! depends on the SessionManager's concrete shape:
//! NAPI (`codelet-napi`) and the Rust binary (`codelet-sessions`) each
//! provide their own implementation.
//!
//! SCHED-013: Session-scoped /loop entries are still self-managed via
//! per-entry spawned tokio tasks in LoopStore. That module is not lifted
//! by RPC-058 (RPC-059 covers it) — it continues to live under
//! `codelet/napi/src/scheduler/loop_store.rs`.

pub mod agent_job;
pub mod catch_up;
pub mod cron_utils;
pub mod crud;
pub mod engine;
pub mod job_log;
pub mod shell_job;
pub mod state;
pub mod trigger;
pub mod types;

pub use engine::{evaluate_and_run, evaluate_schedules, spawn_scheduler};
pub use state::SchedulerState;
pub use types::{EvaluationResult, ScheduleEntry, SchedulesFile};

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// The smallest piece of context the engine forwards into the agent-job
/// spawn hook. Carries everything the NAPI / sessions implementations
/// need to spawn a fresh subordinate session without the engine knowing
/// any of the SessionManager's concrete types.
#[derive(Debug, Clone)]
pub struct ScheduleTrigger {
    /// The schedule's name (lookup key in `spec/schedules.json`).
    pub name: String,
    /// Absolute project root path.
    pub project_path: String,
    /// Default model resolved at engine construction time via
    /// [`SchedulerHooks::default_model`].
    pub default_model: String,
    /// The role text to inject into the spawned session (agent only).
    pub role: Option<String>,
    /// The initial prompt sent to the spawned session.
    pub prompt: String,
    /// The session id the engine generated for this trigger.
    pub session_id: Uuid,
    /// The display name the engine generated for this trigger
    /// (`"[scheduled] NAME — TIMESTAMP"`).
    pub session_name: String,
}

/// Side-channel into the host process the engine runs in. NAPI wraps
/// its global SessionManager singleton;
/// `codelet-sessions::SessionManager` provides its own impl. The trait
/// is `async_trait` so the engine can `.await` each hook.
#[async_trait]
pub trait SchedulerHooks: Send + Sync {
    /// Current number of live sessions (used by the engine's session
    /// limit guard before spawning a new agent job).
    async fn get_session_count(&self) -> usize;

    /// Snapshot of currently-live session ids — used by the engine's
    /// `sweep_completed` pass.
    async fn get_live_session_ids(&self) -> Vec<Uuid>;

    /// Spawn a scheduled subordinate session. Implementations route this
    /// to whatever SessionManager they own. Returns `Err(String)` on
    /// any failure so the engine can record `failed` in the schedule
    /// log without dragging in `anyhow`.
    async fn spawn_scheduled_session(&self, trigger: ScheduleTrigger) -> Result<(), String>;

    /// The user's default model name, looked up at trigger time so a
    /// model change between ticks is picked up. Empty string means
    /// "no model configured" and the engine treats it as a hard error
    /// for agent jobs.
    fn default_model(&self) -> String;

    /// Look up the session id of a recently-spawned scheduled session
    /// by the schedule's name. Used by [`engine::trigger_and_update`]
    /// when writing the job log so the `session_id` column points at
    /// the spawned session. Default returns None so impls that don't
    /// track scheduled sessions (tests, noop builds) compile.
    async fn find_session_by_schedule_name(&self, _schedule_name: &str) -> Option<Uuid> {
        None
    }
}

/// A no-op [`SchedulerHooks`] used by builds that don't link a
/// SessionManager (tests, `noop` feature). Every method returns the
/// neutral element of its return type.
pub struct NoopSchedulerHooks;

#[async_trait]
impl SchedulerHooks for NoopSchedulerHooks {
    async fn get_session_count(&self) -> usize {
        0
    }

    async fn get_live_session_ids(&self) -> Vec<Uuid> {
        Vec::new()
    }

    async fn spawn_scheduled_session(&self, _trigger: ScheduleTrigger) -> Result<(), String> {
        Err("SessionManager not available".to_string())
    }

    fn default_model(&self) -> String {
        String::new()
    }
}

/// Type alias used throughout the engine for the boxed hooks handle.
pub type Hooks = Arc<dyn SchedulerHooks>;
