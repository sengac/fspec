//! Feature: spec/features/resumed-session-input-token-accounting.feature
//!
//! BUG-326: a cold-resumed session reports the same input tokens as the live
//! session (the last request's total input, without its output), and its
//! restored context fill is the live inclusive occupancy (input + output of
//! the last request). Real `run_agent_stream` turns against an offline
//! wiremock Anthropic endpoint, real request settlement, real cold
//! `resume_session` in a fresh `SessionManager`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
pub mod support;
#[path = "cov057_support/wire.rs"]
pub mod wire;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::request_settlement::{replace_request_context, settle_request};
use codelet_core::persistence::{append_message, load_session};
use support::{cold_resume, manager, TestResult};
use wire::cached_turn;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cold_resumed_session_reports_the_live_input_tokens() -> TestResult {
    let (_directory, live, session_id, _) = manager().await?;
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    // A committed compaction boundary, so the restored session publishes
    // its context fill on resume.
    let mut manifest = load_session(uuid)?;
    append_message(&mut manifest, "user", "retained original")?;
    // The boundary must name the session's own model to be restorable.
    let mut candidate = boundary::candidate(&manifest)?;
    candidate.checkpoint.model = "claude-sonnet-4".into();
    candidate.source = codelet_core::persistence::compaction_boundary::capture_compaction_source(
        &manifest,
        "claude-sonnet-4",
        "dag-v1",
    )?;
    let promoted = commit_compaction(candidate, "claude-sonnet-4", |_, _| Ok(()))?;
    publish_compaction(&promoted, |_| Ok(()))?;

    // @step Given a live session whose last request used 180000 input, 1000 cache read and 7 output tokens
    cached_turn(
        &live,
        &session_id,
        "measured prompt",
        "done",
        180_000,
        1_000,
    )
    .await?;
    let live_input = {
        let session = live.get_session(&session_id.value)?;
        let inner = session.inner.lock().await;
        inner.token_tracker.input_tokens
    };
    assert_eq!(live_input, 181_000, "live total input");
    drop(live);

    // @step When the session is cold-resumed in a fresh session manager
    let restored = cold_resume(&session_id)?;
    let background = restored.get_session(&session_id.value)?;

    // @step Then the resumed token tracker's input tokens equal the live input tokens 181000
    assert_eq!(
        background.inner.lock().await.token_tracker.input_tokens,
        live_input
    );
    // @step And the manifest's current context tokens are 181000
    assert_eq!(
        load_session(uuid)?.token_usage.current_context_tokens,
        181_000
    );
    // @step And the resumed context occupancy is the live inclusive occupancy 181007
    let snapshot = background
        .committed_context_snapshot()
        .ok_or("restored committed context snapshot")?;
    let live_fill = codelet_rpc_types::RequestContextSnapshot {
        model: "claude-sonnet-4".into(),
        generation: 1,
        request: 1,
        physical_request: None,
        input_tokens: Some(181_000),
        output_tokens: Some(7),
        capacity: Some(snapshot.context_fill.context_window as u64),
        threshold: Some(snapshot.context_fill.threshold as u64),
        provenance: codelet_rpc_types::ContextProvenance::Measured,
    }
    .context_fill()?
    .ok_or("live context fill")?;
    assert_eq!(snapshot.context_fill.effective_tokens, 181_007.0);
    assert_eq!(
        snapshot.context_fill.effective_tokens,
        live_fill.effective_tokens
    );
    assert_eq!(
        snapshot.context_fill.fill_percentage,
        live_fill.fill_percentage
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_resumed_output_tokens_exclude_reasoning() -> TestResult {
    let (_directory, live, session_id, _) = manager().await?;
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    // @step Given a session whose settled requests produced 40 output and 25 reasoning tokens
    for (request, output, reasoning) in [("req-a", 30, 20), ("req-b", 10, 5)] {
        let usage = rig::completion::Usage {
            input_tokens: 100,
            output_tokens: output,
            total_tokens: 100 + output + reasoning,
            cache_read_input_tokens: None,
            cache_creation_input_tokens: None,
            reasoning_tokens: Some(reasoning),
        };
        settle_request(uuid, request, &usage)?;
        replace_request_context(uuid, Some(usage))?;
    }
    drop(live);

    // @step When the session is cold-resumed in a fresh session manager
    let restored = cold_resume(&session_id)?;
    let background = restored.get_session(&session_id.value)?;
    let tracker = background.inner.lock().await.token_tracker.clone();

    // @step Then the resumed token tracker's output tokens are 40
    assert_eq!(tracker.output_tokens, 40);
    // @step And its reasoning tokens are 25
    assert_eq!(tracker.reasoning_tokens, 25);
    Ok(())
}
