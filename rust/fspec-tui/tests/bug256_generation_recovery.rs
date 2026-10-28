#![doc = "Feature: spec/features/transcript-generation-recovery.feature"]

#[path = "bug244/app_support.rs"]
mod app_support;
#[path = "bug244/support.rs"]
pub mod support;

use app_support::{drain, rendered};
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::{
    app::transcript_sync::TranscriptAction, Action, App, EmbeddedFspecBackend,
};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::StreamChunk;
use std::sync::Arc;
use support::{Fixture, TestResult};

fn app(fixture: &Fixture) -> Result<App, Box<dyn std::error::Error>> {
    let watcher = Arc::new(WorkUnitsWatcher::new(fixture.directory.path())?);
    let service = Arc::new(
        SharedFspecService::with_session_manager(watcher, fixture.manager.clone())
            .with_cwd(fixture.directory.path().into()),
    );
    Ok(App::new(Arc::new(EmbeddedFspecBackend::new(
        tokio::runtime::Handle::current(),
        service,
    ))))
}

async fn queued_page(app: &mut App) -> Result<Action, Box<dyn std::error::Error>> {
    while let Some(task) = app.next_pending_task() {
        let _ = task.await;
    }
    while let Some(action) = app.try_recv_action() {
        if matches!(action, Action::Transcript(TranscriptAction::Loaded { .. })) {
            return Ok(action);
        }
        app.dispatch(action);
    }
    Err("expected a real queued transcript page".into())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn resume_recovers_a_missed_server_clear_without_reopening() -> TestResult {
    // @step Given an open tab displaying content from a previous server generation
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("old-generation-only".into()));
    background.handle_output(StreamChunk::done());
    let mut app = app(&fixture)?;
    app.dispatch(Action::ReattachSession(fixture.id.clone()));
    drain(&mut app).await;
    assert!(rendered(&app, &fixture.id)?.contains("old-generation-only"));
    // @step When the server clears without delivering its lifecycle event and the user resumes
    tokio::task::block_in_place(|| background.clear_history())?;
    background.handle_output(StreamChunk::text("new-generation-first".into()));
    background.handle_output(StreamChunk::done());
    background.handle_output(StreamChunk::text("new-generation-second".into()));
    background.handle_output(StreamChunk::done());
    app.dispatch(Action::AttachToSession(fixture.id.clone()));
    drain(&mut app).await;
    // @step Then only new generation content renders exactly once in order
    let text = rendered(&app, &fixture.id)?;
    assert!(!text.contains("old-generation-only"), "{text}");
    assert_eq!(text.matches("new-generation-first").count(), 1, "{text}");
    assert_eq!(text.matches("new-generation-second").count(), 1);
    assert!(text.find("new-generation-first") < text.find("new-generation-second"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn archive_installation_during_partial_hydration_recovers_atomically() -> TestResult {
    // @step Given a tab with a partial payload and pinned snapshot from an unseeded generation
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text(format!(
        "stream-tail-unique {}",
        "α".repeat(80_000)
    )));
    background.handle_output(StreamChunk::done());
    let mut app = app(&fixture)?;
    app.dispatch(Action::ReattachSession(fixture.id.clone()));
    let first = queued_page(&mut app).await?;
    let late = first.clone();
    if let Action::Transcript(TranscriptAction::Loaded {
        result: Ok(page), ..
    }) = &first
    {
        assert!(page.next_cursor.byte_offset > 0);
        assert!(!page.complete);
    } else {
        return Err("partial page fixture failed".into());
    }
    app.dispatch(first);
    // @step When archive installation changes generation and an old page arrives late
    assert!(background.seed_archival_transcript([
        Ok((StreamChunk::text("archive-first-unique".into()), None)),
        Ok((StreamChunk::done(), None)),
    ])?);
    drain(&mut app).await;
    app.dispatch(Action::AttachToSession(fixture.id.clone()));
    drain(&mut app).await;
    let before_late = rendered(&app, &fixture.id)?;
    app.dispatch(late);
    drain(&mut app).await;
    // @step Then old bytes and late responses are discarded and authoritative history renders once
    let text = rendered(&app, &fixture.id)?;
    assert_eq!(
        text.matches("archive-first-unique").count(),
        1,
        "archive missing or duplicated"
    );
    assert_eq!(text.matches("stream-tail-unique").count(), 1);
    assert!(text.find("archive-first-unique") < text.find("stream-tail-unique"));
    assert_eq!(text, before_late);
    assert!(!text.contains("Transcript incomplete"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn unchanged_generation_reconnect_preserves_deduplication() -> TestResult {
    // @step Given an open hydrated tab with existing transcript content
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("unchanged-original".into()));
    background.handle_output(StreamChunk::done());
    let mut app = app(&fixture)?;
    app.dispatch(Action::ReattachSession(fixture.id.clone()));
    drain(&mut app).await;
    // @step When the user resumes twice without a generation change
    for _ in 0..2 {
        app.dispatch(Action::AttachToSession(fixture.id.clone()));
        drain(&mut app).await;
    }
    background.handle_output(StreamChunk::text("unchanged-next".into()));
    background.handle_output(StreamChunk::done());
    app.dispatch(Action::Transcript(TranscriptAction::Poll));
    drain(&mut app).await;
    // @step Then existing and subsequent content each render exactly once
    let text = rendered(&app, &fixture.id)?;
    assert_eq!(text.matches("unchanged-original").count(), 1);
    assert_eq!(text.matches("unchanged-next").count(), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn transient_page_failures_preserve_displayed_history() -> TestResult {
    // @step Given an open hydrated tab with existing transcript content
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("transport-preserved".into()));
    background.handle_output(StreamChunk::done());
    let mut app = app(&fixture)?;
    app.dispatch(Action::ReattachSession(fixture.id.clone()));
    drain(&mut app).await;
    // @step When a transport failure occurs before resume succeeds
    app.dispatch(Action::Transcript(TranscriptAction::Poll));
    let mut failed = queued_page(&mut app).await?;
    if let Action::Transcript(TranscriptAction::Loaded { result, .. }) = &mut failed {
        *result = Err("injected transient transport disconnection".into());
    }
    app.dispatch(failed);
    assert_eq!(
        rendered(&app, &fixture.id)?
            .matches("transport-preserved")
            .count(),
        1
    );
    background.handle_output(StreamChunk::text("transport-next".into()));
    background.handle_output(StreamChunk::done());
    app.dispatch(Action::AttachToSession(fixture.id.clone()));
    drain(&mut app).await;
    // @step Then displayed content remains intact and catch up renders each event once
    let text = rendered(&app, &fixture.id)?;
    assert_eq!(text.matches("transport-preserved").count(), 1);
    assert_eq!(text.matches("transport-next").count(), 1);
    assert!(!text.contains("Transcript incomplete"));
    Ok(())
}
