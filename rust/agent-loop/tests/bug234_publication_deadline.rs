//! Feature: spec/features/compaction-elapsed-time-runway.feature

#[path = "support/bug234_publication.rs"]
mod publication;
#[path = "support/bug234_timing.rs"]
mod timing;

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_core::persistence::load_session;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_rpc_types::StreamChunk;
use codelet_tools::inject_summary::InjectSummaryArgs;
use codelet_tools::{GenerateCompactionArgs, GenerateCompactionTool, InjectSummaryTool};
use rig::{client::CompletionClient, completion::CompletionRequestBuilder, tool::Tool};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

struct Budget;

impl Drop for Budget {
    fn drop(&mut self) {
        codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);
    }
}

fn publications(harness: &timing::Harness) -> usize {
    harness
        .chunks()
        .iter()
        .filter(|chunk| {
            matches!(
                chunk,
                StreamChunk::CompactionComplete { .. } | StreamChunk::CommittedCompaction { .. }
            )
        })
        .count()
}

async fn admission_expiry() -> timing::TestResult {
    // @step Given both compaction routes produce a valid candidate before the shared deadline
    let harness = timing::Harness::new().await?;
    harness.prior_generation().await?;
    let original = load_session(harness.session.id)?.compaction_commit;
    let completed = publications(&harness);
    let prepared = Arc::new(AtomicBool::new(false));
    {
        let mut inner = harness.session.inner.lock().await;
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::new("offline-test")?;
        let model = client.completion_model("o3");
        let observed = prepared.clone();
        inner.install_selected_preparer(
            SelectedRequestPreparer::new(
                model.clone(),
                CompletionRequestBuilder::new(model, "unused")
                    .max_tokens(4096)
                    .build(),
                codelet_providers::request_admission::AdmissionLimits {
                    context_capacity: Some(200_000),
                    output_reservation: Some(4096),
                },
            )
            .map_history(move |messages| {
                observed.store(true, Ordering::Release);
                publication::advance_clock();
                messages.to_vec()
            }),
        );
    }
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        harness.session.id,
        Some(Arc::new(|_, _| Box::pin(async { Ok(timing::DAG.into()) }))),
    );
    tokio::time::pause();
    // @step When that deadline expires during admission or before deferred application
    let result = harness.call(false).await;
    tokio::time::resume();
    assert!(prepared.load(Ordering::Acquire));
    // @step Then no new durable generation is promoted and no completion event is emitted
    if result.is_ok()
        || load_session(harness.session.id)?.compaction_commit != original
        || publications(&harness) != completed
    {
        return Err("CLI promoted or published an expired candidate during admission".into());
    }
    assert!(result.expect_err("expired admission").contains("deadline"));
    assert!(harness
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    let injection = register_queue(&harness).await?;
    InjectSummaryTool::new(harness.session.id)
        .call(InjectSummaryArgs {
            content: timing::DAG.into(),
        })
        .await?;
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(61)).await;
    let injected = injection.apply(&mut *harness.session.inner.lock().await, &harness.session);
    tokio::time::resume();
    assert!(injected?, "injection has no generated candidate deadline");
    Ok(())
}

async fn register_queue(
    harness: &timing::Harness,
) -> Result<Arc<codelet_agent_loop::turn_compaction::TurnCompaction>, Box<dyn std::error::Error>> {
    let inner = harness.session.inner.lock().await;
    let project = load_session(harness.session.id)?.project;
    Ok(register_generate_compaction_handler(
        harness.session.id,
        &inner,
        project,
        harness.session.clone(),
        None,
    ))
}

async fn expiry_after_promotion() -> timing::TestResult {
    let harness = timing::Harness::new().await?;
    harness.prior_generation().await?;
    let original = load_session(harness.session.id)?
        .compaction_commit
        .ok_or("prior generation")?;
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        harness.session.id,
        Some(Arc::new(|_, _| Box::pin(async { Ok(timing::DAG.into()) }))),
    );
    let output = publication::ExpiringOutput(codelet_agent_loop::BackgroundOutput::with_provider(
        harness.session.clone(),
        "openai".into(),
        None,
    ));
    tokio::time::pause();
    let result = codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round(
        &mut *harness.session.inner.lock().await,
        harness.session.id,
        &Arc::new(Mutex::new(codelet_core::TokenState::default())),
        &harness.session.compaction_in_progress,
        &output,
    )
    .await;
    tokio::time::resume();
    result?;
    let promoted = load_session(harness.session.id)?
        .compaction_commit
        .ok_or("promoted generation")?;
    assert_eq!(
        promoted.identity.generation,
        original.identity.generation + 1
    );
    assert!(!promoted.publication_pending);
    harness
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()?;
    Ok(())
}

async fn deferred_expiry() -> timing::TestResult {
    let harness = timing::Harness::new().await?;
    harness.prior_generation().await?;
    let original = load_session(harness.session.id)?.compaction_commit;
    let completed = publications(&harness);
    let queue = register_queue(&harness).await?;
    GenerateCompactionTool::new(harness.session.id)
        .call(GenerateCompactionArgs { session_id: None })
        .await?;
    assert_eq!(
        load_session(harness.session.id)?.compaction_commit,
        original
    );
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(61)).await;
    let result = queue.apply(&mut *harness.session.inner.lock().await, &harness.session);
    tokio::time::resume();
    if result.is_ok()
        || load_session(harness.session.id)?.compaction_commit != original
        || publications(&harness) != completed
    {
        return Err("generation promoted or published an expired deferred candidate".into());
    }
    // @step And the previously committed generation remains authoritative
    assert_eq!(
        load_session(harness.session.id)?.compaction_commit,
        original
    );
    assert!(result
        .expect_err("expired application")
        .contains("deadline"));
    assert!(harness
        .session
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_expiry_before_durable_publication_rejects_an_otherwise_valid_candidate() {
    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(Some(
        Duration::from_secs(60),
    ));
    let _budget = Budget;
    let mut failures = Vec::new();
    for result in [
        Box::pin(admission_expiry()).await,
        Box::pin(deferred_expiry()).await,
        Box::pin(expiry_after_promotion()).await,
    ] {
        if let Err(error) = result {
            failures.push(error.to_string());
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}
