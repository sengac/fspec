//! Shared receipt fixtures for the submission-receipt recovery scenarios.

use super::fixture::{Harness, TestResult};
use codelet_core::persistence::input_receipts::{self, ExecutionOutcome, InputDisposition};
use codelet_sessions::background_session::BridgeImageData;
use serde_json::{json, Value};
use std::{
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
};

pub type Failure = Box<dyn std::error::Error>;

/// A 1x1 transparent PNG payload.
pub fn image() -> Option<Vec<BridgeImageData>> {
    Some(vec![BridgeImageData {
        data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=".into(),
        media_type: "image/png".into(),
    }])
}

pub struct Process(pub Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Re-run one exact test of this binary as a child that reports via mailbox.
pub fn spawn(test: &str, variable: &str, mailbox: &Path) -> Result<Process, Failure> {
    Ok(Process(
        Command::new(std::env::current_exe()?)
            .args(["--exact", test, "--nocapture"])
            .env(variable, mailbox)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    ))
}

pub async fn mailbox(child: &mut Process, mailbox: &Path) -> Result<Value, Failure> {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        loop {
            if let Ok(bytes) = std::fs::read(mailbox) {
                if let Ok(identity) = serde_json::from_slice::<Value>(&bytes) {
                    return Ok::<_, Failure>(identity);
                }
            }
            if let Some(status) = child.0.try_wait()? {
                return Err(format!("child exited before acknowledgment: {status}").into());
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await?
}

/// Child body: leave one input at each durable boundary, then await death.
pub fn dispositions_child(mailbox: &Path) -> TestResult {
    let harness = Harness::new()?;
    let session = harness.session.id;
    let mut ids = serde_json::Map::new();
    for text in ["pending", "canceled", "admitted", "dispatching"] {
        harness
            .session
            .send_input(format!("{text} original"), None)?;
        let receipt = input_receipts::list(session)?
            .into_iter()
            .find(|receipt| receipt.text() == format!("{text} original"))
            .ok_or("receipt")?;
        match text {
            "canceled" => input_receipts::settle(session, receipt.id, InputDisposition::Cancelled)?,
            "admitted" | "dispatching" => {
                codelet_agent_loop::persist::persist_input_original(
                    &session,
                    receipt.id,
                    receipt.content.clone(),
                )?;
                input_receipts::admit(session, receipt.id)?;
                if text == "dispatching" {
                    input_receipts::record_execution(
                        session,
                        receipt.id,
                        ExecutionOutcome::DispatchStarted,
                    )?;
                }
            }
            _ => {}
        }
        ids.insert(text.into(), json!(receipt.id));
    }
    let bytes =
        serde_json::to_vec(&json!({"root":harness.directory.path(),"session":session,"ids":ids}))?;
    let mut receipt = std::fs::File::create(mailbox)?;
    receipt.write_all(&bytes)?;
    receipt.sync_all()?;
    loop {
        std::thread::park();
    }
}

/// Count conversation-membership occurrences of one submission.
pub fn memberships(session: uuid::Uuid, id: uuid::Uuid) -> Result<usize, Failure> {
    Ok(codelet_core::persistence::load_session(session)?
        .messages
        .iter()
        .filter(|message| message.message_id == id)
        .count())
}
