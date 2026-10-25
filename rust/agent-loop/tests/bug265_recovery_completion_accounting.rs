//! Feature: spec/features/recovery-completion-accounting.feature

#[path = "support/automatic_compaction.rs"]
mod automatic;
#[path = "support/bug265.rs"]
mod support;
#[path = "support/bug265_wire.rs"]
mod wire;

use automatic::Harness;
use support::{script, Environment, TestResult};
use wire::Reply;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn automatic_compaction_preserves_the_consumed_nudge_budget() -> TestResult {
    // @step Given a real background task has consumed its only zero-progress nudge
    // @step When a context overflow is compacted and the shared loop retries automatically
    // @step Then the resumed task has no fresh nudge allowance
    // @step And only three inference requests occur with no extra user input
    support::assert_budget_recovery(false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn loop_abort_recovery_preserves_the_consumed_nudge_budget() -> TestResult {
    // @step Given a real background task has consumed its only zero-progress nudge
    // @step When repetitive provider output triggers the shared loop's corrective retry
    // @step Then the resumed task has no fresh nudge allowance
    // @step And only three inference requests occur with corrective context and no extra user input
    support::assert_budget_recovery(true).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_genuine_next_user_task_receives_a_fresh_nudge_budget() -> TestResult {
    // @step Given a real background task exhausted its one-nudge allowance
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    fixture.session.set_continue_state(true, 1);
    let observations = script(&fixture, |_| Reply::Text).await;
    fixture.send();
    fixture.until_done().await;
    assert_eq!(fixture.session.inner.lock().await.continue_nudges_used, 1);
    assert_eq!(
        observations
            .lock()
            .map_err(|error| error.to_string())?
            .len(),
        2
    );
    // @step When the user submits a genuine next task
    fixture.send();
    fixture.until_done().await;
    // @step Then each task consumes exactly one nudge and finishes at its budget boundary
    assert_eq!(fixture.session.inner.lock().await.continue_nudges_used, 1);
    assert_eq!(
        observations
            .lock()
            .map_err(|error| error.to_string())?
            .len(),
        4
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn rejections_across_automatic_compaction_reach_goal_escalation() -> TestResult {
    // @step Given two native Done calls were rejected for the active goal
    // @step When automatic compaction resumes the task and two more native Done calls are rejected
    // @step Then the same goal reaches four rejections and pauses for repeated completion failures
    // @step And the registry and inner session retain four rejections
    support::assert_goal_recovery(false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn rejections_across_loop_abort_recovery_reach_goal_escalation() -> TestResult {
    // @step Given two native Done calls were rejected for the active goal
    // @step When a repetitive-output corrective retry resumes the task and two more native Done calls are rejected
    // @step Then the same goal reaches four rejections and pauses for repeated completion failures
    // @step And the registry and inner session retain four rejections
    support::assert_goal_recovery(true).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn genuine_user_dispatch_and_identical_goal_writes_preserve_rejections() -> TestResult {
    // @step Given a native Done rejection belongs to the active goal
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    let goal = Some(("finish the verified task".into(), None));
    fixture.session.set_goal_state(goal.clone());
    let observations = script(&fixture, |number| match number % 3 {
        0 => Reply::RejectDone,
        1 => Reply::Text,
        _ => Reply::Failure,
    })
    .await;
    fixture.send();
    fixture.until_done().await;
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 1);
    assert_eq!(fixture.session.inner.lock().await.done_rejections, 1);
    assert_eq!(fixture.session.inner.lock().await.done_rejections, 1);
    assert_eq!(fixture.session.inner.lock().await.done_rejections, 1);
    // @step When the user submits another task and later writes the identical goal before another task
    fixture.send();
    fixture.until_done().await;
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 2);
    assert_eq!(fixture.session.inner.lock().await.done_rejections, 2);
    fixture.session.set_goal_state(goal.clone());
    fixture.send();
    fixture.until_done().await;
    // @step Then each rejected native Done adds to the same goal lifetime count
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 3);
    assert_eq!(fixture.session.inner.lock().await.done_rejections, 3);
    assert_eq!(fixture.session.get_goal_state(), goal);
    // @step And Done remains armed before every provider request
    let observations = observations.lock().map_err(|error| error.to_string())?;
    assert_eq!(observations.len(), 9);
    assert_eq!(observations[3].rejections, 1);
    assert_eq!(observations[6].rejections, 2);
    assert!(observations
        .iter()
        .all(|entry| entry.armed && entry.done_registered));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn replacing_and_clearing_a_goal_reset_its_rejection_accounting() -> TestResult {
    // @step Given a native Done rejection belongs to the active goal
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    fixture
        .session
        .set_goal_state(Some(("original goal".into(), None)));
    let observations = script(&fixture, |number| match number {
        0 | 3 => Reply::RejectDone,
        1 | 4 => Reply::Text,
        _ => Reply::Failure,
    })
    .await;
    fixture.send();
    fixture.until_done().await;
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 1);
    // @step When the user replaces the goal and receives a rejection for the replacement
    fixture
        .session
        .set_goal_state(Some(("replacement goal".into(), Some("exit 0".into()))));
    fixture.send();
    fixture.until_done().await;
    // @step Then the replacement has only its own rejection and its new verification setting
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 1);
    let goal = codelet_tools::get_session_goal(fixture.session.id).ok_or("replacement goal")?;
    assert_eq!(goal.text, "replacement goal");
    assert_eq!(goal.verify.as_deref(), Some("exit 0"));
    // @step When the user clears that goal and submits another task
    fixture.session.set_goal_state(None);
    fixture.send();
    fixture.until_done().await;
    // @step Then both rejection counts are zero and no stale goal or armed Done remains
    let inner = fixture.session.inner.lock().await;
    assert!(inner.goal.is_none());
    assert_eq!(inner.done_rejections, 0);
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 0);
    assert!(codelet_tools::get_session_goal(fixture.session.id).is_none());
    assert!(!codelet_tools::is_continue_armed(fixture.session.id));
    let observations = observations.lock().map_err(|error| error.to_string())?;
    assert_eq!(observations.len(), 7);
    assert_eq!(observations[3].rejections, 0);
    assert!(!observations[6].armed);
    assert!(!observations[6].done_registered);
    Ok(())
}
