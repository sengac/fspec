use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::SelectedRequestPreparer;
use rig::client::CompletionClient;
use rig::completion::{CompletionError, CompletionModel, CompletionRequest};
use rig::message::ToolChoice;
use rig::providers::{anthropic, openai};
use serde_json::{json, Value};

#[path = "cmpct071_builtin_serializers/support.rs"]
mod support;
use support::{assert_request_physical, request, server, TestResult};

async fn assert_rejected<M: CompletionModel + 'static>(
    model: M,
    server: &wiremock::MockServer,
    request: CompletionRequest,
    json_error: bool,
) -> TestResult {
    let original = format!("{request:?}");
    let Err(error) = model.prepare_request(request.clone()) else {
        return Err("invalid selected request must not be certified".into());
    };
    if json_error {
        assert!(matches!(error, CompletionError::JsonError(_)));
    }
    assert!(model
        .prepare_request_for_generation(request.clone(), 4)
        .is_err());
    let preparer = SelectedRequestPreparer::new(
        model.clone(),
        request.clone(),
        AdmissionLimits {
            context_capacity: Some(1_000_000),
            output_reservation: Some(4096),
        },
    );
    assert!(matches!(
        preparer.prepare(
            &request.chat_history.clone().into_iter().collect::<Vec<_>>(),
            4
        ),
        Err(RequestAdmissionError::Preparation(_))
    ));
    assert!(model.stream(request.clone()).await.is_err());
    assert_eq!(format!("{request:?}"), original);
    assert!(server
        .received_requests()
        .await
        .ok_or("missing request capture")?
        .is_empty());
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity-options.feature
#[tokio::test]
async fn anthropic_preserves_explicit_api_key_tool_policies() -> TestResult {
    // @step Given a selected Anthropic API key model with an explicit tool policy
    for (policy, expected) in [
        (ToolChoice::Auto, json!({"type":"auto"})),
        (ToolChoice::None, json!({"type":"none"})),
        (ToolChoice::Required, json!({"type":"any"})),
        (
            ToolChoice::Specific {
                function_names: vec!["selected_tool".into()],
            },
            json!({"type":"tool","name":"selected_tool"}),
        ),
    ] {
        let server = server().await;
        let model = anthropic::Client::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("test-model");
        let mut request = request(model.clone(), json!({}))?;
        request.tool_choice = Some(policy);
        // @step When each supported tool policy is prepared then physically sent
        let body = assert_request_physical(model, &server, request, "/v1/messages").await?;
        // @step Then prepared and physical bytes retain the selected tool policy
        // @step And preparation sends nothing and leaves the input request unchanged
        assert_eq!(body["tool_choice"], expected);
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity-options.feature
#[tokio::test]
async fn anthropic_rejects_unrepresentable_api_key_tool_policies() -> TestResult {
    // @step Given a selected Anthropic API key model requiring multiple specific tools
    let server = server().await;
    let model = anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    for names in [vec!["first".into(), "second".into()], vec![]] {
        let mut request = request(model.clone(), json!({}))?;
        request.tool_choice = Some(ToolChoice::Specific {
            function_names: names,
        });
        // @step When admission preparation and physical submission are attempted
        // @step Then both return errors without sending or changing the source request
        assert_rejected(model.clone(), &server, request, false).await?;
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity-options.feature
#[tokio::test]
async fn anthropic_retains_mandatory_textual_documents() -> TestResult {
    // @step Given selected Anthropic API key and OAuth models with mandatory textual documents
    for api_key in ["test-key", "sk-ant-oat-test"] {
        let server = server().await;
        let model = anthropic::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .base_url(server.uri())
            .build()?
            .completion_model("test-model");
        let mut request = request(model.clone(), json!({}))?;
        let document = rig::completion::Document {
            id: "mandatory-file-id".into(),
            text: "authoritative-file-evidence".into(),
            additional_props: [
                ("source".into(), "retained-file-source".into()),
                ("revision".into(), "retained-file-revision".into()),
            ]
            .into(),
        };
        let expected = document.to_string();
        request.documents = vec![document];
        // @step When each complete request is prepared then physically sent
        let body = assert_request_physical(model, &server, request, "/v1/messages").await?;
        // @step Then prepared and physical bytes retain the complete document text and metadata
        // @step And preparation sends nothing and leaves the input request unchanged
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(body["messages"][0]["content"][0]["text"], expected);
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-physical-serializer-parity-options.feature
#[tokio::test]
async fn openai_responses_rejects_malformed_optional_parameters_without_panicking() -> TestResult {
    // @step Given a selected OpenAI Responses model with malformed typed optional parameters
    let server = server().await;
    let model = openai::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()?
        .completion_model("test-model");
    for params in [
        json!({"store":"not-a-boolean"}),
        json!({"reasoning":true}),
        json!({"metadata":[]}),
        Value::Bool(true),
    ] {
        let request = request(model.clone(), params)?;
        // @step When admission preparation and physical submission are attempted
        // @step Then both return errors without sending or changing the source request
        assert_rejected(model.clone(), &server, request, true).await?;
    }
    Ok(())
}
