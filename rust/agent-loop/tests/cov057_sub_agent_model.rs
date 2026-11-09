//! Feature: spec/features/sub-agent-model-inheritance.feature
//!
//! COV-057: DeepSearch / AgentManager handlers re-captured from the live
//! session after a mid-session model switch. Sessions live in an isolated,
//! offline `SessionManager`; the switch goes through the real
//! `SessionManagerHandle::set_model` (the `set_session_model` RPC), and the
//! handlers are re-registered with the same `bridges::register_*` calls the
//! agent loop makes at the start of each turn. DeepSearch is invoked through
//! the parent's real `DeepSearchTool`; the sub-agent's entry diagnostic
//! records the captured provider/model/limits. Cloud sub-agents are kept
//! offline by removing their API key just before the call, so they stop at
//! provider construction; the profile scenario reaches a loopback server.

#[path = "support/bug268_environment.rs"]
mod environment;
#[path = "cov057_sub_agent_model/harness.rs"]
mod harness;

use environment::TestResult;
use harness::{Fixture, CLAUDE};
use serde_json::json;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

/// Scenario: DeepSearch uses updated model after mid-session model switch
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_deep_search_uses_updated_model_after_switch() -> TestResult {
    // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    let fixture = Fixture::created_with(CLAUDE).await?;
    // @step And the DeepSearch handler was registered at session creation
    fixture.register_handlers().await?;
    // @step When the user switches the model to "google/gemini-2.5-pro" via set_session_model
    fixture.switch_model("google", "gemini-2.5-pro")?;
    // @step And the user invokes DeepSearch
    fixture.register_handlers().await?;
    let (result, entry) = fixture
        .deep_search(Some("GOOGLE_GENERATIVE_AI_API_KEY"))
        .await;
    // @step Then the DeepSearch sub-agent should use provider "gemini" and model "gemini-2.5-pro"
    assert!(
        entry.contains("provider='gemini' model=Some(\"gemini-2.5-pro\")"),
        "{entry}"
    );
    assert!(result.is_err_and(|error| error.contains("gemini")));
    Ok(())
}

/// Scenario: AgentManager uses updated model after mid-session model switch
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_agent_manager_uses_updated_model_after_switch() -> TestResult {
    // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    let fixture = Fixture::created_with(CLAUDE).await?;
    // @step And the AgentManager handler was registered at session creation
    fixture.register_handlers().await?;
    // @step When the user switches the model to "google/gemini-2.5-pro" via set_session_model
    fixture.switch_model("google", "gemini-2.5-pro")?;
    // @step And the user spawns a subordinate via AgentManager
    fixture.register_handlers().await?;
    let (model, _, _) = fixture.spawn().await?;
    // @step Then the subordinate should be created with the updated model "gemini-2.5-pro"
    assert_eq!(model.as_deref(), Some("google/gemini-2.5-pro"));
    Ok(())
}

/// Scenario: DeepSearch respects facade_override for custom models
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_deep_search_respects_facade_override() -> TestResult {
    // @step Given a session was created with a MODEL-004 custom model registered under "openai" with facade_override "claude"
    let fixture = Fixture::created_with(CLAUDE).await?;
    {
        let session = fixture.manager.get_session(&fixture.id.to_string())?;
        let mut inner = session.inner.lock().await;
        inner.provider_manager_mut().set_model_direct(
            "openai",
            "custom-routed-model",
            Some(200000),
            Some(8192),
            Some("claude".to_string()),
        )?;
    }
    fixture.register_handlers().await?;
    // @step When the user invokes DeepSearch
    let (result, entry) = fixture.deep_search(Some("ANTHROPIC_API_KEY")).await;
    // @step Then the DeepSearch sub-agent should use provider "claude" not "openai"
    assert!(entry.contains("provider='claude'"), "{entry}");
    assert!(!entry.contains("provider='openai'"), "{entry}");
    assert!(result.is_err_and(|error| error.contains("claude")));
    Ok(())
}

/// Scenario: Handler re-registration updates all four captured values
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_re_registration_updates_all_four_captured_values() -> TestResult {
    // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    let fixture = Fixture::created_with(CLAUDE).await?;
    fixture.register_handlers().await?;
    // @step When the user switches the model to "google/gemini-2.5-pro" via set_session_model resolving context_window 1048576 and max_output_tokens 65536
    fixture.switch_model("google", "gemini-2.5-pro")?;
    fixture.register_handlers().await?;
    // @step Then the DeepSearch handler should capture provider "gemini", model "gemini-2.5-pro", context_window 1048576, and max_output_tokens 65536
    let (_, entry) = fixture
        .deep_search(Some("GOOGLE_GENERATIVE_AI_API_KEY"))
        .await;
    assert!(
        entry.contains("provider='gemini' model=Some(\"gemini-2.5-pro\")"),
        "{entry}"
    );
    assert!(
        entry.contains("context_window=Some(1048576) max_output_tokens=Some(65536)"),
        "{entry}"
    );
    // @step And the AgentManager handler should capture model "gemini-2.5-pro", context_window 1048576, and max_output_tokens 65536
    std::env::set_var(
        "GOOGLE_GENERATIVE_AI_API_KEY",
        "AIza-synthetic-loopback-only",
    );
    assert_eq!(
        fixture.spawn().await?,
        (
            Some("google/gemini-2.5-pro".to_string()),
            Some(1048576),
            Some(65536)
        )
    );
    Ok(())
}

/// Scenario: No regression when model is never changed
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_no_regression_when_model_is_never_changed() -> TestResult {
    // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    let fixture = Fixture::created_with(CLAUDE).await?;
    // @step And no model switch occurs during the session
    fixture.register_handlers().await?;
    // @step When the user invokes DeepSearch
    fixture.register_handlers().await?;
    let (result, entry) = fixture.deep_search(Some("ANTHROPIC_API_KEY")).await;
    // @step Then the DeepSearch sub-agent should use provider "claude" and model "claude-sonnet-4-20250514"
    assert!(
        entry.contains("provider='claude' model=Some(\"claude-sonnet-4-20250514\")"),
        "{entry}"
    );
    assert!(result.is_err_and(|error| error.contains("claude")));
    Ok(())
}

/// Scenario: session_set_model_profile also triggers handler re-registration
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_profile_switch_also_triggers_re_registration() -> TestResult {
    // @step Given a session was created with model "anthropic/claude-sonnet-4-20250514"
    let fixture = Fixture::created_with(CLAUDE).await?;
    fixture.register_handlers().await?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cov057", "object": "chat.completion", "created": 1, "model": "gpt-4o",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "profiled"},
                "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 2, "total_tokens": 12}
        })))
        .mount(&server)
        .await;
    let user_dir = fixture.environment.directory.path().join(".fspec");
    std::fs::create_dir_all(&user_dir)?;
    // BUG-448: PROV-171 — a profile model needs declared limits; the test's
    // isolated environment has no catalog entry for gpt-4o.
    let profile = json!({"baseUrl": server.uri(), "apiKey": "synthetic-loopback-only",
        "contextWindow": 128000,
        "customModels": [{"id": "gpt-4o", "maxOutputTokens": 4096}]});
    let config = json!({"providers": {"openai": {"profiles": {"local": profile}}}});
    std::fs::write(user_dir.join("fspec-config.json"), config.to_string())?;
    // @step When the user switches the model via set_session_model to profile "openai:local" model "gpt-4o"
    fixture.switch_model("openai:local", "gpt-4o")?;
    // @step And the user invokes DeepSearch
    fixture.register_handlers().await?;
    let (result, entry) = fixture.deep_search(None).await;
    // @step Then the DeepSearch sub-agent should use provider "openai" and model "gpt-4o"
    assert!(
        entry.contains("provider='openai' model=Some(\"gpt-4o\")"),
        "{entry}"
    );
    assert_eq!(result.as_deref(), Ok("profiled"));
    let requests = server.received_requests().await.unwrap_or_default();
    let request = requests
        .iter()
        .find(|request| request.url.path().ends_with("/chat/completions"))
        .ok_or("the profile endpoint saw no DeepSearch request")?;
    assert_eq!(request.body_json::<serde_json::Value>()?["model"], "gpt-4o");
    Ok(())
}
