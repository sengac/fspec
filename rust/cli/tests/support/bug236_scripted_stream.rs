use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use rig::client::CompletionClient;
use rig::completion::{
    CompletionError, CompletionModel, CompletionRequest, CompletionResponse, GetTokenUsage, Usage,
};
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize},
    Arc, Mutex,
};

#[path = "../fixtures/interruption_tool.rs"]
mod interruption_tool;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub type Step = Result<RawStreamingChoice<Reply>, CompletionError>;

#[derive(Clone, Serialize, Deserialize)]
pub struct Reply(Option<Usage>);

impl GetTokenUsage for Reply {
    fn token_usage(&self) -> Option<Usage> {
        self.0
    }
}

pub fn unreported_reply() -> Step {
    Ok(RawStreamingChoice::FinalResponse(Reply(None)))
}

#[derive(Clone)]
struct Model {
    inner: rig::providers::openai::completion::CompletionModel,
    scripts: Arc<Mutex<VecDeque<Vec<Step>>>>,
}

impl CompletionModel for Model {
    type Response = Reply;
    type StreamingResponse = Reply;
    type Client = Self;

    fn make(client: &Self::Client, _: impl Into<String>) -> Self {
        client.clone()
    }

    fn prepare_request(&self, request: CompletionRequest) -> Result<Vec<u8>, CompletionError> {
        self.inner.prepare_request(request)
    }

    async fn completion(
        &self,
        _: CompletionRequest,
    ) -> Result<CompletionResponse<Reply>, CompletionError> {
        Err(CompletionError::ProviderError(
            "script only supports streaming".into(),
        ))
    }

    async fn stream(
        &self,
        _: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Reply>, CompletionError> {
        let script = self
            .scripts
            .lock()
            .map_err(|error| CompletionError::ProviderError(error.to_string()))?
            .pop_front()
            .ok_or_else(|| CompletionError::ProviderError("unexpected extra request".into()))?;
        Ok(StreamingCompletionResponse::stream(Box::pin(
            futures::stream::iter(script),
        )))
    }
}

pub fn usage(input: u64, output: u64) -> Usage {
    Usage {
        input_tokens: input,
        output_tokens: output,
        total_tokens: input + output,
        ..Default::default()
    }
}

pub fn streamed() -> Step {
    Ok(RawStreamingChoice::Usage(Usage {
        input_tokens: 30_000,
        cache_read_input_tokens: Some(150_000),
        output_tokens: 1,
        total_tokens: 180_001,
        ..Default::default()
    }))
}

pub fn final_reply(input: u64, output: u64) -> Step {
    Ok(RawStreamingChoice::FinalResponse(Reply(Some(Usage {
        reasoning_tokens: Some(3),
        total_tokens: input + output + 3,
        ..usage(input, output)
    }))))
}

pub fn text(value: &str) -> Step {
    Ok(RawStreamingChoice::Message(value.into()))
}

pub fn tool() -> Step {
    Ok(RawStreamingChoice::ToolCall(RawStreamingToolCall {
        id: "inspect-1".into(),
        call_id: Some("inspect-1".into()),
        name: "inspect".into(),
        arguments: serde_json::json!({}),
        signature: None,
        additional_params: None,
    }))
}

pub fn aborted() -> Vec<Step> {
    vec![
        Ok(RawStreamingChoice::ResponseAttemptStarted),
        streamed(),
        text("tentative"),
        Ok(RawStreamingChoice::ResponseAttemptAborted),
        text("accepted"),
        final_reply(0, 17),
    ]
}

pub fn session(provider: ProviderType) -> TestResult<Session> {
    let mut manager = ProviderManager::for_testing(provider.clone(), Some(1_000_000), Some(4096));
    manager.set_model_direct(
        provider.as_str(),
        if provider == ProviderType::Gemini {
            "gemini-3-flash-preview"
        } else {
            "o3"
        },
        Some(1_000_000),
        Some(4096),
        None,
    )?;
    Ok(Session::from_provider_manager(manager))
}

#[derive(Default)]
struct Output;
impl StreamOutput for Output {
    fn emit(&self, _: StreamEvent) {}
}

pub async fn run(provider: ProviderType, scripts: Vec<Vec<Step>>) -> TestResult<Session> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    codelet_core::persistence::reset_stores_for_tests();
    let manifest = codelet_core::persistence::create_session_with_provider(
        "BUG-236 physical requests",
        directory.path(),
        provider.as_str(),
    )?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::new("offline-script")?;
    let model = Model {
        inner: client.completion_model("o3"),
        scripts: Arc::new(Mutex::new(scripts.into())),
    };
    let interrupted = Arc::new(AtomicBool::new(false));
    let notify = Arc::new(tokio::sync::Notify::new());
    let agent = rig::agent::AgentBuilder::new(model)
        .preamble("Offline accounting test")
        .tool(interruption_tool::RecordingTool {
            name: "inspect",
            execution: interruption_tool::Execution {
                completed: Arc::new(AtomicUsize::new(0)),
                started: Arc::new(AtomicUsize::new(0)),
                interrupted: interrupted.clone(),
                notify: notify.clone(),
                interrupt_at: None,
            },
        })
        .build();
    let mut session = session(provider)?;
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "Continue",
            &mut session,
            interrupted,
            Arc::new(AtomicBool::new(false)),
            notify,
            &Output,
            manifest.id,
        ),
    )
    .await??;
    Ok(session)
}
