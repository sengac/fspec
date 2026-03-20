//! Scheduler state — SCHED-006
//!
//! In-memory state for overlap detection, job queueing, and session limit deferral.
//! The SchedulerState is created once when the scheduler spawns and passed by
//! reference to evaluate_and_run on each tick.

use super::types::ScheduleEntry;
use std::collections::{HashMap, VecDeque};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// In-memory scheduler state tracking active runs, queued, and deferred jobs.
pub struct SchedulerState {
    /// Currently running sessions per schedule: schedule_name → session_id.
    /// Only agent jobs are tracked here (shell jobs use processes, not sessions).
    pub active_runs: RwLock<HashMap<String, Uuid>>,
    /// Jobs waiting for the SAME schedule's previous run to complete (overlap=queue).
    pub queued_jobs: RwLock<VecDeque<QueuedJob>>,
    /// Jobs waiting for ANY session slot to open (session limit reached).
    pub deferred_jobs: RwLock<VecDeque<DeferredJob>>,
}

/// A job queued due to overlap policy = "queue".
#[derive(Debug, Clone)]
pub struct QueuedJob {
    pub schedule_name: String,
    pub entry: ScheduleEntry,
}

/// A job deferred because MAX_SESSIONS was reached.
#[derive(Debug, Clone)]
pub struct DeferredJob {
    pub schedule_name: String,
    pub entry: ScheduleEntry,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerState {
    /// Create a new empty scheduler state.
    pub fn new() -> Self {
        Self {
            active_runs: RwLock::new(HashMap::new()),
            queued_jobs: RwLock::new(VecDeque::new()),
            deferred_jobs: RwLock::new(VecDeque::new()),
        }
    }

    /// Check overlap policy for a schedule. Returns the action to take.
    pub async fn check_overlap(&self, name: &str, entry: &ScheduleEntry) -> OverlapAction {
        let active = self.active_runs.read().await;
        if !active.contains_key(name) {
            return OverlapAction::Proceed;
        }

        let policy = entry.overlap_policy.as_deref().unwrap_or("skip");
        match policy {
            "queue" => {
                info!("Schedule '{}': previous run active, queuing", name);
                OverlapAction::Queue
            }
            _ => {
                // Default is "skip"
                info!("Schedule '{}': previous run active, skipping", name);
                OverlapAction::Skip
            }
        }
    }

    /// Enqueue a job for a schedule with overlap=queue policy.
    /// Replaces any existing queued entry for the same schedule (no duplicates).
    pub async fn enqueue(&self, name: &str, entry: &ScheduleEntry) {
        let mut queue = self.queued_jobs.write().await;
        // Remove existing entry for the same schedule (replace semantics)
        queue.retain(|j| j.schedule_name != name);
        queue.push_back(QueuedJob {
            schedule_name: name.to_string(),
            entry: entry.clone(),
        });
    }

    /// Defer a job because the session limit was reached.
    /// Replaces any existing deferred entry for the same schedule.
    pub async fn defer(&self, name: &str, entry: &ScheduleEntry) {
        let mut deferred = self.deferred_jobs.write().await;
        deferred.retain(|j| j.schedule_name != name);
        deferred.push_back(DeferredJob {
            schedule_name: name.to_string(),
            entry: entry.clone(),
        });
        info!("Schedule '{}': deferred (session limit reached)", name);
    }

    /// Record an active agent session for a schedule.
    pub async fn record_active_run(&self, name: &str, session_id: Uuid) {
        self.active_runs
            .write()
            .await
            .insert(name.to_string(), session_id);
    }

    /// Sweep active_runs: remove entries whose session IDs are no longer alive.
    /// Returns the names of schedules whose runs completed.
    pub async fn sweep_completed(&self, live_session_ids: &[Uuid]) -> Vec<String> {
        let mut active = self.active_runs.write().await;
        let mut completed = Vec::new();
        active.retain(|name, sid| {
            if live_session_ids.contains(sid) {
                true
            } else {
                completed.push(name.clone());
                false
            }
        });
        completed
    }

    /// Drain ONE queued job for a schedule that just completed.
    /// Returns the job to fire, if any.
    pub async fn drain_queued_for(&self, completed_name: &str) -> Option<QueuedJob> {
        let mut queue = self.queued_jobs.write().await;
        if let Some(pos) = queue
            .iter()
            .position(|j| j.schedule_name == completed_name)
        {
            queue.remove(pos)
        } else {
            None
        }
    }

    /// Drain ONE deferred job (FIFO). Returns the job to fire, if any.
    pub async fn drain_one_deferred(&self) -> Option<DeferredJob> {
        self.deferred_jobs.write().await.pop_front()
    }
}

/// Action to take after checking overlap policy.
#[derive(Debug, PartialEq)]
pub enum OverlapAction {
    /// No active run — proceed with trigger.
    Proceed,
    /// Active run + skip policy — do not trigger.
    Skip,
    /// Active run + queue policy — enqueue for later.
    Queue,
}
