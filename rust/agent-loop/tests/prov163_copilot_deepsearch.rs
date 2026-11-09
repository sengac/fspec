//! PROV-163: DeepSearch sub-agents run on GitHub Copilot.
//!
//! Feature: spec/features/copilot-deepsearch-sub-agents.feature

#[path = "support/bug268_environment.rs"]
mod environment;

use codelet_providers::copilot::{write_copilot_auth, CopilotAuthJson};
use environment::{Environment, TestResult};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_copilot_sessions_deepsearch_query_is_answered_by_a_copilot_sub_agent() -> TestResult {
    // @step Given a github-copilot provider backed by a local Copilot endpoint that answers "found it"
    let environment = Environment::new()?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "deepsearch", "object": "chat.completion", "created": 1, "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "found it"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15}
        })))
        .mount(&server)
        .await;
    write_copilot_auth(&CopilotAuthJson {
        github_oauth_token: "synthetic-account".into(),
        copilot_token: Some("synthetic-short-token".into()),
        copilot_token_expires_at: Some(u64::MAX),
        endpoints_api: Some(server.uri()),
        enterprise_url: None,
    })
    .await?;

    // @step When DeepSearch runs a query for that provider
    let answer = codelet_agent_loop::deep_search_handler::execute_deep_search(
        environment.directory.path(),
        "where is the config loaded?",
        None,
        3,
        "github-copilot",
        Some("gpt-4o"),
        0,
        1,
        None,
        None,
    )
    .await;

    // @step Then the sub-agent's answer "found it" is returned
    assert_eq!(answer.as_deref(), Ok("found it"));

    // @step And the Copilot endpoint received the DeepSearch request
    let requests = server.received_requests().await.unwrap_or_default();
    assert!(
        requests
            .iter()
            .any(|request| request.url.path().ends_with("/chat/completions")),
        "the Copilot chat endpoint saw no request: {:?}",
        requests
            .iter()
            .map(|r| r.url.path().to_string())
            .collect::<Vec<_>>()
    );
    Ok(())
}
