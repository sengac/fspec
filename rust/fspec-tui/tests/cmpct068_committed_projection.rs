//! Feature: spec/features/committed-compaction-context-projection.feature

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
use serde_json::json;
use std::sync::Arc;

mod common;

fn committed(
    generation: u64,
    tokens: u32,
    coverage: Option<u64>,
    reconciled: bool,
) -> Result<StreamChunk, serde_json::Error> {
    serde_json::from_value(json!({
        "CommittedCompaction": {
        "snapshot": {
            "generation": generation,
            "tokens": {
                "input_tokens": tokens, "output_tokens": 0,
                "cache_read_input_tokens": null, "cache_creation_input_tokens": null,
                "tokens_per_second": null, "cumulative_billed_input": null,
                "cumulative_billed_output": null, "reasoning_tokens": null
            },
            "contextFill": {"fill_percentage": tokens / 10, "effective_tokens": tokens,
                "threshold": 1000.0, "context_window": 2000.0},
            "originalTokens": 1050,
            "coveredTurns": coverage,
            "retainedTurns": null,
            "estimated": true,
            "reconciled": reconciled
        }}
    }))
}

#[test]
fn scenario_delayed_completion_cannot_overwrite_a_newer_request_context(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a measured request context from a committed generation
    let (mut app, session) = app();
    let measured = StreamChunk::ContextSnapshot {
        snapshot: codelet_rpc_types::RequestContextSnapshot {
            model: "model-a".into(),
            generation: 7,
            request: 2,
            physical_request: Some("physical-2".into()),
            input_tokens: Some(600),
            output_tokens: Some(50),
            capacity: Some(2000),
            threshold: Some(1000),
            provenance: codelet_rpc_types::ContextProvenance::Measured,
        },
    };
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::TokenUpdate {
            tokens: codelet_rpc_types::TokenTracker {
                input_tokens: 600,
                output_tokens: 900,
                ..Default::default()
            },
        },
    ));
    app.dispatch(Action::ChunkReceived(session.clone(), measured));
    // @step When that generation's delayed completion snapshot arrives twice
    for _ in 0..2 {
        app.dispatch(Action::ChunkReceived(
            session.clone(),
            committed(7, 200, Some(12), false)?,
        ));
    }
    // @step Then the measured request fill and current counters remain unchanged
    let state = app
        .agent_view_store()
        .token_state_for(&session)
        .ok_or("missing measured token state")?;
    assert_eq!(state.input_tokens, 600);
    assert_eq!(state.output_tokens, 900);
    assert_eq!(state.context_fill_pct, 65);
    assert_eq!(state.context_request, Some(2));
    assert!(!state.estimated);
    // @step And the completion notice is delivered exactly once
    let notices = notices(&mut app);
    assert_eq!(
        notices.len(),
        1,
        "request context must not acknowledge completion delivery"
    );
    assert!(notices[0].contains("generation 7"));
    Ok(())
}

fn app() -> (App, SessionId) {
    let backend: Arc<dyn FspecBackend> = Arc::new(common::MockBackend::new());
    let mut app = App::new(backend);
    let session = SessionId::new("committed-projection");
    app.dispatch(Action::SessionCreated(session.clone()));
    (app, session)
}

fn notices(app: &mut App) -> Vec<String> {
    let mut notices = Vec::new();
    while let Some(action) = app.try_recv_action() {
        if let Action::EmitSessionNotice(_, text) = action {
            notices.push(text);
        }
    }
    notices
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_committed_compaction_refreshes_counters_before_another_provider_call(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given my session header shows a full context before compaction
    let (mut app, session) = app();
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::context_fill_update(codelet_rpc_types::ContextFillInfo {
            fill_percentage: 105,
            effective_tokens: 1050.0,
            threshold: 1000.0,
            context_window: 2000.0,
        }),
    ));
    // @step When a durable DAG generation commits with a new context snapshot
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    let previous_key = std::env::var_os("ANTHROPIC_API_KEY");
    std::env::set_var("ANTHROPIC_API_KEY", "test-no-network");
    let inner = codelet_cli::session::Session::new(Some("claude"));
    match previous_key {
        Some(previous) => std::env::set_var("ANTHROPIC_API_KEY", previous),
        None => std::env::remove_var("ANTHROPIC_API_KEY"),
    }
    let inner = inner?;
    let (input, _) = tokio::sync::mpsc::channel(4);
    let (chunks, _) = tokio::sync::broadcast::channel(16);
    let (statuses, _) = tokio::sync::broadcast::channel(4);
    let (cleared, _) = tokio::sync::broadcast::channel(4);
    let background = codelet_sessions::background_session::BackgroundSession::new(
        uuid::Uuid::new_v4(),
        "projection".into(),
        directory.path().to_string_lossy().into(),
        None,
        None,
        inner,
        input,
        None,
        None,
        None,
        chunks,
        statuses,
        cleared,
    );
    background.update_tokens(1050, 0);
    let mut received = background.subscribe_to_stream();
    background.handle_output(committed(7, 200, Some(12), false)?);
    app.dispatch(Action::ChunkReceived(session.clone(), received.try_recv()?));
    // @step Then the background counters and header immediately show that generation's estimated tokens and trigger-relative fill
    let state = app
        .agent_view_store()
        .token_state_for(&session)
        .ok_or("missing token state")?;
    assert_eq!(state.input_tokens, 200);
    assert_eq!(state.context_fill_pct, 20);
    assert_eq!(state.context_threshold_tokens, 1000);
    assert_eq!(background.get_tokens().0, 200);
    // @step And the completion notice identifies the same generation and measured durable coverage
    let notices = notices(&mut app);
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("generation 7"));
    assert!(notices[0].contains("12 turns"));
    assert!(notices[0].contains("estimated"));
    Ok(())
}

#[test]
fn scenario_unknown_coverage_is_not_presented_as_measured_zero() -> Result<(), serde_json::Error> {
    // @step Given a committed compaction has no measured durable coverage
    let (mut app, session) = app();
    let snapshot = committed(8, 220, None, false)?;
    // @step When its context snapshot reaches my session
    app.dispatch(Action::ChunkReceived(session, snapshot));
    // @step Then the completion notice explicitly reports unknown coverage
    let notices = notices(&mut app);
    assert_eq!(notices.len(), 1);
    assert!(notices[0].contains("coverage unknown"));
    assert!(!notices[0].contains("0 turns"));
    // @step And no rejected or uncommitted operation produces a success badge
    let (mut rejected, rejected_session) = self::app();
    rejected.dispatch(Action::ChunkReceived(
        rejected_session.clone(),
        StreamChunk::Error {
            error: "candidate rejected before durable promotion".into(),
        },
    ));
    assert!(rejected
        .agent_view_store()
        .compaction_reduction_for(&rejected_session)
        .is_none());
    assert!(self::notices(&mut rejected).is_empty());
    Ok(())
}

#[test]
fn scenario_reconciliation_does_not_duplicate_completion_or_restore_stale_counters(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a newer committed generation is already displayed after notification reconciliation
    let (mut app, session) = app();
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        committed(10, 300, Some(20), true)?,
    ));
    assert!(
        notices(&mut app).is_empty(),
        "reconciliation does not claim delivery"
    );
    // @step When an older or duplicate committed snapshot arrives
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        committed(9, 900, Some(10), false)?,
    ));
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        committed(10, 300, Some(20), false)?,
    ));
    // @step Then my header retains the newer generation and counters
    assert_eq!(
        app.agent_view_store()
            .token_state_for(&session)
            .ok_or("missing token state")?
            .input_tokens,
        300
    );
    // @step And no duplicate completion notice is delivered
    assert!(notices(&mut app).is_empty());
    Ok(())
}
