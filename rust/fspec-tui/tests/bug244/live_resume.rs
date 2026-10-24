#![doc = "Feature: spec/features/active-session-first-resume.feature"]

use crate::support::{Fixture, TestResult};
use codelet_core::{persistence, SessionManagerHandle};
use codelet_rpc_types::{StreamChunk, TranscriptAssembler};
use rig::message::Message;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn first_resume_of_an_active_session_preserves_runtime_sources() -> TestResult {
    // @step Given an active real session whose persisted originals already exist in its runtime journal and model history
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let mut manifest = persistence::load_session(background.id)?;
    for (role, text) in [
        ("user", "intentional repeated message"),
        ("assistant", "runtime response"),
        ("user", "intentional repeated message"),
    ] {
        persistence::append_message_with_metadata(
            &mut manifest,
            role,
            text,
            std::collections::HashMap::from([(
                "message".into(),
                serde_json::json!({"role":role,"content":text}),
            )]),
        )?;
    }
    let (messages, counters) = {
        let mut inner = background.inner.lock().await;
        inner.messages.extend([
            Message::user("intentional repeated message"),
            Message::assistant("runtime response"),
            Message::user("intentional repeated message"),
            Message::assistant("unpersisted runtime tail"),
        ]);
        inner.token_tracker.input_tokens = 317;
        inner.token_tracker.output_tokens = 19;
        inner.token_tracker.cumulative_billed_input = 431;
        inner.token_tracker.cumulative_billed_output = 29;
        (
            inner.messages.clone(),
            (
                inner.token_tracker.input_tokens,
                inner.token_tracker.output_tokens,
                inner.token_tracker.cumulative_billed_input,
                inner.token_tracker.cumulative_billed_output,
            ),
        )
    };
    for chunk in [
        StreamChunk::UserInput {
            text: "intentional repeated message".into(),
        },
        StreamChunk::text("runtime response".into()),
        StreamChunk::done(),
        StreamChunk::UserInput {
            text: "intentional repeated message".into(),
        },
        StreamChunk::text("unpersisted runtime tail".into()),
    ] {
        background.handle_output(chunk);
    }
    let expected = serde_json::to_value(background.get_buffered_output(usize::MAX))?;
    let committed = serde_json::to_value(persistence::load_session(background.id)?)?;
    let mut assembler = TranscriptAssembler::default();
    loop {
        let page = background.get_transcript_page(assembler.cursor().cloned(), None, 1024)?;
        assembler.accept(&page)?;
        if page.complete {
            break;
        }
    }
    let cursor = assembler.cursor().cloned().ok_or("saved live cursor")?;
    // @step When the session is manually resumed for the first time while a tab holds its cursor
    fixture.manager.resume_session(&fixture.id)?;
    // @step Then the runtime generation transcript order model messages and counters remain unchanged
    let page = background.get_transcript_page(Some(cursor.clone()), None, 1024)?;
    assert!(page.complete);
    assert!(page.fragment.is_none());
    assert_eq!(page.next_cursor, cursor);
    assert_eq!(
        serde_json::to_value(background.get_buffered_output(usize::MAX))?,
        expected
    );
    let inner = background.inner.lock().await;
    assert_eq!(inner.messages, messages);
    assert_eq!(
        (
            inner.token_tracker.input_tokens,
            inner.token_tracker.output_tokens,
            inner.token_tracker.cumulative_billed_input,
            inner.token_tracker.cumulative_billed_output,
        ),
        counters
    );
    assert!(inner.restored_replay.is_none());
    drop(inner);
    assert_eq!(
        serde_json::to_value(persistence::load_session(background.id)?)?,
        committed
    );
    // @step And intentional identical messages remain separate events and the saved cursor continues with new output
    assert_eq!(
        background
            .get_buffered_output(usize::MAX)
            .iter()
            .filter(|chunk| matches!(chunk, StreamChunk::UserInput { text } if text == "intentional repeated message"))
            .count(),
        2
    );
    background.handle_output(StreamChunk::text("next runtime output".into()));
    let page = background.get_transcript_page(Some(cursor), None, 1024)?;
    let chunk = assembler.accept(&page)?.ok_or("new runtime event")?;
    assert!(matches!(chunk, StreamChunk::Text { text, .. } if text == "next runtime output"));
    Ok(())
}
