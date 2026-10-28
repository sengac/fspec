#[path = "cmpct056/support.rs"]
pub mod support;

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_tools::inject_summary::execute_inject_summary;
use support::{assert_no_completion, Fixture, DAG};

#[tokio::test]
#[serial_test::serial]
async fn scenario_deferred_application_cannot_redirect_a_captured_candidate() {
    // @step Given a candidate submitted for a captured live target
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let mut target = fixture.target.inner.lock().await;
    let queue = register_generate_compaction_handler(
        fixture.target.id,
        &target,
        fixture.directory.path().into(),
        fixture.target.clone(),
        Some(fixture.manager.clone()),
    );
    execute_inject_summary(fixture.target.id, DAG.into()).expect("captured submission");
    let before = serde_json::json!({"messages": target.messages, "tokens": target.token_tracker});
    let mut target_events = fixture.target.subscribe_to_stream();
    let mut caller_events = fixture.caller.subscribe_to_stream();
    // @step When deferred application names a different live target
    let error = queue
        .apply(&mut target, &fixture.caller)
        .expect_err("a captured candidate cannot publish to a different target");
    // @step Then application rejects without promoting either durable session
    assert!(
        error.contains("target identity"),
        "explicit rejection: {error}"
    );
    assert_eq!(
        serde_json::json!({"messages": target.messages, "tokens": target.token_tracker}),
        before
    );
    for session_id in [fixture.target.id, fixture.caller.id] {
        let manifest =
            codelet_core::persistence::load_session(session_id).expect("unchanged durable session");
        assert!(manifest.compaction_commit.is_none());
        assert_eq!(manifest.messages.len(), 2);
    }
    assert_no_completion(&mut target_events);
    assert_no_completion(&mut caller_events);
    // @step And the candidate remains available only to its captured target
    assert!(queue
        .apply(&mut target, &fixture.target)
        .expect("captured owner can still apply its accepted candidate"));
    let manifest = codelet_core::persistence::load_session(fixture.target.id)
        .expect("committed captured target");
    assert_eq!(
        manifest.compaction.expect("exact submitted DAG").summary,
        DAG
    );
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_after_submission_cannot_promote_the_pending_candidate() {
    // @step Given a candidate submitted for a captured live target
    let fixture = Fixture::new("http://127.0.0.1:1").await;
    let mut target = fixture.target.inner.lock().await;
    let queue = register_generate_compaction_handler(
        fixture.target.id,
        &target,
        fixture.directory.path().into(),
        fixture.target.clone(),
        Some(fixture.manager.clone()),
    );
    execute_inject_summary(fixture.target.id, DAG.into()).expect("captured submission");
    let before = serde_json::json!({"messages": target.messages, "tokens": target.token_tracker});
    let mut events = fixture.target.subscribe_to_stream();
    // @step When the target is interrupted before deferred application
    fixture.target.interrupt();
    let error = queue
        .apply(&mut target, &fixture.target)
        .expect_err("interruption cannot become durable compaction success");
    // @step Then application returns cancellation without promoting durable context
    assert!(error.contains("cancel"), "explicit cancellation: {error}");
    assert_eq!(
        serde_json::json!({"messages": target.messages, "tokens": target.token_tracker}),
        before
    );
    // @step And inference remains blocked without completion or changed counters
    assert!(target.ensure_compaction_inference_ready().is_err());
    let manifest = codelet_core::persistence::load_session(fixture.target.id)
        .expect("unchanged durable session");
    assert!(manifest.compaction_commit.is_none());
    assert_eq!(manifest.messages.len(), 2);
    assert_no_completion(&mut events);
}
