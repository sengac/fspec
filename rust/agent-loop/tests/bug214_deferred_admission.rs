//! Feature: spec/features/deferred-compaction-admission-cancellation.feature

#[path = "../../cli/tests/bug214/model.rs"]
mod model;
#[path = "cmpct056/support.rs"]
pub mod support;

use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_core::persistence::load_session;
use codelet_providers::{
    request_admission::AdmissionLimits, selected_request::SelectedRequestPreparer,
};
use codelet_tools::inject_summary::execute_inject_summary;
use rig::{client::CompletionClient, completion::CompletionRequestBuilder};

#[tokio::test]
#[serial_test::serial]
async fn scenario_deferred_application_vetoes_cancellation_during_replacement_admission(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a valid summary is queued for its captured session
    let fixture = support::Fixture::new("http://127.0.0.1:1").await;
    let original = serde_json::to_value(load_session(fixture.target.id)?)?;
    let mut inner = fixture.target.inner.lock().await;
    let queue = register_generate_compaction_handler(
        fixture.target.id,
        &inner,
        fixture.directory.path().into(),
        fixture.target.clone(),
        Some(fixture.manager.clone()),
    );
    execute_inject_summary(fixture.target.id, support::DAG.into())?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .build()?;
    let model = model::InterruptingModel {
        inner: client.completion_model("o3"),
        interrupted: fixture.target.is_interrupted.clone(),
        during_admission: true,
    };
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    inner.install_selected_preparer(SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(100_000),
            output_reservation: Some(4096),
        },
    ));
    let mut chunks = fixture.target.subscribe_to_stream();
    // @step When the target is interrupted while the replacement request is serialized
    let result = queue.apply(&mut inner, &fixture.target);
    // @step Then application returns cancellation without changing durable context
    assert!(result.is_err(), "{result:?}");
    assert!(result.err().ok_or("cancelled result")?.contains("cancel"));
    assert_eq!(
        serde_json::to_value(load_session(fixture.target.id)?)?,
        original
    );
    // @step And no completion is emitted and inference remains blocked
    assert!(inner.ensure_compaction_inference_ready().is_err());
    support::assert_no_completion(&mut chunks);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_a_cross_target_caller_can_cancel_replacement_admission(
) -> Result<(), Box<dyn std::error::Error>> {
    use rig::tool::Tool;
    use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};
    // @step Given a caller generates a valid summary for another live session
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"id":"dag", "object":"chat.completion", "created":1,
            "model":"o3", "usage":{"prompt_tokens":500,"completion_tokens":50,"total_tokens":550},
            "choices":[{"index":0,"message":{"role":"assistant","content":support::DAG},
            "finish_reason":"stop"}]}),
        ))
        .mount(&server)
        .await;
    let fixture = support::Fixture::new(&server.uri()).await;
    let original = serde_json::to_value(load_session(fixture.target.id)?)?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .build()?;
    let model = model::InterruptingModel {
        inner: client.completion_model("o3"),
        interrupted: fixture.caller.is_interrupted.clone(),
        during_admission: true,
    };
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    fixture
        .target
        .inner
        .lock()
        .await
        .install_selected_preparer(SelectedRequestPreparer::new(
            model,
            template,
            AdmissionLimits {
                context_capacity: Some(100_000),
                output_reservation: Some(4096),
            },
        ));
    let mut chunks = fixture.target.subscribe_to_stream();
    // @step When the caller is interrupted while the target replacement request is serialized
    let result = fixture
        .tool(&fixture.caller)
        .call(codelet_tools::GenerateCompactionArgs {
            session_id: Some(fixture.target.id.to_string()),
        })
        .await;
    // @step Then generation returns cancellation without changing the target durable context
    assert!(result.is_err(), "{result:?}");
    assert_eq!(
        serde_json::to_value(load_session(fixture.target.id)?)?,
        original
    );
    // @step And the target emits no compaction completion
    support::assert_no_completion(&mut chunks);
    Ok(())
}
