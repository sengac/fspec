//! Feature: spec/features/lossless-transcript-tui-hydration.feature
//! Feature: spec/features/lossless-transcript-delivery.feature

#[path = "bug244/support.rs"]
mod support;

#[path = "bug244/history_fixture.rs"]
mod history_fixture;

#[path = "bug244/app_support.rs"]
mod app_support;

use app_support::{drain, rendered};
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{SessionId, StreamChunk};
use support::{Fixture, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn reopened_tabs_render_the_complete_transcript_without_duplicate_tool_cards() -> TestResult {
    // @step Given a real saved long transcript with a complete tool result
    let fixture = Fixture::new().await?;
    fixture.seed()?;
    fixture.cold_restore()?;
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        let mut app = App::new(backend);
        // @step When the TUI resumes it twice and reopens its tab using either transport
        for _repeat in 0..2 {
            app.dispatch(Action::AttachToSession(fixture.id.clone()));
            drain(&mut app).await;
        }
        app.dispatch(Action::SessionRemoved(fixture.id.clone()));
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        drain(&mut app).await;
        // @step Then its rendered original text and full tool result remain complete and occur once
        let context = app
            .agent_view_store()
            .session_context_for(&fixture.id)
            .ok_or("missing context")?;
        let rendered = context
            .scrollback
            .chunks()
            .iter()
            .filter_map(|chunk| context.scrollback.full_text_for_seq(chunk.seq))
            .collect::<Vec<_>>()
            .join("\n");
        for index in 0..2600 {
            assert_eq!(
                rendered.matches(&format!("original-{index:04}")).count(),
                1,
                "original {index} missing or duplicated"
            );
        }
        assert_eq!(rendered.matches("complete original tool result").count(), 1);
        // @step And original timestamps remain associated with the rendered turns
        let original = codelet_core::persistence::get_session_archival_page(
            uuid::Uuid::parse_str(&fixture.id.value)?,
            None,
            1024,
        )?
        .fragment
        .ok_or("missing original record")?;
        assert!(context
            .transcript_originals
            .values()
            .flatten()
            .any(
                |coordinates| coordinates.message_id == original.message_id.to_string()
                    && coordinates.timestamp == original.timestamp.to_rfc3339()
            ));
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn resuming_a_previously_restored_active_session_preserves_runtime_state() -> TestResult {
    use codelet_core::SessionManagerHandle;

    // @step Given a cold restored session with new live model messages and token counters
    let fixture = Fixture::new().await?;
    fixture.seed()?;
    fixture.cold_restore()?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let model_before = {
        let mut inner = background.inner.lock().await;
        inner.messages.push(rig::message::Message::assistant(
            "unpersisted post-restore work",
        ));
        inner.token_tracker.input_tokens = 317;
        inner.token_tracker.cumulative_billed_output = 29;
        inner.messages.clone()
    };
    background.handle_output(StreamChunk::text("unpersisted post-restore work".into()));
    let transcript_before = serde_json::to_value(background.get_buffered_output(usize::MAX))?;
    // @step When the active session is manually resumed again
    fixture.manager.resume_session(&fixture.id)?;
    // @step Then its live model messages counters and transcript remain unchanged
    let inner = background.inner.lock().await;
    assert_eq!(inner.messages, model_before);
    assert_eq!(inner.token_tracker.input_tokens, 317);
    assert_eq!(inner.token_tracker.cumulative_billed_output, 29);
    drop(inner);
    assert_eq!(
        serde_json::to_value(background.get_buffered_output(usize::MAX))?,
        transcript_before
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn lost_live_wakeups_recover_without_duplicating_the_displayed_transcript() -> TestResult {
    // @step Given an open hydrated session with original text
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("original-before-poll".into()));
    background.handle_output(StreamChunk::done());
    let (embedded, websocket, server) = fixture.backends().await?;
    for (index, backend) in [embedded, websocket].into_iter().enumerate() {
        let mut app = App::new(backend);
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        drain(&mut app).await;
        // @step When live output arrives without a notification and the transcript is polled
        let new_text = format!("new-after-lost-wakeup-{index}");
        let live = StreamChunk::text(new_text.clone());
        background.handle_output(live.clone());
        background.handle_output(StreamChunk::done());
        app.dispatch(Action::Transcript(
            codelet_fspec_tui::app::transcript_sync::TranscriptAction::Poll,
        ));
        drain(&mut app).await;
        // @step And the same live notification is received twice
        for _repeat in 0..2 {
            app.dispatch(Action::Transcript(
                codelet_fspec_tui::app::transcript_sync::TranscriptAction::Wake(fixture.id.clone()),
            ));
        }
        drain(&mut app).await;
        let text = rendered(&app, &fixture.id)?;
        // @step Then the original and new text each render exactly once
        assert_eq!(text.matches("original-before-poll").count(), 1);
        assert_eq!(text.matches(&new_text).count(), 1);
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn failed_hydration_preserves_displayed_content_and_exposes_retry_state() -> TestResult {
    use codelet_core::SessionManagerHandle;

    // @step Given an open hydrated session with original text
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("preserved-before-failure".into()));
    background.handle_output(StreamChunk::done());
    let (embedded, websocket, server) = fixture.backends().await?;
    let mut apps = [App::new(embedded), App::new(websocket)];
    for app in &mut apps {
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        drain(app).await;
    }
    // @step When its next transcript page fails
    SessionManagerHandle::destroy_session(fixture.manager.as_ref(), &fixture.id)?;
    for app in &mut apps {
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        drain(app).await;
        let text = rendered(app, &fixture.id)?;
        // @step Then the original text remains visible with a session scoped incomplete transcript notice
        assert_eq!(text.matches("preserved-before-failure").count(), 1);
        assert!(text.contains("Transcript incomplete"), "{text}");
        assert!(text.contains("retry"), "{text}");
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn late_hydration_cannot_corrupt_a_reopened_tab_or_another_session() -> TestResult {
    // @step Given a real session whose hydration response has not reached the TUI
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("first-session-only".into()));
    background.handle_output(StreamChunk::done());
    let other_id = SessionId::new(
        fixture
            .manager
            .create_session(
                "anthropic/claude-sonnet-4",
                &fixture.directory.path().to_string_lossy(),
            )
            .await?,
    );
    let other = fixture.manager.get_session(&other_id.value)?;
    other.handle_output(StreamChunk::text("other-session-only".into()));
    other.handle_output(StreamChunk::done());
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        let mut app = App::new(backend);
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        while let Some(task) = app.next_pending_task() {
            let _ = task.await;
        }
        let mut delayed = Vec::new();
        while let Some(action) = app.try_recv_action() {
            delayed.push(action);
        }
        // @step When its tab is closed and reopened before that response is applied
        app.dispatch(Action::SessionRemoved(fixture.id.clone()));
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        app.dispatch(Action::ReattachSession(other_id.clone()));
        for action in delayed {
            app.dispatch(action);
        }
        drain(&mut app).await;
        // @step Then only the current tab transcript renders once and no other session receives its chunks
        let first = rendered(&app, &fixture.id)?;
        let second = rendered(&app, &other_id)?;
        assert_eq!(first.matches("first-session-only").count(), 1);
        assert!(!first.contains("other-session-only"));
        assert_eq!(second.matches("other-session-only").count(), 1);
        assert!(!second.contains("first-session-only"));
    }
    server.abort();
    Ok(())
}
