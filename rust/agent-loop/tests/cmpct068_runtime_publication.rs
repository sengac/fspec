//! Feature: spec/features/runtime-uncommitted-context-publication.feature

use codelet_agent_loop::inject_summary_handler::apply_pending_dag_and_emit;
use codelet_rpc_types::StreamChunk;
use std::sync::{Arc, Mutex};

#[tokio::test]
#[serial_test::serial]
async fn scenario_runtime_candidate_application_cannot_announce_an_uncommitted_generation(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a submitted DAG candidate without durable promotion
    let previous = std::env::var_os("ANTHROPIC_API_KEY");
    std::env::set_var("ANTHROPIC_API_KEY", "test-no-network");
    let result = codelet_cli::session::Session::new(Some("claude"));
    match previous {
        Some(value) => std::env::set_var("ANTHROPIC_API_KEY", value),
        None => std::env::remove_var("ANTHROPIC_API_KEY"),
    }
    let mut session = result?;
    session.token_tracker.input_tokens = 1050;
    let pending = Arc::new(Mutex::new(Some(
        "<system-reminder>\n<!-- type:compaction-dag -->\n## D0\nsubmitted candidate\n</system-reminder>".into(),
    )));
    let events = Mutex::new(Vec::new());
    // @step When the runtime attempts deferred candidate application
    apply_pending_dag_and_emit(&mut session, &pending, 1050, &|event| {
        events
            .lock()
            .expect("captured runtime event lock")
            .push(event);
    });
    // @step Then no committed snapshot or completion notice is published
    assert!(
        events
            .lock()
            .expect("captured runtime events")
            .iter()
            .all(|event| {
                !matches!(
                    event,
                    StreamChunk::CommittedCompaction { .. }
                        | StreamChunk::CompactionComplete { .. }
                )
            }),
        "a submitted candidate is not a durable commit"
    );
    // @step And the authoritative context counters remain unchanged
    assert_eq!(session.token_tracker.input_tokens, 1050);
    Ok(())
}
