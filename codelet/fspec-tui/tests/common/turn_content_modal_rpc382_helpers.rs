//! Shared helpers for `tests/turn_content_modal_rpc382.rs` (RPC-382).
//!
//! Split out so the test file proper stays under the 300-LoC ceiling.
//! Duplicated from `tests/turn_select_mode_rpc381.rs` rather than shared
//! because the RPC-381 `seed_turns` pushes `source: None` chunks, while
//! the modal needs a real `ChunkSource` whose `text` is the full body
//! the modal must surface.

#![allow(dead_code)]

use std::sync::Arc;

use codelet_fspec_tui::views::agent::rendered_chunk::ChunkSource;
use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend, RenderedChunk, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::Terminal;

use crate::common::MockBackend;

pub fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

pub fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

pub fn tab() -> Event {
    key(KeyCode::Tab)
}

/// Push one turn carrying a real `ChunkSource` so the modal has a full
/// body to render. `lines` mirror the source text split on hard breaks.
pub fn push_text_turn(app: &mut App, id: &SessionId, seq: u64, text: &str) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .expect("SessionContext present");
    ctx.scrollback.push(RenderedChunk {
        seq,
        lines: text.lines().map(|l| Line::from(l.to_string())).collect(),
        source: Some(ChunkSource {
            text: text.to_string(),
            color: Color::White,
            kind: ChunkKind::AssistantText,
            is_streaming: false,
            full_text: None,
        }),
    });
}

/// An App in `ViewMode::Agent` with one session seeded with three turns
/// whose bodies carry distinct markers (FIRSTBODY / SECONDBODY / THIRD).
pub fn agent_app_with_text_turns() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    push_text_turn(&mut app, &sid("s-1"), 0, "first FIRSTBODY");
    push_text_turn(&mut app, &sid("s-1"), 1, "second SECONDBODY");
    push_text_turn(&mut app, &sid("s-1"), 2, "third THIRDBODY");
    app
}

/// An App whose LAST (auto-selected) turn is far taller than any sane
/// viewport: `TOPMARKER` sits at the very top of the body (so a
/// stick-to-bottom scrollback scrolls it OFF screen — "collapsed"),
/// while the modal renders the body from the top and therefore shows it.
pub fn agent_app_with_collapsed_last_turn() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    push_text_turn(&mut app, &sid("s-1"), 0, "first turn");
    push_text_turn(&mut app, &sid("s-1"), 1, "second turn");
    let mut long = String::from("TOPMARKER\n");
    for i in 0..60 {
        long.push_str(&format!("filler line {i}\n"));
    }
    long.push_str("BOTTOMMARKER");
    push_text_turn(&mut app, &sid("s-1"), 2, &long);
    app
}

/// Drain the App's action bus, dispatching each queued Action back into
/// the App so reducer side-effects run synchronously.
pub fn drain_app(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

pub fn selected_seq(app: &App, id: &SessionId) -> Option<u64> {
    app.agent_view_store()
        .session_context_for(id)
        .and_then(|c| c.scrollback.selected_seq())
}

/// `turn_modal_seq` lives on `AgentView` (== `navigator.agent`).
pub fn modal_seq(app: &App) -> Option<u64> {
    app.navigator().agent.turn_modal_seq
}

/// Render the App and return the rows of glyphs.
pub fn render_rows(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            row
        })
        .collect()
}

pub fn joined(rows: &[String]) -> String {
    rows.join("\n")
}
