//! Feature: spec/features/copilot-lineage-boundaries.feature
#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "support/bug268_lineage.rs"]
mod support;

use codelet_providers::session::{ProviderSession, ProviderSessionExtension, StateWriter};
use environment::{Environment, TestResult};
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

#[tokio::test]
#[serial_test::serial]
async fn cancelled_or_failed_requests_invalidate_prior_owned_lineage() -> TestResult {
    // @step Given a successful Copilot request owns opaque for an exact typed history
    let _environment = Environment::new()?;
    for failure in [false, true] {
        let server = support::server().await;
        let session = ProviderSession::for_provider("github-copilot");
        let model = support::model(&server, &session)?;
        let history = support::complete(&model, vec![], "seed").await?;
        // @step When its next real request fails or is cancelled before settlement
        let request = CompletionRequestBuilder::new(
            model.clone(),
            if failure { "failure" } else { "slow cancel" },
        )
        .messages(history.clone())
        .build();
        let mut stream = model.stream(request).await?;
        if failure {
            let outcome = async {
                while let Some(item) = stream.next().await {
                    item?;
                }
                Ok::<_, rig::completion::CompletionError>(())
            }
            .await;
            assert!(outcome.is_err());
        } else {
            assert!(
                tokio::time::timeout(Duration::from_millis(100), stream.next())
                    .await
                    .is_err()
            );
            support::wait_requests(&server, 2).await?;
        }
        drop(stream);
        // @step Then reconstructing the same session cannot replay the former opaque
        let reconstructed = support::model(&server, &session)?;
        support::complete(&reconstructed, history, "probe").await?;
        assert!(support::last_request(&server)
            .await?
            .get("reasoning_opaque")
            .is_none());
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn late_superseded_settlement_cannot_overwrite_or_clear_newer_lineage() -> TestResult {
    let _environment = Environment::new()?;
    for outcome in ["success", "failure", "cancel"] {
        // @step Given an older Copilot request is still in flight
        let server = support::server().await;
        let session = ProviderSession::for_provider("github-copilot");
        let model = support::model(&server, &session)?;
        let history = support::complete(&model, vec![], "seed").await?;
        let older_model = model.clone();
        let older_history = history.clone();
        let older = tokio::spawn(async move {
            support::complete(
                &older_model,
                older_history,
                if outcome == "failure" {
                    "slow failure"
                } else {
                    "slow success"
                },
            )
            .await
            .map_err(|error| error.to_string())
        });
        support::wait_requests(&server, 2).await?;
        // @step When a newer request settles before that older request succeeds fails or is cancelled
        session.begin_turn();
        let history = support::complete(&model, history, "new").await?;
        if outcome == "cancel" {
            older.abort();
            assert!(older.await.err().is_some_and(|error| error.is_cancelled()));
        } else {
            assert_eq!(older.await?.is_err(), outcome == "failure");
        }
        // @step Then the following compatible request retains only the newer opaque
        let reconstructed = support::model(&server, &session)?;
        support::complete(&reconstructed, history, "probe").await?;
        assert_eq!(
            support::last_request(&server).await?["reasoning_opaque"],
            "synthetic-opaque-new"
        );
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn opaque_remains_private_and_caller_supplied_values_are_rejected() -> TestResult {
    // @step Given a successful Copilot request owns opaque for an exact typed history
    let _environment = Environment::new()?;
    let server = support::server().await;
    let session = ProviderSession::for_provider("github-copilot");
    let model = support::model(&server, &session)?;
    let history = support::complete(&model, vec![], "seed").await?;
    // @step When diagnostics serialization writers and caller supplied opaque are exercised
    let writes = Arc::new(AtomicUsize::new(0));
    let observed = writes.clone();
    let writer: StateWriter = Arc::new(move |_| {
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    session.bind_writers(|_| writer.clone())?;
    assert!(!format!("{session:?}").contains("synthetic-opaque"));
    let extension = session
        .extension::<codelet_providers::copilot::session::CopilotSession>("github-copilot")
        .ok_or("missing extension")?;
    assert!(extension.status().is_none());
    let mut request = CompletionRequestBuilder::new(model.clone(), "probe")
        .messages(history)
        .build();
    request.additional_params = Some(json!({"reasoning_opaque":"caller-supplied-unowned"}));
    let prepared = model.prepare_request(request.clone())?;
    assert!(!String::from_utf8(prepared)?.contains("caller-supplied-unowned"));
    let mut stream = model.stream(request).await?;
    while let Some(item) = stream.next().await {
        item?;
    }
    // @step Then only the transport sends owned opaque and no diagnostic or checkpoint adopts it
    assert_eq!(
        support::last_request(&server).await?["reasoning_opaque"],
        "synthetic-opaque-seed"
    );
    assert!(!serde_json::to_string(&stream.response)?.contains("synthetic-opaque"));
    assert!(session.prepare_generation(4, "gpt-5")?.is_empty());
    assert_eq!(writes.load(Ordering::SeqCst), 0);
    session.restore(&[json!({"reasoning_opaque":"caller-supplied-unowned"}).to_string()])?;
    support::complete(&model, vec![], "probe").await?;
    assert!(support::last_request(&server)
        .await?
        .get("reasoning_opaque")
        .is_none());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn deployment_identity_remains_distinct_at_the_same_effective_api_origin() -> TestResult {
    // @step Given a successful Copilot request owns opaque for a public GitHub deployment
    let _environment = Environment::new()?;
    let server = support::server().await;
    let session = ProviderSession::for_provider("github-copilot");
    let public = support::model(&server, &session)?;
    let history = support::complete(&public, vec![], "seed").await?;
    // @step When an enterprise deployment reconstructs the same model at the same loopback API origin
    let enterprise = support::model_for_deployment(
        &server,
        &session,
        codelet_providers::copilot::CopilotDeploymentType::Enterprise {
            host: "synthetic-enterprise.invalid".into(),
        },
    )?;
    support::complete(&enterprise, history, "probe").await?;
    // @step Then the real request cannot reuse public deployment opaque
    assert!(support::last_request(&server)
        .await?
        .get("reasoning_opaque")
        .is_none());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn actual_provider_selection_invalidates_copilot_without_changing_codex_state() -> TestResult
{
    let _environment = Environment::new()?;
    for switched in [false, true] {
        // @step Given a successful Copilot request owns opaque for an exact typed history
        let server = support::server().await;
        let mut session = ProviderSession::for_provider("github-copilot");
        let codex = Arc::new(codelet_providers::codex::session::CodexSession::default());
        let metadata = codex.prepare_generation(17, "synthetic-codex-model")?;
        codex.install_committed_generation(17, &metadata)?;
        let initial_checkpoint = codex
            .history
            .snapshot()?
            .ok_or("missing initial checkpoint")?;
        assert_eq!(initial_checkpoint.generation, 17);
        assert_eq!(
            initial_checkpoint.model.as_deref(),
            Some("synthetic-codex-model")
        );
        let initial_status = codex.status();
        session.register("codex", codex.clone());
        let model = support::model(&server, &session)?;
        let history = support::complete(&model, vec![], "seed").await?;
        // @step When a read only provider lookup or an actual provider selection occurs
        if switched {
            session.ensure_provider("codex");
            session.ensure_provider("github-copilot");
        } else {
            assert!(session
                .extension::<codelet_providers::codex::session::CodexSession>("codex")
                .is_some());
        }
        // @step Then lookup preserves lineage but selection invalidates Copilot without resetting another extension
        support::complete(&support::model(&server, &session)?, history, "probe").await?;
        assert_eq!(
            support::last_request(&server)
                .await?
                .get("reasoning_opaque")
                .is_some(),
            !switched
        );
        assert_eq!(codex.history.generation()?, 17);
        assert_eq!(codex.history.snapshot()?, Some(initial_checkpoint));
        assert_eq!(codex.status(), initial_status);
    }
    Ok(())
}
