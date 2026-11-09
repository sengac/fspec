//! BUG-512 — the MODEL-016 live check reads the selected model on a
//! llama.cpp router (several models behind one server).
//!
//! Feature: spec/features/the-llama-cpp-live-check-reads-the-selected-model-on-a-router.feature
//! Research: spec/attachments/BUG-512/llama-cpp-router-mode-research.md
//!
//! The llama.cpp server is a local wiremock double, so every answer is fixed.

use codelet_providers::local_reasoning::{
    resolve_local_reasoning, LiveCheck, LiveCheckReport, ThinkingLevelsDeclaration,
};
use codelet_test_helpers::{OrFail, TestResult};
use serde_json::{json, Value};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const GEMMA: &str = "gemma-4";

/// llama.cpp `/props` of one model whose template reports `levels`.
fn model_props(levels: &[&str]) -> Value {
    json!({
        "model_alias": GEMMA,
        "chat_template_caps": {"supports_reasoning_effort": true, "reasoning_effort_levels": levels}
    })
}

/// The router's own `/props` (server-models.cpp:2021-2046): no template caps.
fn router_props() -> Value {
    json!({
        "role": "router",
        "max_instances": 4,
        "models_autoload": true,
        "model_alias": "llama-server",
        "model_path": "none",
        "default_generation_settings": {"params": {}, "n_ctx": 0}
    })
}

/// A router that answers the bare `GET /props` with its own props and
/// `GET /props?model=<model>&autoload=false` with `answer`.
async fn router(model: &str, answer: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .and(query_param_is_missing("model"))
        .respond_with(ResponseTemplate::new(200).set_body_json(router_props()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .and(query_param("model", model))
        .and(query_param("autoload", "false"))
        .respond_with(answer)
        .mount(&server)
        .await;
    server
}

/// The live check of a llama.cpp profile model declaring `efforts`, at the
/// server's `/v1` base URL.
fn check(
    server: &MockServer,
    efforts: &[&str],
) -> Result<LiveCheck, codelet_test_helpers::TestError> {
    let declaration: ThinkingLevelsDeclaration =
        serde_json::from_value(json!({ "efforts": efforts }))?;
    let reasoning =
        resolve_local_reasoning("llama", GEMMA, Some("llama-cpp"), None, Some(&declaration))?
            .or_fail("the model declares levels")?;
    Ok(
        LiveCheck::for_model(&format!("{}/v1", server.uri()), None, GEMMA, &reasoning)
            .or_fail("llama.cpp has a live endpoint")?,
    )
}

fn failed(report: LiveCheckReport) -> Result<String, codelet_test_helpers::TestError> {
    match report {
        LiveCheckReport::Failed(message) => Ok(message),
        other => Err(format!("expected a failed check, got {other:?}").into()),
    }
}

#[tokio::test]
async fn on_a_router_the_live_check_reads_the_selected_models_own_props() -> TestResult {
    // @step Given a llama.cpp router serving "gemma-4" and "qwen3" whose bare GET /props answers the router's own props
    // @step And a llama.cpp profile whose model "gemma-4" declares efforts "none, low, high"
    // @step And the router answers GET /props for model "gemma-4" without autoload with reasoning_effort_levels "low, high"
    let server = router(
        GEMMA,
        ResponseTemplate::new(200).set_body_json(model_props(&["low", "high"])),
    )
    .await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .and(query_param("model", "qwen3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(model_props(&["minimal"])))
        .mount(&server)
        .await;
    let check = check(&server, &["none", "low", "high"])?;
    // @step When the model's capability is resolved
    let report = check.run().await;
    // @step Then no mismatch is reported
    assert_eq!(report, LiveCheckReport::Agrees);
    Ok(())
}

#[tokio::test]
async fn a_router_models_own_levels_that_differ_from_the_declaration_are_reported() -> TestResult {
    // @step Given a llama.cpp router whose model "gemma-4" reports reasoning_effort_levels "low, medium, high" on GET /props for that model
    let server = router(
        GEMMA,
        ResponseTemplate::new(200).set_body_json(model_props(&["low", "medium", "high"])),
    )
    .await;
    // @step And a llama.cpp profile whose model "gemma-4" declares efforts "low, high, max"
    let check = check(&server, &["low", "high", "max"])?;
    // @step When the model's capability is resolved
    let report = check.run().await;
    // @step Then a mismatch is reported naming "gemma-4", the declared "low, high, max" and the reported "low, medium, high"
    let message = match report {
        LiveCheckReport::Mismatch(message) => message,
        other => return Err(format!("expected a mismatch, got {other:?}").into()),
    };
    assert!(
        message.contains(GEMMA)
            && message.contains("low, high, max")
            && message.contains("low, medium, high"),
        "{message}"
    );
    Ok(())
}

#[tokio::test]
async fn a_router_that_has_not_loaded_the_model_is_reported_as_a_check_that_could_not_be_made(
) -> TestResult {
    // @step Given a llama.cpp router that answers GET /props for model "gemma-4" without autoload with HTTP 400 and the error message "model is not loaded"
    let server = router(
        GEMMA,
        ResponseTemplate::new(400).set_body_json(json!({"error": {
            "code": 400,
            "message": "model is not loaded",
            "type": "invalid_request_error"
        }})),
    )
    .await;
    // @step And a llama.cpp profile whose model "gemma-4" declares efforts "low, high"
    let check = check(&server, &["low", "high"])?;
    // @step When the model's capability is resolved
    let report = check.run().await;
    // @step Then a failed live check is reported naming "gemma-4", GET /props and "model is not loaded"
    let message = failed(report)?;
    assert!(
        message.contains(GEMMA)
            && message.contains("GET /props")
            && message.contains("model is not loaded"),
        "{message}"
    );
    // @step And the levels offered are "low, high"
    assert!(
        message.contains("its declared levels still apply"),
        "{message}"
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
async fn props_without_template_capabilities_are_a_check_that_could_not_be_made() -> TestResult {
    // @step Given a llama.cpp server whose GET /props answers the router's own props with no chat_template_caps
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .respond_with(ResponseTemplate::new(200).set_body_json(router_props()))
        .mount(&server)
        .await;
    // @step And a llama.cpp profile whose model "gemma-4" declares efforts "low, high"
    let check = check(&server, &["low", "high"])?;
    // @step When the model's capability is resolved
    let report = check.run().await;
    // @step Then a failed live check is reported naming "gemma-4", GET /props and chat_template_caps.reasoning_effort_levels
    let message = failed(report)?;
    assert!(
        message.contains(GEMMA)
            && message.contains("GET /props")
            && message.contains("chat_template_caps.reasoning_effort_levels"),
        "{message}"
    );
    Ok(())
}

#[tokio::test]
async fn a_single_model_server_is_checked_as_before() -> TestResult {
    // @step Given a single-model llama.cpp server that answers GET /props with reasoning_effort_levels "low, high" whatever the query
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/props"))
        .respond_with(ResponseTemplate::new(200).set_body_json(model_props(&["low", "high"])))
        .mount(&server)
        .await;
    // @step And a llama.cpp profile whose model "gemma-4" declares efforts "low, high"
    let check = check(&server, &["low", "high"])?;
    // @step When the model's capability is resolved
    let report = check.run().await;
    // @step Then no mismatch is reported
    assert_eq!(report, LiveCheckReport::Agrees);
    Ok(())
}
