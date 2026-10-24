use codelet_providers::request_admission::{AdmissionLimits, RequestAdmissionError};
use codelet_providers::selected_request::SelectedRequestPreparer;
use futures::StreamExt;
use rig::completion::{
    CompletionModel, CompletionRequest, CompletionRequestBuilder, ToolDefinition,
};
use rig::message::{AssistantContent, Message};
use serde_json::{json, Value};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(""),
        )
        .mount(&server)
        .await;
    server
}

pub fn request<M: CompletionModel>(model: M, params: Value) -> TestResult<CompletionRequest> {
    let mut request = CompletionRequestBuilder::new(model, "verified durable suffix")
        .preamble("mandatory selected instructions and file reminders".into())
        .documents(vec![rig::completion::Document {
            id: "required-file".into(),
            text: "authoritative-file-evidence".into(),
            additional_props: Default::default(),
        }])
        .max_tokens(4096)
        .temperature(0.4)
        .tools(vec![ToolDefinition {
            name: "selected_tool".into(),
            description: "mandatory evidence schema".into(),
            parameters: json!({"type":"object","properties":{"evidence":{"type":"string"}},"required":["evidence"]}),
        }])
        .additional_params(params)
        .build();
    request.chat_history = rig::OneOrMany::many(vec![
        Message::user("unchanged validated DAG"),
        Message::assistant("retained assistant response"),
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::one(AssistantContent::tool_call_with_call_id(
                "selected_tool",
                "selected_tool".into(),
                "selected_tool",
                json!({"evidence":"ordered-call-marker"}),
            )),
        },
        Message::tool_result_with_call_id(
            "selected_tool",
            Some("selected_tool".into()),
            "ordered-result-marker",
        ),
        Message::user("verified durable suffix"),
    ])?;
    Ok(request)
}

pub async fn assert_request_physical<M: CompletionModel + 'static>(
    model: M,
    server: &MockServer,
    request: CompletionRequest,
    expected_path: &str,
) -> TestResult<Value> {
    let before = format!("{request:?}");
    let has_documents = !request.documents.is_empty();
    let prepared = model.prepare_request(request.clone())?;
    let reservation = request.max_tokens.unwrap_or(4096);
    let preparer = SelectedRequestPreparer::new(
        model.clone(),
        request.clone(),
        AdmissionLimits {
            context_capacity: Some(u64::MAX),
            output_reservation: Some(reservation),
        },
    );
    let admission = preparer.prepare(
        &request.chat_history.clone().into_iter().collect::<Vec<_>>(),
        4,
    );
    if reservation == u64::MAX {
        assert!(matches!(
            admission,
            Err(RequestAdmissionError::InvalidLimits { capacity, reservation })
                if capacity == u64::MAX && reservation == u64::MAX
        ));
    } else {
        let admitted = admission?;
        assert_eq!(prepared, admitted.request_bytes());
        assert_eq!(admitted.generation(), 4);
        assert_eq!(
            admitted.input_tokens(),
            codelet_common::token_estimator::count_tokens(
                &serde_json::from_slice::<Value>(&prepared)?.to_string()
            ) as u64
        );
        assert_eq!(admitted.output_reservation(), reservation);
    }
    assert_eq!(
        prepared,
        model
            .clone()
            .prepare_request_for_generation(request.clone(), 4)?
    );
    assert_eq!(format!("{request:?}"), before);
    assert!(server
        .received_requests()
        .await
        .ok_or("missing request capture")?
        .is_empty());
    if let Ok(mut response) = model.stream(request).await {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while response.next().await.is_some() {}
        })
        .await?;
    }
    let requests = server
        .received_requests()
        .await
        .ok_or("missing request capture")?;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), expected_path);
    assert_eq!(
        prepared, requests[0].body,
        "admission must retain exact physical bytes"
    );
    let text = String::from_utf8(prepared.clone())?;
    for mandatory in [
        "mandatory selected instructions",
        "file reminders",
        "selected_tool",
        "unchanged validated DAG",
        "retained assistant response",
        "verified durable suffix",
    ] {
        assert!(text.contains(mandatory), "missing {mandatory}");
    }
    if has_documents {
        assert!(text.contains("authoritative-file-evidence"));
    }
    let call = text
        .find("ordered-call-marker")
        .ok_or("missing tool call")?;
    let result = text
        .find("ordered-result-marker")
        .ok_or("missing tool result")?;
    let suffix = text
        .find("verified durable suffix")
        .ok_or("missing suffix")?;
    assert!(call < result && result < suffix);
    Ok(serde_json::from_slice(&prepared)?)
}
