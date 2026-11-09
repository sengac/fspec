//! BUG-642: the /loop list prompt column cuts a long prompt on a char
//! boundary.
//!
//! Feature: spec/features/text-cuts-land-on-char-boundaries.feature

use crate::common::TaskOutcome;
use codelet_fspec_tui::app::loop_parser::LoopSubcommand;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{RegisteredLoop, SessionId};
use codelet_test_helpers::{OrFail, TestResult};
use std::sync::Arc;

mod common;
use common::MockBackend;

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

fn scrollback_text(app: &App, id: &SessionId) -> String {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default()
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_loop_list_prompt_column_cuts_before_a_multi_byte_character() -> TestResult {
    // @step Given an App whose open session has a loop whose prompt is 24 ASCII bytes followed by "é" and more text
    let ascii = "p".repeat(24);
    let mock = Arc::new(MockBackend::new());
    mock.seed_loop_list_result(Ok(vec![RegisteredLoop {
        id: "a1b2c3d4".to_string(),
        session_id: SessionId::new("s-1"),
        prompt: format!("{ascii}é and more text"),
        interval_seconds: 60,
        created_at: "2026-05-24T00:00:00Z".to_string(),
        expires_at: "2026-05-27T00:00:00Z".to_string(),
        last_run_at: None,
    }]));
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(SessionId::new("s-1")));
    drain_pending(&mut app).await?;
    // @step When /loop list runs
    app.dispatch(Action::LoopSubcommandParsed(LoopSubcommand::List));
    let listed = format!("{ascii}...");
    codelet_test_helpers::deadlock_guard("loop list notice", async {
        loop {
            if drain_pending(&mut app).await.is_err()
                || scrollback_text(&app, &SessionId::new("s-1")).contains("Active loops:")
            {
                return;
            }
            codelet_test_helpers::yield_now().await;
        }
    })
    .await
    .or_fail("timed out waiting for the loop list notice")?;
    drain_pending(&mut app).await?;
    // @step Then the notice lists the prompt as the 24 ASCII bytes followed by "..."
    let text = scrollback_text(&app, &SessionId::new("s-1"));
    assert!(text.contains(&listed), "{text}");
    Ok(())
}
