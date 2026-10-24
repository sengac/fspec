#![doc = "Feature: spec/features/process-crash-transcript-recovery.feature"]

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_core::persistence::transcript::{JournalEvent, JournalRecord, JournalStore, ToolState};
use codelet_test_helpers::setup_temp_directory;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::json;
use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::{Duration, Instant};

#[path = "support/bug246.rs"]
mod support;
use support::TestResult;

type Killed = (tempfile::TempDir, uuid::Uuid, Vec<JournalRecord>);

fn killed(mode: &str) -> Result<Killed, Box<dyn std::error::Error>> {
    let handshake = setup_temp_directory()?;
    let ready = handshake.path().join("ready");
    let mut child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "streaming_subprocess",
            "--ignored",
            "--nocapture",
        ])
        .env("BUG249_READY", &ready)
        .env("BUG249_MODE", mode)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready.exists() && Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Err(format!("isolated agent exited before acknowledgement: {status}").into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill()?;
    child.wait()?;
    let acknowledged: serde_json::Value = serde_json::from_slice(&std::fs::read(ready)?)?;
    let root = acknowledged["root"].as_str().ok_or("missing root")?;
    let id = acknowledged["id"]
        .as_str()
        .ok_or("missing session")?
        .parse()?;
    let records = serde_json::from_value(acknowledged["records"].clone())?;
    std::fs::rename(root, handshake.path().join("data"))?;
    Ok((handshake, id, records))
}

#[derive(Clone)]
struct KillSentinel {
    root: std::path::PathBuf,
    ready: std::path::PathBuf,
    id: uuid::Uuid,
}

impl Tool for KillSentinel {
    const NAME: &'static str = "KillSentinel";
    type Args = serde_json::Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Write once before process termination".into(),
            parameters: json!({"type":"object","properties":{}}),
        }
    }

    async fn call(&self, _: Self::Args) -> Result<String, Self::Error> {
        std::fs::write(self.root.join("sentinel"), "one\n")?;
        let records = JournalStore::new(&self.root, self.id)
            .read()
            .map_err(|_| std::io::Error::other("journal unavailable at dispatch"))?;
        std::fs::write(
            &self.ready,
            serde_json::to_vec(&json!({"root":self.root,"id":self.id,"records":records}))?,
        )?;
        std::future::pending().await
    }
}

#[test]
fn a_killed_process_preserves_its_acknowledged_prefix() -> TestResult {
    // @step Given an isolated streaming process with a durable transcript store
    // @step When a known text prefix is acknowledged and the process is killed before completion
    let (directory, id, acknowledged) = killed("prefix")?;
    let store = JournalStore::new(&directory.path().join("data"), id);
    assert_eq!(store.read()?, acknowledged);
    let projection = store.project(&HashSet::new())?;
    assert!(projection.records.iter().any(|record| {
        // @step Then a fresh process recovers that exact prefix and its original identities
        matches!(&record.event, JournalEvent::Text { text } if text == "acknowledged prefix α")
    }));
    Ok(())
}

#[test]
fn a_killed_process_excludes_its_aborted_response_attempt() -> TestResult {
    // @step Given an isolated streaming process that aborts one response attempt and retries
    // @step When the successful replacement prefix is acknowledged before process termination
    let (directory, id, acknowledged) = killed("attempt")?;
    let store = JournalStore::new(&directory.path().join("data"), id);
    assert_eq!(store.read()?, acknowledged);
    // @step Then cold recovery exposes the replacement prefix once and excludes aborted text
    for _ in 0..2 {
        let projection = store.project(&HashSet::new())?;
        assert_eq!(projection.records.len(), 1);
        assert!(matches!(&projection.records[0].event,
            JournalEvent::Text { text } if text == "authentic replacement α"));
    }
    Ok(())
}

#[test]
fn a_killed_process_preserves_an_invocation_that_never_started() -> TestResult {
    // @step Given an isolated streaming process with an acknowledged tool invocation
    // @step When the process is terminated before dispatch admission
    let (directory, id, acknowledged) = killed("requested")?;
    let store = JournalStore::new(&directory.path().join("data"), id);
    assert_eq!(store.read()?, acknowledged);
    // @step Then cold recovery marks the invocation not-started without executing its side effect
    for _ in 0..2 {
        let projection = store.project(&HashSet::new())?;
        assert_eq!(projection.tools.len(), 1);
        assert_eq!(projection.tools[0].state, ToolState::NotStarted);
        assert!(!directory.path().join("data/sentinel").exists());
    }
    Ok(())
}

#[test]
fn a_killed_tool_process_never_repeats_an_unknown_side_effect() -> TestResult {
    // @step Given a tool invocation durably recorded before dispatch
    // @step When the tool writes a sentinel and the process is killed before result capture
    let (directory, id, acknowledged) = killed("tool")?;
    let store = JournalStore::new(&directory.path().join("data"), id);
    for _ in 0..2 {
        assert_eq!(store.read()?, acknowledged);
        // @step Then recovery shows outcome-unknown and never writes the sentinel again
        let projection = store.project(&HashSet::new())?;
        assert_eq!(projection.tools.len(), 1);
        assert_eq!(projection.tools[0].state, ToolState::OutcomeUnknown);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("data/sentinel"))?,
            "one\n"
        );
    }
    Ok(())
}

#[tokio::test]
#[ignore]
async fn streaming_subprocess() -> TestResult {
    let ready = std::env::var("BUG249_READY")?;
    let mode = std::env::var("BUG249_MODE")?;
    let (directory, session, _) = support::fixture()?;
    let _admission =
        codelet_tools::tool_admission::register_auxiliary(session.id, directory.path().into())?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    assert!(support::read(session.id)?.is_empty());
    let mut events = session.subscribe_to_stream();
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::ResponseAttemptStarted);
    output.emit(StreamEvent::Text("acknowledged prefix α".into()));
    let observed = events.try_recv()?;
    assert!(matches!(
        observed,
        codelet_rpc_types::StreamChunk::ResponseAttemptStarted
    ));
    assert!(matches!(
        events.try_recv()?,
        codelet_rpc_types::StreamChunk::Text { .. }
    ));
    if mode == "attempt" {
        output.emit(StreamEvent::ResponseAttemptAborted);
        output.emit(StreamEvent::ResponseAttemptStarted);
        output.emit(StreamEvent::Text("authentic replacement α".into()));
    }
    if mode == "requested" {
        output.emit(StreamEvent::ToolCall(
            codelet_cli::interactive::ToolCallEvent {
                id: "sentinel".into(),
                call_id: Some("native-sentinel".into()),
                name: "KillSentinel".into(),
                args: json!({}),
            },
        ));
    }
    if mode == "tool" {
        let agent = rig::agent::AgentBuilder::new(codelet_providers::stub_model::StubModel::new())
            .additional_params(
                json!({"stub_tool_call":{"id":"sentinel", "name":"KillSentinel",
                "arguments":{"work_unit_id":"NONE"}}}),
            )
            .tool(KillSentinel {
                root: directory.path().into(),
                ready: ready.into(),
                id: session.id,
            })
            .build();
        run_agent_stream(
            codelet_core::RigAgent::with_default_depth(agent),
            "write once",
            &mut *session.inner.lock().await,
            session.is_interrupted.clone(),
            Arc::new(AtomicBool::new(false)),
            session.interrupt_notify.clone(),
            &output,
            session.id,
        )
        .await?;
        return Err("sentinel tool unexpectedly completed".into());
    }
    let records = JournalStore::new(directory.path(), session.id).read()?;
    std::fs::write(
        ready,
        serde_json::to_vec(&json!({"root":directory.path(),"id":session.id,"records":records}))?,
    )?;
    std::future::pending::<()>().await;
    Ok(())
}
