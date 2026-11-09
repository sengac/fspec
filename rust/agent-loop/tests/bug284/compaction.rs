//! Real Codex loopback turns with parallel once-only effects, admitted fresh
//! inputs that trigger the stream driver's automatic pre-prompt compaction,
//! and cold restoration from durable originals.

use super::fixture::{history, Harness, MODEL};
use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{build_user_content_with_images, run_agent_stream};
use codelet_core::persistence::input_receipts::{self, ExecutionOutcome, InputOrigin};
use codelet_core::persistence::*;
use codelet_core::RigAgent;
use codelet_providers::codex::{refreshing_client::RefreshingCodexClient, CodexResponsesModel};
use rig::{completion::ToolDefinition, message::Message, tool::Tool};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};

pub type Failure = Box<dyn std::error::Error>;

pub struct ParallelEffect(pub PathBuf);

impl Tool for ParallelEffect {
    const NAME: &'static str = "write_parallel_effect";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write one named once-only effect".into(),
            parameters: json!({"type":"object","properties":{"name":{"type":"string"}}}),
        }
    }

    async fn call(&self, args: Value) -> Result<String, Self::Error> {
        let name = args["name"].as_str().unwrap_or("unnamed");
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.0.join(format!("parallel-{name}")))?;
        Ok(format!("parallel {name} executed once"))
    }
}

pub fn completed(id: &str, output: Vec<Value>) -> String {
    crate::transport::event(json!({"type":"response.completed","response":{
        "id":id,"status":"completed","end_turn":true,"output":output,
        "usage":{"input_tokens":40,"output_tokens":5,"total_tokens":45}}}))
}

pub fn answer(id: &str, text: &str) -> String {
    completed(
        id,
        vec![
            json!({"type":"message","id":format!("{id}-message"),"role":"assistant",
        "content":[{"type":"output_text","text":text,"annotations":[]}]}),
        ],
    )
}

pub fn parallel_calls() -> String {
    let call = |name: &str| {
        json!({"type":"function_call","id":format!("item-{name}"),"call_id":format!("call-{name}"),
        "name":"write_parallel_effect","arguments":json!({"name":name,"work_unit_id":"NONE"}).to_string()})
    };
    completed("parallel-response", vec![call("a"), call("b")])
}

/// Run one real streamed Codex turn on the harness runtime.
pub async fn turn(
    harness: &Harness,
    prompt: &str,
    bodies: Vec<String>,
    automatic_compaction: bool,
) -> Result<crate::transport::Server, Failure> {
    let server = crate::transport::Server::start(bodies, false).await?;
    harness.session.reset_interrupt();
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        harness.session.id,
        harness.directory.path().into(),
    )?;
    let mut inner = harness.session.inner.lock().await;
    if automatic_compaction {
        inner.token_tracker.input_tokens = 199_000;
    }
    let client = rig::providers::openai::Client::<RefreshingCodexClient>::builder()
        .api_key("synthetic-offline-key")
        .base_url(&server.url)
        .http_client(RefreshingCodexClient::new_api_key().with_test_endpoint(&server.url)?)
        .build()?;
    let model = CodexResponsesModel::new(client, MODEL)
        .with_history(history(&inner)?)
        .with_request_admission(codelet_providers::request_admission::AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        });
    let agent = rig::agent::AgentBuilder::new(model)
        .tool(ParallelEffect(harness.directory.path().into()))
        .build();
    codelet_test_helpers::deadlock_guard(
        "run_agent_stream( RigAgent::with_default_depth(agent), pr...",
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            prompt,
            &mut inner,
            harness.session.is_interrupted.clone(),
            harness.session.compaction_in_progress.clone(),
            harness.session.interrupt_notify.clone(),
            &BackgroundOutput::with_provider(harness.session.clone(), "codex".into(), None),
            harness.session.id,
        ),
    )
    .await??;
    Ok(server)
}

/// Retain, store, admit and install a genuine user input exactly as the
/// task loop's admission does, returning its immutable identity.
pub async fn admit(harness: &Harness, text: &str) -> Result<uuid::Uuid, Failure> {
    let session = harness.session.id;
    let epoch = harness.session.compaction_epoch();
    let id = harness
        .session
        .retain_submission(InputOrigin::User, Some(epoch), text, &[])?;
    let receipt = input_receipts::get(session, id)?.ok_or("receipt")?;
    codelet_agent_loop::persist::persist_input_original(&session, id, receipt.content)?;
    input_receipts::admit(session, id)?;
    let message = Message::User {
        content: build_user_content_with_images(text, None),
    };
    harness
        .session
        .inner
        .lock()
        .await
        .install_admitted_input(id, message);
    input_receipts::record_execution(session, id, ExecutionOutcome::DispatchStarted)?;
    Ok(id)
}

/// Deterministic compaction builder covering the captured durable prefix.
pub async fn register_builder(session: uuid::Uuid) -> Result<(), Failure> {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        while codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler(session) {
            codelet_test_helpers::yield_now().await;
        }
    })
    .await?;
    codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler(
        session,
        Some(Arc::new(
            |_, source: compaction_boundary::CompactionSource| {
                Box::pin(async move {
                    let last = source.prefix.len().saturating_sub(1);
                    Ok(format!("<dag-node depth=\"D1\" turns=\"0-{last}\" label=\"Work\">Parallel effects completed once; canceled inputs never admitted</dag-node>"))
                })
            },
        )),
    );
    Ok(())
}

/// Replace the runtime with a cold restore from durable originals.
pub async fn cold(harness: &Harness) -> Result<(), Failure> {
    reset_stores_for_tests();
    let manifest = load_session(harness.session.id)?;
    let envelopes = get_session_message_envelopes(manifest.id)?;
    let mut manager = codelet_providers::ProviderManager::for_testing(
        codelet_providers::ProviderType::Codex,
        Some(200_000),
        Some(4096),
    );
    manager.set_model_direct("codex", MODEL, Some(200_000), Some(4096), None)?;
    let mut cold = codelet_cli::session::Session::from_provider_manager(manager);
    cold.preflight_compaction_restore(&manifest, &envelopes)?;
    cold.restore_compaction_boundary(&manifest)?;
    cold.restore_provider_state(&envelopes)?;
    cold.messages = crate::replay_content::Replay::parse(&envelopes)?.messages;
    cold.bind_provider_persistence(manifest.id)?;
    cold.ensure_compaction_inference_ready()?;
    *harness.session.inner.lock().await = cold;
    Ok(())
}
