//! TUI-117 test support: builds on the TUI-116 App + MockBackend harness
//! (`tui116_stdin_notice_answer_box_support`, mounted by the test file) with
//! request/clear helpers and a colour probe for the answer box error row.
//! Every helper returns its failure instead of panicking.

use codelet_fspec_tui::{Action, App};
use codelet_rpc_types::{SessionId, StreamChunk};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::tui116_support::{drain, exec_request, key, Outcome, TestResult};

/// `command` in `exec_session` of `session` asks for input (push chunk or
/// probe), then the app settles.
pub async fn asks(
    app: &mut App,
    session: &SessionId,
    exec_session: &str,
    command: &str,
) -> TestResult {
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        codelet_rpc_types::StreamChunk::ExecStdinRequest {
            request: exec_request(exec_session, command),
        },
    ));
    drain(app).await
}

/// The backend cleared `session`'s request (the command exited or printed
/// again), then the app settles.
pub async fn request_cleared(app: &mut App, session: &SessionId) -> TestResult {
    app.dispatch(Action::ChunkReceived(
        session.clone(),
        StreamChunk::ExecStdinRequestCleared,
    ));
    drain(app).await
}

pub async fn shift(app: &mut App, code: KeyCode) -> TestResult {
    key(app, code, KeyModifiers::SHIFT).await
}

/// Foreground colour of the first cell of `needle` on screen, or an error
/// naming the screen when `needle` is not shown.
pub fn colour_of(app: &mut App, needle: &str) -> Outcome<Option<Color>> {
    let area = Rect::new(0, 0, 160, 30);
    let mut buffer = Buffer::empty(area);
    app.render(area, &mut buffer);
    let mut screen = Vec::new();
    for y in area.y..area.bottom() {
        let cells: Vec<String> = (area.x..area.right())
            .map(|x| buffer[(x, y)].symbol().to_string())
            .collect();
        for start in 0..cells.len() {
            if cells[start..].concat().starts_with(needle) {
                let x = area.x.saturating_add(u16::try_from(start)?);
                return Ok(buffer[(x, y)].style().fg);
            }
        }
        screen.push(cells.concat());
    }
    Err(format!("{needle:?} is not shown in:\n{}", screen.join("\n")).into())
}

/// Fail unless `needle` is painted red.
pub fn expect_red(app: &mut App, needle: &str) -> TestResult {
    match colour_of(app, needle)? {
        Some(Color::Red) => Ok(()),
        other => Err(format!("{needle:?} is painted {other:?}, not red").into()),
    }
}
