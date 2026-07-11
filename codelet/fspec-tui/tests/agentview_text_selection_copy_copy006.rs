//! Feature: spec/features/agentview-text-selection-copy.feature
//!
//! COPY-006 — wire selection + copy into the AgentView scrollback
//! end-to-end. Each test drives real `Event::Mouse` events through
//! `App::handle_event`, pumps the emitted `Action`s back through
//! `App::dispatch`, and asserts the observable side effects:
//!   - the injected OSC 52 clipboard writer's bytes (COPY-001), and
//!   - the live-selection / highlight state exposed by the public
//!     `ScrollbackList::text_selection_active` /
//!     `selection_highlight_span_count` test seams (COPY-005/006).
//!
//! The clipboard writer is an `Arc<Mutex<Vec<u8>>>`-backed sink injected
//! via `App::set_clipboard_writer_for_test`. Long-press selection is
//! driven via `App::poll_selection_tick_for_test`, the public seam that
//! mirrors the production run loop's 16ms render-tick arm.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use codelet_fspec_tui::views::agent::RenderedChunk;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ---------------------------------------------------------------------
// Test scaffolding
// ---------------------------------------------------------------------

/// An `Arc<Mutex<Vec<u8>>>`-backed writer so the test can inspect the
/// exact clipboard bytes after driving a copy through the App.
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

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

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Build an App focused on an AgentView session, with an injected
/// clipboard sink. Returns the App and the shared clipboard buffer.
fn app_with_clipboard() -> (App, Arc<Mutex<Vec<u8>>>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    app.set_clipboard_writer_for_test(Box::new(SharedWriter(buf.clone())));
    (app, buf)
}

/// Push `count` single-line chunks ("row-0", "row-1", ...) into the
/// focused session's scrollback.
fn seed_rows(app: &mut App, count: usize) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(&sid("s-1"))
        .expect("SessionContext present");
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            seq: i as u64,
            lines: vec![Line::from(format!("row-{i}"))],
            source: None,
        });
    }
}

/// Push a single explicit-text row and return its 0-based visual index.
fn seed_line(app: &mut App, text: &str) -> u64 {
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
fn render_app(app: &mut App, w: u16, h: u16) {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    terminal
        .draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
}

/// Drain + dispatch every queued Action so emitted selection Actions run
/// their reducers synchronously (mirrors `scrollback_scroll_rpc094.rs`).
fn drain(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Snapshot of the injected clipboard buffer as raw bytes.
fn clip_bytes(buf: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
    buf.lock().expect("clipboard buffer mutex").clone()
}

/// The OSC 52 sequence the App emits for `text`: `ESC ] 52 ; c ;
/// <base64> BEL`. Lets a test assert the copied payload exactly.
fn osc52(text: &str) -> Vec<u8> {
    let mut out = b"\x1b]52;c;".to_vec();
    out.extend_from_slice(STANDARD.encode(text.as_bytes()).as_bytes());
    out.push(0x07);
    out
}

/// True while the focused scrollback has a live text selection.
fn selection_active(app: &App) -> bool {
    app.agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|c| c.scrollback.text_selection_active())
        .unwrap_or(false)
}

/// Number of REVERSED highlight spans painted for the live selection.
fn highlight_spans(app: &App) -> usize {
    app.agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|c| c.scrollback.selection_highlight_span_count())
        .unwrap_or(0)
}

/// Current scrollback scroll offset (visual rows).
fn scroll_offset(app: &App) -> usize {
    app.agent_view_store()
        .session_context_for(&sid("s-1"))
        .map(|c| c.scrollback.scroll_state().offset)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------
// Scenario 1: Dragging across two lines copies their text and keeps the
//             highlight
// ---------------------------------------------------------------------

#[test]
fn dragging_across_two_lines_copies_their_text_and_keeps_the_highlight() {
    // @step Given an AgentView showing a multi-line transcript with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    seed_rows(&mut app, 40);
    render_app(&mut app, 80, 40);
    // Anchor the viewport at the top so viewport row N maps to visual
    // row N ("row-N"), making the copied text deterministic.
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16; // header row is 0; scrollback band starts at y=1.

    // @step When I drag from the middle of one line to the middle of the line below and release
    // Down at the start of "row-0", drag to the far edge of the "row-1"
    // line (past its content), release — a linewise two-row selection.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        79,
        rect_y + 1,
    ));
    drain(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Up(MouseButton::Left),
        79,
        rect_y + 1,
    ));
    drain(&mut app);

    // @step Then the two lines of text without any scrollbar glyphs are written to the clipboard
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52("row-0\nrow-1"),
        "drag over two lines must copy both gutter-free lines"
    );
    assert!(!bytes.windows(3).any(|w| w == [0xe2, 0x94, 0x82]));

    // @step And the selection stays highlighted
    assert!(
        selection_active(&app),
        "selection must persist after commit (rule [2])"
    );
    render_app(&mut app, 80, 40);
    assert!(
        highlight_spans(&app) > 0,
        "the highlight overlay must still be painted after copy"
    );
}

// ---------------------------------------------------------------------
// Scenario 2: Long-pressing a line selects and copies it
// ---------------------------------------------------------------------

#[test]
fn long_pressing_a_line_selects_and_copies_it() {
    // @step Given an AgentView showing a multi-line transcript with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    seed_rows(&mut app, 40);
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;

    // @step When I press and hold on a line for about half a second and release
    let _ = app.handle_event(&mouse(
        MouseEventKind::Down(MouseButton::Left),
        2,
        rect_y + 3,
    ));
    drain(&mut app);
    // The recognizer's long-press threshold is ~400ms (real Instant); wait
    // past it, then poll the tick seam so the Begin gesture fires.
    std::thread::sleep(std::time::Duration::from_millis(450));
    app.poll_selection_tick_for_test();
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 2, rect_y + 3));
    drain(&mut app);

    // @step Then the line under the press becomes selected and its text is written to the clipboard
    assert_eq!(
        clip_bytes(&clip),
        osc52("row-3"),
        "long-press then release must copy the line under the press"
    );
}

// ---------------------------------------------------------------------
// Scenario 3: Wheel scrolling still works and does not select or copy
// ---------------------------------------------------------------------

#[test]
fn wheel_scrolling_still_works_and_does_not_select_or_copy() {
    // @step Given an AgentView showing a multi-line transcript with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    seed_rows(&mut app, 200);
    render_app(&mut app, 80, 40);
    let before = scroll_offset(&app);
    let rect_y = 1u16;

    // @step When I scroll the mouse wheel over the transcript
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, rect_y + 4));
    drain(&mut app);

    // @step Then the scrollback scrolls normally
    let after = scroll_offset(&app);
    assert!(
        after < before,
        "wheel ScrollUp must scroll the scrollback; before={before} after={after}"
    );

    // @step And no selection is created and nothing is written to the clipboard
    assert!(!selection_active(&app), "wheel must not create a selection");
    assert!(
        clip_bytes(&clip).is_empty(),
        "wheel scrolling must not write to the clipboard"
    );
}

// ---------------------------------------------------------------------
// Scenario 4: A quick click does not select or copy
// ---------------------------------------------------------------------

#[test]
fn a_quick_click_does_not_select_or_copy() {
    // @step Given an AgentView showing a multi-line transcript with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    seed_rows(&mut app, 40);
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;

    // @step When I quickly click a line without dragging
    // Down then immediate Up, with NO drag and NO long-press tick between.
    let _ = app.handle_event(&mouse(
        MouseEventKind::Down(MouseButton::Left),
        4,
        rect_y + 2,
    ));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 4, rect_y + 2));
    drain(&mut app);

    // @step Then nothing is selected and nothing is written to the clipboard
    assert!(
        !selection_active(&app),
        "a quick click must not create a selection"
    );
    assert!(
        clip_bytes(&clip).is_empty(),
        "a quick click must not write to the clipboard"
    );
}

// ---------------------------------------------------------------------
// Scenario 5: Esc clears an active selection without copying
// ---------------------------------------------------------------------

#[test]
fn esc_clears_an_active_selection_without_copying() {
    // @step Given an AgentView with an active text selection
    let (mut app, clip) = app_with_clipboard();
    seed_rows(&mut app, 40);
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;
    // Drive a Down + Drag to open a live (uncommitted) selection.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        3,
        rect_y + 1,
    ));
    drain(&mut app);
    render_app(&mut app, 80, 40);
    assert!(selection_active(&app), "precondition: selection is active");
    assert!(highlight_spans(&app) > 0, "precondition: highlight painted");
    // Nothing copied yet (no Up/Commit).
    assert!(clip_bytes(&clip).is_empty());

    // @step When I press Esc
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    drain(&mut app);

    // @step Then the highlight disappears
    assert!(!selection_active(&app), "Esc must clear the live selection");
    render_app(&mut app, 80, 40);
    assert_eq!(
        highlight_spans(&app),
        0,
        "Esc must remove the highlight overlay"
    );

    // @step And nothing is written to the clipboard by the Esc press
    assert!(clip_bytes(&clip).is_empty(), "Esc must not copy anything");
}

// ---------------------------------------------------------------------
// Scenario 6: Scrolling clears an active selection
// ---------------------------------------------------------------------

#[test]
fn scrolling_clears_an_active_selection() {
    // @step Given an AgentView with an active text selection
    let (mut app, _clip) = app_with_clipboard();
    seed_rows(&mut app, 200);
    render_app(&mut app, 80, 40);
    // Scroll up off the tail so there is room to scroll further, then
    // anchor a live selection via Down + Drag.
    app.dispatch(Action::ScrollbackPageUp);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 3, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        3,
        rect_y + 1,
    ));
    drain(&mut app);
    assert!(selection_active(&app), "precondition: selection is active");
    let offset_before = scroll_offset(&app);

    // @step When I scroll the transcript
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, rect_y + 4));
    drain(&mut app);

    // @step Then the selection is cleared and the highlight is removed
    assert!(
        !selection_active(&app),
        "scrolling must clear the live selection (rule [7])"
    );
    render_app(&mut app, 80, 40);
    assert_eq!(
        highlight_spans(&app),
        0,
        "scrolling must remove the highlight overlay"
    );

    // @step And the transcript scrolls
    let offset_after = scroll_offset(&app);
    assert!(
        offset_after < offset_before,
        "the transcript must scroll; before={offset_before} after={offset_after}"
    );
}

// ---------------------------------------------------------------------
// Scenario 7: Copying a line abutting the scrollbar excludes the
//             scrollbar glyph
// ---------------------------------------------------------------------

#[test]
fn copying_a_line_abutting_the_scrollbar_excludes_the_scrollbar_glyph() {
    // @step Given an AgentView whose answer line visually abuts the scrollbar gutter
    // Render at 80 wide: on overflow a 2-col gutter is reserved so
    // content_width = 78. Seed a first "answer" row exactly 78 chars wide
    // so it reaches the gutter edge, then enough filler rows to force the
    // scrollbar (and gutter) to appear.
    let (mut app, clip) = app_with_clipboard();
    let answer: String = "A".repeat(78);
    let _answer_seq = seed_line(&mut app, &answer);
    seed_rows(&mut app, 60); // filler: total rows > viewport ⇒ gutter reserved.
    render_app(&mut app, 80, 40);
    // Anchor the viewport at the very top so the answer row (visual row 0)
    // is the first visible row of the scrollback band.
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;

    // @step When I select that full line and release
    // Down at the start of the answer row, drag to its far content edge
    // (past the last content column, into the reserved gutter), release.
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Drag(MouseButton::Left), 79, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 79, rect_y));
    drain(&mut app);

    // @step Then the clipboard text contains the answer text but not the │ scrollbar glyph
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52(&answer),
        "the full answer line must be copied, clamped to the gutter-free content width"
    );
    // The │ (U+2502) is a 3-byte UTF-8 sequence \xe2\x94\x82; assert none
    // of those bytes leaked into the copied payload.
    assert!(
        !bytes.windows(3).any(|w| w == [0xe2, 0x94, 0x82]),
        "the scrollbar glyph │ must NOT appear in the copied text"
    );
}

// ---------------------------------------------------------------------
// Scenario 8: Mouse capture remains enabled throughout selection and
//             copy
// ---------------------------------------------------------------------

#[test]
fn mouse_capture_remains_enabled_throughout_selection_and_copy() {
    // @step Given an AgentView showing a multi-line transcript with mouse capture enabled
    let (mut app, clip) = app_with_clipboard();
    seed_rows(&mut app, 40);
    render_app(&mut app, 80, 40);
    app.dispatch(Action::ScrollbackHome);
    render_app(&mut app, 80, 40);
    let rect_y = 1u16;

    // @step When I complete a drag selection and copy
    let _ = app.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, rect_y));
    drain(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        79,
        rect_y + 1,
    ));
    drain(&mut app);
    let _ = app.handle_event(&mouse(
        MouseEventKind::Up(MouseButton::Left),
        79,
        rect_y + 1,
    ));
    drain(&mut app);

    // @step Then mouse capture was never disabled during the flow
    // Structural invariant (rule [8]): the COPY-006 selection flow issues
    // NO DisableMouseCapture / mouse-tracking toggle — its only side
    // effect is the OSC 52 clipboard write. The injected clipboard writer
    // is the SOLE sink the flow touches; assert it holds exactly the
    // OSC 52 copy bytes and carries no DisableMouseCapture escape
    // (CSI ?1000/1002/1003/1006 l) that a capture-disable would emit.
    let bytes = clip_bytes(&clip);
    assert_eq!(
        bytes,
        osc52("row-0\nrow-1"),
        "the copy must land on the clipboard sink"
    );
    // A DisableMouseCapture would write `ESC [ ? ... l` bytes; none of the
    // mouse-mode disable terminators may appear in the clipboard stream.
    assert!(
        !bytes.windows(3).any(|w| w == b"[?1"),
        "no DisableMouseCapture escape may be issued for the selection flow"
    );
    // And the selection is still live (capture stayed on, highlight kept).
    assert!(
        selection_active(&app),
        "capture never dropped, so the live selection persists after copy"
    );
}
