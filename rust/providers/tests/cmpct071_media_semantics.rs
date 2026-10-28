//! Feature: spec/features/selected-request-media-semantics.feature

use codelet_providers::codex::history::CodexHistory;
use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::SelectedRequestPreparer;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder, ToolDefinition};
use rig::message::{ImageMediaType, Message, UserContent};
use serde_json::{json, Value};

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn schema() -> ToolDefinition {
    ToolDefinition {
        name: "inspect_media_definition".into(),
        description: "Describes tool arguments, not submitted media".into(),
        parameters: json!({
            "type":"object",
            "properties": {
                "image":{"type":"string"},
                "image_url":{"type":"string"},
                "source":{"type":"string"},
                "audio":{"type":"string"},
                "input_audio":{"type":"string"},
                "video":{"type":"string"},
                "inlineData":{"type":"string"},
                "fileData":{"type":"string"},
                "file_id":{"type":"string"}
            }
        }),
    }
}

fn assert_preparation<M: CompletionModel + 'static>(model: M, media: bool) -> TestResult {
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .tools(vec![schema()])
        .build();
    let preparer = SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    );
    let content = if media {
        Message::User {
            content: rig::OneOrMany::one(UserContent::image_url(
                "https://example.test/image.png",
                Some(ImageMediaType::PNG),
                None,
            )),
        }
    } else {
        Message::user("committed DAG with a verified text suffix")
    };
    let result = preparer.prepare(&[content], 1);
    if media {
        assert!(
            matches!(result, Err(RequestAdmissionError::UnknownMediaOccupancy)),
            "actual image must not be certified: {result:?}"
        );
    } else {
        let prepared = result?;
        assert_eq!(
            prepared.input_tokens(),
            codelet_common::token_estimator::count_tokens(
                &serde_json::from_slice::<Value>(prepared.request_bytes())?.to_string()
            ) as u64
        );
        assert_eq!(prepared.output_reservation(), 4096);
        assert_eq!(prepared.generation(), 1);
        let body: Value = serde_json::from_slice(prepared.request_bytes())?;
        let serialized_tools = serde_json::to_string(&body["tools"])?;
        assert!(serialized_tools.contains("inspect_media_definition"));
        for property in schema().parameters["properties"]
            .as_object()
            .ok_or("schema properties")?
            .keys()
        {
            assert!(serialized_tools.contains(property), "missing {property}");
        }
    }
    Ok(())
}

async fn assert_native_adapters(media: bool) -> TestResult {
    let server = fixture::Server::start(vec![], false).await?;
    let history = CodexHistory::default();
    let before = history.snapshot()?;
    assert_preparation(server.models()?.0.with_history(history.clone()), media)?;
    assert_preparation(server.models()?.1, media)?;
    let openai = rig::providers::openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&server.url)
        .build()?
        .completion_model("test-model");
    assert_preparation(openai, media)?;
    let anthropic = rig::providers::anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&server.url)
        .build()?
        .completion_model("test-model");
    assert_preparation(anthropic, media)?;
    let gemini = rig::providers::gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&server.url)
        .build()?
        .completion_model("test-model");
    assert_preparation(gemini, media)?;
    assert_eq!(before, history.snapshot()?);
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    Ok(())
}

#[tokio::test]
async fn media_vocabulary_in_native_tool_schemas_does_not_prevent_text_admission() -> TestResult {
    // @step Given selected native adapters with complete tool schemas describing media fields
    // @step When their text requests are prepared for the next committed generation
    // @step Then every schema byte is retained and counted without transmitting a request
    assert_native_adapters(false).await
}

#[tokio::test]
async fn native_tool_schemas_cannot_hide_real_media_with_unknown_occupancy() -> TestResult {
    // @step Given selected native adapters with tool schemas and actual image content
    // @step When their complete requests are prepared for the next committed generation
    // @step Then typed unknown media rejection leaves checkpoints and physical transport unchanged
    assert_native_adapters(true).await
}
