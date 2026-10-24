#![doc = "Feature: spec/features/transcript-request-anchors.feature"]

use codelet_agent_loop::{persist::persist_user_message, BackgroundOutput};
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_core::persistence::{get_session_archival_page, get_session_message_envelopes};
use serde_json::Value;

#[path = "support/bug246.rs"]
mod support;
use support::{fixture, read, TestResult};

#[tokio::test]
#[serial_test::serial]
async fn consecutive_empty_failures_retain_their_causal_archive_order() -> TestResult {
    // @step Given two requests failing before assistant output followed by a successful response
    let (directory, session, _) = fixture()?;
    std::fs::remove_dir(directory.path().join("messages/messages.jsonl"))?;
    for input in ["first request", "second request"] {
        persist_user_message(&session.id, input)?;
        let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
        output.emit(StreamEvent::Error("private provider failure".into()));
    }
    persist_user_message(&session.id, "third request")?;
    let output = BackgroundOutput::with_provider(session.clone(), "codex".into(), None);
    output.emit(StreamEvent::Text("successful response".into()));
    output.emit(StreamEvent::Done(Some("end_turn".into())));
    // @step When their canonical archival pages are repeatedly reassembled
    for _ in 0..2 {
        codelet_core::persistence::reset_stores_for_tests();
        let mut cursor = None;
        let mut encoded = String::new();
        let mut order = Vec::new();
        loop {
            let page = get_session_archival_page(session.id, cursor, 4096)?;
            assert!(serde_json::to_vec(&page)?.len() <= 4096);
            if let Some(fragment) = page.fragment {
                assert_eq!(fragment.byte_offset, encoded.len());
                encoded.push_str(&fragment.json_fragment);
                if encoded.len() == fragment.total_bytes {
                    let value: Value = serde_json::from_str(&encoded)?;
                    if let Some(outcome) = value["_transcript"]["event"]["outcome"].as_str() {
                        order.push(outcome.to_owned());
                    } else {
                        order.push(
                            value["message"]["content"][0]["text"]
                                .as_str()
                                .ok_or("unexpected public archive record")?
                                .to_owned(),
                        );
                    }
                    encoded.clear();
                }
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        // @step Then each safe failure follows its user input before the next request
        assert_eq!(
            order,
            [
                "first request",
                "provider_error",
                "second request",
                "provider_error",
                "third request",
                "successful response",
                "completed"
            ]
        );
        // @step And the successful original appears once without transcript metadata in model history
        let originals = read(session.id)?;
        assert_eq!(originals.len(), 4);
        let model = get_session_message_envelopes(session.id)?.join("");
        assert_eq!(model.matches("successful response").count(), 1);
        assert!(!model.contains("_transcript"));
        assert!(!model.contains("private provider failure"));
    }
    Ok(())
}
