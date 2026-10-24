//! Feature: spec/features/streaming-loop-abort-retry-regression.feature

#![cfg(feature = "test-support")]

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use codelet_agent_loop::{background_output::LoopDetectionWiring, FspecAgentHooks};
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_sessions::session_manager::SessionManager;

struct StreamHook;

impl Drop for StreamHook {
    fn drop(&mut self) {
        codelet_providers::stub_model::clear_looping_stream_hook();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_genuine_sustained_loops_still_cancel_persist_recover_and_stop_retrying(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a stub provider streaming a normal prefix followed by sustained complete word repetition
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_core::persistence::reset_stores_for_tests();
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_providers::stub_provider::register_stub_provider();
    let prefix: Vec<_> = (0..20).map(|index| format!("prefix{index} ")).collect();
    let mut words = prefix.clone();
    words.extend((0..300).map(|index| format!("{} ", ["alpha", "beta", "gamma"][index % 3])));
    let stream_length = words.len();
    let poll = Arc::new(AtomicUsize::new(0));
    codelet_providers::stub_model::set_looping_stream_hook(words, poll.clone());
    let _hook = StreamHook;
    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));
    manager.set_default_model("stub/canned");
    let id = uuid::Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &id,
            "stub/canned",
            directory
                .path()
                .to_str()
                .ok_or("project path is not UTF-8")?,
            "bug216-retries",
        )
        .await?;
    let session = manager.get_session(&id)?;
    let mut receiver = manager.chunks_tx().subscribe();
    let attempts = LoopDetectionWiring::default().max_retries as usize + 1;

    // @step When the production agent loop processes the input and exhausts its retry cap
    session.send_input("explain the architecture".into(), None)?;
    let mut completions = 0;
    let mut attempt_lengths = Vec::new();
    let mut text_deltas = 0;
    tokio::time::timeout(Duration::from_secs(60), async {
        while completions < attempts {
            let (received_id, chunk) = receiver.recv().await?;
            if received_id.value == id {
                match chunk {
                    StreamChunk::Text { .. } => text_deltas += 1,
                    StreamChunk::Done => {
                        completions += 1;
                        attempt_lengths.push(text_deltas);
                        text_deltas = 0;
                    }
                    _ => {}
                }
            }
        }
        while session.get_status() != SessionStatus::Idle || session.has_pending_loop_abort_note() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;

    // @step Then every attempt is cancelled before consuming the full provider stream
    let total_polled = poll.load(Ordering::Acquire);
    assert_eq!(completions, attempts);
    assert!(total_polled < attempts * stream_length);
    assert!(
        total_polled >= attempts * (prefix.len() + 60),
        "each abort requires two independent complete evidence windows: {total_polled}"
    );
    assert!(attempt_lengths
        .iter()
        .all(|&length| length >= prefix.len() + 60 && length < stream_length));

    // @step And persisted assistant output retains the prefix and loop abort marker
    let manifest = codelet_core::persistence::load_session(session.id)?;
    let messages = codelet_core::persistence::get_session_messages(&manifest)?;
    let assistants: Vec<_> = messages
        .iter()
        .filter(|message| message.role == "assistant")
        .collect();
    assert_eq!(assistants.len(), attempts);
    for message in assistants {
        let text = message
            .content
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("prefix0 prefix1 prefix2"));
        assert!(message
            .content
            .contains("Response cut off: repetitive output detected"));
        assert!(!text.contains(&"alpha beta gamma ".repeat(30)));
    }

    // @step And retry context contains a corrective note and retries are bounded
    let history = codelet_providers::stub_model::last_request_history().join("\n");
    assert!(history.contains("previous response was cut off"));
    assert!(history.contains("fresh approach"));
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(poll.load(Ordering::Acquire), total_polled);
    assert!(!session.has_pending_loop_abort_note());
    Ok(())
}
