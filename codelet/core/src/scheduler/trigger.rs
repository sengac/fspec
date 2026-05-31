//! Job trigger and persistence — RPC-058 lift from
//! codelet/napi/src/scheduler/trigger.rs.
//!
//! Handles the trigger_and_update cycle: log "triggered", execute job,
//! log result, and update schedules.json with last_run_at/status.

use super::job_log::{append_log_entry, JobLogEntry};
use super::state::SchedulerState;
use super::types::{ScheduleEntry, SchedulesFile};
use super::Hooks;
use chrono::Utc;
use std::path::Path;
use tracing::{info, warn};

/// Trigger a job and update the schedules.json file with the result.
pub async fn trigger_and_update(
    schedules_path: &Path,
    name: &str,
    job_type: &str,
    project_path: &str,
    entry: &ScheduleEntry,
    state: &SchedulerState,
    hooks: Hooks,
) -> Result<(), anyhow::Error> {
    let log_path = Path::new(project_path).join("spec/schedule-log.jsonl");
    let start = std::time::Instant::now();

    // Log "triggered" event
    let session_id_for_log = hooks
        .find_session_by_schedule_name(name)
        .await
        .map(|s| s.to_string());
    let triggered_entry = JobLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        event: "triggered".to_string(),
        schedule: name.to_string(),
        job_type: job_type.to_string(),
        session_id: session_id_for_log.clone(),
        duration_ms: None,
        exit_code: None,
        error: None,
        message: None,
    };
    append_log_entry(&log_path, &triggered_entry).await;

    // Execute the job
    let status = match job_type {
        "agent" => {
            let default_model = hooks.default_model();
            let result = super::agent_job::trigger_agent_job_from_entry(
                name,
                project_path,
                entry,
                &default_model,
                hooks.clone(),
            )
            .await;
            // If agent job succeeded, record the active run for overlap detection
            if result.is_ok() {
                if let Some(sid) = hooks.find_session_by_schedule_name(name).await {
                    state.record_active_run(name, sid).await;
                }
            }
            result
        }
        "shell" => trigger_shell_job(name, project_path, entry).await,
        other => {
            warn!("Unknown job type '{}' for schedule '{}'", other, name);
            Ok(())
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // Log completed/failed event
    let (event, error_msg, exit_code) = match &status {
        Ok(()) => ("completed".to_string(), None, None),
        Err(e) => {
            let msg = e.to_string();
            let code = if msg.contains("exit code") {
                msg.split("exit code ")
                    .nth(1)
                    .and_then(|s| s.trim().parse::<i32>().ok())
            } else {
                None
            };
            ("failed".to_string(), Some(msg), code)
        }
    };

    let result_entry = JobLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        event,
        schedule: name.to_string(),
        job_type: job_type.to_string(),
        session_id: hooks
            .find_session_by_schedule_name(name)
            .await
            .map(|s| s.to_string()),
        duration_ms: Some(duration_ms),
        exit_code,
        error: error_msg,
        message: None,
    };
    let log_path_clone = log_path.clone();
    tokio::spawn(async move { append_log_entry(&log_path_clone, &result_entry).await });

    let run_status = match &status {
        Ok(()) => "success",
        Err(_) => "error",
    };

    // Update last_run_at and last_run_status in schedules.json
    update_last_run(schedules_path, name, run_status).await?;

    status
}

/// Update last_run_at and last_run_status for a schedule.
pub async fn update_last_run(
    schedules_path: &Path,
    name: &str,
    status: &str,
) -> Result<(), anyhow::Error> {
    let content = tokio::fs::read_to_string(schedules_path).await?;
    let mut schedules: SchedulesFile = serde_json::from_str(&content)?;

    if let Some(entry) = schedules.schedules.get_mut(name) {
        entry.last_run_at = Some(Utc::now().to_rfc3339());
        entry.last_run_status = Some(status.to_string());
    }

    let json = serde_json::to_string_pretty(&schedules)?;
    tokio::fs::write(schedules_path, json).await?;

    info!("Updated schedule '{}': status={}", name, status);
    Ok(())
}

/// SCHED-005: Trigger a shell job by delegating to the shell_job module.
async fn trigger_shell_job(
    name: &str,
    project_path: &str,
    entry: &ScheduleEntry,
) -> Result<(), anyhow::Error> {
    let result = super::shell_job::trigger_shell_job(name, project_path, entry).await?;
    if result.exit_code != 0 {
        return Err(anyhow::anyhow!(
            "Shell job '{}' failed with exit code {}",
            name,
            result.exit_code
        ));
    }
    Ok(())
}
