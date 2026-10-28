//! Feature: spec/features/measured-reasoning-cold-session-restore.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod fixture;
#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_cli::interactive::run_agent_stream;
use codelet_core::persistence::compaction_commit::{commit_compaction, reconcile_compaction};
use codelet_core::persistence::*;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::RigAgent;
use codelet_rpc_types::{StreamChunk, TokenRestoreState};
use serde_json::json;
use std::sync::{atomic::AtomicBool, Arc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn measured_reasoning_remains_cumulative_after_compacted_cold_session_restore(
) -> support::TestResult {
    let Some(session_id) = support::cold_process_session()? else {
        // @step Given two completed physical requests with measured output and reasoning
        let (directory, manager, session_id, _) = support::manager().await?;
        let bodies = [30, 7]
            .into_iter()
            .map(|reasoning| {
                let mut response = fixture::baseline::response();
                response["id"] = json!(format!("response-{reasoning}"));
                response["usage"] = json!({
                    "input_tokens":100,"output_tokens":70+reasoning,"total_tokens":170+reasoning,
                    "input_tokens_details":{"cached_tokens":0},
                    "output_tokens_details":{"reasoning_tokens":reasoning}
                });
                fixture::event(json!({"type":"response.completed","response":response}))
            })
            .collect();
        let server = fixture::Server::start(bodies, false).await?;
        let background = manager.get_session(&session_id.value)?;
        let output = codelet_agent_loop::background_output::BackgroundOutput::with_provider(
            background.clone(),
            "openai".into(),
            None,
        );
        for _request in 0..2 {
            let model = server.models()?.0.with_request_admission(
                codelet_providers::request_admission::AdmissionLimits {
                    context_capacity: Some(200_000),
                    output_reservation: Some(4_000),
                },
            );
            let agent = RigAgent::new(rig::agent::AgentBuilder::new(model).build(), 1);
            let mut inner = background.inner.lock().await;
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                run_agent_stream(
                    agent,
                    "measured request",
                    &mut inner,
                    Arc::new(AtomicBool::new(false)),
                    Arc::new(AtomicBool::new(false)),
                    Arc::new(tokio::sync::Notify::new()),
                    &output,
                    background.id,
                ),
            )
            .await??;
        }
        assert_eq!(server.requests.lock().map_err(|_| "wire lock")?.len(), 2);
        let manifest = load_session(background.id)?;
        assert_eq!(manifest.token_usage.reasoning_tokens, 37);
        assert_eq!(manifest.token_usage.cumulative_billed_input, 200);
        assert_eq!(manifest.token_usage.cumulative_billed_output, 177);
        assert_eq!(manifest.request_settlements.len(), 2);
        // @step When the session compacts and a fresh process restores its saved generation
        let inner = background.inner.lock().await;
        let model = inner.current_model_id().ok_or("selected model")?;
        let mut candidate = boundary::candidate(&manifest)?;
        candidate.source = inner.capture_durable_compaction(manifest.id)?;
        let last_turn = manifest
            .messages
            .len()
            .checked_sub(1)
            .ok_or("saved turns")?;
        candidate.dag = codelet_core::compaction::validate_dag_candidate(
        &format!(
            "<dag-node depth=\"D1\" turns=\"0-{last_turn}\" label=\"Measured work\">Two measured requests completed.</dag-node>\n<dag-files>\n</dag-files>"
        ),
        &codelet_core::compaction::DagValidationContext {
            persisted_turn_count: manifest.messages.len(),
            required_coverage_end: Some(last_turn),
            ..Default::default()
        },
    )?;
        candidate.checkpoint.model = model.clone();
        candidate.checkpoint.payload =
            serde_json::to_value(inner.provider_session.prepare_generation(1, &model)?)?;
        drop(inner);
        let promoted = commit_compaction(candidate, &model, |_, _| Ok(()))?;
        reconcile_compaction(
            &promoted
                .compaction_commit
                .as_ref()
                .ok_or("committed boundary")?
                .identity,
        )?;
        drop(output);
        drop(background);
        drop(manager);
        return support::run_cold_process(
            directory.path(),
            &session_id,
            "measured_reasoning_remains_cumulative_after_compacted_cold_session_restore",
        )
        .await;
    };
    let manager = support::cold_resume(&session_id)?;
    let background = manager.get_session(&session_id.value)?;
    let before = load_session(background.id)?;
    assert_eq!(
        before
            .compaction_commit
            .as_ref()
            .ok_or("committed boundary")?
            .identity
            .generation,
        1
    );
    let saved = &before.token_usage;
    let counters = || TokenRestoreState {
        current_context: saved.current_context_tokens as i64,
        cumulative_billed_output: saved.cumulative_billed_output as i64,
        cache_read: saved.cache_read_tokens as i64,
        cache_creation: saved.cache_creation_tokens as i64,
        cumulative_billed_input: saved.cumulative_billed_input as i64,
        cumulative_billed_output_second: saved.cumulative_billed_output as i64,
    };
    // @step Then live cached and emitted reasoning equal the cumulative measured reasoning
    // @step And repeating restoration preserves billing and physical settlement identities
    for _restore in 0..2 {
        let mut events = manager.chunks_tx().subscribe();
        manager
            .restore_session_messages(&session_id, get_session_message_envelopes(background.id)?)?;
        manager.restore_session_token_state(&session_id, counters())?;
        assert_eq!(background.get_tokens().2, Some(37));
        let inner = background.inner.lock().await;
        assert_eq!(inner.token_tracker.reasoning_tokens, 37);
        assert_eq!(inner.token_tracker.cumulative_billed_input, 200);
        assert_eq!(inner.token_tracker.cumulative_billed_output, 177);
        drop(inner);
        let mut measured_update = false;
        while let Ok((_, chunk)) = events.try_recv() {
            if let StreamChunk::TokenUpdate { tokens } = chunk {
                assert_eq!(tokens.reasoning_tokens, Some(37));
                assert_eq!(tokens.cumulative_billed_output, Some(177));
                measured_update = true;
            }
        }
        assert!(measured_update, "restore omitted measured reasoning update");
        let after = load_session(background.id)?;
        assert_eq!(after.token_usage, before.token_usage);
        assert_eq!(after.request_settlements, before.request_settlements);
    }
    Ok(())
}
