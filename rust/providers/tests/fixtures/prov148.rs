use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use codelet_providers::request_admission::AdmissionLimits;
use futures::StreamExt;
use rig::completion::{CompletionModel, CompletionRequestBuilder};
use rig::providers::openai::{self, responses_api::ResponsesCompletionModel};
use rig::streaming::StreamedAssistantContent;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "prov147.rs"]
pub mod response_fixture;
pub use response_fixture as baseline;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub type Final = rig::providers::openai::responses_api::streaming::StreamingCompletionResponse;

pub struct Server {
    pub url: String,
    pub requests: Arc<Mutex<Vec<Value>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    pub async fn start(bodies: Vec<String>, hold: bool) -> Result<Self, std::io::Error> {
        Self::start_with_disconnects(bodies.into_iter().map(Some).collect(), hold).await
    }

    pub async fn start_with_disconnects(
        bodies: Vec<Option<String>>,
        hold: bool,
    ) -> Result<Self, std::io::Error> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let url = format!("http://{}", listener.local_addr()?);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            for body in bodies {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let mut data = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let Ok(count) = socket.read(&mut buffer).await else {
                        return;
                    };
                    if count == 0 {
                        return;
                    }
                    data.extend_from_slice(&buffer[..count]);
                    if let Some(end) = data.windows(4).position(|part| part == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&data[..end]);
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        if data.len() >= end + 4 + length {
                            if let Ok(request) =
                                serde_json::from_slice(&data[end + 4..end + 4 + length])
                            {
                                if let Ok(mut requests) = recorded.lock() {
                                    requests.push(request);
                                }
                            }
                            break;
                        }
                    }
                }
                let Some(body) = body else {
                    continue;
                };
                let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                if socket
                    .write_all(format!("{header}{body}").as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
                if hold {
                    let _ = socket.read(&mut buffer).await;
                }
            }
        });
        Ok(Self {
            url,
            requests,
            task,
        })
    }

    pub fn models(
        &self,
    ) -> Result<
        (
            CodexResponsesModel,
            ResponsesCompletionModel<RefreshingCodexClient>,
        ),
        Box<dyn std::error::Error>,
    > {
        let client = openai::Client::<RefreshingCodexClient>::builder()
            .api_key("test-key")
            .base_url(&self.url)
            .http_client(RefreshingCodexClient::new_api_key())
            .build()?;
        Ok((
            CodexResponsesModel::new(client.clone(), "shared-model").with_request_admission(
                AdmissionLimits {
                    context_capacity: Some(100_000),
                    output_reservation: Some(1000),
                },
            ),
            ResponsesCompletionModel::new(client, "shared-model"),
        ))
    }
}

#[derive(Default, Debug)]
pub struct Observed {
    pub attempts: Vec<&'static str>,
    pub text: String,
    pub reasoning: String,
    pub errors: Vec<String>,
    pub finals: Vec<u64>,
    pub usage: Vec<u64>,
    pub tools: usize,
}

pub async fn collect<Model: CompletionModel<StreamingResponse = Final>>(
    model: Model,
) -> Result<Observed, Box<dyn std::error::Error>> {
    let request = CompletionRequestBuilder::new(model.clone(), "Hello")
        .preamble("Test instructions".to_string())
        .build();
    let mut stream = model.stream(request).await?;
    let mut observed = Observed::default();
    let mut snapshot = (0, 0);
    while let Some(event) = stream.next().await {
        match event {
            Ok(StreamedAssistantContent::ResponseAttemptStarted) => {
                observed.attempts.push("started");
                snapshot = (observed.text.len(), observed.reasoning.len());
            }
            Ok(StreamedAssistantContent::ResponseAttemptAborted) => {
                observed.attempts.push("aborted");
                observed.text.truncate(snapshot.0);
                observed.reasoning.truncate(snapshot.1);
            }
            Ok(StreamedAssistantContent::Text(text)) => observed.text.push_str(&text.text),
            Ok(StreamedAssistantContent::Reasoning(reasoning)) => {
                observed.reasoning.push_str(&reasoning.reasoning.join(""));
            }
            Ok(StreamedAssistantContent::ReasoningDelta { reasoning, .. }) => {
                observed.reasoning.push_str(&reasoning)
            }
            Ok(StreamedAssistantContent::Final(response)) => {
                observed.finals.push(response.usage.total_tokens)
            }
            Ok(StreamedAssistantContent::Usage(usage)) => observed.usage.push(usage.total_tokens),
            Ok(StreamedAssistantContent::ToolCall(_)) => observed.tools += 1,
            Err(error) => observed.errors.push(error.to_string()),
            _ => {}
        }
    }
    let aggregate: String = stream
        .choice
        .iter()
        .filter_map(|content| match content {
            rig::message::AssistantContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(aggregate, observed.text);
    Ok(observed)
}

pub fn event(value: Value) -> String {
    format!("data: {value}\n\n")
}

pub fn text() -> String {
    event(
        json!({"type":"response.output_text.delta", "delta":"Hello", "output_index":0, "content_index":0, "sequence_number":1}),
    )
}

pub fn terminal(status: &str, end_turn: Option<bool>, reason: Option<&str>) -> String {
    let mut response = baseline::response();
    response["status"] = json!(status);
    if let Some(end_turn) = end_turn {
        response["end_turn"] = json!(end_turn);
    }
    if let Some(reason) = reason {
        response["incomplete_details"] = json!({"reason": reason});
    }
    if status == "failed" {
        response["error"] = json!({"code":"bad_request", "message":"repair the schema"});
    }
    event(json!({"type":format!("response.{status}"), "sequence_number":3, "response":response}))
}

pub fn tool() -> String {
    event(
        json!({"type":"response.output_item.done", "output_index":1,"sequence_number":2,
        "item":{"type":"function_call","id":"fc_test","call_id":"call_test", "name":"inspect", "arguments":"{}", "status":"completed"}}),
    )
}

pub async fn run(body: String) -> Result<Observed, Box<dyn std::error::Error>> {
    let server = Server::start(vec![body], false).await?;
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        collect(server.models()?.0),
    )
    .await?
}
