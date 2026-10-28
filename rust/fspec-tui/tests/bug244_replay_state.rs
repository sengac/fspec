//! Feature: spec/features/transcript-hydration-live-state.feature

#[path = "bug244/support.rs"]
pub mod support;

#[path = "bug244/app_support.rs"]
mod app_support;

use codelet_fspec_tui::app::transcript_sync::TranscriptAction;
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::StreamChunk;
use support::{Fixture, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn historical_hydration_preserves_current_provider_waiting_state() -> TestResult {
    // @step Given an open session whose original transcript page has not reached the TUI
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("historical original".into()));
    background.handle_output(StreamChunk::done());
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        let mut app = App::new(backend);
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        while let Some(task) = app.next_pending_task() {
            task.await?;
        }
        let mut delayed = Vec::new();
        while let Some(action) = app.try_recv_action() {
            delayed.push(action);
        }
        // @step When a current provider waiting notification arrives before the historical page is applied
        app.dispatch(Action::ChunkReceived(
            fixture.id.clone(),
            StreamChunk::ProviderWaiting {
                message: Some("current provider request".into()),
            },
        ));
        for action in delayed {
            app.dispatch(action);
        }
        app_support::drain(&mut app).await;
        // @step Then the original text renders without clearing the current provider waiting indicator
        assert!(app_support::rendered(&app, &fixture.id)?.contains("historical original"));
        let context = app
            .agent_view_store()
            .session_context_for(&fixture.id)
            .ok_or("missing context")?;
        assert_eq!(
            context.provider_waiting.as_deref(),
            Some("current provider request")
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn live_content_notifications_dismiss_provider_waiting_without_rendering_twice() -> TestResult
{
    // @step Given a hydrated tab receiving live provider waiting notifications
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    let (embedded, websocket, server) = fixture.backends().await?;
    for (index, backend) in [embedded, websocket].into_iter().enumerate() {
        let mut app = App::new(backend);
        app.bootstrap().await?;
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        app_support::drain(&mut app).await;
        background.handle_output(StreamChunk::ProviderWaiting {
            message: Some("live waiting".into()),
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                app_support::drain(&mut app).await;
                if app
                    .agent_view_store()
                    .session_context_for(&fixture.id)
                    .is_some_and(|context| {
                        context.provider_waiting.as_deref() == Some("live waiting")
                    })
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await?;
        // @step When fresh response content arrives through either subscribed transport
        let response = format!("live content notification {index}");
        background.handle_output(StreamChunk::text(response.clone()));
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                app_support::drain(&mut app).await;
                if app_support::rendered(&app, &fixture.id)
                    .is_ok_and(|text| text.contains(&response))
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await?;
        // @step Then the waiting indicator clears and the response renders exactly once
        assert_eq!(
            app.agent_view_store()
                .session_context_for(&fixture.id)
                .ok_or("missing context")?
                .provider_waiting,
            None
        );
        assert_eq!(
            app_support::rendered(&app, &fixture.id)?
                .matches(&response)
                .count(),
            1
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn idle_transcript_polling_retains_bounded_task_state() -> TestResult {
    // @step Given a hydrated idle session with no additional transcript events
    let fixture = Fixture::new().await?;
    let (embedded, _websocket, server) = fixture.backends().await?;
    let mut app = App::new(embedded);
    app.dispatch(Action::ReattachSession(fixture.id.clone()));
    app_support::drain(&mut app).await;
    // @step When many periodic transcript polls complete in the production action path
    for _poll in 0..30 {
        app.dispatch(Action::Transcript(TranscriptAction::Poll));
        let action = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Some(action) = app.try_recv_action() {
                    break action;
                }
                tokio::task::yield_now().await;
            }
        })
        .await?;
        app.dispatch(action);
    }
    let mut retained = 0;
    while let Some(task) = app.next_pending_task() {
        task.await?;
        retained += 1;
    }
    // @step Then completed page tasks are released instead of accumulating with elapsed idle time
    assert!(retained <= 2, "retained {retained} completed polling tasks");
    server.abort();
    Ok(())
}
