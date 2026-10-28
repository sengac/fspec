#[path = "cmpct056/support.rs"]
pub mod support;

use std::time::Duration;
use support::{assert_clean, assert_no_completion, snapshot, Fixture, Server};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn scenario_caller_interruption_cancels_cross_target_generation() {
    // @step Given an Idle target with a cross-target builder waiting at a barrier
    let server = Server::new(200).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut events = fixture.target.subscribe_to_stream();
    let mut operation = fixture.start();
    let arrival = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::select! {
            _ = server.arrived.wait() => Ok(()),
            result = &mut operation => Err(format!("operation exited before builder barrier: {result:?}")),
        }
    }).await;
    if arrival.is_err() {
        operation.abort();
        let _ = (&mut operation).await;
    }
    arrival
        .expect("builder barrier deadline")
        .expect("builder owns captured target");
    // @step When the requesting caller is interrupted without aborting its task
    fixture.caller.interrupt();
    let outcome = tokio::time::timeout(Duration::from_secs(3), &mut operation).await;
    if outcome.is_err() {
        operation.abort();
        let _ = operation.await;
    }
    // @step Then the operation returns cancellation within the harness deadline
    let error = outcome
        .expect("caller interruption must cancel the operation without task abortion")
        .expect("operation joins")
        .expect_err("interrupted caller cannot complete cross-target compaction");
    assert!(
        error.to_string().contains("cancel"),
        "typed outcome: {error}"
    );
    assert_eq!(snapshot(&fixture.target).await, before);
    // @step And the unchanged target returns to Idle without completion
    assert_clean(&fixture.target);
    assert_no_completion(&mut events);
    let manifest = codelet_core::persistence::load_session(fixture.target.id)
        .expect("unchanged durable target");
    assert!(manifest.compaction_commit.is_none());
    assert_eq!(manifest.messages.len(), 2);
}
