//! Feature: spec/features/archival-transcript-history-access.feature

#[path = "bug244/support.rs"]
mod support;

#[path = "bug244/app_support.rs"]
mod app_support;

use app_support::drain;
use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{StreamChunk, ToolCallInfo, ToolResultInfo};
use support::{Fixture, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn original_events_remain_inspectable_outside_the_resident_scrollback() -> TestResult {
    // @step Given a real session containing original text and complete tool content
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::tool_call(ToolCallInfo {
        id: "inspected-call".into(),
        name: "read_file".into(),
        input: serde_json::json!({"path":"original-history.txt"}).to_string(),
    }));
    let original = format!(
        "{}\noriginal-inspector-tail",
        "full-result-line 雪🙂\n".repeat(20_000)
    );
    background.handle_output(StreamChunk::tool_result(ToolResultInfo {
        tool_call_id: "inspected-call".into(),
        content: original.clone(),
        is_error: false,
    }));
    background.handle_output(StreamChunk::done());
    background.handle_output(StreamChunk::text("resident archive tail".into()));
    background.handle_output(StreamChunk::done());
    let model_before = background.inner.lock().await.messages.clone();
    let manifest_before =
        serde_json::to_value(codelet_core::persistence::load_session(background.id)?)?;
    let (embedded, websocket, server) = fixture.backends().await?;
    for backend in [embedded, websocket] {
        let mut app = App::new(backend);
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        drain(&mut app).await;
        let context = app
            .agent_view_store_mut()
            .session_context_mut_for(&fixture.id)
            .ok_or("missing context")?;
        context.scrollback.trim_to_cap(2, 0);
        assert!(context.scrollback.trimmed_rows_total() > 0);
        assert!(context
            .scrollback
            .chunks()
            .iter()
            .flat_map(|chunk| &chunk.lines)
            .any(|line| line.to_string().contains("/archive")));
        let before = background.get_buffered_output(usize::MAX).len();
        // @step When the TUI inspects an original event using its archive command through either transport
        app.dispatch(Action::InputSubmitted("/archive 1".into()));
        drain(&mut app).await;
        let context = app
            .agent_view_store()
            .session_context_for(&fixture.id)
            .ok_or("missing context")?;
        let inspector = context
            .scrollback
            .chunks()
            .iter()
            .filter_map(|chunk| context.scrollback.full_text_for_seq(chunk.seq))
            .find(|text| text.contains("Archive event 1"))
            .ok_or("missing archive inspector content")?;
        // @step Then the complete original payload is accessible in the turn inspector
        assert!(inspector.contains("inspected-call"));
        assert!(inspector.contains("original-inspector-tail"));
        assert_eq!(
            app_support::rendered(&app, &fixture.id)?
                .matches("Archive event 1")
                .count(),
            1
        );
        // @step And no input is sent to the model
        assert_eq!(background.get_buffered_output(usize::MAX).len(), before);
        assert_eq!(background.inner.lock().await.messages, model_before);
        assert_eq!(
            serde_json::to_value(codelet_core::persistence::load_session(background.id)?)?,
            manifest_before
        );
    }
    server.abort();
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn archive_inspection_rejects_replacement_source_generations() -> TestResult {
    // @step Given a hydrated tab whose backend transcript has been replaced without a reset notification
    let fixture = Fixture::new().await?;
    let background = fixture.manager.get_session(&fixture.id.value)?;
    background.handle_output(StreamChunk::text("displayed original generation".into()));
    background.handle_output(StreamChunk::done());
    let (embedded, websocket, server) = fixture.backends().await?;
    let mut apps = [App::new(embedded), App::new(websocket)];
    for app in &mut apps {
        app.dispatch(Action::ReattachSession(fixture.id.clone()));
        drain(app).await;
    }
    tokio::task::block_in_place(|| background.clear_history())?;
    background.handle_output(StreamChunk::text("replacement generation payload".into()));
    for app in &mut apps {
        // @step When the user inspects an event from the displayed archive
        app.dispatch(Action::InputSubmitted("/archive 0".into()));
        drain(app).await;
        // @step Then a visible archive generation failure preserves the displayed history without presenting replacement content
        let text = app_support::rendered(app, &fixture.id)?;
        assert!(text.contains("Archive incomplete"), "{text}");
        assert!(text.contains("generation"), "{text}");
        assert!(text.contains("displayed original generation"));
        assert!(!text.contains("replacement generation payload"));
    }
    server.abort();
    Ok(())
}
