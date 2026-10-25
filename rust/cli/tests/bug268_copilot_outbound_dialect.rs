//! Feature: spec/features/copilot-outbound-dialect.feature

#[path = "../../agent-loop/tests/support/bug268_environment.rs"]
mod environment;
#[path = "support/prov161_runtime.rs"]
mod runtime;
#[path = "support/bug268.rs"]
mod support;
#[path = "support/bug268_wire.rs"]
mod wire;

use codelet_providers::LlmProvider;
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, CompletionRequestBuilder},
};
use runtime::TestResult;

#[tokio::test]
#[serial_test::serial]
async fn client_model_requests_honor_the_endpoint_table() -> TestResult {
    // @step Given synthetic Copilot credentials and local endpoints for the documented model table
    let _environment = environment::Environment::new()?;
    for (model, responses) in [
        ("gpt-5", true),
        ("gpt-6", true),
        ("gpt-5-mini", false),
        ("gpt-4o", false),
        ("claude-sonnet-4.5", false),
        ("unknown-local", false),
    ] {
        let server = wire::server(false, "").await;
        let provider = support::provider(&server, model)?;
        let selected = provider.client().completion_model(model);
        // @step When actual client completion models send requests
        selected
            .completion(CompletionRequestBuilder::new(selected.clone(), "hello").build())
            .await?;
        // @step Then each URI and request body match the selected dialect with Copilot headers
        support::assert_request(&server, model, responses, 1).await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn public_client_agents_execute_in_both_dialects() -> TestResult {
    use rig::completion::Prompt;
    // @step Given a public Copilot client agent and synthetic loopback endpoints
    let _environment = environment::Environment::new()?;
    for (model, responses) in [("gpt-5", true), ("gpt-5-mini", false)] {
        let server = wire::server(false, "").await;
        let provider = support::provider(&server, model)?;
        let agent = provider.client().agent(model).build();
        // @step When the client agent actually completes a prompt for each selected dialect
        let answer = agent.prompt("hello").await?;
        // @step Then captured URI body headers and ordinary answer match the chosen dialect
        assert_eq!(answer, "answer");
        support::assert_request(&server, model, responses, 1).await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn nested_responses_images_retain_real_middleware_classification() -> TestResult {
    // @step Given a Copilot Responses request carries an image within a user input message
    let _environment = environment::Environment::new()?;
    let server = wire::server(false, "").await;
    let provider = support::provider(&server, "gpt-5")?;
    let model = provider.client().completion_model("gpt-5");
    let message = rig::message::Message::User {
        content: rig::OneOrMany::many(vec![
            rig::message::UserContent::text("describe this synthetic image"),
            rig::message::UserContent::image_url("data:image/png;base64,aW1hZ2U=", None, None),
        ])?,
    };
    // @step When the real completion model sends that request through middleware
    model
        .completion(CompletionRequestBuilder::new(model.clone(), message).build())
        .await?;
    // @step Then the captured nested image is preserved and the vision request header is true
    let requests = server.received_requests().await.ok_or("missing capture")?;
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = requests[0].body_json()?;
    assert!(
        body["input"]
            .as_array()
            .is_some_and(|input| input.iter().any(|message| {
                message["content"]
                    .as_array()
                    .is_some_and(|content| content.iter().any(|item| item["type"] == "input_image"))
            })),
        "nested image missing from synthetic request: {body}"
    );
    assert_eq!(
        requests[0]
            .headers
            .get("copilot-vision-request")
            .ok_or("vision header absent")?,
        "true"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn nonstreaming_provider_requests_honor_both_dialects() -> TestResult {
    // @step Given synthetic Copilot credentials and local endpoints for Chat and Responses
    let _environment = environment::Environment::new()?;
    for (model, responses) in [("gpt-5", true), ("gpt-5-mini", false)] {
        let server = wire::server(false, "").await;
        let provider = support::provider(&server, model)?;
        let message = codelet_common::Message::user("hello");
        // @step When the provider completes a message through its public nonstreaming entry point
        provider.complete(&[message]).await?;
        // @step Then each URI and request body match the selected dialect with Copilot headers
        support::assert_request(&server, model, responses, 1).await?;
    }
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn native_agents_retain_tool_nudge_and_done_continuation() -> TestResult {
    // @step Given an armed native Copilot agent and a local tool stop and Done sequence in each dialect
    let _environment = environment::Environment::new()?;
    for (model, responses) in [("gpt-5", true), ("gpt-5-mini", false)] {
        // @step When the shared stream loop completes the task
        // @step Then tool results retain invocation identities and exactly one nudge precedes accepted Done
        // @step And physical usage is billed once per request without extra inference
        support::native_sequence(model, responses).await?;
    }
    Ok(())
}
