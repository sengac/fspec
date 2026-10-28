#[path = "cmpct056/support.rs"]
pub mod support;

use std::time::Duration;
use support::{assert_clean, assert_no_completion, snapshot, Fixture, Server};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial_test::serial]
async fn scenario_changed_target_identity_rejects_the_captured_candidate() {
    // @step Given an Idle target captured by a DAG builder waiting at a barrier
    let server = Server::new(200).await;
    let fixture = Fixture::new(&server.url).await;
    let before = snapshot(&fixture.target).await;
    let mut events = fixture.target.subscribe_to_stream();
    let operation = fixture.start();
    tokio::time::timeout(Duration::from_secs(3), server.arrived.wait())
        .await
        .expect("builder captured target");
    // @step When the selected target model changes before the builder returns
    fixture
        .target
        .inner
        .lock()
        .await
        .provider_manager_mut()
        .set_model_direct(
            "openai",
            "changed-target-model",
            Some(100_000),
            Some(4096),
            None,
        )
        .expect("change selected target identity");
    server.release.wait().await;
    let result = tokio::time::timeout(Duration::from_secs(3), operation)
        .await
        .expect("bounded identity rejection")
        .expect("generation task joins");
    // @step Then the captured candidate is rejected without changing durable context
    assert!(
        result.is_err(),
        "captured source cannot migrate to another model"
    );
    assert_eq!(snapshot(&fixture.target).await, before);
    // @step And the unchanged target returns to Idle without completion
    assert_clean(&fixture.target);
    assert_no_completion(&mut events);
}
