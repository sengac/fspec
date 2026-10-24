use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, PoisonError,
};
use tokio::sync::Notify;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[path = "../fixtures/interruption_tool.rs"]
mod interruption_tool;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const REASONING: &str = "Check the guard, then preserve idempotence.";

pub fn response(reasoning: &str, text: &str, stop: Option<&str>) -> ResponseTemplate {
    let chunk = json!({
        "id":"thinking-only", "object":"chat.completion.chunk", "created":1,
        "model":"selected-model",
        "choices":[{"index":0,"delta":{"role":"assistant",
            "reasoning_content":reasoning,"content":text},"finish_reason":stop}],
        "usage":{"prompt_tokens":100,"completion_tokens":4096,"total_tokens":4196}
    });
    sse(chunk)
}

pub fn tool_response() -> ResponseTemplate {
    sse(json!({
        "id":"tool-progress", "object":"chat.completion.chunk", "created":1,
        "model":"selected-model",
        "choices":[{"index":0,"delta":{"role":"assistant",
            "reasoning_content":REASONING,
            "tool_calls":[{"index":0,"id":"inspect-1","type":"function",
                "function":{"name":"inspect","arguments":"{}"}}]},
            "finish_reason":"tool_calls"}],
        "usage":{"prompt_tokens":100,"completion_tokens":4096,"total_tokens":4196}
    }))
}

fn sse(chunk: Value) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

pub struct Output {
    pub events: Mutex<Vec<StreamEvent>>,
    interrupted: Arc<AtomicBool>,
    cancel_on_thinking: bool,
    pub cancel_on_retry: bool,
}

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        if (self.cancel_on_thinking && matches!(&event, StreamEvent::Thinking(_)))
            || (self.cancel_on_retry
                && matches!(&event, StreamEvent::Status(text) if text.starts_with("Thinking exhaustion detected")))
        {
            self.interrupted.store(true, Ordering::Release);
        }
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

pub struct Fixture {
    pub session: Session,
    pub output: Output,
    pub server: MockServer,
    pub tool_calls: Arc<AtomicUsize>,
    pub request_settings: Option<Value>,
    session_id: uuid::Uuid,
    _directory: tempfile::TempDir,
}

impl Fixture {
    pub async fn new(
        responses: Vec<ResponseTemplate>,
        armed: bool,
        cancel_on_thinking: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        let manifest = codelet_core::persistence::create_session_with_provider(
            "thinking exhaustion",
            directory.path(),
            "openai",
        )?;
        let server = MockServer::start().await;
        let response_count = responses.len();
        for (index, template) in responses.into_iter().enumerate() {
            let mut mock = Mock::given(method("POST"))
                .respond_with(template)
                .with_priority(u8::try_from(index + 1)?);
            if index + 1 < response_count {
                mock = mock.up_to_n_times(1);
            }
            mock.mount(&server).await;
        }
        let manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(128_000), Some(4096));
        let mut session = Session::from_provider_manager(manager);
        session.continue_enabled = armed;
        session.continue_budget = 1;
        Ok(Self {
            session,
            output: Output {
                events: Mutex::new(Vec::new()),
                interrupted: Arc::new(AtomicBool::new(false)),
                cancel_on_thinking,
                cancel_on_retry: false,
            },
            server,
            tool_calls: Arc::new(AtomicUsize::new(0)),
            request_settings: None,
            session_id: manifest.id,
            _directory: directory,
        })
    }

    pub async fn run(&mut self) -> TestResult {
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .base_url(self.server.uri())
                .build()?;
        let mut builder = client.agent("selected-model").max_tokens(4096).tool(
            interruption_tool::RecordingTool {
                name: "inspect",
                execution: interruption_tool::Execution {
                    completed: self.tool_calls.clone(),
                    started: Arc::new(AtomicUsize::new(0)),
                    interrupted: self.output.interrupted.clone(),
                    notify: Arc::new(Notify::new()),
                    interrupt_at: None,
                },
            },
        );
        if let Some(settings) = &self.request_settings {
            builder = builder.additional_params(settings.clone());
        }
        let agent = builder.build();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_agent_stream(
                RigAgent::new(agent, 4),
                "finish the task",
                &mut self.session,
                self.output.interrupted.clone(),
                Arc::new(AtomicBool::new(false)),
                Arc::new(Notify::new()),
                &self.output,
                self.session_id,
            ),
        )
        .await??;
        Ok(())
    }

    pub async fn requests(&self) -> Result<Vec<wiremock::Request>, Box<dyn std::error::Error>> {
        self.server
            .received_requests()
            .await
            .ok_or_else(|| "request log missing".into())
    }

    pub fn statuses(&self) -> Vec<String> {
        self.output
            .events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Status(text) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn retries(&self) -> usize {
        self.statuses()
            .iter()
            .filter(|text| text.starts_with("Thinking exhaustion detected"))
            .count()
    }
}
