//! Scheduler module — SCHED-003
//!
//! Core scheduler engine: a tokio task that evaluates cron schedules
//! from `spec/schedules.json` every 30 seconds and triggers jobs.
//! Also evaluates session-scoped /loop entries (SCHED-011).

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
