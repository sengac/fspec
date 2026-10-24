use super::metrics;
use codelet_agent_loop::generate_compaction_handler::{
    register_compactor_sub_agent_handler, register_generate_compaction_handler,
};
use codelet_core::persistence::{append_message_with_metadata, load_session, save_session};
use codelet_providers::request_admission::AdmissionLimits;
use codelet_tools::{GenerateCompactionArgs, GenerateCompactionTool};
use rig::{client::CompletionClient, completion::CompletionRequestBuilder, tool::Tool};
use serde_json::{json, Value};
use std::sync::{atomic::Ordering, Arc, Mutex};
use wiremock::{Mock, ResponseTemplate};

pub const DAG: &str =
    "<dag-node depth=\"D1\" turns=\"0-3\" label=\"Evidence\">Complete source</dag-node>";
pub const OUTPUT: u64 = 4096;

pub struct Harness {
    pub fixture: metrics::Fixture,
    pub limits: AdmissionLimits,
}

impl Drop for Harness {
    fn drop(&mut self) {
        codelet_tools::set_generate_compaction_handler(self.fixture.session.id, None);
        codelet_tools::set_inject_summary_handler(self.fixture.session.id, None);
    }
}

impl Harness {
    pub async fn new(capacity: u64, padding: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let fixture = metrics::Fixture::new().await?;
        let mut manifest = load_session(fixture.session.id)?;
        manifest.messages.clear();
        save_session(&manifest)?;
        for position in 0..4 {
            let content = format!(
                "evidence-{position}-start {} evidence-{position}-end",
                "x".repeat(padding)
            );
            append_message_with_metadata(
                &mut manifest,
                "user",
                &content,
                [
                    ("uuid".into(), json!(uuid::Uuid::new_v4())),
                    ("message".into(), json!({"role":"user", "content":content})),
                ]
                .into(),
            )?;
            let page = codelet_core::persistence::originals::retrieve_originals(
                serde_json::from_value(json!({"session_id":manifest.id,
                    "start_turn":position, "end_turn":position, "limit":1, "max_bytes":61440}))?,
            )?;
            assert_eq!(
                page.records.len(),
                1,
                "fixture original must be retrievable intact"
            );
        }
        let harness = Self {
            fixture,
            limits: AdmissionLimits {
                context_capacity: Some(capacity),
                output_reservation: Some(OUTPUT),
            },
        };
        harness.configure().await?;
        Ok(harness)
    }

    pub async fn configure(&self) -> metrics::TestResult {
        let mut inner = self.fixture.session.inner.lock().await;
        inner.provider_manager_mut().set_model_direct(
            "openai",
            "o3",
            self.limits.context_capacity.map(|value| value as usize),
            Some(OUTPUT as usize),
            None,
        )?;
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline-runway")
                .base_url(self.fixture.server.uri())
                .build()?;
        let model = client.completion_model("o3");
        inner.install_selected_preparer(
            codelet_providers::selected_request::SelectedRequestPreparer::new(
                model.clone(),
                CompletionRequestBuilder::new(model, "unused")
                    .max_tokens(OUTPUT)
                    .build(),
                self.limits,
            ),
        );
        register_compactor_sub_agent_handler(
            self.fixture.session.id,
            &inner,
            std::env::temp_dir(),
            self.fixture.session.compaction_in_progress.clone(),
        );
        Ok(())
    }

    pub async fn prior_generation(&self) -> metrics::TestResult {
        self.fixture.server.reset().await;
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(response(json!({"role":"assistant", "content":DAG}), false))
            .mount(&self.fixture.server)
            .await;
        self.call(false).await?;
        assert!(load_session(self.fixture.session.id)?
            .compaction_commit
            .is_some());
        Ok(())
    }

    pub async fn mount(&self, summary: &'static str, cancel: bool, provider_error: bool) {
        self.fixture.server.reset().await;
        let session = self.fixture.session.clone();
        Mock::given(wiremock::matchers::method("POST"))
            .respond_with(move |request: &wiremock::Request| {
                let body: Value = request.body_json().expect("exploration JSON");
                let tools = has_tools(&body);
                let previous = body["messages"]
                    .as_array()
                    .expect("messages")
                    .iter()
                    .filter(|message| message["role"] == "tool")
                    .count();
                if tools && provider_error && previous > 0 {
                    return ResponseTemplate::new(400).set_body_json(json!({"error":{
                        "message":"ordinary provider capacity failure, not local admission"}}));
                }
                if !tools && cancel {
                    session.is_interrupted.store(true, Ordering::Release);
                    session.interrupt_notify.notify_waiters();
                }
                let position = previous % 4;
                let message = if tools {
                    json!({"role":"assistant", "content":null, "tool_calls":[{
                        "id":format!("original-{previous}"), "type":"function", "function":{
                            "name":"SessionSearch", "arguments":json!({
                                "action_type":"originals", "session_id":session.id,
                                "start_turn":position, "end_turn":position, "limit":1,
                                "max_bytes":61440
                            }).to_string()}}]})
                } else {
                    json!({"role":"assistant", "content":summary})
                };
                let template = response(message, tools);
                if !tools && cancel {
                    template.set_delay(std::time::Duration::from_millis(100))
                } else {
                    template
                }
            })
            .mount(&self.fixture.server)
            .await;
    }

    pub async fn call(&self, generated: bool) -> Result<(), String> {
        if generated {
            let queue = {
                let inner = self.fixture.session.inner.lock().await;
                register_generate_compaction_handler(
                    self.fixture.session.id,
                    &inner,
                    std::env::temp_dir(),
                    self.fixture.session.clone(),
                    None,
                )
            };
            GenerateCompactionTool::new(self.fixture.session.id)
                .call(GenerateCompactionArgs { session_id: None })
                .await
                .map_err(|error| error.to_string())?;
            let mut inner = self.fixture.session.inner.lock().await;
            queue.apply(&mut inner, &self.fixture.session)?;
            Ok(())
        } else {
            let output = codelet_agent_loop::BackgroundOutput::with_provider(
                self.fixture.session.clone(),
                "openai".into(),
                None,
            );
            let mut inner = self.fixture.session.inner.lock().await;
            codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
                &mut inner,
                self.fixture.session.id,
                &Arc::new(Mutex::new(codelet_core::TokenState::default())),
                &self.fixture.session.compaction_in_progress,
                &output,
                &codelet_cli::compaction_cancellation::CompactionCancellation::new(
                    self.fixture.session.is_interrupted.clone(),
                    Some(self.fixture.session.interrupt_notify.clone()),
                ),
            )
            .await
            .map_err(|error| error.to_string())
        }
    }

    pub async fn bodies(&self) -> Vec<Value> {
        self.fixture
            .server
            .received_requests()
            .await
            .expect("requests")
            .iter()
            .map(|request| request.body_json().expect("request JSON"))
            .collect()
    }

    pub async fn assert_admitted(&self) -> metrics::TestResult {
        for request in self
            .fixture
            .server
            .received_requests()
            .await
            .ok_or("requests")?
        {
            codelet_providers::request_admission::admit_complete_request(
                &request.body,
                self.limits,
            )?;
        }
        Ok(())
    }
}

pub fn has_tools(body: &Value) -> bool {
    body["tools"]
        .as_array()
        .is_some_and(|tools| !tools.is_empty())
}

fn response(message: Value, tools: bool) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "id":"runway", "object":"chat.completion", "created":1, "model":"o3",
        "choices":[{"index":0,"message":message,"finish_reason":if tools {"tool_calls"} else {"stop"}}],
        "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
    }))
}
