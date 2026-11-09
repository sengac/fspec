use super::{
    fixture::{Harness, TestResult},
    retention,
};
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_provider_setup_and_pre_prompt_compaction_failure_cannot_split_admitted_history(
) -> TestResult {
    // @step Given an admitted input waiting for its first provider request
    let mut harness = Harness::new()?;
    harness.seed().await?;
    assert!(!std::path::Path::new(&std::env::var("CODEX_HOME")?)
        .join("auth.json")
        .exists());
    let mut events = harness.session.subscribe_to_stream();
    harness
        .session
        .send_input("accepted before provider setup".into(), None)?;
    // @step When provider construction or pre-prompt compaction fails
    harness.start()?;
    let completion = retention::done(&mut events, 1).await;
    harness.stop().await;
    completion?;
    let chunks = harness.session.get_buffered_output(128);
    assert!(
        chunks
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::Error { .. })),
        "provider setup did not fail as arranged"
    );
    assert_eq!(
        harness
            .requests
            .as_ref()
            .ok_or("requests")?
            .requests
            .lock()
            .map_err(|_| "requests")?
            .len(),
        2
    );
    // @step Then runtime and cold replay retain the same admitted original exactly once
    let manifest = load_session(harness.session.id)?;
    let originals = get_session_messages_full(&manifest)?;
    assert_eq!(
        originals
            .iter()
            .filter(|stored| stored.metadata["message"]["content"][0]["text"]
                == "accepted before provider setup")
            .count(),
        1
    );
    // @step And no successful provider completion or tool effect is fabricated
    assert_eq!(
        std::fs::read_to_string(harness.directory.path().join("effect"))?,
        "executed once"
    );
    retention::equal_replay(&harness).await?;
    Ok(())
}
