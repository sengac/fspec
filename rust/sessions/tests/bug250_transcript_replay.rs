#![doc = "Feature: spec/features/transcript-only-replay.feature"]

#[path = "../src/replay_content.rs"]
mod replay_content;
#[path = "../src/replay_media.rs"]
mod replay_media;
#[path = "../src/replay_transcript.rs"]
mod replay_transcript;

use codelet_core::persistence::transcript::{
    JournalEvent, JournalIdentity, JournalRecord, TerminalOutcome,
};
use codelet_rpc_types::{InterruptionReason, StreamChunk};
use serde_json::json;
use uuid::Uuid;

#[test]
fn transcript_replay_remains_display_only_and_preserves_truthful_interruption(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given an archived incomplete prefix and typed terminal outcomes
    let identity = JournalIdentity {
        session_id: Uuid::new_v4(),
        request_id: Uuid::new_v4(),
        attempt_id: Uuid::new_v4(),
        message_id: Uuid::new_v4(),
    };
    for (sequence, event) in [
        JournalEvent::Text {
            text: "authentic incomplete prefix".into(),
        },
        JournalEvent::Terminal {
            outcome: TerminalOutcome::ProviderError,
        },
        JournalEvent::Terminal {
            outcome: TerminalOutcome::UserCancelled,
        },
        JournalEvent::Terminal {
            outcome: TerminalOutcome::LoopInterrupted,
        },
        JournalEvent::Terminal {
            outcome: TerminalOutcome::CompactionFailed,
        },
        JournalEvent::ToolStarted {
            id: "unknown-call".into(),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let record = JournalRecord::new(identity.clone(), sequence as u64, event);
        let encoded = json!({"_transcript":record}).to_string();
        // @step When the shared replay decoder reads transcript-only records
        let replay = replay_content::Replay::parse(&[encoded])?;
        // @step Then display chunks retain the prefix and outcome while model messages and envelopes stay empty
        assert!(replay.messages.is_empty());
        assert!(replay.envelopes.is_empty());
        assert_eq!(replay.chunks.len(), 1);
        match sequence {
            0 => assert!(matches!(&replay.chunks[0], StreamChunk::Text { text, .. }
                if text == "authentic incomplete prefix")),
            1 => assert!(matches!(&replay.chunks[0], StreamChunk::Error { error }
                if error == TerminalOutcome::ProviderError.message())),
            2 => assert!(matches!(
                replay.chunks[0],
                StreamChunk::InterruptedWithReason {
                    reason: InterruptionReason::User,
                    ..
                }
            )),
            3 => assert!(matches!(
                replay.chunks[0],
                StreamChunk::InterruptedWithReason {
                    reason: InterruptionReason::LoopDetector,
                    ..
                }
            )),
            _ => assert!(matches!(
                replay.chunks[0],
                StreamChunk::UserNotification { .. }
            )),
        }
    }
    Ok(())
}
