use super::{environment::TestResult, fixture};
use codelet_providers::codex::{history::CodexHistory, CodexResponsesModel, NativeCapabilities};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, CompletionRequestBuilder, GetTokenUsage},
    streaming::StreamedAssistantContent,
};
use serde_json::{json, Value};

pub const OPAQUE: &str = "synthetic-owned+/=秘密";
pub const UNSAFE_KINDS: [&str; 6] = [
    "refusal",
    "content_filter",
    "max_output_tokens",
    "failed",
    "interrupted_refusal",
    "incomplete_tool",
];

pub fn output() -> Vec<Value> {
    vec![
        json!({"type":"reasoning","id":"reason","summary":[{"type":"summary_text","text":"checking"}],"encrypted_content":OPAQUE}),
        json!({"type":"web_search_call","id":"hosted-search","status":"completed","action":{"type":"search","query":"synthetic"}}),
        message("partial", "commentary"),
    ]
}

pub fn known_output(incomplete: bool, completion: bool) -> Vec<Value> {
    let mut owned = output();
    owned[1]["action"] = match (incomplete, completion) {
        (false, false) => json!({"type":"search","query":"synthetic"}),
        (false, true) => {
            json!({"type":"search","queries":["synthetic"],"sources":[{"type":"url","url":"https://example.test","title":"Synthetic"}]})
        }
        (true, false) => json!({"type":"open_page","url":"https://example.test"}),
        (true, true) => json!({"type":"find","url":"https://example.test","pattern":"synthetic"}),
    };
    owned[2]["content"][0]["annotations"] = json!([{"type":"url_citation","url":"https://example.test","title":"Synthetic","start_index":0,"end_index":7}]);
    owned[2]["content"][0]["logprobs"] = json!([{"token":"partial","logprob":-0.5,"bytes":[112],"top_logprobs":[{"token":"partial","logprob":-0.5,"bytes":null}]}]);
    owned
}

pub fn unsafe_terminal(kind: &str) -> String {
    let mut owned = output();
    if kind.contains("refusal") {
        owned[2]["content"] = json!([{"type":"refusal","refusal":"Cannot comply"}]);
    }
    if kind == "incomplete_tool" {
        owned.push(json!({"type":"function_call","id":"native-item","call_id":"native-call","name":"Read","arguments":"{}"}));
    }
    let status = if kind == "refusal" {
        "completed"
    } else if kind == "failed" {
        "failed"
    } else {
        "incomplete"
    };
    let reason = if status == "incomplete" {
        Some(
            if kind.starts_with("interrupted") || kind == "incomplete_tool" {
                "interrupted"
            } else {
                kind
            },
        )
    } else {
        None
    };
    terminal(owned, status, false, reason)
}

pub fn change_account() -> TestResult {
    let path = std::path::PathBuf::from(std::env::var("CODEX_HOME")?).join("auth.json");
    let mut value: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    value["tokens"]["account_id"] = json!("different-synthetic-account");
    std::fs::write(path, serde_json::to_vec(&value)?)?;
    Ok(())
}

pub async fn assert_authority_errors(
    stream: &mut rig::streaming::StreamingCompletionResponse<fixture::Final>,
    account: bool,
) -> TestResult {
    let mut errors = Vec::new();
    while let Some(event) = stream.next().await {
        match event {
            Err(error) => errors.push(error.to_string()),
            Ok(StreamedAssistantContent::Final(_) | StreamedAssistantContent::ToolCall(_)) => {
                return Err("stale authority settled successfully".into())
            }
            _ => {}
        }
    }
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains(if account {
            "authentication"
        } else {
            "generation"
        }),
        "{errors:?}"
    );
    Ok(())
}

pub fn message(text: &str, phase: &str) -> Value {
    json!({"type":"message","id":format!("message-{phase}"),"role":"assistant","phase":phase,"status":"completed",
        "content":[{"type":"output_text","text":text,"annotations":[]}]})
}

pub fn malformed_output() -> Vec<Vec<Value>> {
    let mut cases = vec![
        vec![],
        vec![json!({"type":"unknown_native","secret":OPAQUE})],
        vec![json!({"type":"reasoning","summary":"malformed","encrypted_content":OPAQUE})],
        vec![
            json!({"type":"web_search_call","id":"search","status":"in_progress","action":{"type":"search","query":"q"}}),
        ],
        vec![
            json!({"type":"function_call_output","call_id":"unowned","output":"not a client result"}),
        ],
    ];
    for (pointer, value) in [
        ("/content/0/annotations", json!({"unexpected":OPAQUE})),
        ("/content/0/logprobs", json!("invalid")),
        ("/content/0/text", json!("")),
        ("/id", json!("")),
        ("/phase", json!(null)),
        ("/status", json!("unknown")),
        (
            "/content/0/annotations",
            json!([{"type":"unknown","payload":OPAQUE}]),
        ),
        (
            "/content/0/annotations",
            json!([{"type":"url_citation","url":"https://example.test","title":"Synthetic","start_index":4,"end_index":1}]),
        ),
        (
            "/content/0/logprobs",
            json!([{"token":"word","logprob":-0.1,"bytes":[256]}]),
        ),
    ] {
        let mut item = message("partial", "commentary");
        if pointer == "/content/0/logprobs" {
            item["content"][0]["logprobs"] = value;
        } else if let Some(field) = item.pointer_mut(pointer) {
            *field = value;
        }
        cases.push(vec![item]);
    }
    for item in [
        json!({"type":"reasoning","id":"reason","summary":[{"type":"summary_text","text":""}]}),
        json!({"type":"reasoning","id":"reason","summary":[{"type":"summary_text","text":"ok"}],"encrypted_content":null}),
        json!({"type":"reasoning","id":"reason","summary":[],"encrypted_content":OPAQUE,"unowned":true}),
        json!({"type":"web_search_call","id":"search","status":"completed","action":{"type":"find","url":"https://example.test","pattern":[]}}),
        json!({"role":"assistant","content":[{"type":"output_text","text":"untyped"}]}),
    ] {
        cases.push(vec![item]);
    }
    cases
}

pub fn terminal(output: Vec<Value>, status: &str, end_turn: bool, reason: Option<&str>) -> String {
    let mut response = json!({"id":"synthetic-response","object":"response","created_at":1,"model":"shared-model",
        "status":status,"end_turn":end_turn,"output":output,"usage":{"input_tokens":12,"output_tokens":8,"total_tokens":20}});
    if let Some(reason) = reason {
        response["incomplete_details"] = json!({"reason":reason});
    }
    if status == "failed" {
        response["error"] = json!({"code":"synthetic_failure","message":"synthetic failure"});
    }
    fixture::event(json!({"type":format!("response.{status}"),"response":response}))
}

pub fn model(server: &fixture::Server, history: CodexHistory) -> TestResult<CodexResponsesModel> {
    Ok(server
        .models()?
        .0
        .with_history(history)
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        }))
}

pub fn request(model: &CodexResponsesModel) -> rig::completion::CompletionRequest {
    CompletionRequestBuilder::new(model.clone(), "Hello")
        .preamble("Test instructions".into())
        .build()
}

#[derive(Default, Debug)]
pub struct Observed {
    pub text: String,
    pub usage: Vec<u64>,
    pub finals: Vec<u64>,
    pub records: Vec<(String, u64)>,
    pub errors: Vec<String>,
    pub tools: usize,
    pub aborted: usize,
}

pub async fn collect(model: &CodexResponsesModel) -> TestResult<Observed> {
    let mut stream = model.stream(request(model)).await?;
    let mut observed = Observed::default();
    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamedAssistantContent::Text(text)) => observed.text.push_str(&text.text),
            Ok(StreamedAssistantContent::Usage(usage)) => observed.usage.push(usage.total_tokens),
            Ok(StreamedAssistantContent::Final(response)) => {
                observed.finals.push(response.usage.total_tokens);
                observed.records.extend(
                    response
                        .request_usage_records()
                        .into_iter()
                        .map(|(id, usage)| (id, usage.total_tokens)),
                );
            }
            Ok(StreamedAssistantContent::ToolCall(_)) => observed.tools += 1,
            Ok(StreamedAssistantContent::ResponseAttemptAborted) => {
                observed.aborted += 1;
                observed.text.clear();
            }
            Err(error) => observed.errors.push(error.to_string()),
            _ => {}
        }
    }
    Ok(observed)
}

pub fn requests(server: &fixture::Server) -> TestResult<Vec<Value>> {
    Ok(server
        .requests
        .lock()
        .map_err(|_| "capture poisoned")?
        .clone())
}

pub fn assert_failure(observed: &Observed, server: &fixture::Server, usage: usize) -> TestResult {
    assert_eq!(observed.errors.len(), 1, "{observed:?}");
    assert!(observed.finals.is_empty(), "{observed:?}");
    assert_eq!(observed.tools, 0);
    assert_eq!(observed.usage, vec![20; usage]);
    assert_eq!(requests(server)?.len(), usage);
    assert!(!observed.errors[0].contains(OPAQUE));
    Ok(())
}

pub async fn stop_at_usage(
    model: &CodexResponsesModel,
) -> TestResult<rig::streaming::StreamingCompletionResponse<fixture::Final>> {
    let mut stream = model.stream(request(model)).await?;
    while let Some(event) = stream.next().await {
        if matches!(event?, StreamedAssistantContent::Usage(_)) {
            return Ok(stream);
        }
    }
    Err("no physical usage observed".into())
}

pub fn authenticated_model(
    server: &fixture::Server,
    history: CodexHistory,
) -> TestResult<CodexResponsesModel> {
    use codelet_providers::codex::refreshing_client::RefreshingCodexClient;
    let home = std::path::PathBuf::from(std::env::var("CODEX_HOME")?);
    std::fs::create_dir_all(&home)?;
    let auth = json!({"tokens":{"id_token":"synthetic-id","access_token":"synthetic-access","refresh_token":"synthetic-refresh","account_id":"synthetic-account"}});
    std::fs::write(home.join("auth.json"), serde_json::to_vec(&auth)?)?;
    let http = RefreshingCodexClient::new_oauth(
        "synthetic-access".into(),
        "synthetic-refresh".into(),
        "synthetic-account".into(),
        Some(3600),
        "http://127.0.0.1:9".into(),
    )
    .with_test_endpoint(&format!("{}/responses", server.url))?;
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("synthetic-key")
        .base_url(&server.url)
        .http_client(http)
        .build()?;
    Ok(CodexResponsesModel::new(client, "shared-model")
        .with_history(history)
        .with_native_capabilities(NativeCapabilities {
            hosted_search: true,
        })
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(100000),
            output_reservation: Some(1000),
        }))
}
