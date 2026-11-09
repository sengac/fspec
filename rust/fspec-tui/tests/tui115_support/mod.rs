//! TUI-115 test support: App + MockBackend harness and screen-row helpers
//! for `tui115_running_composer_steer_queue.rs` (split out to keep the test
//! file under 300 lines).

use crate::common::TaskOutcome;
use std::error::Error;
use std::sync::Arc;

use codelet_fspec_tui::{App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, SessionStatus, StreamChunk};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::common::MockBackend;

pub const RUNNING_HINT: &str = "Enter steer · Alt+Enter queue · Esc interrupt";
pub const COMPACTING_HINT: &str = "Enter or Alt+Enter queue · runs after compaction";

pub async fn drain(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
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

pub async fn key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<(), codelet_test_helpers::TestError> {
    app.handle_event(&Event::Key(KeyEvent::new(code, modifiers)));
    drain(app).await?;
    Ok(())
}

pub async fn type_text(app: &mut App, text: &str) -> Result<(), codelet_test_helpers::TestError> {
    for character in text.chars() {
        key(app, KeyCode::Char(character), KeyModifiers::NONE).await?;
    }
    Ok(())
}

/// Render one frame and return its rows as text.
pub fn screen(app: &mut App) -> Result<Vec<String>, Box<dyn Error>> {
    let mut term = Terminal::new(TestBackend::new(160, 30))?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))?;
    let buffer = term.backend().buffer();
    let width = buffer.area.width as usize;
    Ok(buffer
        .content
        .chunks(width)
        .map(|row| {
            row.iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
        })
        .collect())
}

/// Index of the first row containing `needle`.
pub fn row_of(rows: &[String], needle: &str) -> Result<usize, Box<dyn Error>> {
    rows.iter()
        .position(|row| row.contains(needle))
        .ok_or_else(|| format!("no row contains {needle:?} in:\n{}", rows.join("\n")).into())
}

pub fn rows_containing(rows: &[String], needle: &str) -> usize {
    rows.iter().filter(|row| row.contains(needle)).count()
}

/// The agent view of a new session whose status is `status`.
pub async fn agent_view_with(
    status: SessionStatus,
) -> Result<(App, Arc<MockBackend>, SessionId), Box<dyn Error>> {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await?;
    drain(&mut app).await?;
    let session = SessionId::new("s-busy");
    mock.script_create_session(session.clone());
    key(&mut app, KeyCode::Char('.'), KeyModifiers::NONE).await?;
    key(&mut app, KeyCode::Enter, KeyModifiers::NONE).await?;
    assert_eq!(app.active_view(), ViewMode::Agent);
    app.agent_view_store_mut()
        .set_session_status(session.clone(), status);
    // A frame render caches the focused session's status on the view.
    screen(&mut app)?;
    Ok((app, mock, session))
}

/// The session echoes `text` as a user input (steer at once, queue on
/// start): the event lands in its transcript and wakes the TUI.
pub async fn session_echoes(
    app: &mut App,
    mock: &MockBackend,
    session: &SessionId,
    text: &str,
) -> Result<(), codelet_test_helpers::TestError> {
    let echo = StreamChunk::UserInput {
        text: text.to_string(),
    };
    mock.push_buffered_output(echo.clone());
    mock.push_chunk(session.clone(), echo);
    drain(app).await?;
    Ok(())
}

/// The queued-list rows: between the activity row and the composer row.
pub fn queued_rows(
    rows: &[String],
    activity: &str,
    composer_row: usize,
) -> Result<Vec<String>, Box<dyn Error>> {
    let activity_row = row_of(rows, activity)?;
    let listed = rows.get(activity_row + 1..composer_row).ok_or_else(|| {
        format!("activity row {activity_row} is not above composer row {composer_row}")
    })?;
    Ok(listed.to_vec())
}
