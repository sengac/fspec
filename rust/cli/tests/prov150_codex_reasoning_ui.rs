//! Feature: spec/features/codex-reasoning-ui.feature
#[path = "../../providers/tests/fixtures/prov148.rs"]
pub mod transport;

use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive::stream_loop::run_agent_stream;
use codelet_cli::session::Session;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use serde_json::json;
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use tokio::sync::Notify;
use transport::{event, terminal, text, Server, TestResult};

#[derive(Default)]
struct RecordingOutput(Mutex<Vec<StreamEvent>>);

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn cli_reasoning_presentation_respects_completed_delivery_selection() -> TestResult {
    let temporary = tempfile::tempdir()?;
    codelet_common::set_data_directory(temporary.path().to_path_buf())?;
    for enabled in [false, true] {
        // @step Given a live Codex agent with incremental and completed-only reasoning
        let body = [
            event(json!({"type":"response.reasoning_summary_text.delta", "item_id":"reason", "summary_index":0, "delta":"Plan"})),
            event(json!({"type":"response.reasoning_summary_text.done", "item_id":"reason", "summary_index":0, "text":"Plan"})),
            event(json!({"type":"response.output_item.done", "item":{"type":"reasoning", "id":"reason", "encrypted_content":"opaque-secret",
                "summary":[{"type":"summary_text","text":"Plan"},{"type":"summary_text","text":"Verify"}]}})),
            text(), terminal("completed", Some(true), None),
        ].concat();
        let server = Server::start(vec![body], false).await?;
        let model = server.models()?.0.with_completed_reasoning(enabled);
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble("Local test instructions")
            .build();
        let manager = ProviderManager::for_testing(ProviderType::Codex, Some(272_000), Some(4096));
        let mut session = Session::from_provider_manager(manager);
        let output = RecordingOutput::default();
        // @step When the CLI stream loop runs with completed delivery enabled or disabled
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_agent_stream(
                RigAgent::with_default_depth(agent),
                "Hello",
                &mut session,
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicBool::new(false)),
                Arc::new(Notify::new()),
                &output,
                uuid::Uuid::new_v4(),
            ),
        )
        .await??;
        let events = output.0.lock().map_err(|_| "UI recording lock poisoned")?;
        let visible = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Thinking(text) => Some(format!("thinking:{text}")),
                StreamEvent::Text(text) => Some(format!("answer:{text}")),
                _ => None,
            })
            .collect::<Vec<_>>();
        let expected = if enabled {
            vec!["thinking:Plan", "thinking:Verify", "answer:Hello"]
        } else {
            vec!["thinking:Plan", "answer:Hello"]
        };
        // @step Then thinking events honor the selection and ordinary answer events remain visible
        assert_eq!(visible, expected);
        // @step And encrypted continuation content never reaches the UI
        assert!(!format!("{events:?}").contains("opaque-secret"));
    }
    Ok(())
}
