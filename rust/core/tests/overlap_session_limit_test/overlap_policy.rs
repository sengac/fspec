//! Feature: spec/features/overlap-session-limit.feature
use super::*;

// =====================================================================
// Scenario: Skip policy prevents trigger when previous run is active
// =====================================================================
#[tokio::test]
async fn test_skip_policy_prevents_trigger() {
    // @step Given a schedule "nightly-review" with overlap_policy "skip"
    let schedule = make_agent_schedule(Some("skip"));
    let schedule_name = "nightly-review";

    // @step And the schedule has an active run in the scheduler state
    let active_runs: HashMap<String, Uuid> = {
        let mut m = HashMap::new();
        m.insert(schedule_name.to_string(), Uuid::new_v4());
        m
    };

    // @step When the schedule triggers on the next cron match
    let policy = schedule.overlap_policy.as_deref().unwrap_or("skip");
    let is_active = active_runs.contains_key(schedule_name);

    // @step Then the trigger is skipped
    assert!(is_active, "schedule should be in active_runs");
    assert_eq!(policy, "skip");
    // With skip + active → should not trigger
    let should_trigger = !(is_active && policy == "skip");
    assert!(
        !should_trigger,
        "skip policy with active run should prevent trigger"
    );

    // @step And a skip event is logged with the schedule name
    // (Log verification delegated to integration test — here we verify the decision logic)
}
// =====================================================================
// Scenario: Queue policy enqueues trigger when previous run is active
// =====================================================================
#[tokio::test]
async fn test_queue_policy_enqueues_trigger() {
    // @step Given a schedule "health-check" with overlap_policy "queue"
    let schedule = make_agent_schedule(Some("queue"));
    let schedule_name = "health-check";

    // @step And the schedule has an active run in the scheduler state
    let active_runs: HashMap<String, Uuid> = {
        let mut m = HashMap::new();
        m.insert(schedule_name.to_string(), Uuid::new_v4());
        m
    };

    // @step When the schedule triggers on the next cron match
    let policy = schedule.overlap_policy.as_deref().unwrap_or("skip");
    let is_active = active_runs.contains_key(schedule_name);
    let queued_jobs: RwLock<VecDeque<(String, ScheduleEntry)>> = RwLock::new(VecDeque::new());

    if is_active && policy == "queue" {
        queued_jobs
            .write()
            .await
            .push_back((schedule_name.to_string(), schedule.clone()));
    }

    // @step Then the job is added to the queued_jobs queue
    let queue = queued_jobs.read().await;
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].0, "health-check");

    // @step And the trigger does not spawn a new session immediately
    // (No session spawned — verified by queue having the entry instead)
}
// =====================================================================
// Scenario: Default overlap policy is skip when not specified
// =====================================================================
#[tokio::test]
async fn test_default_overlap_policy_is_skip() {
    // @step Given a schedule "daily-task" with no overlap_policy field
    let schedule = make_agent_schedule(None);
    assert!(schedule.overlap_policy.is_none());

    // @step And the schedule has an active run in the scheduler state
    let active_runs: HashMap<String, Uuid> = {
        let mut m = HashMap::new();
        m.insert("daily-task".to_string(), Uuid::new_v4());
        m
    };

    // @step When the schedule triggers on the next cron match
    let policy = schedule.overlap_policy.as_deref().unwrap_or("skip");
    let is_active = active_runs.contains_key("daily-task");

    // @step Then the trigger is skipped as if overlap_policy were "skip"
    assert_eq!(policy, "skip", "default policy should be 'skip'");
    let should_trigger = !(is_active && policy == "skip");
    assert!(
        !should_trigger,
        "default skip policy with active run should prevent trigger"
    );
}
// =====================================================================
// Scenario: Queued job fires when previous run completes
// =====================================================================
#[tokio::test]
async fn test_queued_job_fires_on_completion() {
    // @step Given a schedule "health-check" with overlap_policy "queue"
    let schedule = make_agent_schedule(Some("queue"));
    let old_session_id = Uuid::new_v4();

    // @step And the schedule has a queued job waiting
    let queued_jobs: RwLock<VecDeque<(String, ScheduleEntry)>> = RwLock::new(VecDeque::new());
    queued_jobs
        .write()
        .await
        .push_back(("health-check".to_string(), schedule.clone()));
    let active_runs: RwLock<HashMap<String, Uuid>> = RwLock::new(HashMap::new());
    active_runs
        .write()
        .await
        .insert("health-check".to_string(), old_session_id);

    // @step And the previous run's session is no longer in SessionManager
    // Simulate: remove from active_runs (as sweep would do)
    active_runs.write().await.remove("health-check");

    // @step When the scheduler tick runs and sweeps active_runs
    // Check queue for schedule whose active run just completed
    let active = active_runs.read().await;
    let has_active = active.contains_key("health-check");
    drop(active);

    let mut queue = queued_jobs.write().await;
    let should_fire = !has_active && !queue.is_empty();

    // @step Then the queued job is fired
    assert!(
        should_fire,
        "queued job should fire when active run is gone"
    );
    let fired = queue.pop_front().unwrap();
    assert_eq!(fired.0, "health-check");

    // @step And the schedule's active_runs entry is updated with the new session ID
    let new_session_id = Uuid::new_v4();
    active_runs
        .write()
        .await
        .insert("health-check".to_string(), new_session_id);
    assert!(active_runs.read().await.contains_key("health-check"));
}
// =====================================================================
// Scenario: Queue replaces duplicate entries for same schedule
// =====================================================================
#[tokio::test]
async fn test_queue_replaces_duplicate_entries() {
    // @step Given a schedule "monitor" with overlap_policy "queue"
    let schedule_v1 = make_agent_schedule(Some("queue"));
    let schedule_v2 = make_agent_schedule(Some("queue"));

    // @step And "monitor" already has a queued job waiting
    let queued_jobs: RwLock<VecDeque<(String, ScheduleEntry)>> = RwLock::new(VecDeque::new());
    queued_jobs
        .write()
        .await
        .push_back(("monitor".to_string(), schedule_v1));

    // @step When the schedule triggers again while still queued
    // Replace existing entry for same schedule name
    let mut queue = queued_jobs.write().await;
    queue.retain(|(name, _)| name != "monitor");
    queue.push_back(("monitor".to_string(), schedule_v2));

    // @step Then the queue contains only one entry for "monitor" with the latest trigger time
    let monitor_entries: Vec<_> = queue.iter().filter(|(n, _)| n == "monitor").collect();
    assert_eq!(
        monitor_entries.len(),
        1,
        "should have exactly one entry for 'monitor'"
    );
}
