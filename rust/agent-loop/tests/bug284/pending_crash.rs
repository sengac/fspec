use super::fixture::{Harness, TestResult};
use codelet_core::persistence::*;
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_process_death_after_queue_acknowledgment_retains_the_pending_submission(
) -> TestResult {
    if let Some(mailbox) = std::env::var_os("BUG284_PENDING_CHILD") {
        let harness = Harness::new()?;
        harness
            .session
            .send_input("acknowledged pending original".into(), None)?;
        let bytes = serde_json::to_vec(
            &json!({"root":harness.directory.path(),"session":harness.session.id}),
        )?;
        let mut receipt = std::fs::File::create(mailbox)?;
        receipt.write_all(&bytes)?;
        receipt.sync_all()?;
        loop {
            std::thread::park();
        }
    }
    // @step Given a pending submission was acknowledged before the task loop dequeued it
    let directory = codelet_test_helpers::setup_temp_directory()?;
    let mailbox = directory.path().join("pending-child.json");
    let mut child = Process(Command::new(std::env::current_exe()?)
        .args(["--exact", "pending_crash::scenario_process_death_after_queue_acknowledgment_retains_the_pending_submission", "--nocapture"])
        .env("BUG284_PENDING_CHILD", &mailbox)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if mailbox.exists() {
                if let Ok(bytes) = std::fs::read(&mailbox) {
                    if let Ok(identity) = serde_json::from_slice::<Value>(&bytes) {
                        return Ok::<_, Box<dyn std::error::Error>>(identity);
                    }
                }
            }
            if let Some(status) = child.0.try_wait()? {
                return Err(format!("pending child exited before acknowledgment: {status}").into());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?
    .and_then(|identity| {
        // @step When the submitting process is killed and persisted state is reopened
        child.0.kill()?;
        let status = child.0.wait()?;
        assert!(!status.success());
        let root = std::path::PathBuf::from(identity["root"].as_str().ok_or("child root")?);
        codelet_common::set_data_directory(root.clone())?;
        reset_stores_for_tests();
        let id = serde_json::from_value(identity["session"].clone())?;
        let originals = get_session_messages_full(&load_session(id)?)?;
        let replay = crate::replay_content::Replay::parse(&get_session_message_envelopes(id)?)?;
        reset_stores_for_tests();
        std::fs::remove_dir_all(root)?;
        // @step Then the pending original remains recoverable exactly once
        assert_eq!(
            originals
                .iter()
                .filter(
                    |original| original.metadata["message"]["content"][0]["text"]
                        == "acknowledged pending original"
                )
                .count(),
            1,
            "acknowledged input was lost when its process died before dequeue"
        );
        // @step And it is absent from conversational replay and cannot execute automatically
        assert!(replay.messages.is_empty());
        Ok(())
    })
}
