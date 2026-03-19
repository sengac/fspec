//! Scheduler module — SCHED-003
//!
//! Core scheduler engine: a tokio task that evaluates cron schedules
//! from `spec/schedules.json` every 30 seconds and triggers jobs.
//!
//! SCHED-013: Session-scoped /loop entries are now self-managed via
//! per-entry spawned tokio tasks in LoopStore (decoupled from the
//! engine's 30-second tick).

pub mod agent_job;
pub mod catch_up;
pub mod cron_utils;
pub mod engine;
pub mod job_log;
pub mod loop_store;
pub mod shell_job;
pub mod state;
pub mod trigger;
pub mod types;

pub use engine::{evaluate_and_run, evaluate_schedules, spawn_scheduler};
pub use loop_store::LoopStore;
pub use state::SchedulerState;
pub use types::EvaluationResult;
