//! Feature: spec/features/cold-compaction-publication-reconciliation.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod fixture;
#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_core::RigAgent;
use codelet_providers::request_admission::AdmissionLimits;
use std::sync::{atomic::AtomicBool, Arc};

struct Output;

impl StreamOutput for Output {
    fn emit(&self, _: StreamEvent) {}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_cold_notification_loss_blocks_inference_until_exact_identity_reconciliation(
) -> support::TestResult {
    let Some(session_id) = support::cold_process_session()? else {
        // @step Given a durable generation whose completion publication failed
        let (directory, manager, session_id, _) = support::manager().await?;
        let mut manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
        append_message(&mut manifest, "user", "covered original")?;
        let background = manager.get_session(&session_id.value)?;
        let inner = background.inner.lock().await;
        let model_id = inner.current_model_id().ok_or("selected model")?;
        let mut candidate = boundary::candidate(&manifest)?;
        candidate.source = inner.capture_durable_compaction(manifest.id)?;
        candidate.checkpoint.model = model_id.clone();
        candidate.checkpoint.payload =
            serde_json::to_value(inner.provider_session.prepare_generation(1, &model_id)?)?;
        drop(inner);
        let promoted = commit_compaction(candidate, &model_id, |_, _| Ok(()))?;
        assert!(publish_compaction(&promoted, |_| Err("receiver lost".into())).is_err());
        drop(manager);
        return support::run_cold_process(
            directory.path(),
            &session_id,
            "scenario_cold_notification_loss_blocks_inference_until_exact_identity_reconciliation",
        )
        .await;
    };
    let manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    let identity = manifest
        .compaction_commit
        .as_ref()
        .ok_or("pending boundary")?
        .identity
        .clone();
    // @step When a fresh session restores the pending generation and attempts inference
    let manager = support::cold_resume(&session_id)?;
    let background = manager.get_session(&session_id.value)?;
    let response = fixture::event(serde_json::json!({
        "type":"response.completed", "response":fixture::baseline::response()
    }));
    let server = fixture::Server::start(vec![response], false).await?;
    let model = server.models()?.0.with_request_admission(AdmissionLimits {
        context_capacity: Some(200000),
        output_reservation: Some(4096),
    });
    let agent = RigAgent::new(
        rig::agent::AgentBuilder::new(model.clone())
            .max_tokens(4096)
            .build(),
        1,
    );
    let mut inner = background.inner.lock().await;
    inner
        .provider_manager_mut()
        .override_model_limits(Some(200000), Some(4096));
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            "next",
            &mut inner,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            &Output,
            manifest.id,
        ),
    )
    .await?;
    // @step Then no physical provider request is sent
    assert!(
        result.is_err(),
        "pending publication must block the actual inference entrypoint"
    );
    let failure = result.err().ok_or("pending publication was not rejected")?;
    assert!(failure.to_string().contains("compaction"), "{failure}");
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    // @step And a different generation identity cannot release the block
    let mut wrong = identity.clone();
    wrong.generation += 1;
    assert!(inner.reconcile_durable_compaction(&wrong).is_err());
    assert!(inner.ensure_compaction_inference_ready().is_err());
    // @step And exact identity reconciliation preserves the generation and permits the next request
    let reconciled = inner.reconcile_durable_compaction(&identity)?;
    assert_eq!(
        reconciled
            .compaction_commit
            .as_ref()
            .ok_or("boundary")?
            .identity,
        identity
    );
    assert!(
        !reconciled
            .compaction_commit
            .as_ref()
            .ok_or("boundary")?
            .publication_pending
    );
    inner.ensure_compaction_inference_ready()?;
    let agent = RigAgent::new(
        rig::agent::AgentBuilder::new(model)
            .max_tokens(4096)
            .build(),
        1,
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            "next",
            &mut inner,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            &Output,
            manifest.id,
        ),
    )
    .await??;
    assert_eq!(server.requests.lock().map_err(|_| "request lock")?.len(), 1);
    assert_eq!(
        load_session(manifest.id)?.compaction_commit,
        reconciled.compaction_commit
    );
    Ok(())
}
