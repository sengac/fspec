//! Feature: spec/features/background-compaction-request-ordering.feature

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_rpc_types::{
    CommittedContextSnapshot, ContextFillInfo, ContextProvenance, RequestContextSnapshot,
    StreamChunk, TokenTracker,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_delayed_completion_preserves_newer_background_request_counters(
) -> support::TestResult {
    // @step Given background counters describe a measured request from a committed generation
    let (_directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut events = background.subscribe_to_stream();
    let measured = RequestContextSnapshot {
        model: "claude-sonnet-4".into(),
        generation: 7,
        request: 2,
        physical_request: Some("request-2".into()),
        input_tokens: Some(600),
        output_tokens: Some(50),
        capacity: Some(2000),
        threshold: Some(1000),
        provenance: ContextProvenance::Measured,
    };
    background.update_tokens(600, 900);
    background.update_reasoning_tokens(80);
    background.handle_output(StreamChunk::ContextSnapshot {
        snapshot: measured.clone(),
    });
    let committed = CommittedContextSnapshot {
        generation: 7,
        tokens: TokenTracker {
            input_tokens: 200,
            output_tokens: 0,
            ..Default::default()
        },
        context_fill: ContextFillInfo {
            fill_percentage: 20,
            effective_tokens: 200.0,
            threshold: 1000.0,
            context_window: 2000.0,
        },
        original_tokens: 1050,
        covered_turns: Some(12),
        retained_turns: Some(1),
        estimated: true,
        reconciled: false,
    };
    // @step When its delayed committed snapshot and an older request snapshot arrive
    for _ in 0..2 {
        background.handle_output(StreamChunk::CommittedCompaction {
            snapshot: committed.clone(),
        });
    }
    let mut stale = measured;
    stale.request = 1;
    stale.input_tokens = Some(1900);
    background.handle_output(StreamChunk::ContextSnapshot { snapshot: stale });
    // @step Then the background counters retain the measured request values
    assert_eq!(
        background.get_tokens(),
        (600, 900, Some(80)),
        "delayed completion replaced live request counters"
    );
    assert_eq!(
        background
            .committed_context_snapshot()
            .ok_or("commit snapshot")?
            .generation,
        7
    );
    let mut requests = Vec::new();
    let mut completions = 0;
    while let Ok(event) = events.try_recv() {
        match event {
            StreamChunk::ContextSnapshot { snapshot } => requests.push(snapshot.request),
            StreamChunk::CommittedCompaction { .. } => completions += 1,
            _ => {}
        }
    }
    // @step And observers receive the completion once without receiving the stale request
    assert_eq!(requests, [2]);
    assert_eq!(completions, 1);
    Ok(())
}
