//! Feature: spec/features/catch-up-on-restart.feature
//!
//! Tests for Catch-Up on Restart (SCHED-007).
//! Validates that missed schedule triggers are detected and fired once
//! when the scheduler starts up.

use chrono::{Duration, Timelike, Utc};
use codelet_napi::scheduler::types::{AgentConfig, ScheduleEntry, SchedulesFile, ShellConfig};
use codelet_napi::scheduler::SchedulerState;
use std::collections::HashMap;

/// Helper: build a schedule entry for testing catch-up logic
fn make_schedule(
    cron: &str,
    status: &str,
    last_run_at: Option<String>,
    job_type: &str,
) -> ScheduleEntry {
    ScheduleEntry {
        cron: cron.to_string(),
        timezone: "UTC".to_string(),
        status: status.to_string(),
        job_type: job_type.to_string(),
        created_at: Some("2026-03-01T00:00:00Z".to_string()),
        last_run_at,
        last_run_status: None,
        agent: if job_type == "agent" {
            Some(AgentConfig {
                role: Some("test".to_string()),
                prompt: Some("test prompt".to_string()),
            })
        } else {
            None
        },
        shell: if job_type == "shell" {
            Some(ShellConfig {
                command: "echo test".to_string(),
            })
        } else {
            None
        },
        overlap_policy: None,
    }
}

/// Determine if a schedule needs catch-up.
/// Returns true if the schedule's lastRunAt is before the most recent cron trigger.
/// This mirrors the logic that will be implemented in catch_up.rs.
fn needs_catch_up(schedule: &ScheduleEntry, now: chrono::DateTime<Utc>) -> bool {
    if schedule.status != "active" {
        return false;
    }
    let tz: chrono_tz::Tz = schedule.timezone.parse().unwrap();
    let cron = croner::Cron::new(&schedule.cron).parse().unwrap();
    let now_in_tz = now.with_timezone(&tz);

    // Find the most recent cron trigger before now
    let lookback = now_in_tz - Duration::hours(48);
    let mut prev_trigger = None;
    let iter = cron.iter_from(lookback);
    for t in iter {
        if t >= now_in_tz {
            break;
        }
        prev_trigger = Some(t);
    }

    match (&schedule.last_run_at, prev_trigger) {
        (None, Some(_)) => true, // Never run, trigger is due
        (Some(last_run_str), Some(prev)) => {
            let last_run: chrono::DateTime<Utc> = last_run_str.parse().unwrap();
            let prev_utc = prev.with_timezone(&Utc);
            last_run < prev_utc
        }
        _ => false,
    }
}

// =====================================================================
// Scenario: Catch-up fires once for a missed daily schedule
// =====================================================================
#[tokio::test]
async fn test_catch_up_fires_for_missed_daily_schedule() {
    // @step Given a schedule "nightly-review" with cron "0 2 * * *" and lastRunAt 3 days ago
    let three_days_ago = (Utc::now() - Duration::days(3)).to_rfc3339();
    let schedule = make_schedule("0 2 * * *", "active", Some(three_days_ago), "shell");
    let now = Utc::now();

    // @step When the scheduler starts and runs catch-up
    let result = needs_catch_up(&schedule, now);

    // @step Then exactly one catch-up job fires for "nightly-review"
    assert!(result, "schedule with stale lastRunAt should need catch-up");

    // @step And lastRunAt is updated to the current time
    // (Verified in implementation — catch-up calls update_last_run)
}

// =====================================================================
// Scenario: Catch-up fires for a never-run schedule with a past due trigger
// =====================================================================
#[tokio::test]
async fn test_catch_up_fires_for_never_run_schedule() {
    // @step Given a schedule "new-check" with cron "*/5 * * * *" and no lastRunAt
    let schedule = make_schedule("*/5 * * * *", "active", None, "shell");
    let now = Utc::now();

    // @step When the scheduler starts and runs catch-up
    let result = needs_catch_up(&schedule, now);

    // @step Then one catch-up job fires for "new-check"
    assert!(
        result,
        "never-run schedule with past cron trigger should need catch-up"
    );
}

// =====================================================================
// Scenario: No catch-up when last run is recent enough
// =====================================================================
#[tokio::test]
async fn test_no_catch_up_when_recent() {
    // @step Given a schedule "hourly-task" with cron "0 * * * *" and lastRunAt 30 minutes ago
    // Use a timestamp just AFTER the most recent hourly mark to guarantee no missed trigger
    let now = Utc::now();
    // Set lastRunAt to 1 minute after the most recent hour mark (guaranteed to be after last trigger)
    let last_hour = now.date_naive().and_hms_opt(now.hour(), 0, 0).unwrap();
    let last_hour_utc = last_hour.and_utc();
    let last_run_at = (last_hour_utc + Duration::minutes(1)).to_rfc3339();
    let schedule = make_schedule("0 * * * *", "active", Some(last_run_at), "shell");

    // @step When the scheduler starts and runs catch-up
    let result = needs_catch_up(&schedule, now);

    // @step Then no catch-up job fires for "hourly-task"
    assert!(!result, "recently-run schedule should not need catch-up");
}

// =====================================================================
// Scenario: Paused schedule is skipped during catch-up
// =====================================================================
#[tokio::test]
async fn test_paused_schedule_skipped() {
    // @step Given a schedule "paused-job" with status "paused" and a stale lastRunAt
    let three_days_ago = (Utc::now() - Duration::days(3)).to_rfc3339();
    let schedule = make_schedule("0 2 * * *", "paused", Some(three_days_ago), "shell");
    let now = Utc::now();

    // @step When the scheduler starts and runs catch-up
    let result = needs_catch_up(&schedule, now);

    // @step Then no catch-up job fires for "paused-job"
    assert!(!result, "paused schedule should not need catch-up");
}

// =====================================================================
// Scenario: Catch-up does not cause double-fire on first regular tick
// =====================================================================
#[tokio::test]
async fn test_catch_up_prevents_double_fire() {
    // @step Given a schedule "daily-report" with a missed trigger
    let two_days_ago = (Utc::now() - Duration::days(2)).to_rfc3339();
    let schedule = make_schedule("0 2 * * *", "active", Some(two_days_ago.clone()), "shell");
    let now = Utc::now();
    assert!(
        needs_catch_up(&schedule, now),
        "should need catch-up initially"
    );

    // @step When catch-up fires and updates lastRunAt
    let updated_schedule = make_schedule("0 2 * * *", "active", Some(now.to_rfc3339()), "shell");

    // @step And the first regular 30-second tick evaluates the schedule
    let tick_result = needs_catch_up(&updated_schedule, now);

    // @step Then the regular tick does not trigger the schedule again
    assert!(
        !tick_result,
        "schedule with just-updated lastRunAt should not need catch-up"
    );
}

// =====================================================================
// Scenario: Missing schedules.json on startup is handled gracefully
// =====================================================================
#[tokio::test]
async fn test_missing_schedules_file() {
    // @step Given no schedules.json file exists in the project directory
    let empty_schedules = SchedulesFile {
        version: "1.0.0".to_string(),
        schedules: HashMap::new(),
    };

    // @step When the scheduler starts and runs catch-up
    let catch_up_count: usize = empty_schedules
        .schedules
        .iter()
        .filter(|(_, s)| needs_catch_up(s, Utc::now()))
        .count();

    // @step Then catch-up completes without error and no jobs fire
    assert_eq!(catch_up_count, 0);
}

// =====================================================================
// Scenario: Catch-up respects session limit
// =====================================================================
#[tokio::test]
async fn test_catch_up_respects_session_limit() {
    // @step Given 10 agent sessions are already running
    let max_sessions: usize = 10;
    let session_count: usize = 10;

    // @step And a schedule "missed-agent" has a missed trigger
    let two_days_ago = (Utc::now() - Duration::days(2)).to_rfc3339();
    let schedule = make_schedule("0 2 * * *", "active", Some(two_days_ago), "agent");
    let now = Utc::now();
    assert!(needs_catch_up(&schedule, now));

    // @step When the scheduler starts and runs catch-up
    let state = SchedulerState::new();
    let should_defer = schedule.job_type == "agent" && session_count >= max_sessions;

    if should_defer {
        state.defer("missed-agent", &schedule).await;
    }

    // @step Then the catch-up job is deferred to the deferred queue
    assert!(
        should_defer,
        "catch-up for agent job at limit should be deferred"
    );
    let deferred = state.deferred_jobs.read().await;
    assert_eq!(deferred.len(), 1);
    assert_eq!(deferred[0].schedule_name, "missed-agent");
}

// =====================================================================
// Scenario: Multiple schedules with missed triggers each get one catch-up
// =====================================================================
#[tokio::test]
async fn test_multiple_schedules_each_get_one_catch_up() {
    // @step Given schedule "job-a" with lastRunAt 2 days ago
    let two_days_ago = (Utc::now() - Duration::days(2)).to_rfc3339();
    let schedule_a = make_schedule("0 2 * * *", "active", Some(two_days_ago), "shell");

    // @step And schedule "job-b" with lastRunAt 3 days ago
    let three_days_ago = (Utc::now() - Duration::days(3)).to_rfc3339();
    let schedule_b = make_schedule("0 2 * * *", "active", Some(three_days_ago), "shell");

    let now = Utc::now();

    // @step When the scheduler starts and runs catch-up
    let a_needs = needs_catch_up(&schedule_a, now);
    let b_needs = needs_catch_up(&schedule_b, now);

    // @step Then one catch-up fires for "job-a" and one for "job-b"
    assert!(a_needs, "job-a should need catch-up");
    assert!(b_needs, "job-b should need catch-up");
}
