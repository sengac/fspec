//! RPC-448 — TUI daemon switcher.
//!
//! Feature: spec/features/tui-daemon-switcher.feature
//!
//! Harness: App + two MockBackends ("alpha" connected, "work" saved) behind
//! a stub DaemonDirectory; rendered at 120x24 like the RPC-445 dropdown.

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use codelet_fspec_tui::daemons::{DaemonChoice, DaemonDirectory};
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

struct StubDirectory {
    backends: HashMap<String, Arc<MockBackend>>,
}

#[async_trait]
impl DaemonDirectory for StubDirectory {
    fn list(&self) -> anyhow::Result<Vec<DaemonChoice>> {
        Ok(vec![
            DaemonChoice {
                name: "alpha".into(),
                url: "ws://127.0.0.1:4001".into(),
                saved: false,
            },
            DaemonChoice {
                name: "work".into(),
                url: "wss://fspec.example.com".into(),
                saved: true,
            },
        ])
    }

    async fn connect(&self, choice: &DaemonChoice) -> anyhow::Result<Arc<dyn FspecBackend>> {
        let backend = self
            .backends
            .get(&choice.name)
            .ok_or_else(|| anyhow::anyhow!("unknown daemon {}", choice.name))?;
        Ok(backend.clone())
    }
}

fn unit(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

struct Fixture {
    app: App,
    alpha: Arc<MockBackend>,
}

/// An App connected to "alpha" (ALPHA-001) with "work" (WORK-001) saved.
async fn connected_to_alpha() -> Result<Fixture, codelet_test_helpers::TestError> {
    let alpha = Arc::new(MockBackend::new());
    alpha.seed_work_units(vec![unit("ALPHA-001")]);
    let work = Arc::new(MockBackend::new());
    work.seed_work_units(vec![unit("WORK-001")]);
    let backend: Arc<dyn FspecBackend> = alpha.clone();
    let mut app = App::new(backend);
    let directory = StubDirectory {
        backends: HashMap::from([
            ("alpha".to_string(), alpha.clone()),
            ("work".to_string(), work),
        ]),
    };
    app.set_daemon_directory(Arc::new(directory), "alpha".to_string());
    app.bootstrap().await.or_fail("bootstrap")?;
    drain(&mut app).await?;
    Ok(Fixture { app, alpha })
}

async fn drain(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    for _ in 0..5 {
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
            while let Some(handle) = app.next_pending_task() {
                handle.await.completed_or_cancelled()?;
            }
        }
        // TEST-042: scheduler rounds, not a time window: on the
        // current-thread test runtime every ready task (a spawned backend
        // call, a subscriber forwarding a push) runs before this resumes.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }
    Ok(())
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn render(app: &mut App) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("terminal")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

fn workspace_row(buf: &Buffer) -> Result<u16, codelet_test_helpers::TestError> {
    Ok((0..buf.area.height)
        .find(|&y| row(buf, y).contains("Checkpoints:"))
        .or_fail("checkpoint status row")?
        + 1)
}

fn board_ids(app: &App) -> Vec<String> {
    app.board_store()
        .work_units()
        .iter()
        .map(|unit| unit.id.clone())
        .collect()
}

async fn switch_to_work(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&key(KeyCode::Char('s')));
    drain(app).await?;
    app.handle_event(&key(KeyCode::Down));
    drain(app).await?;
    app.handle_event(&key(KeyCode::Enter));
    drain(app).await?;
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_switcher_lists_local_and_saved_daemons() -> TestResult {
    // @step Given a daemon directory with local daemon "alpha" (connected) and saved daemon "work"
    let mut fixture = connected_to_alpha().await?;

    // @step When the user presses "s" on the board
    fixture.app.handle_event(&key(KeyCode::Char('s')));
    drain(&mut fixture.app).await?;

    // @step Then a dropdown lists "alpha" and "work" with "alpha" marked as connected
    let buf = render(&mut fixture.app)?;
    let first = workspace_row(&buf)? + 2;
    assert!(
        row(&buf, first).contains("alpha (connected)"),
        "{}",
        row(&buf, first)
    );
    assert!(
        row(&buf, first + 1).contains("work (saved)"),
        "{}",
        row(&buf, first + 1)
    );
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn choosing_a_daemon_rescopes_the_board_to_it() -> TestResult {
    // @step Given the board is connected to "alpha" showing "ALPHA-001" and "work" has "WORK-001"
    let mut fixture = connected_to_alpha().await?;
    assert_eq!(board_ids(&fixture.app), ["ALPHA-001"]);

    // @step When the user switches to "work"
    switch_to_work(&mut fixture.app).await?;

    // @step Then the board shows "WORK-001"
    assert_eq!(board_ids(&fixture.app), ["WORK-001"]);

    // @step And a later work-unit push from "alpha" does not reach the board
    fixture.alpha.push_work_units(vec![unit("ALPHA-002")]);
    drain(&mut fixture.app).await?;
    assert_eq!(board_ids(&fixture.app), ["WORK-001"]);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn switching_leaves_the_previous_daemons_sessions_running() -> TestResult {
    // @step Given the board is connected to "alpha" with an open session
    let mut fixture = connected_to_alpha().await?;
    fixture
        .app
        .dispatch(Action::SessionCreated(SessionId::new("s-alpha")));
    drain(&mut fixture.app).await?;
    fixture.app.dispatch(Action::BackToBoard);

    // @step When the user switches to "work"
    switch_to_work(&mut fixture.app).await?;

    // @step Then no session on "alpha" was destroyed or interrupted
    assert_eq!(fixture.alpha.destroy_session_calls(), 0);
    assert_eq!(fixture.alpha.interrupt_calls(), 0);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn the_header_names_the_connected_daemon() -> TestResult {
    // @step Given the board is connected to "alpha"
    let mut fixture = connected_to_alpha().await?;

    // @step When the user switches to "work"
    switch_to_work(&mut fixture.app).await?;

    // @step Then header row 1 shows "Daemon: work"
    let buf = render(&mut fixture.app)?;
    let text = row(&buf, workspace_row(&buf)?);
    assert!(text.contains("Daemon: work"), "{text}");
    Ok(())
}
