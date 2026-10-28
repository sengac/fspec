use codelet_core::RigAgent;
use codelet_providers::codex::history::CodexHistory;
use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::{media_occupancy, SelectedRequestPreparer};
use futures::StreamExt;
use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequestBuilder, ToolDefinition};
use rig::message::Message;
use serde_json::json;

#[path = "fixtures/prov148.rs"]
pub mod fixture;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone)]
struct UnsupportedSerializer;

impl CompletionModel for UnsupportedSerializer {
    type Response = serde_json::Value;
    type StreamingResponse = fixture::Final;
    type Client = ();

    fn make(_: &(), _: impl Into<String>) -> Self {
        Self
    }

    async fn completion(
        &self,
        _: rig::completion::CompletionRequest,
    ) -> Result<rig::completion::CompletionResponse<Self::Response>, rig::completion::CompletionError>
    {
        Err(rig::completion::CompletionError::RequestError(
            "not called".into(),
        ))
    }

    async fn stream(
        &self,
        _: rig::completion::CompletionRequest,
    ) -> Result<
        rig::streaming::StreamingCompletionResponse<Self::StreamingResponse>,
        rig::completion::CompletionError,
    > {
        Err(rig::completion::CompletionError::RequestError(
            "not called".into(),
        ))
    }
}

fn limits() -> AdmissionLimits {
    AdmissionLimits {
        context_capacity: Some(200_000),
        output_reservation: Some(4_096),
    }
}

async fn assert_physical<M: CompletionModel + 'static>(
    model: M,
    server: &fixture::Server,
) -> TestResult {
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .preamble("mandatory selected instructions".into())
        .max_tokens(4_096)
        .tools(vec![ToolDefinition {
            name: "selected_tool".into(),
            description: "mandatory schema".into(),
            parameters: json!({"type":"object","properties":{"evidence":{"type":"string"}}}),
        }])
        .build();
    let messages = vec![
        Message::user("unchanged validated DAG"),
        Message::user("verified durable suffix"),
    ];
    let prepared = SelectedRequestPreparer::new(model.clone(), template.clone(), limits())
        .prepare(&messages, 1)?;
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    let mut request = template;
    request.chat_history = rig::OneOrMany::many(messages)?;
    if let Ok(mut stream) = model.stream(request).await {
        while stream.next().await.is_some() {}
    }
    {
        let actual = server.requests.lock().map_err(|_| "request lock")?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(prepared.request_bytes())?,
            actual[0]
        );
    }
    let agent = RigAgent::with_default_depth(
        rig::agent::AgentBuilder::new(model)
            .preamble("actual agent preamble")
            .max_tokens(4_096)
            .build(),
    );
    let captured = agent
        .selected_request_preparer("current prompt", limits())
        .await?
        .prepare(&[Message::user("candidate with suffix")], 1)?;
    assert!(String::from_utf8(captured.request_bytes().to_vec())?.contains("actual agent preamble"));
    Ok(())
}

#[tokio::test]
async fn selected_preparation_uses_the_physical_provider_request_without_changing_history(
) -> TestResult {
    // @step Given an actual selected provider with instructions schemas and a verified suffix
    let response = fixture::event(
        json!({"type":"response.completed", "response":fixture::baseline::response()}),
    );
    let server = fixture::Server::start(vec![response], false).await?;
    let history = CodexHistory::default();
    let model = server
        .models()?
        .0
        .with_history(history.clone())
        .with_request_admission(limits());
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .preamble("mandatory selected instructions".into())
        .tools(vec![ToolDefinition {
            name: "selected_tool".into(),
            description: "mandatory schema".into(),
            parameters: json!({"type":"object","properties":{"evidence":{"type":"string"}}}),
        }])
        .build();
    let preparer = SelectedRequestPreparer::new(model.clone(), template.clone(), limits());
    let messages = vec![
        Message::user("unchanged validated DAG"),
        Message::user("verified durable suffix"),
    ];
    let before = serde_json::to_vec(&history.snapshot()?)?;
    // @step When its next generation request is prepared and admitted before promotion
    let prepared = preparer.prepare(&messages, 1)?;
    // @step Then the certificate retains the physical request bytes and the unchanged source checkpoint
    assert_eq!(prepared.generation(), 1);
    assert_eq!(
        prepared.input_tokens(),
        codelet_common::token_estimator::count_tokens(
            &serde_json::from_slice::<serde_json::Value>(prepared.request_bytes())?.to_string()
        ) as u64
    );
    assert_eq!(prepared.output_reservation(), 4_096);
    assert_eq!(serde_json::to_vec(&history.snapshot()?)?, before);
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    history.transition_generation(1)?;
    let mut request = template;
    request.chat_history = rig::OneOrMany::many(messages)?;
    let mut stream = model.stream(request).await?;
    while let Some(event) = stream.next().await {
        event?;
    }
    {
        let actual = server.requests.lock().map_err(|_| "request lock")?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(prepared.request_bytes())?,
            actual[0]
        );
    }
    let body = String::from_utf8(prepared.request_bytes().to_vec())?;
    for mandatory in [
        "mandatory selected instructions",
        "selected_tool",
        "unchanged validated DAG",
        "verified durable suffix",
    ] {
        assert!(body.contains(mandatory));
    }
    let anthropic_server = fixture::Server::start(vec![String::new()], false).await?;
    let anthropic = rig::providers::anthropic::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&anthropic_server.url)
        .build()?
        .completion_model("test-model");
    assert_physical(anthropic, &anthropic_server).await?;
    let openai_server = fixture::Server::start(vec![String::new()], false).await?;
    let openai = rig::providers::openai::CompletionsClient::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&openai_server.url)
        .build()?
        .completion_model("test-model");
    assert_physical(openai, &openai_server).await?;
    let responses_server = fixture::Server::start(vec![String::new()], false).await?;
    assert_physical(responses_server.models()?.1, &responses_server).await?;
    let gemini_server = fixture::Server::start(vec![String::new()], false).await?;
    let gemini = rig::providers::gemini::Client::<reqwest::Client>::builder()
        .api_key("test-key")
        .base_url(&gemini_server.url)
        .build()?
        .completion_model("test-model");
    assert_physical(gemini, &gemini_server).await?;
    Ok(())
}

#[tokio::test]
async fn selected_preparation_cannot_certify_unknown_media_or_unavailable_serialization(
) -> TestResult {
    // @step Given a selected provider request with media or no supported physical serializer
    let server = fixture::Server::start(vec![], false).await?;
    let model = server.models()?.0;
    let template = CompletionRequestBuilder::new(model.clone(), "unused").build();
    let preparer = SelectedRequestPreparer::new(model, template, limits());
    let messages = vec![Message::user("text")];
    // @step When preparation attempts to certify the complete request
    // @step Then preparation returns a typed failure without sending or changing history
    assert!(matches!(
        preparer.prepare(&[], 1),
        Err(RequestAdmissionError::Preparation(_))
    ));
    assert!(matches!(
        preparer.prepare(&messages, 0),
        Err(RequestAdmissionError::Preparation(_))
    ));
    assert_eq!(
        media_occupancy(
            &json!({"input":[{"type":"input_image","image_url":"https://example.test/image"}]})
        ),
        None
    );
    assert_eq!(
        media_occupancy(
            &json!({"contents":[{"parts":[{"inlineData":{"mimeType":"image/png","data":"AA=="}}]}]})
        ),
        None
    );
    assert_eq!(
        media_occupancy(&json!({"input":[{"type":"input_text","text":"ordinary text"}]})),
        Some(0)
    );
    let template = CompletionRequestBuilder::new(UnsupportedSerializer, "unused").build();
    assert!(matches!(
        SelectedRequestPreparer::new(UnsupportedSerializer, template, limits())
            .prepare(&messages, 1),
        Err(RequestAdmissionError::Preparation(_))
    ));
    let image = Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::image_url(
            "https://example.test/image",
            None,
            None,
        )),
    };
    assert!(matches!(
        preparer.prepare(&[image], 1),
        Err(RequestAdmissionError::UnknownMediaOccupancy)
    ));
    assert!(server
        .requests
        .lock()
        .map_err(|_| "request lock")?
        .is_empty());
    Ok(())
}
