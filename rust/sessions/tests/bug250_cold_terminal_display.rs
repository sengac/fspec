#![doc = "Feature: spec/features/cold-transcript-terminal-replay.feature"]

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_cli::session::system_reminders::is_system_reminder;
use codelet_core::persistence::transcript::{JournalEvent, JournalStore};
use codelet_rpc_types::StreamChunk;
use rig::message::Message;
use support::TestResult;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn cold_session_display_includes_one_original_safe_failure_after_its_prefix() -> TestResult {
    // @step Given an assistant prefix followed by a durably archived provider failure
    let (_directory, manager, id, _) = support::manager().await?;
    let session = manager.get_session(&id.value)?;
    let output = BackgroundOutput::with_provider(session.clone(), "anthropic".into(), None);
    output.emit(StreamEvent::Text("cold authentic prefix".into()));
    output.emit(StreamEvent::Error(
        "Bearer sk-secret private_prompt raw_provider".into(),
    ));
    let terminal = JournalStore::configured(session.id)?
        .read()?
        .into_iter()
        .find(|record| matches!(record.event, JournalEvent::Terminal { .. }))
        .ok_or("terminal was not archived")?;
    for _ in 0..2 {
        // @step When the session is cold restored twice
        let restored = support::cold_resume(&id)?;
        let session = restored.get_session(&id.value)?;
        let chunks = session.get_buffered_output(100);
        // @step Then each display contains the prefix followed by one safe failure with its original identity and timestamp
        let prefix = chunks
            .iter()
            .position(|chunk| {
                matches!(chunk,
            StreamChunk::Text { text, .. } if text == "cold authentic prefix")
            })
            .ok_or("missing cold prefix")?;
        let failures: Vec<_> = chunks
            .iter()
            .enumerate()
            .filter_map(|(index, chunk)| {
                matches!(chunk, StreamChunk::Error { error } if error == "Provider request failed.")
                    .then_some(index)
            })
            .collect();
        assert_eq!(failures.len(), 1);
        assert!(prefix < failures[0]);
        let mut cursor = None;
        let mut snapshot = None;
        let mut found = false;
        loop {
            let page = session.get_transcript_page(cursor, snapshot, 4096)?;
            if let Some(fragment) = &page.fragment {
                if let Some(origin) = &fragment.original {
                    if origin.message_id == terminal.id.to_string() {
                        assert_eq!(origin.timestamp, terminal.timestamp.to_rfc3339());
                        found = true;
                    }
                }
            }
            if page.complete {
                break;
            }
            snapshot = Some(page.snapshot_end);
            cursor = Some(page.next_cursor);
        }
        assert!(
            found,
            "terminal metadata must retain original journal identity"
        );
        // @step And restored model history contains no terminal prose
        let inner = session.inner.lock().await;
        let dialogue = inner
            .messages
            .iter()
            .filter(|message| !is_system_reminder(message))
            .collect::<Vec<_>>();
        assert_eq!(dialogue.len(), 1);
        assert_eq!(*dialogue[0], Message::assistant("cold authentic prefix"));
        let model = serde_json::to_string(&inner.messages)?;
        assert!(!model.contains("Provider request failed"));
        assert!(!model.contains("sk-secret"));
    }
    Ok(())
}
