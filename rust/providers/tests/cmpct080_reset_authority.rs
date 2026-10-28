//! Feature: spec/features/codex-explicit-reset-authority.feature

use codelet_providers::{
    codex::{history::CodexHistory, session::CodexSession},
    session::ProviderSession,
};
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};
use std::{fs, sync::Arc};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn history(session: &ProviderSession) -> Result<&CodexHistory, Box<dyn std::error::Error>> {
    Ok(&session
        .extension::<CodexSession>("codex")
        .ok_or("Codex missing")?
        .history)
}

fn request() -> Value {
    json!({"model":"shared-model","input":[{"role":"user","content":[{"type":"input_text","text":"new work"}]}]})
}

#[test]
fn explicit_reset_removes_continuation_and_cold_restores_its_generation_watermark() -> TestResult {
    // @step Given a durable Codex continuation and an in-flight request from its generation
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let session = ProviderSession::for_provider("codex");
    session.bind_writers(|_| {
        let path = path.clone();
        Arc::new(move |metadata| {
            fs::write(&path, serde_json::to_vec(&metadata)?).map_err(|error| {
                codelet_providers::session::ExtensionError::Persistence(error.to_string())
            })?;
            Ok(())
        })
    })?;
    history(&session)?.commit(&request(), &[])?;
    let stale = fs::read_to_string(&path)?;
    let generation = history(&session)?.generation()?;
    // @step When I explicitly reset the provider session twice
    session.reset()?;
    session.reset()?;
    // @step Then live and durable continuation are absent with an advancing reset watermark
    assert!(history(&session)?.snapshot()?.is_none());
    assert_eq!(history(&session)?.generation()?, generation + 2);
    let durable: Value = serde_json::from_slice(&fs::read(&path)?)?;
    assert!(durable["codexContinuation"].is_null());
    assert_eq!(durable["_codexReset"]["generation"], generation + 2);
    // @step And a cold restore rejects older checkpoints and committed boundaries and accepts new work at the reset generation
    let cold = ProviderSession::for_provider("codex");
    cold.restore(&[durable.to_string()])?;
    assert!(history(&cold)?.snapshot()?.is_none());
    assert_eq!(history(&cold)?.generation()?, generation + 2);
    assert!(cold.validate_restore(&[stale]).is_err());
    let old_session = ProviderSession::for_provider("codex");
    let old_metadata = old_session.prepare_generation(1, "shared-model")?;
    let old_boundary = json!({"_synthetic":true,"_compactionSummary":true,"_committedCompaction":{"version":1,"generation":1},"codexContinuation":old_metadata["codexContinuation"]}).to_string();
    assert!(cold
        .validate_committed_restore(1, &old_metadata, &[old_boundary])
        .is_err());
    history(&cold)?.commit(&request(), &[])?;
    assert_eq!(
        history(&cold)?
            .snapshot()?
            .ok_or("new checkpoint missing")?
            .generation,
        generation + 2
    );
    // @step And committed generation boundaries remain present checkpoints
    cold.transition_generation(generation + 3)?;
    assert_eq!(
        history(&cold)?
            .snapshot()?
            .ok_or("boundary hidden")?
            .generation,
        generation + 3
    );
    Ok(())
}

#[test]
fn a_newer_committed_boundary_supersedes_retained_older_reset_records() -> TestResult {
    // @step Given a durable reset record retained behind a newer committed Codex boundary
    let directory = setup_temp_directory()?;
    let path = directory.path().join("checkpoint.json");
    let session = ProviderSession::for_provider("codex");
    session.bind_writers(|_| {
        let path = path.clone();
        Arc::new(move |metadata| {
            fs::write(&path, serde_json::to_vec(&metadata)?).map_err(|error| {
                codelet_providers::session::ExtensionError::Persistence(error.to_string())
            })?;
            Ok(())
        })
    })?;
    history(&session)?.commit(&request(), &[])?;
    session.reset()?;
    let retained = fs::read_to_string(&path)?;
    let metadata = session.prepare_generation(2, "shared-model")?;
    let boundary = json!({"_synthetic":true,"_compactionSummary":true,"_committedCompaction":{"version":1,"generation":2},"codexContinuation":metadata["codexContinuation"]}).to_string();
    let replay = vec![boundary, retained];
    // @step When I validate and cold restore the complete retained replay
    let cold = ProviderSession::for_provider("codex");
    cold.validate_restore(&replay)?;
    cold.restore(&replay)?;
    // @step Then the newer committed checkpoint remains present and authoritative
    let checkpoint = history(&cold)?
        .snapshot()?
        .ok_or("committed boundary removed")?;
    assert_eq!(checkpoint.generation, 2);
    assert_eq!(checkpoint.model.as_deref(), Some("shared-model"));
    // @step And older reset records cannot remove the newer continuation
    assert_eq!(
        serde_json::to_value(checkpoint)?,
        metadata["codexContinuation"]
    );
    history(&cold)?.prepare(&mut request())?;
    Ok(())
}

#[path = "fixtures/prov147.rs"]
pub mod fixture;

#[tokio::test]
async fn an_in_flight_response_cannot_publish_after_explicit_reset() -> TestResult {
    use codelet_providers::{
        codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel},
        request_admission::AdmissionLimits,
    };
    use rig::completion::{CompletionModel, CompletionRequestBuilder};
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};
    // @step Given a nonstreaming Codex request awaiting its physical response
    let session = ProviderSession::for_provider("codex");
    history(&session)?.commit(&request(), &[])?;
    let generation = history(&session)?.generation()?;
    let server = MockServer::start().await;
    let resetting = session.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &wiremock::Request| {
            // @step When an explicit reset removes continuation before that response arrives
            assert!(resetting.reset().is_ok());
            ResponseTemplate::new(200).set_body_json(fixture::response())
        })
        .expect(1)
        .mount(&server)
        .await;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .http_client(RefreshingCodexClient::new_api_key())
        .build()?;
    let model = CodexResponsesModel::new(client, "shared-model")
        .with_history(history(&session)?.clone())
        .with_request_admission(AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(1000),
        });
    let result = model
        .completion(CompletionRequestBuilder::new(model.clone(), "new work").build())
        .await;
    // @step Then the old response fails without publishing completion or restoring continuation
    assert!(result.is_err());
    assert!(history(&session)?.snapshot()?.is_none());
    // @step And the reset generation remains authoritative
    assert_eq!(history(&session)?.generation()?, generation + 1);
    server.verify().await;
    Ok(())
}

#[test]
fn bare_or_malformed_reset_metadata_cannot_authorize_missing_continuation() -> TestResult {
    // @step Given an existing Codex continuation
    for marker in [
        Value::Null,
        json!({"version":999,"generation":1,"protocol":"full-current-dag"}),
        json!({"version":1,"generation":0,"protocol":"full-current-dag"}),
        json!({"version":1,"generation":1,"protocol":"other"}),
    ] {
        let session = ProviderSession::for_provider("codex");
        history(&session)?.commit(&request(), &[])?;
        let previous = history(&session)?.snapshot()?;
        // @step When I restore null continuation without a valid advancing reset marker
        let envelope = json!({"codexContinuation":null,"_codexReset":marker}).to_string();
        let result = session.restore(&[envelope]);
        // @step Then restore fails without replacing the authoritative checkpoint
        assert!(result.is_err());
        assert_eq!(history(&session)?.snapshot()?, previous);
        // @step And inference remains blocked rather than becoming a fresh session
        assert!(history(&session)?.prepare(&mut request()).is_err());
    }
    Ok(())
}

#[test]
fn explicit_non_codex_authority_invalidates_only_untyped_dormant_continuation() -> TestResult {
    // @step Given a dormant Codex extension and ordinary non-Codex replay including a committed summary without Codex metadata
    let session = ProviderSession::for_provider("codex");
    history(&session)?.commit(&request(), &[])?;
    let typed = json!({"codexContinuation":history(&session)?.snapshot()?}).to_string();
    let ordinary = vec![
        json!({"_synthetic":true,"_compactionSummary":true,"_committedCompaction":{"version":1,"generation":1},"message":{"role":"user","content":"committed ordinary summary"}}).to_string(),
        json!({"message":{"role":"user","content":"legacy"}}).to_string(),
    ];
    let original = ordinary.clone();
    let previous = history(&session)?.snapshot()?;
    let generation = history(&session)?.generation()?;
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    session.bind_writers(|_| {
        let writes = writes.clone();
        Arc::new(move |_| {
            writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
    })?;
    // @step When I validate and install that replay with explicit non-Codex authority
    session.validate_restore_for_provider("anthropic", &ordinary)?;
    assert_eq!(history(&session)?.snapshot()?, previous);
    assert_eq!(history(&session)?.generation()?, generation);
    session.restore_for_provider("anthropic", &ordinary)?;
    // @step Then the replay remains unchanged and dormant continuation is absent
    assert_eq!(ordinary, original);
    assert!(history(&session)?.snapshot()?.is_none());
    assert_eq!(writes.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(history(&session)?.generation()?, generation + 1);
    // @step And typed Codex replay still restores under non-Codex selection
    let fresh = ProviderSession::for_provider("codex");
    fresh.validate_restore_for_provider("anthropic", std::slice::from_ref(&typed))?;
    fresh.restore_for_provider("anthropic", &[typed])?;
    assert!(history(&fresh)?.snapshot()?.is_some());
    // @step And active Codex replay still rejects missing or null checkpoints under bare or qualified selection
    for provider in ["codex", "codex/shared-model"] {
        assert!(fresh
            .validate_restore_for_provider(provider, &ordinary)
            .is_err());
        assert!(fresh
            .validate_restore_for_provider(
                provider,
                &[json!({"codexContinuation":null}).to_string()]
            )
            .is_err());
    }
    Ok(())
}
