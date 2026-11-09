//! BUG-562 — the engine limit lookup (BUG-547) reads the selected model's
//! own props on a llama.cpp router.
//!
//! Feature: spec/features/the-engine-limit-lookup-reads-the-selected-model-on-a-llama-cpp-router.feature
//! Research: spec/attachments/BUG-512/llama-cpp-router-mode-research.md
//!
//! The llama.cpp server is a local wiremock double, so every answer is fixed.

use codelet_providers::engine_limits::context_window_report;
use codelet_providers::local_reasoning::LocalEngine;
use codelet_test_helpers::{OrFail, OrFailErr, TestResult};
use serde_json::{json, Value};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const GEMMA: &str = "gemma-4";

fn props(n_ctx: u64) -> Value {
    json!({"default_generation_settings": {"params": {}, "n_ctx": n_ctx}})
}

/// A router that answers the bare `GET /props` with its own props (n_ctx 0)
/// and `GET /props?model=gemma-4&autoload=false` with `answer`.
async fn router(answer: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .and(query_param_is_missing("model"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "role": "router",
            "model_alias": "llama-server",
            "default_generation_settings": {"params": {}, "n_ctx": 0}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .and(query_param("model", GEMMA))
        .and(query_param("autoload", "false"))
        .respond_with(answer)
        .mount(&server)
        .await;
    server
}

fn base_url(server: &MockServer) -> String {
    format!("{}/v1", server.uri())
}

#[tokio::test]
async fn on_a_router_the_context_window_comes_from_the_selected_models_own_props() -> TestResult {
    // @step Given a llama.cpp router whose bare GET /props answers the router's own props with n_ctx 0
    // @step And the router answers GET /props for model "gemma-4" without autoload with n_ctx 32768
    let server = router(ResponseTemplate::new(200).set_body_json(props(32768))).await;
    // @step When the running context window of "gemma-4" is asked for
    let limit =
        context_window_report(LocalEngine::LlamaCpp, &base_url(&server), None, GEMMA).await?;
    // @step Then it is 32768 from llama.cpp GET /props default_generation_settings.n_ctx
    assert_eq!(limit.tokens, 32768);
    assert_eq!(
        limit.source,
        "llama.cpp GET /props default_generation_settings.n_ctx"
    );
    Ok(())
}

#[tokio::test]
async fn a_router_that_has_not_loaded_the_model_fails_the_lookup_with_its_own_reason() -> TestResult
{
    // @step Given a llama.cpp router that answers GET /props for model "gemma-4" without autoload with HTTP 400 and the error message "model is not loaded"
    let server = router(ResponseTemplate::new(400).set_body_json(json!({"error": {
        "code": 400,
        "message": "model is not loaded",
        "type": "invalid_request_error"
    }})))
    .await;
    // @step When the running context window of "gemma-4" is asked for
    let reason = context_window_report(LocalEngine::LlamaCpp, &base_url(&server), None, GEMMA)
        .await
        .or_fail_err("the lookup fails")?;
    // @step Then the lookup fails naming llama.cpp GET /props and "model is not loaded"
    assert!(
        reason.contains("llama.cpp GET /props") && reason.contains("model is not loaded"),
        "{reason}"
    );
    // @step And the router was never asked to load the model
    let requests = server
        .received_requests()
        .await
        .or_fail("requests are recorded")?;
    assert!(!requests.is_empty());
    for request in &requests {
        let autoload = request
            .url
            .query_pairs()
            .find(|(key, _)| key == "autoload")
            .map(|(_, value)| value.into_owned());
        assert_eq!(autoload.as_deref(), Some("false"), "{}", request.url);
    }
    Ok(())
}

#[tokio::test]
async fn a_single_model_server_is_asked_as_before() -> TestResult {
    // @step Given a single-model llama.cpp server that answers GET /props with n_ctx 65536 whatever the query
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .respond_with(ResponseTemplate::new(200).set_body_json(props(65536)))
        .mount(&server)
        .await;
    // @step When the running context window of "gemma-4" is asked for
    let limit =
        context_window_report(LocalEngine::LlamaCpp, &base_url(&server), None, GEMMA).await?;
    // @step Then it is 65536
    assert_eq!(limit.tokens, 65536);
    Ok(())
}
