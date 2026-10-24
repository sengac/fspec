//! Feature: spec/features/runtime-request-context-fill.feature

use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{request_admission::AdmissionLimits, ProviderManager, ProviderType};
use serde_json::json;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

#[path = "../../providers/tests/fixtures/prov148.rs"]
mod fixture;

struct Output {
    events: Mutex<Vec<StreamEvent>>,
    backend: codelet_agent_loop::background_output::BackgroundOutput,
}

#[derive(Clone)]
struct Inspect;

impl rig::tool::Tool for Inspect {
    const NAME: &'static str = "inspect";
    type Error = std::io::Error;
    type Args = serde_json::Value;
    type Output = serde_json::Value;

    async fn definition(&self, _: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.into(),
            description: "Inspect request continuation".into(),
            parameters: json!({"type":"object"}),
        }
    }

    async fn call(&self, arguments: Self::Args) -> Result<Self::Output, Self::Error> {
        Ok(arguments)
    }
}

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event.clone());
        }
        self.backend.emit(event);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn multisegment_backend_fill_uses_latest_physical_usage() -> fixture::TestResult {
    // @step Given a current request snapshot and cumulative output from earlier segments
    let temporary = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(temporary.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    let manifest = codelet_core::persistence::create_session("context", temporary.path())?;
    let bodies = [false, true]
        .into_iter()
        .map(|end_turn| {
            let mut response = fixture::baseline::response();
            response["id"] = json!(format!("response-{end_turn}"));
            response["end_turn"] = json!(end_turn);
            response["usage"] = json!({"input_tokens":6000,"output_tokens":2000,"total_tokens":8000,
            "input_tokens_details":{"cached_tokens":400},
            "output_tokens_details":{"reasoning_tokens":300}});
            if !end_turn {
                response["output"] = json!([{
                    "type":"function_call", "id":"fc_test", "call_id":"call_test",
                    "name":"inspect", "arguments":"{}", "status":"completed"
                }]);
            }
            let terminal = fixture::event(json!({"type":"response.completed","response":response}));
            if end_turn {
                terminal
            } else {
                format!("{}{terminal}", fixture::tool())
            }
        })
        .collect();
    let server = fixture::Server::start(bodies, false).await?;
    let model = server.models()?.0.with_request_admission(AdmissionLimits {
        context_capacity: Some(20000),
        output_reservation: Some(100),
    });
    let agent = RigAgent::new(
        rig::agent::AgentBuilder::new(model).tool(Inspect).build(),
        2,
    );
    let manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(20000), Some(100));
    let session = Session::from_provider_manager(manager);
    let (input, _) = tokio::sync::mpsc::channel(4);
    let (chunks, _) = tokio::sync::broadcast::channel(128);
    let (statuses, _) = tokio::sync::broadcast::channel(4);
    let (cleared, _) = tokio::sync::broadcast::channel(4);
    let background = Arc::new(
        codelet_sessions::background_session::BackgroundSession::new(
            manifest.id,
            "fill".into(),
            temporary.path().to_string_lossy().into(),
            None,
            None,
            session,
            input,
            None,
            None,
            None,
            chunks,
            statuses,
            cleared,
        ),
    );
    let mut received = background.subscribe_to_stream();
    let output = Output {
        events: Mutex::new(Vec::new()),
        backend: codelet_agent_loop::background_output::BackgroundOutput::with_provider(
            background.clone(),
            "openai".into(),
            None,
        ),
    };
    let mut session = background.inner.lock().await;
    // @step When live and final backend and TUI token updates arrive
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            agent,
            "hello",
            &mut session,
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
            Arc::new(tokio::sync::Notify::new()),
            &output,
            manifest.id,
        ),
    )
    .await??;
    let events = output.events.lock().map_err(|_| "event lock")?;
    let fill = events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::ContextFill(fill) => Some(fill),
            _ => None,
        })
        .next_back()
        .ok_or_else(|| format!("missing backend context fill: {events:?}"))?;
    // @step Then both surfaces retain the same threshold-relative current request fill
    assert_eq!(
        fill.effective_tokens, 8000,
        "outer aggregate replaced latest physical occupancy"
    );
    // @step And cumulative generated spend remains separately visible
    assert!(
        events.iter().any(|event| matches!(
            event, StreamEvent::Tokens(tokens)
                if tokens.output_tokens >= 3400 && tokens.reasoning_tokens.unwrap_or(0) >= 600
        )),
        "missing aggregate spend: {:?}",
        events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Tokens(tokens) => Some(tokens),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
    let backend: Arc<dyn codelet_fspec_tui::FspecBackend> = Arc::new(common::MockBackend::new());
    let mut app = codelet_fspec_tui::App::new(backend);
    let session_id = codelet_rpc_types::SessionId::new(manifest.id.to_string());
    app.dispatch(codelet_fspec_tui::Action::SessionCreated(
        session_id.clone(),
    ));
    while let Ok(chunk) = received.try_recv() {
        app.dispatch(codelet_fspec_tui::Action::ChunkReceived(
            session_id.clone(),
            chunk,
        ));
    }
    let state = app
        .agent_view_store()
        .token_state_for(&session_id)
        .ok_or("TUI token state")?;
    assert_eq!(u32::from(state.context_fill_pct), fill.fill_percentage);
    assert_eq!(state.context_threshold_tokens, fill.threshold as u64);
    assert!(state.output_tokens >= 3400);
    assert!(state.reasoning_tokens >= 600);
    assert_eq!(server.requests.lock().map_err(|_| "wire lock")?.len(), 2);
    Ok(())
}

mod common;
