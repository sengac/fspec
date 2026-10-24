use rig::client::CompletionClient;
use rig::completion::{CompletionModel, CompletionRequest};
use rig::message::{AssistantContent, Message};
use rig::providers::gemini;
use serde_json::{json, Value};

#[path = "cmpct071_builtin_serializers/support.rs"]
mod support;
use support::{assert_request_physical, request, server, TestResult};

fn history<M: CompletionModel>(model: M, aliases: bool) -> TestResult<CompletionRequest> {
    let mut request = request(model, json!({}))?;
    request.chat_history = rig::OneOrMany::many(vec![
        Message::user("unchanged validated DAG"),
        Message::assistant("retained assistant response"),
        Message::Assistant {
            id: None,
            content: rig::OneOrMany::many(vec![
                AssistantContent::tool_call_with_call_id(
                    "call_after_snapshot",
                    "native_lookup".into(),
                    "lookup",
                    json!({"path":"src/deep.rs","evidence":"ordered-call-marker"}),
                ),
                AssistantContent::tool_call_with_call_id(
                    "call_second",
                    "native_inspect".into(),
                    "inspect",
                    json!({"path":"src/other.rs"}),
                ),
            ])?,
        },
        Message::tool_result_with_call_id(
            if aliases {
                "native_inspect"
            } else {
                "call_second"
            },
            Some("native_inspect".into()),
            "complete second result",
        ),
        Message::tool_result_with_call_id(
            if aliases {
                "result_lookup"
            } else {
                "call_after_snapshot"
            },
            Some("native_lookup".into()),
            "ordered-result-marker: complete first result with retained tail",
        ),
        Message::user("verified durable suffix"),
    ])?;
    Ok(request)
}

fn assert_pairing(body: &Value) {
    let contents = &body["contents"];
    assert_eq!(contents[3]["parts"][0]["functionCall"]["name"], "lookup");
    assert_eq!(
        contents[3]["parts"][0]["functionCall"]["args"],
        json!({"path":"src/deep.rs","evidence":"ordered-call-marker"})
    );
    assert_eq!(contents[3]["parts"][1]["functionCall"]["name"], "inspect");
    assert_eq!(
        contents[4]["parts"][0]["functionResponse"]["name"],
        "inspect"
    );
    assert_eq!(
        contents[5]["parts"][0]["functionResponse"]["name"],
        "lookup"
    );
    assert_eq!(
        contents[4]["parts"][0]["functionResponse"]["response"]["result"],
        "complete second result"
    );
    assert_eq!(
        contents[5]["parts"][0]["functionResponse"]["response"]["result"],
        "ordered-result-marker: complete first result with retained tail"
    );
}

/// Feature: spec/features/builtin-provider-gemini-tool-pairing.feature
#[tokio::test]
async fn streaming_gemini_pairs_tool_results_by_original_function() -> TestResult {
    for aliases in [false, true] {
        // @step Given a selected Gemini model with distinct call identifiers and two named functions
        let server = server().await;
        let model = gemini::Client::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("test-model");
        let request = history(model.clone(), aliases)?;
        // @step When the history is prepared for admission and physically streamed
        let body = assert_request_physical(
            model,
            &server,
            request,
            "/v1beta/models/test-model:streamGenerateContent",
        )
        .await?;
        // @step Then every native response names its original function with complete ordered evidence
        // @step And prepared bytes match physical bytes without mutating source identities
        assert_pairing(&body);
    }
    Ok(())
}

/// Feature: spec/features/builtin-provider-gemini-tool-pairing.feature
#[tokio::test]
async fn direct_gemini_pairs_tool_results_by_original_function() -> TestResult {
    for aliases in [false, true] {
        // @step Given a selected Gemini model with distinct call identifiers and two named functions
        let server = server().await;
        let model = gemini::Client::<reqwest::Client>::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()?
            .completion_model("test-model");
        let request = history(model.clone(), aliases)?;
        let before = format!("{request:?}");
        let prepared = model.prepare_request(request.clone())?;
        assert_eq!(before, format!("{request:?}"));
        assert!(server
            .received_requests()
            .await
            .ok_or("missing capture")?
            .is_empty());
        // @step When the history completes through the physical HTTP adapter
        let _response = model.completion(request).await;
        let requests = server.received_requests().await.ok_or("missing capture")?;
        // @step Then every native response names its original function with complete ordered evidence
        // @step And prepared bytes match physical bytes without mutating source identities
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url.path(),
            "/v1beta/models/test-model:generateContent"
        );
        assert_eq!(requests[0].body, prepared);
        assert_pairing(&serde_json::from_slice(&prepared)?);
    }
    Ok(())
}
