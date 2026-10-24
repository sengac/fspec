use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex, PoisonError,
};
use tokio::sync::Notify;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn response(parts: Value, reason: &str) -> ResponseTemplate {
    measured_response(
        parts,
        reason,
        json!({
            "promptTokenCount":100,"candidatesTokenCount":10,"totalTokenCount":110
        }),
    )
}

pub fn measured_response(parts: Value, reason: &str, usage: Value) -> ResponseTemplate {
    let frame = json!({"candidates":[{"content":{"role":"model","parts":parts},
        "finishReason":reason,"index":0}],"usageMetadata":usage});
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {frame}\n\n"))
}

pub fn text(text: &str) -> ResponseTemplate {
    response(json!([{"text":text}]), "STOP")
}

pub fn tool(name: &str) -> ResponseTemplate {
    let args = if name == "done" {
        json!({"summary":"Task verified","evidence":["local check passed"],
            "goal_assessment":"The requested task is complete and the local check passed."})
    } else {
        json!({})
    };
    let name = if name == "done" { "Done" } else { name };
    response(json!([{"functionCall":{"name":name,"args":args}}]), "STOP")
}

pub fn enter_helper() -> Vec<ResponseTemplate> {
    vec![tool("inspect"), text("")]
}

pub struct Output {
    events: Mutex<Vec<StreamEvent>>,
    interrupted: Arc<AtomicBool>,
    pub cancel_on_text: bool,
}

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        if self.cancel_on_text
            && matches!(&event, StreamEvent::Text(text) if text == "partial helper")
        {
            self.interrupted.store(true, Ordering::Release);
        }
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}

impl Output {
    pub fn events(&self) -> Vec<StreamEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn statuses(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                StreamEvent::Status(text) => Some(text),
                _ => None,
            })
            .collect()
    }

    pub fn terminals(&self) -> Vec<Option<String>> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                StreamEvent::Done(reason) => Some(reason),
                _ => None,
            })
            .collect()
    }
}

struct Inspect {
    calls: Arc<AtomicUsize>,
    large: bool,
}

impl Tool for Inspect {
    const NAME: &'static str = "inspect";
    type Error = std::io::Error;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: "inspect".into(),
            description: "Inspect local evidence".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, _: Value) -> Result<Value, Self::Error> {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(if self.large && index > 0 {
            json!({"evidence":"large evidence ".repeat(3_000)})
        } else {
            json!({"evidence":"checked"})
        })
    }
}

pub struct Fixture {
    pub session: Session,
    pub output: Output,
    pub session_id: uuid::Uuid,
    pub directory: tempfile::TempDir,
    pub calls: Arc<AtomicUsize>,
    pub large_tool_result: bool,
    server: MockServer,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        codelet_tools::set_continue_armed(self.session_id, false);
        codelet_tools::set_session_goal(self.session_id, None);
        codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(self.session_id, None);
    }
}

impl Fixture {
    pub async fn new(responses: Vec<ResponseTemplate>) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        codelet_core::persistence::reset_stores_for_tests();
        let manifest = codelet_core::persistence::create_session_with_provider(
            "Gemini helper settlement",
            directory.path(),
            "gemini",
        )?;
        let server = MockServer::start().await;
        for (index, response) in responses.into_iter().enumerate() {
            Mock::given(method("POST"))
                .respond_with(response)
                .with_priority(u8::try_from(index + 1)?)
                .up_to_n_times(1)
                .mount(&server)
                .await;
        }
        let mut manager =
            ProviderManager::for_testing(ProviderType::Gemini, Some(128_000), Some(1000));
        manager.set_model_direct("google", "gemini-2.5-pro", Some(128_000), Some(1000), None)?;
        let mut session = Session::from_provider_manager(manager);
        session.continue_enabled = true;
        session.continue_budget = 1;
        codelet_tools::set_continue_armed(manifest.id, true);
        assert_eq!(session.current_provider_name(), "gemini");
        Ok(Self {
            session,
            output: Output {
                events: Mutex::new(Vec::new()),
                interrupted: Arc::new(AtomicBool::new(false)),
                cancel_on_text: false,
            },
            session_id: manifest.id,
            directory,
            calls: Arc::new(AtomicUsize::new(0)),
            large_tool_result: false,
            server,
        })
    }

    pub async fn run(&mut self) -> TestResult {
        let client: rig::providers::gemini::Client = rig::providers::gemini::Client::builder()
            .api_key("offline")
            .base_url(self.server.uri())
            .build()?;
        let agent = client
            .agent("gemini-2.5-pro")
            .max_tokens(1000)
            .tool(Inspect {
                calls: self.calls.clone(),
                large: self.large_tool_result,
            })
            .tool(codelet_tools::DoneTool::new(self.session_id))
            .build();
        tokio::time::timeout(
            std::time::Duration::from_secs(8),
            run_agent_stream(
                RigAgent::new(agent, 8),
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

    pub async fn requests(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let requests = self
            .server
            .received_requests()
            .await
            .ok_or("request log missing")?;
        let helper = requests.get(2).ok_or("helper request missing")?;
        assert!(
            String::from_utf8_lossy(&helper.body).contains("Based on the information retrieved"),
            "helper request: {}",
            String::from_utf8_lossy(&helper.body)
        );
        requests
            .iter()
            .map(|request| serde_json::from_slice(&request.body).map_err(Into::into))
            .collect()
    }
}
