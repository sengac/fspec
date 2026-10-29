use super::fixture::{Harness, TestResult};
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;
use std::time::Duration;
use tokio::sync::broadcast;

pub async fn done(events: &mut broadcast::Receiver<StreamChunk>, expected: usize) -> TestResult {
    tokio::time::timeout(Duration::from_secs(10), async {
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
    tokio::time::timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    Ok(())
}

pub async fn retained(harness: &Harness, texts: &[String]) -> TestResult {
    let manifest = load_session(harness.session.id)?;
    let originals = get_session_messages_full(&manifest)?;
    for text in texts {
        assert_eq!(
            originals
                .iter()
                .filter(|stored| { stored.metadata["message"]["content"][0]["text"] == *text })
                .count(),
            1,
            "canceled input must remain recoverable exactly once"
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
        codelet_cli::session::replay::same_replay_messages(&replay.messages, &inner.messages),
        "cold replay differs from the admitted runtime conversation"
    );
    Ok(())
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
