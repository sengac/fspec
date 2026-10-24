#![doc = "Feature: spec/features/transcript-executor-admission.feature"]

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_core::RigAgent;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, TestResult};

#[derive(Clone)]
struct Sentinel(PathBuf);

struct FaultOutput {
    output: BackgroundOutput,
    path: PathBuf,
}

impl StreamOutput for FaultOutput {
    fn emit(&self, event: StreamEvent) {
        self.output.emit(event);
    }

    fn prepare_tool_dispatch(&self, id: &str) -> Result<(), String> {
        std::fs::rename(&self.path, self.path.with_extension("held"))
            .map_err(|error| error.to_string())?;
        std::fs::write(&self.path, "unavailable").map_err(|error| error.to_string())?;
        self.output.prepare_tool_dispatch(id)
    }
}

impl Tool for Sentinel {
    const NAME: &'static str = "Sentinel";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write a real sentinel".into(),
            parameters: json!({"type":"object","properties":{}}),
        }
    }

    async fn call(&self, _: Value) -> Result<String, Self::Error> {
        std::fs::write(&self.0, "executed")?;
        Ok("authentic result".into())
    }
}

#[tokio::test]
#[serial_test::serial]
async fn the_real_executor_cannot_bypass_failed_transcript_admission() -> TestResult {
    // @step Given a provider requesting a sentinel-writing tool and transcript storage failing at dispatch
    let (directory, session, _) = fixture()?;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(session.id, directory.path().into())?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    let sentinel = directory.path().join("sentinel");
    let agent = rig::agent::AgentBuilder::new(codelet_providers::stub_model::StubModel::new())
        .additional_params(json!({"stub_tool_call":{"id":"sentinel", "name":"Sentinel",
            "arguments":{"work_unit_id":"NONE"}}}))
        .tool(Sentinel(sentinel.clone()))
        .build();
    let output = FaultOutput {
        output: BackgroundOutput::with_provider(session.clone(), "stub".into(), None),
        path: directory
            .path()
            .join("transcripts")
            .join(session.id.to_string()),
    };
    // @step When the actual streaming executor consumes that provider request
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_agent_stream(
            RigAgent::with_default_depth(agent),
            "write sentinel",
            &mut *session.inner.lock().await,
            session.is_interrupted.clone(),
            Arc::new(AtomicBool::new(false)),
            session.interrupt_notify.clone(),
            &output,
            session.id,
        ),
    )
    .await?;
    // @step Then the executor returns a recovery error without creating the sentinel
    assert!(
        !sentinel.exists(),
        "failed admission must prevent actual effects"
    );
    assert!(
        result.is_err(),
        "executor must report failed durable admission"
    );
    assert!(support::read(session.id)?.is_empty());
    Ok(())
}
