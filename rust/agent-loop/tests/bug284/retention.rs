use super::fixture::{Harness, TestResult};
use codelet_core::persistence::input_receipts::{self, InputDisposition};
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;
use tokio::sync::broadcast;

pub async fn done(events: &mut broadcast::Receiver<StreamChunk>, expected: usize) -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        let mut count = 0;
        while count < expected {
            if matches!(events.recv().await?, StreamChunk::Done) {
                count += 1;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await?
}

pub async fn wait_for(path: &std::path::Path) -> TestResult {
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        while !path.exists() {
            codelet_test_helpers::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

pub async fn retained(harness: &Harness, texts: &[String]) -> TestResult {
    let manifest = load_session(harness.session.id)?;
    let conversation = get_session_messages_full(&manifest)?;
    let receipts = input_receipts::list(harness.session.id)?;
    for text in texts {
        let matching: Vec<_> = receipts
            .iter()
            .filter(|receipt| receipt.text() == *text)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "canceled input must remain recoverable exactly once"
        );
        assert_eq!(matching[0].disposition, InputDisposition::Cancelled);
        assert!(
            !conversation
                .iter()
                .any(|stored| stored.metadata["message"]["content"][0]["text"] == *text),
            "canceled input joined the admitted conversation"
        );
    }
    let runtime = serde_json::to_string(&harness.session.inner.lock().await.messages)?;
    for text in texts {
        assert!(!runtime.contains(text), "canceled input reached runtime");
    }
    assert_eq!(
        std::fs::read_to_string(harness.directory.path().join("effect"))?,
        "executed once"
    );
    assert_eq!(
        harness
            .requests
            .as_ref()
            .ok_or("seed requests")?
            .requests
            .lock()
            .map_err(|_| "requests")?
            .len(),
        2
    );
    assert!(!harness
        .session
        .get_buffered_output(128)
        .iter()
        .any(|chunk| matches!(chunk, StreamChunk::Error { .. })));
    Ok(())
}

pub async fn equal_replay(harness: &Harness) -> TestResult {
    let envelopes = get_session_message_envelopes(harness.session.id)?;
    let replay = crate::replay_content::Replay::parse(&envelopes)?;
    let inner = harness.session.inner.lock().await;
    assert!(
        codelet_cli::session::replay::same_replay_messages(
            &replay.messages,
            &conversation(&inner.messages)
        ),
        "cold replay differs from the admitted runtime conversation\ncold: {}\nruntime: {}",
        serde_json::to_string(&replay.messages)?,
        serde_json::to_string(&inner.messages)?
    );
    Ok(())
}

/// Runtime conversation without the session's current system reminders,
/// which are turn decoration rather than admitted conversation.
pub fn conversation(runtime: &[rig::message::Message]) -> Vec<rig::message::Message> {
    codelet_cli::session::system_reminders::partition_for_compaction(runtime).1
}

pub fn hook(harness: &mut Harness) -> TestResult {
    use codelet_core::lifecycle_hooks::{CompiledHookDefinition, CompiledLifecycleHooks};
    let root = harness.directory.path();
    let hooks = CompiledLifecycleHooks {
        global_timeout: 10,
        global_shell: None,
        session_start: Vec::new(),
        session_end: Vec::new(),
        user_prompt_submit: vec![CompiledHookDefinition {
            name: "admission barrier".into(),
            command: format!(
                "touch '{}/entered'; while [ ! -e '{}/release' ]; do sleep 0.01; done",
                root.display(),
                root.display()
            ),
            blocking: true,
            timeout: 10,
        }],
        notification: Vec::new(),
        pre_tool_use: Vec::new(),
        post_tool_use: Vec::new(),
    };
    std::sync::Arc::get_mut(&mut harness.session)
        .ok_or("session already shared")?
        .lifecycle_hooks = Some(std::sync::Arc::new(hooks));
    Ok(())
}

/// Drive a queued input until its admission owns publication; the owned
/// admission worker waits until the returned sender releases it.
pub async fn hold_admission(
    harness: &Harness,
    text: &str,
) -> Result<(uuid::Uuid, std::sync::mpsc::Sender<()>), Box<dyn std::error::Error>> {
    let session = harness.session.clone();
    let release = codelet_agent_loop::input_admission::hold_next_admission(session.id);
    session.send_input(text.into(), None)?;
    let submission = input_receipts::list(session.id)?
        .into_iter()
        .find(|receipt| receipt.text() == text)
        .ok_or("acknowledged submission has no receipt")?
        .id;
    codelet_test_helpers::deadlock_guard("the awaited block", async {
        while codelet_agent_loop::input_admission::admitting_submission(session.id)
            != Some(submission)
        {
            codelet_test_helpers::yield_now().await;
        }
    })
    .await?;
    Ok((submission, release))
}
