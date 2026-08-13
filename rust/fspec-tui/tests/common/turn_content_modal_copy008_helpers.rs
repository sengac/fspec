//! Shared helpers for `tests/turn_content_modal_text_selection_copy_copy008.rs`
//! (COPY-008).
//!
//! Split out so the test file proper stays under the 300-LoC ceiling.
//! Builds an `App` in `ViewMode::Agent` with a real `ChunkSource` turn,
//! opens the turn-content modal, injects a `Vec<u8>`-backed OSC 52
//! clipboard writer, and exposes the modal geometry so tests can drive
//! `Event::Mouse` over the modal BODY rect deterministically.

#![allow(dead_code)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use codelet_fspec_tui::components::dialog_theme_rows::{fixed_dialog_rect, turn_modal_geometry};
use codelet_fspec_tui::views::agent::rendered_chunk::ChunkSource;
use codelet_fspec_tui::{Action, App, ChunkKind, FspecBackend, RenderedChunk, ViewMode};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Line;
use ratatui::Terminal;

use crate::common::MockBackend;

/// An `Arc<Mutex<Vec<u8>>>`-backed writer so a test can inspect the
/// exact clipboard bytes after driving a copy through the App.
pub struct SharedWriter(pub Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("clipboard buffer mutex")
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

pub fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

pub fn tab() -> Event {
    key(KeyCode::Tab)
}

pub fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
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

/// Drain the App's action bus, dispatching each queued Action back into
/// the App so reducer side-effects run synchronously.
pub fn drain_app(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Render the App into a `w`x`h` TestBackend so `last_render_area` is
/// cached and the modal overlay is painted.
pub fn render_app(app: &mut App, w: u16, h: u16) -> Vec<String> {
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

/// `turn_modal_seq` lives on `AgentView` (== `navigator.agent`).
pub fn modal_seq(app: &App) -> Option<u64> {
    app.navigator().agent.turn_modal_seq
}

/// `turn_modal_offset` lives on `AgentView`.
pub fn modal_offset(app: &App) -> usize {
    app.navigator().agent.turn_modal_offset
}

/// The modal BODY rect (content area) for `body` inside a `w`x`h`
/// terminal. Inner origin is `(rect.x + 2, rect.y + 4)` (border, padding,
/// title and gap — mirrors `TurnContentModal::render`'s scrollbar
/// `bar_area`), `content_width` cols wide and `viewport_rows` tall. The
/// modal scrollbar gutter sits in the column at `x + content_width`.
pub fn modal_body_rect(w: u16, h: u16, body: &str) -> Rect {
    let area = Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    };
    let rect = fixed_dialog_rect(area);
    let geom = turn_modal_geometry(area, body);
    Rect {
        x: rect.x + 2,
        y: rect.y + 4,
        width: geom.content_width as u16,
        height: geom.viewport_rows as u16,
    }
}

/// The OSC 52 sequence the App emits for `text`: `ESC ] 52 ; c ;
/// <base64> BEL`. Lets a test assert the copied payload exactly.
pub fn osc52(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
    out.push(0x07);
    out
}

/// Snapshot of the injected clipboard buffer as raw bytes.
pub fn clip_bytes(buf: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    buf.lock().expect("clipboard buffer mutex").clone()
}

/// Build an App in `ViewMode::Agent` seeded with one `ChunkSource` turn
/// (`body`), an injected OSC 52 clipboard sink, with the turn-content
/// modal OPEN on that turn. Returns the App and the shared clipboard
/// buffer. `w`x`h` is the render size used to open the modal (so the
/// modal geometry matches subsequent `modal_body_rect(w, h, body)`).
pub fn open_modal_app(body: &str, w: u16, h: u16) -> (App, Arc<Mutex<Vec<u8>>>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    push_text_turn(&mut app, &sid("s-1"), 0, body);
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    app.set_clipboard_writer_for_test(Box::new(SharedWriter(buf.clone())));
    // Enter SELECT mode (auto-selects the last turn) then open the modal.
    let _ = app.handle_event(&tab());
    drain_app(&mut app);
    let _ = app.handle_event(&key(KeyCode::Enter));
    drain_app(&mut app);
    // Render once so `last_render_area` is cached at this size.
    let _ = render_app(&mut app, w, h);
    (app, buf)
}

/// True while the open modal holds a live text selection (COPY-008 —
/// `AgentView.turn_modal_selection` is `Some`).
pub fn modal_selection_active(app: &App) -> bool {
    app.navigator().agent.turn_modal_selection.is_some()
}
