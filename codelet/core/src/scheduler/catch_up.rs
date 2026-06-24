//! Catch-up on restart — RPC-058 lift from
//! codelet/napi/src/scheduler/catch_up.rs.
//!
//! On scheduler startup, checks each active schedule for missed triggers.
//! If lastRunAt < previous cron trigger time, fires at most ONE catch-up run.
//! Runs once before the regular 30-second tick loop begins.

use super::cron_utils::{self, MAX_SESSIONS};
use super::state::SchedulerState;
use super::types::{ScheduleEntry, SchedulesFile};
use super::Hooks;
use chrono::Utc;
use std::path::Path;
use tracing::{error, info};

/// Run catch-up checks for all schedules on startup.
///
/// For each active schedule, detects if a cron trigger was missed while
/// fspec was closed. Fires at most ONE catch-up per schedule.
/// Respects overlap policies and session limits via SchedulerState.
pub async fn run_catch_up(
    project_path: &str,
    state: &SchedulerState,
    hooks: Hooks,
) -> Result<(), anyhow::Error> {
    let schedules_path = Path::new(project_path).join("spec/schedules.json");

    // Read schedules — missing file is not an error
    let content = match tokio::fs::read_to_string(&schedules_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            info!("No schedules.json found — skipping catch-up");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to read schedules.json for catch-up: {}", e);
            return Ok(());
        }
    };

    let schedules_file: SchedulesFile = match serde_json::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            error!("Malformed schedules.json during catch-up: {}", e);
            return Ok(());
        }
    };

    let now = Utc::now();
    let mut catch_up_count = 0;

    for (name, schedule) in &schedules_file.schedules {
        if needs_catch_up(schedule, now) {
            info!("Catch-up needed for schedule '{}'", name);
            catch_up_count += 1;

            // Update lastRunAt immediately to prevent double-fire on first tick
            if let Err(e) = update_last_run_now(&schedules_path, name).await {
                error!("Failed to update lastRunAt for catch-up '{}': {}", name, e);
                continue;
            }

            // Fire the catch-up job through normal evaluate_and_run path
            // This respects overlap policy and session limits via state
            let action = state.check_overlap(name, schedule).await;
            match action {
                super::state::OverlapAction::Skip => {
                    info!("Catch-up for '{}' skipped (overlap policy)", name);
                    continue;
                }
                super::state::OverlapAction::Queue => {
                    state.enqueue(name, schedule).await;
                    info!("Catch-up for '{}' queued (previous run active)", name);
                    continue;
                }
                super::state::OverlapAction::Proceed => {}
            }

            // Check session limit for agent jobs
            if schedule.job_type == "agent" {
                let session_count = hooks.get_session_count().await;
                if session_count >= MAX_SESSIONS {
                    state.defer(name, schedule).await;
                    info!("Catch-up for '{}' deferred (session limit)", name);
                    continue;
                }
            }

            // Trigger the job
            let schedules_path_clone = schedules_path.clone();
            if let Err(e) = super::engine::trigger_catch_up_job(
                &schedules_path_clone,
                name,
                &schedule.job_type,
                project_path,
                schedule,
                state,
                hooks.clone(),
            )
            .await
            {
                error!("Catch-up job '{}' failed: {}", name, e);
            }
        }
    }

    if catch_up_count > 0 {
        info!("Catch-up complete: {} schedule(s) fired", catch_up_count);
    } else {
        info!("Catch-up complete: no missed triggers detected");
    }

    Ok(())
}

/// Check if a schedule needs catch-up (missed a trigger while fspec was closed).
pub fn needs_catch_up(schedule: &ScheduleEntry, now: chrono::DateTime<Utc>) -> bool {
    // Only active schedules
    if schedule.status != "active" {
        return false;
    }

    // Parse timezone using shared utility
    let tz = match cron_utils::parse_timezone(&schedule.timezone, "catch-up check") {
        Ok(tz) => tz,
        Err(_) => return false,
    };

    // Parse cron using shared utility
    let cron = match cron_utils::parse_cron(&schedule.cron, "catch-up check") {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Use shared should_trigger logic (same as engine evaluation)
    cron_utils::should_trigger(&cron, &tz, schedule.last_run_at.as_deref(), now)
}

/// Update lastRunAt to now for a schedule (prevents double-fire on first tick).
async fn update_last_run_now(schedules_path: &Path, name: &str) -> Result<(), anyhow::Error> {
    let content = tokio::fs::read_to_string(schedules_path).await?;
    let mut schedules: SchedulesFile = serde_json::from_str(&content)?;

    if let Some(entry) = schedules.schedules.get_mut(name) {
        entry.last_run_at = Some(Utc::now().to_rfc3339());
        entry.last_run_status = Some("catch-up".to_string());
    }

    let json = serde_json::to_string_pretty(&schedules)?;
    tokio::fs::write(schedules_path, json).await?;
    Ok(())
}
