use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput, ToolCallEvent, ToolResultEvent};
use codelet_core::{persistence::messages::StoredMessage, RigAgent};
use codelet_sessions::background_session::BackgroundSession;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{json, Value};
use std::{io::Write, path::PathBuf, sync::Arc, time::Duration};

#[path = "bug246.rs"]
pub mod recovery;
pub use recovery::{read, TestResult};

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub session: Arc<BackgroundSession>,
    pub output: Arc<BackgroundOutput>,
}

impl Fixture {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (directory, session, _) = recovery::fixture()?;
        std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
        session.set_status(codelet_rpc_types::SessionStatus::Running);
        codelet_agent_loop::persist::persist_user_message(&session.id, "original input")?;
        let output = Arc::new(BackgroundOutput::with_provider(
            session.clone(),
            "stub".into(),
            None,
        ));
        Ok(Self {
            directory,
            session,
            output,
        })
    }

    pub fn result(&self, id: &str, error: bool) {
        self.output.emit(StreamEvent::ToolCall(ToolCallEvent {
            id: id.into(),
            call_id: Some(format!("call-{id}")),
            name: "inspect".into(),
            args: json!({}),
        }));
        self.output.emit(StreamEvent::ToolResult(ToolResultEvent {
            id: id.into(),
            call_id: Some(format!("call-{id}")),
            content: format!("authentic-{id}"),
            is_error: error,
        }));
    }
}

pub struct Gate {
    pub held: tokio::sync::Notify,
    pub release: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>,
    pub worker: std::sync::Mutex<Option<std::thread::JoinHandle<bool>>>,
}

impl Gate {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            held: tokio::sync::Notify::new(),
            release: Default::default(),
            worker: Default::default(),
        })
    }

    pub fn finish(&self) -> Result<bool, Box<dyn std::error::Error>> {
        if let Some(release) = self.release.lock().expect("gate release lock").take() {
            let _ = release.send(());
        }
        Ok(self
            .worker
            .lock()
            .expect("gate worker lock")
            .take()
            .ok_or("gate never started")?
            .join()
            .map_err(|_| "gate worker failed")?)
    }
}

struct InterruptTool {
    session: Arc<BackgroundSession>,
    root: PathBuf,
    gate: Option<Arc<Gate>>,
    fail: bool,
}

impl Tool for InterruptTool {
    const NAME: &'static str = "SettlementSentinel";
    type Args = Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Synthetic interruption".into(),
            parameters: json!({"type":"object"}),
        }
    }

    fn cancel_on_abandon(&self) -> bool {
        true
    }

    async fn call(&self, _: Value) -> Result<String, Self::Error> {
        if let Some(gate) = &self.gate {
            let lock = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(self.root.join("messages.lock"))?;
            lock.lock()?;
            let (sender, receiver) = std::sync::mpsc::channel();
            *gate.release.lock().expect("gate release lock") = Some(sender);
            *gate.worker.lock().expect("gate worker lock") = Some(std::thread::spawn(move || {
                let acknowledged = receiver.recv_timeout(Duration::from_secs(2)).is_ok();
                drop(lock);
                acknowledged
            }));
            gate.held.notify_one();
        }
        if self.fail {
            let archive = self.root.join("messages/messages.jsonl");
            std::fs::rename(&archive, self.root.join("messages/saved.jsonl"))?;
            std::fs::create_dir(archive)?;
        }
        self.session.interrupt();
        std::future::pending().await
    }
}

pub async fn run(fixture: &Fixture, gate: Option<Arc<Gate>>, fail: bool) -> TestResult {
    let _admission = codelet_tools::tool_admission::register_auxiliary(
        fixture.session.id,
        fixture.directory.path().into(),
    )?;
    let agent = rig::agent::AgentBuilder::new(codelet_providers::stub_model::StubModel::new())
        .additional_params(json!({"stub_tool_call":{"id":"running","name":InterruptTool::NAME,"arguments":{"work_unit_id":"NONE"}}}))
        .tool(InterruptTool { session: fixture.session.clone(), root: fixture.directory.path().into(), gate, fail }).build();
    codelet_cli::interactive::run_agent_stream(
        RigAgent::with_default_depth(agent),
        "stop",
        &mut *fixture.session.inner.lock().await,
        fixture.session.is_interrupted.clone(),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        fixture.session.interrupt_notify.clone(),
        fixture.output.as_ref(),
        fixture.session.id,
    )
    .await?;
    Ok(())
}

pub fn chunks(session: &BackgroundSession) -> Vec<Value> {
    session
        .get_buffered_output(10000)
        .iter()
        .map(codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value)
        .collect()
}

pub fn index_fixture(
    count: usize,
) -> Result<(tempfile::TempDir, Vec<uuid::Uuid>), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().into())?;
    codelet_core::persistence::reset_stores_for_tests();
    std::fs::create_dir_all(directory.path().join("messages"))?;
    let mut file = std::io::BufWriter::new(std::fs::File::create(
        directory.path().join("messages/messages.jsonl"),
    )?);
    let mut ids = Vec::new();
    for number in 0..count {
        let content = format!("synthetic original {number}");
        let id = uuid::Uuid::new_v4();
        let record = StoredMessage {
            id,
            content_hash: codelet_core::persistence::messages::compute_hash(content.as_bytes()),
            created_at: chrono::Utc::now(),
            role: "user".into(),
            content,
            token_count: None,
            blob_refs: vec![],
            metadata: Default::default(),
        };
        serde_json::to_writer(&mut file, &record)?;
        file.write_all(b"\n")?;
        ids.push(id);
    }
    file.flush()?;
    Ok((directory, ids))
}

pub fn cancelled_promotion() -> TestResult {
    use codelet_common::read_work::{observe, ReadCancellation};
    use std::sync::atomic::{AtomicUsize, Ordering};
    for cancelled_boundary in [2, 3] {
        let (directory, ids) = index_fixture(64)?;
        let token = ReadCancellation::default();
        let cancelled = token.clone();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let (result, _) = token.run(|| {
            observe(
                Arc::new(move |boundary| {
                    if boundary == "index.publish"
                        && observed.fetch_add(1, Ordering::AcqRel) == cancelled_boundary
                    {
                        cancelled.cancel();
                    }
                }),
                codelet_core::persistence::messages::MessageStore::new,
            )
        });
        assert!(
            result.is_err(),
            "cancellation before promotion boundary {cancelled_boundary} was bypassed"
        );
        assert!(!directory.path().join("messages/messages.idx").exists());
        let cold = codelet_core::persistence::messages::MessageStore::new()?;
        for id in ids {
            assert_eq!(cold.get(id).ok_or("lost original")?.id, id);
        }
    }
    Ok(())
}
