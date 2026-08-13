//! Shared helpers for
//! `tests/board_details_strip_text_selection_copy_copy009.rs` (COPY-009).
//!
//! Split out so the test file proper stays under the 300-LoC ceiling.
//! Builds a real `BoardView` + `BoardStore`, renders it onto a
//! `TestBackend` so `last_details_area` is cached, injects a
//! `Vec<u8>`-backed OSC 52 clipboard writer, and exposes the details
//! strip geometry + rendered-buffer readers so tests can drive real
//! `Event::Mouse` Down/Drag/Up over the strip inner rect deterministically
//! and derive the border-free expected text.

#![allow(dead_code)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use codelet_fspec_tui::{Action, BoardStore, BoardView, Theme};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

/// An `Arc<Mutex<Vec<u8>>>`-backed writer so a test can inspect the exact
/// clipboard bytes after driving a copy through the BoardView.
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

/// Build a `WorkUnitInfo` with an optional description.
pub fn wu(id: &str, title: &str, status: &str, description: Option<&str>) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: title.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: description.map(str::to_string),
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// A fresh BoardView + its action receiver.
pub fn fresh() -> (BoardView, UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

/// Build a BoardView seeded with `units` (first one selected in `backlog`)
/// and an injected OSC 52 clipboard sink. Renders once at `w`x`h` so the
/// details-strip rect is cached. Returns the view, store, action receiver
/// and the shared clipboard buffer.
pub fn board_with_clipboard(
    units: Vec<WorkUnitInfo>,
    w: u16,
    h: u16,
) -> (
    BoardView,
    BoardStore,
    UnboundedReceiver<Action>,
    Arc<Mutex<Vec<u8>>>,
) {
    let mut store = BoardStore::default();
    store.replace_work_units(units);
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    let (view, rx) = fresh();
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    view.set_clipboard_writer_for_test(Box::new(SharedWriter(buf.clone())));
    render(&view, &store, w, h);
    (view, store, rx, buf)
}

/// Render the BoardView onto a `w`x`h` TestBackend so its render-observed
/// geometry (including `last_details_area`) is cached, returning the
/// painted buffer.
pub fn render(view: &BoardView, store: &BoardStore, w: u16, h: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Synthesize an `Event::Mouse` at `(col,row)`.
pub fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// The details-strip inner rect for a `w`x`h` terminal: `split[3]` inner.
/// Layout is: top border(1) + header(4) + separator(1) → details strip at
/// y=6, height 5; `borders::inner_rect` trims one column each side, so at
/// width 120 the inner rect is `{x:1, y:6, width:118, height:5}`.
pub fn details_rect(w: u16, _h: u16) -> Rect {
    Rect {
        x: 1,
        y: 6,
        width: w.saturating_sub(2),
        height: 5,
    }
}

/// Read a single rendered buffer row over the columns `[x0, x0+width)` and
/// trim trailing spaces — mirrors the COPY-008 buffer-read helper and
/// yields the exact border-free on-screen text of that strip row.
pub fn buffer_row_text(buf: &Buffer, x0: u16, y: u16, width: u16) -> String {
    let mut s = String::new();
    for x in x0..x0 + width {
        s.push_str(buf[(x, y)].symbol());
    }
    s.trim_end().to_string()
}

/// The OSC 52 sequence the BoardView emits for `text`.
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

/// True while the BoardView holds a live details-strip text selection
/// (COPY-009 — `BoardView.details_selection` is `Some`).
pub fn strip_selection_active(view: &BoardView) -> bool {
    view.details_selection().is_some()
}
