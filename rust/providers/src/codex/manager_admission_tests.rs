//! Feature: spec/features/codex-manager-request-admission.feature

use super::*;
use crate::{request_admission::RequestAdmissionError, ProviderManager, ProviderType};
use futures::StreamExt;
use rig::completion::CompletionRequestBuilder;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;

#[path = "../../tests/prov159_support/mod.rs"]
mod auth_fixture;
#[path = "../../tests/fixtures/prov148.rs"]
mod fixture;
#[path = "../../tests/custom_test_helpers.rs"]
mod helpers;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn manager() -> Result<(auth_fixture::Fixture, ProviderManager), Box<dyn std::error::Error>> {
    let fixture = auth_fixture::Fixture::new()?;
    std::fs::write(
        fixture.directory.path().join("auth.json"),
        json!({"OPENAI_API_KEY":"sk-manager-admission-test"}).to_string(),
    )?;
    // @step Then the context override wins and output uses the matching catalog limit
    codelet_common::set_data_directory(fixture.directory.path().to_path_buf())?;
    std::fs::create_dir_all(fixture.directory.path().join("cache"))?;
    let model = |context, output| {
        json!({
            "id":"shared-model", "name":"Catalog model", "tool_call":true,
            "limit":{"context":context,"output":output}
        })
    };
    std::fs::write(
        fixture.directory.path().join("cache/models.json"),
        json!({"openai":{"id":"openai","name":"OpenAI","env":[],"models":{
            "shared-model":model(10000,100), "second-model":model(20000,200)
        }}})
        .to_string(),
    )?;
    Ok((fixture, ProviderManager::with_model_support().await?))
}

#[tokio::test]
#[serial_test::serial]
async fn unprofiled_codex_selections_resolve_and_refresh_source_catalog_limits() -> TestResult {
    let (_fixture, mut manager) = manager().await?;
    for (model, context, output) in [("shared-model", 10000, 100), ("second-model", 20000, 200)] {
        manager.set_model_direct_with_profile("codex", model, None, None, None, None)?;
        assert_eq!(manager.current_provider_type(), &ProviderType::Codex);
        assert_eq!(
            manager.selected_model_string(),
            Some(format!("codex/{model}"))
        );
        assert_eq!(manager.raw_model_context_window(), Some(context));
        assert_eq!(manager.raw_model_max_output_tokens(), Some(output));
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn explicit_overrides_remain_separate_from_catalog_limits() -> TestResult {
    let (_fixture, mut manager) = manager().await?;
    manager.set_model_direct("codex", "shared-model", Some(5000), None, None)?;
    assert_eq!(manager.registry_context_window, Some(10000));
    assert_eq!(manager.registry_max_output_tokens, Some(100));
    assert_eq!(manager.context_window(), 5000);
    assert_eq!(manager.max_output_tokens(), 100);
    manager.set_model_direct("codex", "shared-model", None, Some(50), None)?;
    assert_eq!(manager.context_window(), 10000);
    assert_eq!(manager.max_output_tokens(), 50);
    manager.set_model_direct("codex", "shared-model", None, None, None)?;
    assert_eq!(manager.context_window(), 10000);
    assert_eq!(manager.max_output_tokens(), 100);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn profile_selections_inherit_catalog_limits_for_fields_without_overrides() -> TestResult {
    // @step Given a profile model whose identifier also exists in the OpenAI catalog
    let (_fixture, mut manager) = manager().await?;
    manager.set_model_direct("codex", "shared-model", None, None, None)?;
    // @step When I select it with an explicit profile context limit and automatic output
    manager.set_model_direct_with_profile(
        "codex",
        "shared-model",
        Some("local"),
        Some(5000),
        None,
        None,
    )?;
    assert_eq!(manager.raw_model_context_window(), Some(5000));
    assert_eq!(manager.registry_context_window, Some(10000));
    assert_eq!(manager.registry_max_output_tokens, Some(100));
    assert_eq!(manager.raw_model_max_output_tokens(), Some(100));
    assert_eq!(manager.max_output_tokens(), 100);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn unknown_selections_clear_previously_known_limits() -> TestResult {
    let (_fixture, mut manager) = manager().await?;
    manager.set_model_direct("codex", "shared-model", Some(5000), Some(50), None)?;
    manager.set_model_direct("codex", "unknown-model", None, None, None)?;
    assert_eq!(manager.raw_model_context_window(), None);
    assert_eq!(manager.raw_model_max_output_tokens(), None);
    assert_eq!(
        (manager.context_window(), manager.max_output_tokens()),
        (0, 0)
    );
    Ok(())
}

async fn constructed_model(
    manager: &ProviderManager,
    server: &fixture::Server,
) -> Result<CodexResponsesModel, Box<dyn std::error::Error>> {
    let _model_env = helpers::EnvGuard::remove("CODEX_MODEL");
    let mut model = manager.get_codex()?.completion_model();
    model.client = server.models()?.0.client;
    Ok(model)
}

#[tokio::test]
#[serial_test::serial]
async fn a_manager_built_codex_provider_sends_a_fitting_completion() -> TestResult {
    let (_fixture, mut manager) = manager().await?;
    manager.set_model_direct("codex", "shared-model", None, None, None)?;
    let server =
        fixture::Server::start(vec![fixture::baseline::response().to_string()], false).await?;
    let model = constructed_model(&manager, &server).await?;
    let result = model
        .completion(CompletionRequestBuilder::new(model.clone(), "Hello").build())
        .await?;
    assert_eq!(result.usage.total_tokens, 20);
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture lock poisoned")?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn a_manager_built_codex_provider_sends_a_fitting_stream() -> TestResult {
    let (_fixture, mut manager) = manager().await?;
    manager.set_model_direct("codex", "shared-model", None, None, None)?;
    let server = fixture::Server::start(
        vec![fixture::terminal("completed", Some(true), None)],
        false,
    )
    .await?;
    let observed = fixture::collect(constructed_model(&manager, &server).await?).await?;
    assert!(observed.errors.is_empty(), "{:?}", observed.errors);
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.finals, [20]);
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "capture lock poisoned")?
            .len(),
        1
    );
    Ok(())
}

fn admission_error(
    error: CompletionError,
) -> Result<RequestAdmissionError, Box<dyn std::error::Error>> {
    match error {
        CompletionError::RequestError(error) => error
            .downcast_ref::<RequestAdmissionError>()
            .cloned()
            .ok_or_else(|| format!("untyped admission error: {error}").into()),
        error => Err(error.into()),
    }
}

#[tokio::test]
#[serial_test::serial]
async fn a_manager_built_codex_provider_blocks_missing_and_oversized_admission() -> TestResult {
    let (_fixture, mut manager) = manager().await?;
    for (capacity, reservation) in [(None, Some(10)), (Some(1000), None), (Some(100), Some(10))] {
        manager.set_model_direct("codex", "unknown-model", capacity, reservation, None)?;
        for streaming in [false, true] {
            let server = fixture::Server::start(
                vec![fixture::terminal("completed", Some(true), None)],
                false,
            )
            .await?;
            let model = constructed_model(&manager, &server).await?;
            let request =
                CompletionRequestBuilder::new(model.clone(), "large request ".repeat(100)).build();
            let error = if streaming {
                let mut stream = model.stream(request).await?;
                let mut failure = None;
                while let Some(event) = stream.next().await {
                    match event {
                        Err(error) => failure = Some(admission_error(error)?),
                        Ok(StreamedAssistantContent::Final(_)) => {
                            return Err("blocked stream completed".into())
                        }
                        _ => {}
                    }
                }
                failure.ok_or("missing stream admission error")?
            } else {
                admission_error(
                    model
                        .completion(request)
                        .await
                        .err()
                        .ok_or("blocked request completed")?,
                )?
            };
            match (capacity, reservation) {
                (None, _) => assert_eq!(error, RequestAdmissionError::MissingContextCapacity),
                (_, None) => assert_eq!(error, RequestAdmissionError::MissingOutputReservation),
                _ => assert!(matches!(
                    error,
                    RequestAdmissionError::BudgetExceeded { .. }
                )),
            }
            assert!(server
                .requests
                .lock()
                .map_err(|_| "capture lock poisoned")?
                .is_empty());
        }
    }
    Ok(())
}
