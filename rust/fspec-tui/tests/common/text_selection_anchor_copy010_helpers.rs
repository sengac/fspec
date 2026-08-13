//! Shared helpers for `tests/text_selection_anchors_at_pressed_column_copy010.rs`
//! (COPY-010).
//!
//! Split out so the test file proper stays under the 300-LoC ceiling.
//! Provides the AgentView scrollback App scaffolding (App + MockBackend +
//! injected OSC 52 clipboard writer + `poll_selection_tick_for_test`)
//! mirroring `agentview_text_selection_copy_copy006.rs`.

#![allow(dead_code)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use codelet_fspec_tui::views::agent::RenderedChunk;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::Terminal;

use crate::common::MockBackend;

/// An `Arc<Mutex<Vec<u8>>>`-backed writer so the test can inspect the
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

/// Build an App focused on an AgentView session, with an injected
/// clipboard sink. Returns the App and the shared clipboard buffer.
pub fn app_with_clipboard() -> (App, Arc<Mutex<Vec<u8>>>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    app.set_clipboard_writer_for_test(Box::new(SharedWriter(buf.clone())));
    (app, buf)
}

/// Push a single explicit-text row and return its 0-based visual index.
pub fn seed_line(app: &mut App, text: &str) -> u64 {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(&sid("s-1"))
        .expect("SessionContext present");
    let seq = ctx.scrollback.chunk_count() as u64;
    ctx.scrollback.push(RenderedChunk {
        seq,
        lines: vec![Line::from(text.to_string())],
        source: None,
    });
    seq
}

/// Render one frame so the layout caches `last_scrollback_area` and the
/// gutter-free `content_width`.
pub fn render_app(app: &mut App, w: u16, h: u16) {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    terminal
        .draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
}

/// Drain + dispatch every queued Action so emitted selection Actions run
/// their reducers synchronously.
pub fn drain(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

pub fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Snapshot of the injected clipboard buffer as raw bytes.
pub fn clip_bytes(buf: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    buf.lock().expect("clipboard buffer mutex").clone()
}

/// The OSC 52 sequence the App emits for `text`: `ESC ] 52 ; c ;
/// <base64> BEL`.
pub fn osc52(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
    out.push(0x07);
    out
}

/// True while the focused scrollback has a live text selection.
pub fn selection_active(app: &App) -> bool {
    app.agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|c| c.scrollback.text_selection_active())
        .unwrap_or(false)
}

/// Number of REVERSED highlight spans painted for the live selection.
pub fn highlight_spans(app: &App) -> usize {
    app.agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|c| c.scrollback.selection_highlight_span_count())
        .unwrap_or(0)
}
