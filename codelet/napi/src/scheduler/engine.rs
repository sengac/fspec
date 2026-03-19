//! Scheduler engine — SCHED-003
//!
//! Core timer loop that evaluates cron schedules every 30 seconds.
//! Reads spec/schedules.json each tick, evaluates cron expressions
//! with timezone support, and triggers jobs via stubs (SCHED-004/005).
//! Also evaluates session-scoped /loop entries (SCHED-011).

use super::types::{EvaluationResult, ScheduleEntry, SchedulesFile};
use super::state::{OverlapAction, SchedulerState};
use super::job_log::{append_log_entry, JobLogEntry};
use super::cron_utils::{self, MAX_SESSIONS};
use super::loop_store::LoopStore;
use super::trigger;
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// Spawn the scheduler as a background tokio task.
///
/// Runs every 30 seconds, reading and evaluating schedules from
/// `{project_path}/spec/schedules.json`.
///
/// Requires a Tokio runtime `Handle` so this can be called from both
/// async contexts (session creation) and sync NAPI functions (/loop).
///
/// Returns a JoinHandle that can be used for graceful shutdown.
pub fn spawn_scheduler(project_path: String, handle: &tokio::runtime::Handle) -> JoinHandle<()> {
    handle.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let state = Arc::new(SchedulerState::new());
        info!("Scheduler started for project: {}", project_path);

        // SCHED-007: Run catch-up once before the tick loop
        if let Err(e) = super::catch_up::run_catch_up(&project_path, &state).await {
            error!("Catch-up check failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = evaluate_and_run(&project_path, &state).await {
                error!("Scheduler tick error: {}", e);
            }
        }
    })
}

/// Evaluate all schedules and run any that should trigger.
///
/// This is the "full tick" function: sweep completed → evaluate → overlap check →
/// session limit check → trigger → update timestamps → drain queues.
/// Returns Ok(()) on success, or an error if something unexpected fails.
/// Missing/malformed schedules.json is handled gracefully (not an error).
pub async fn evaluate_and_run(
    project_path: &str,
    state: &SchedulerState,
) -> Result<(), anyhow::Error> {
    // Step 1: Sweep active_runs — detect completed sessions
    let live_ids = get_live_session_ids().await;
    let completed = state.sweep_completed(&live_ids).await;

    // Step 2: Drain queued jobs for completed schedules (one per completed schedule)
    for completed_name in &completed {
        if let Some(queued) = state.drain_queued_for(completed_name).await {
            let schedules_path = Path::new(project_path).join("spec/schedules.json");
            info!(
                "Firing queued job for '{}' (previous run completed)",
                queued.schedule_name
            );
            if let Err(e) = trigger::trigger_and_update(
                &schedules_path,
                &queued.schedule_name,
                &queued.entry.job_type,
                project_path,
                &queued.entry,
                state,
            )
            .await
            {
                error!("Failed queued job '{}': {}", queued.schedule_name, e);
            }
        }
    }

    // Step 3: Process ONE deferred job if a session slot is available
    let session_count = get_session_count().await;
    if session_count < max_sessions() {
        if let Some(deferred) = state.drain_one_deferred().await {
            let schedules_path = Path::new(project_path).join("spec/schedules.json");
            info!(
                "Firing deferred job '{}' (session slot available)",
                deferred.schedule_name
            );
            if let Err(e) = trigger::trigger_and_update(
                &schedules_path,
                &deferred.schedule_name,
                &deferred.entry.job_type,
                project_path,
                &deferred.entry,
                state,
            )
            .await
            {
                error!("Failed deferred job '{}': {}", deferred.schedule_name, e);
            }
        }
    }

    // Step 4: Evaluate cron schedules
    let results = evaluate_schedules(project_path).await?;

    for result in &results {
        if !result.triggered {
            continue;
        }

        // Step 5: Overlap check — BEFORE session limit check
        let action = state.check_overlap(&result.name, &result.entry).await;
        let log_path = Path::new(project_path).join("spec/schedule-log.jsonl");
        match action {
            OverlapAction::Skip => {
                let entry = JobLogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    event: "skipped".to_string(),
                    schedule: result.name.clone(),
                    job_type: result.job_type.clone(),
                    session_id: None,
                    duration_ms: None,
                    exit_code: None,
                    error: None,
                    message: Some("Previous run still active".to_string()),
                };
                tokio::spawn(async move { append_log_entry(&log_path, &entry).await });
                continue;
            }
            OverlapAction::Queue => {
                state.enqueue(&result.name, &result.entry).await;
                let entry = JobLogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    event: "queued".to_string(),
                    schedule: result.name.clone(),
                    job_type: result.job_type.clone(),
                    session_id: None,
                    duration_ms: None,
                    exit_code: None,
                    error: None,
                    message: None,
                };
                tokio::spawn(async move { append_log_entry(&log_path, &entry).await });
                continue;
            }
            OverlapAction::Proceed => {}
        }

        // Step 6: Session limit check (agent jobs only)
        if result.job_type == "agent" {
            let current_count = get_session_count().await;
            if current_count >= max_sessions() {
                state.defer(&result.name, &result.entry).await;
                let entry = JobLogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    event: "deferred".to_string(),
                    schedule: result.name.clone(),
                    job_type: result.job_type.clone(),
                    session_id: None,
                    duration_ms: None,
                    exit_code: None,
                    error: None,
                    message: Some(format!("{}/{} sessions active", current_count, max_sessions())),
                };
                let log_path = Path::new(project_path).join("spec/schedule-log.jsonl");
                tokio::spawn(async move { append_log_entry(&log_path, &entry).await });
                continue;
            }
        }

        // Step 7: Trigger the job
        let schedules_path = Path::new(project_path).join("spec/schedules.json");
        if let Err(e) = trigger::trigger_and_update(
            &schedules_path,
            &result.name,
            &result.job_type,
            project_path,
            &result.entry,
            state,
        )
        .await
        {
            error!("Failed to trigger job '{}': {}", result.name, e);
        }
    }

    // Step 8: Evaluate session-scoped /loop entries (SCHED-011)
    evaluate_and_fire_loops().await;

    Ok(())
}

/// Get the number of MAX_SESSIONS (constant).
fn max_sessions() -> usize {
    MAX_SESSIONS
}

/// Get the current count of live sessions from SessionManager.
async fn get_session_count() -> usize {
    let sm = crate::session_manager::SessionManager::instance();
    sm.session_count().await
}

/// Get all live session IDs from SessionManager.
async fn get_live_session_ids() -> Vec<uuid::Uuid> {
    let sm = crate::session_manager::SessionManager::instance();
    sm.live_session_ids().await
}

/// Evaluate all schedules and return which ones should trigger.
///
/// Does NOT actually trigger jobs — just evaluates cron conditions.
/// Returns an empty Vec for missing or malformed schedules files (not an error).
pub async fn evaluate_schedules(
    project_path: &str,
) -> Result<Vec<EvaluationResult>, anyhow::Error> {
    let schedules_path = Path::new(project_path).join("spec/schedules.json");

    // Read schedules file — missing file is not an error
    let content = match tokio::fs::read_to_string(&schedules_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            warn!("No schedules.json found at {}", schedules_path.display());
            return Ok(Vec::new());
        }
        Err(e) => {
            error!(
                "Failed to read schedules.json at {}: {}",
                schedules_path.display(),
                e
            );
            return Ok(Vec::new());
        }
    };

    // Parse JSON — malformed JSON is not an error (log and skip)
    let schedules_file: SchedulesFile = match serde_json::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            error!("Malformed schedules.json: {}", e);
            return Ok(Vec::new());
        }
    };

    let now = Utc::now();
    let mut results = Vec::new();

    for (name, schedule) in &schedules_file.schedules {
        let result = evaluate_single_schedule(name, schedule, now);
        results.push(result);
    }

    Ok(results)
}

/// Evaluate a single schedule against the current time.
fn evaluate_single_schedule(
    name: &str,
    schedule: &ScheduleEntry,
    now: chrono::DateTime<Utc>,
) -> EvaluationResult {
    // Skip paused schedules
    if schedule.status != "active" {
        return EvaluationResult {
            name: name.to_string(),
            triggered: false,
            job_type: schedule.job_type.clone(),
            evaluated_timezone: schedule.timezone.clone(),
            error: None,
            entry: schedule.clone(),
        };
    }

    // Parse timezone using shared utility
    let tz = match cron_utils::parse_timezone(&schedule.timezone, &format!("schedule '{}'", name)) {
        Ok(tz) => tz,
        Err(_) => {
            return EvaluationResult {
                name: name.to_string(),
                triggered: false,
                job_type: schedule.job_type.clone(),
                evaluated_timezone: schedule.timezone.clone(),
                error: Some(format!("Invalid timezone: {}", schedule.timezone)),
                entry: schedule.clone(),
            };
        }
    };

    // Parse cron expression using shared utility
    let cron = match cron_utils::parse_cron(&schedule.cron, &format!("schedule '{}'", name)) {
        Ok(c) => c,
        Err(e) => {
            return EvaluationResult {
                name: name.to_string(),
                triggered: false,
                job_type: schedule.job_type.clone(),
                evaluated_timezone: schedule.timezone.clone(),
                error: Some(e),
                entry: schedule.clone(),
            };
        }
    };

    // Use shared should_trigger logic
    let should_trigger = cron_utils::should_trigger(
        &cron,
        &tz,
        schedule.last_run_at.as_deref(),
        now,
    );

    EvaluationResult {
        name: name.to_string(),
        triggered: should_trigger,
        job_type: schedule.job_type.clone(),
        evaluated_timezone: schedule.timezone.clone(),
        error: None,
        entry: schedule.clone(),
    }
}

/// SCHED-007: Public entry point for catch-up job triggering.
/// Delegates to trigger_and_update with full overlap/session tracking.
pub async fn trigger_catch_up_job(
    schedules_path: &Path,
    name: &str,
    job_type: &str,
    project_path: &str,
    entry: &ScheduleEntry,
    state: &SchedulerState,
) -> Result<(), anyhow::Error> {
    trigger::trigger_and_update(schedules_path, name, job_type, project_path, entry, state).await
}

/// SCHED-011: Evaluate session-scoped /loop entries and fire due prompts.
///
/// For each due loop entry, checks that the originating session is idle,
/// then sends the prompt directly via `session.send_input()`. This runs
/// the prompt in the SAME session that created the loop — not a new one.
async fn evaluate_and_fire_loops() {
    let store = LoopStore::instance();

    // Fast path: skip if no loops registered
    if store.is_empty().await {
        return;
    }

    // Purge expired entries first
    store.purge_expired().await;

    let due = store.get_due().await;
    if due.is_empty() {
        return;
    }

    let sm = crate::session_manager::SessionManager::instance();

    for entry in &due {
        // Check that the session still exists and is idle
        let session_id_str = entry.session_id.to_string();
        let session = match sm.get_session(&session_id_str) {
            Ok(s) => s,
            Err(_) => {
                // Session was destroyed — remove the loop
                warn!(
                    "Loop {}: session {} no longer exists, removing",
                    entry.id, entry.session_id
                );
                store.cancel(&entry.id).await;
                continue;
            }
        };

        // Only fire if session is idle (skip policy — don't queue)
        if session.get_status() != crate::session_manager::SessionStatus::Idle {
            info!(
                "Loop {}: session {} is busy, skipping this tick",
                entry.id, entry.session_id
            );
            continue;
        }

        // Fire the prompt into the originating session
        info!(
            "Loop {} firing: prompt='{}' → session {}",
            entry.id, entry.prompt, entry.session_id
        );
        match session.send_input(entry.prompt.clone(), None) {
            Ok(()) => {
                store.mark_executed(&entry.id).await;
            }
            Err(e) => {
                error!(
                    "Loop {}: failed to send prompt to session {}: {}",
                    entry.id, entry.session_id, e
                );
            }
        }

        // Only fire one loop per session per tick to avoid flooding
        break;
    }
}
