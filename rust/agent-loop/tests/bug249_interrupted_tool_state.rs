#![doc = "Feature: spec/features/interrupted-tool-journal-outcomes.feature"]

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::run_agent_stream;
use codelet_core::{
    persistence::transcript::{JournalEvent, JournalStore, ToolState},
    RigAgent,
};
use codelet_sessions::background_session::BackgroundSession;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, TestResult};

#[derive(Clone)]
struct InterruptedSentinel {
    session: Arc<BackgroundSession>,
    path: PathBuf,
}

impl Tool for InterruptedSentinel {
    const NAME: &'static str = "InterruptedSentinel";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write and interrupt before result capture".into(),
            parameters: json!({"type":"object","properties":{}}),
        }
    }

    async fn call(&self, _: Value) -> Result<String, Self::Error> {
        std::fs::write(&self.path, "executed once")?;
        self.session.interrupt();
        std::future::pending().await
    }
}

#[tokio::test]
#[serial_test::serial]
async fn interrupted_running_tools_retain_unknown_outcomes() -> TestResult {
    // @step Given a running tool that has written an external sentinel
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(session.id, directory.path().into())?;
    let path = directory.path().join("interrupted-sentinel");
    let agent = rig::agent::AgentBuilder::new(codelet_providers::stub_model::StubModel::new())
        .additional_params(
            json!({"stub_tool_call":{"id":"interrupted", "name":"InterruptedSentinel",
            "arguments":{"work_unit_id":"NONE"}}}),
        )
        .tool(InterruptedSentinel {
            session: session.clone(),
            path: path.clone(),
        })
        .build();
    let output = BackgroundOutput::with_provider(session.clone(), "stub".into(), None);
    // @step When a genuine user interruption stops streaming before authentic result capture
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "write then stop",
            &mut *session.inner.lock().await,
            session.is_interrupted.clone(),
            Arc::new(AtomicBool::new(false)),
            session.interrupt_notify.clone(),
            &output,
            session.id,
        ),
    )
    .await??;
    assert_eq!(std::fs::read_to_string(path)?, "executed once");
    // @step Then the durable tool outcome remains unknown rather than completed or cancelled
    let projection = JournalStore::configured(session.id)?.project(&HashSet::new())?;
    assert_eq!(projection.tools.len(), 1);
    assert_eq!(projection.tools[0].state, ToolState::OutcomeUnknown);
    assert!(projection.tools[0].result.is_none());
    let originals = support::read(session.id)?;
    assert!(!originals.is_empty());
    let identities = originals
        .iter()
        .filter_map(|record| record["uuid"].as_str()?.parse().ok())
        .collect();
    let display = JournalStore::configured(session.id)?.project(&identities)?;
    assert!(display
        .records
        .iter()
        .any(|record| matches!(record.event, JournalEvent::ToolOutcomeUnknown { .. })));
    Ok(())
}
