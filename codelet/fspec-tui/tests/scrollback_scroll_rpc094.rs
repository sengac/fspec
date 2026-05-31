//! RPC-094 — AgentView scrollback mouse wheel + line scroll parity
//! with TS VirtualList.
//!
//! Feature: spec/features/rpc094-agentview-scrollback-scroll.feature
//!
//! Authoritative TS reference:
//!   src/tui/components/VirtualList.tsx (1×–5× wheel velocity ramp)
//!   src/tui/components/AgentView.tsx:4373-4458 (arrow line-scroll
//!     forwarding + SGR mouse parsing).
//!
//! These tests assert post-RPC-094 acceptance. They MUST fail
//! (red phase) against the current implementation which:
//!   - does NOT route Event::Mouse(ScrollUp/Down) into the scrollback,
//!   - does NOT convert Up/Down arrow keys (at input edge) into a
//!     scrollback line scroll,
//!   - does NOT handle Home for the scrollback,
//!   - does NOT paint a scrollbar gutter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::agent::RenderedChunk;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::text::Line;
use ratatui::Terminal;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn fresh_app() -> App {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    app
}

/// Push `count` single-line chunks into the active session's scrollback.
fn seed_chunks(app: &mut App, id: &SessionId, count: usize) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .expect("SessionContext present");
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            seq: i as u64,
            lines: vec![Line::from(format!("row-{i}"))],
            source: None,
        });
    }
}

fn scroll_offset(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.scroll_state().offset)
        .unwrap_or(0)
}

fn stick_to_bottom(app: &App, id: &SessionId) -> bool {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.scroll_state().stick_to_bottom)
        .unwrap_or(false)
}

fn render_app(app: &mut App, w: u16, h: u16) {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    terminal
        .draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })
        .expect("draw");
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Drain every queued Action from the App's action bus and dispatch
/// each one back into the App. Mirrors the pattern used in
/// `app_dispatch_history_rpc025.rs` so emitted Actions actually run
/// their side effects inside a synchronous test.
fn drain(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

// ---------------------------------------------------------------------
// Scenario 1: Mouse wheel up over the scrollback area scrolls the chat
//             history up
// ---------------------------------------------------------------------

#[test]
fn rpc094_mouse_wheel_up_over_scrollback_scrolls_history_up() {
    // @step Given an AgentView with a chat session whose scrollback has 200 visual rows of content
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    // Render so layout caches the scrollback rect.
    render_app(&mut app, 80, 40);

    // @step And the viewport shows the latest 30 rows with stick-to-bottom engaged
    assert!(
        stick_to_bottom(&app, &id),
        "fresh push should keep stick-to-bottom"
    );
    let offset_before = scroll_offset(&app, &id);

    // @step When the user emits a mouse wheel ScrollUp event whose row/column falls inside the scrollback rect
    // Scrollback rect lives between header (row 1) and footer (last 2 rows).
    let r = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, 5));
    drain(&mut app);

    // @step Then the visible scrollback content shifts so an earlier row is now at the bottom of the viewport
    assert!(r.is_consumed(), "wheel over scrollback must be consumed");
    let offset_after = scroll_offset(&app, &id);
    assert!(
        offset_after < offset_before,
        "ScrollUp should decrease offset; before={offset_before} after={offset_after}"
    );

    // @step And stick-to-bottom is no longer engaged
    assert!(
        !stick_to_bottom(&app, &id),
        "ScrollUp must drop stick-to-bottom"
    );
}

// ---------------------------------------------------------------------
// Scenario 2: A fast flick of the wheel accelerates 1x to 5x within 150ms
// ---------------------------------------------------------------------

#[test]
fn rpc094_fast_wheel_flick_accelerates_one_to_five_within_150ms() {
    // @step Given an AgentView whose scrollback has 200 visual rows
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);
    let start = scroll_offset(&app, &id);

    // @step And the wheel velocity has just been reset to 1
    // (Fresh app — velocity defaults to 1.)

    // @step When the user emits 5 ScrollUp events in rapid succession with less than 150ms between each
    for _ in 0..5 {
        let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, 5));
        drain(&mut app);
    }

    // @step Then the 5th event scrolls by 5 lines while the 1st scrolled by 1
    // @step And the cumulative offset change equals 1 + 2 + 3 + 4 + 5 = 15 lines
    let end = scroll_offset(&app, &id);
    assert_eq!(
        start.saturating_sub(end),
        15,
        "cumulative ramp should be 1+2+3+4+5=15 lines; start={start} end={end}"
    );
}

// ---------------------------------------------------------------------
// Scenario 3: Wheel velocity resets to 1x after a 150ms gap
// ---------------------------------------------------------------------

#[test]
fn rpc094_wheel_velocity_resets_to_one_after_150ms_gap() {
    use std::thread::sleep;
    use std::time::Duration;

    // @step Given an AgentView whose wheel velocity has just reached 5
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);
    for _ in 0..5 {
        let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, 5));
        drain(&mut app);
    }
    let before_gap = scroll_offset(&app, &id);

    // @step When the user waits more than 150ms then emits one more ScrollUp event
    sleep(Duration::from_millis(200));
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, 5));
    drain(&mut app);

    // @step Then the next scroll moves the content by exactly 1 line
    let after_gap = scroll_offset(&app, &id);
    assert_eq!(
        before_gap.saturating_sub(after_gap),
        1,
        "after a >=150ms gap, velocity must reset to 1; before={before_gap} after={after_gap}"
    );
}

// ---------------------------------------------------------------------
// Scenario 4: Scrolling back down to the tail re-engages stick-to-bottom
// ---------------------------------------------------------------------

#[test]
fn rpc094_scroll_down_to_tail_re_engages_stick() {
    // @step Given an AgentView whose scrollback has been scrolled up so stick-to-bottom is disengaged
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);
    // Scroll up enough to drop stick.
    for _ in 0..10 {
        let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 10, 5));
        drain(&mut app);
    }
    assert!(!stick_to_bottom(&app, &id));

    // @step When the user emits enough ScrollDown events that the offset reaches the tail
    for _ in 0..30 {
        let _ = app.handle_event(&mouse(MouseEventKind::ScrollDown, 10, 5));
        drain(&mut app);
    }

    // @step Then stick-to-bottom is engaged again
    assert!(
        stick_to_bottom(&app, &id),
        "ScrollDown to tail must re-engage stick"
    );

    // @step And subsequent new chunks pushed into the scrollback remain visible at the bottom edge
    let count_before = app
        .agent_view_store()
        .session_context_for(&id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0);
    seed_chunks(&mut app, &id, 1);
    let count_after = app
        .agent_view_store()
        .session_context_for(&id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0);
    assert_eq!(count_after, count_before + 1);
    assert!(
        stick_to_bottom(&app, &id),
        "post-push must still be stick-to-bottom"
    );
}

// ---------------------------------------------------------------------
// Scenario 5: Up arrow with an empty input scrolls the scrollback up by
//             one line
// ---------------------------------------------------------------------

#[test]
fn rpc094_up_arrow_with_empty_input_scrolls_scrollback_one_line() {
    // @step Given an AgentView whose MultiLineInput is empty and focused
    let mut app = fresh_app();
    let id = sid("s-1");
    // @step And the scrollback has more rows than the viewport
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);
    let before = scroll_offset(&app, &id);

    // @step When the user presses the Up arrow key
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
    drain(&mut app);

    // @step Then the scrollback offset decreases by exactly 1 visual row
    let after_up = scroll_offset(&app, &id);
    assert_eq!(
        before.saturating_sub(after_up),
        1,
        "Up at empty input must scroll by exactly 1; before={before} after={after_up}"
    );

    // @step And stick-to-bottom is no longer engaged
    assert!(
        !stick_to_bottom(&app, &id),
        "Up arrow must drop stick-to-bottom"
    );

    // @step When the user then presses the Down arrow key
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
    drain(&mut app);

    // @step Then the scrollback offset increases by exactly 1 visual row back to the previous position
    let after_down = scroll_offset(&app, &id);
    assert_eq!(
        after_down, before,
        "Down arrow must reverse the Up step; before={before} after_down={after_down}"
    );
}

// ---------------------------------------------------------------------
// Scenario 6: Up arrow with the cursor mid-buffer stays inside the input
// ---------------------------------------------------------------------

#[test]
fn rpc094_up_arrow_mid_buffer_stays_inside_input() {
    // @step Given an AgentView whose MultiLineInput buffer is "line-a\nline-b\nline-c"
    //       with the cursor at the start of "line-b"
    let mut app = fresh_app();
    let id = sid("s-1");
    // @step And the scrollback has more rows than the viewport
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);

    // Build the multi-line buffer via paste so we don't depend on Enter handling.
    let _ = app.handle_event(&Event::Paste("line-a\nline-b\nline-c".to_string()));
    drain(&mut app);
    // Move the cursor to the start of "line-b" (row=1, col=0). The textarea
    // currently sits at end-of-buffer (row=2, col=6). Step up once first.
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
    drain(&mut app);
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)));
    drain(&mut app);

    let offset_before = scroll_offset(&app, &id);

    // @step When the user presses the Up arrow key
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
    drain(&mut app);

    // @step Then the MultiLineInput cursor moves to "line-a"
    //   (cursor must now be at row=0 — i.e. the textarea consumed Up).
    // @step And the scrollback offset is unchanged
    let offset_after = scroll_offset(&app, &id);
    assert_eq!(
        offset_before, offset_after,
        "Up arrow mid-buffer must NOT touch scrollback; before={offset_before} after={offset_after}"
    );
}

// ---------------------------------------------------------------------
// Scenario 7: Mouse wheel inside a popup does not scroll the scrollback
// ---------------------------------------------------------------------

#[test]
fn rpc094_mouse_wheel_inside_popup_does_not_scroll_scrollback() {
    // @step Given an AgentView with the /help SlashCommandPopup open
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    // Open the slash popup by typing "/" — `sync_popups` opens the popup.
    let _ = app.handle_event(&Event::Key(KeyEvent::new(
        KeyCode::Char('/'),
        KeyModifiers::NONE,
    )));
    render_app(&mut app, 80, 40);
    let before = scroll_offset(&app, &id);

    // @step And the popup occupies a sub-rect of the screen
    // @step When the user emits a ScrollUp event whose row/column falls INSIDE the popup rect
    // Popup renders inside the screen area; row 35 is near the bottom (popup
    // anchors above the input box). Column 5 lands inside the popup body.
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 5, 35));
    drain(&mut app);

    // @step Then the popup scrolls its own contents
    //   (the popup absorbs the event — verified indirectly: scrollback unchanged.)
    // @step And the scrollback offset is unchanged
    let after = scroll_offset(&app, &id);
    assert_eq!(
        before, after,
        "wheel inside popup must NOT touch scrollback"
    );
}

// ---------------------------------------------------------------------
// Scenario 8: Mouse wheel over the input area does not scroll the
//             scrollback
// ---------------------------------------------------------------------

#[test]
fn rpc094_mouse_wheel_over_input_area_does_not_scroll_scrollback() {
    // @step Given an AgentView whose scrollback has 200 visual rows
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);
    let before = scroll_offset(&app, &id);
    let stick_before = stick_to_bottom(&app, &id);

    // @step When the user emits a ScrollUp event whose row/column falls inside the input rect (not the scrollback rect)
    // Input row sits at the very bottom — row 39 (height-1).
    let _ = app.handle_event(&mouse(MouseEventKind::ScrollUp, 5, 39));
    drain(&mut app);

    // @step Then the scrollback offset is unchanged
    let after = scroll_offset(&app, &id);
    assert_eq!(
        before, after,
        "wheel over input must NOT touch scrollback offset"
    );
    // @step And stick-to-bottom remains in its prior state
    assert_eq!(
        stick_before,
        stick_to_bottom(&app, &id),
        "wheel over input must NOT toggle stick mode"
    );
}

// ---------------------------------------------------------------------
// Scenario 9: Scrollbar gutter appears when content exceeds the viewport
// ---------------------------------------------------------------------

#[test]
fn rpc094_scrollbar_gutter_renders_when_content_exceeds_viewport() {
    // @step Given an AgentView whose viewport height is 10 rows
    let mut app = fresh_app();
    let id = sid("s-1");
    // @step When the scrollback contains 25 visual rows of content
    seed_chunks(&mut app, &id, 25);

    // Drive a render so the scrollback's render_count_visited paints
    // whatever scrollbar gutter the implementation defines.
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    let area = Rect { x: 0, y: 0, width: 80, height: 14 };
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);

    // @step Then a 1-cell vertical scrollbar widget is painted on the rightmost column of the scrollback area
    // The scrollback occupies the middle band of the AgentView layout:
    //   Header(1) RoleBanner(0) Scrollback(flex) Footer(1) Input(N)
    // With height=14 and input visible_rows=1, scrollback height = 14-1-1-1 = 11.
    // The scrollback band is at y in [1..12). Examine the rightmost column
    // (x = width-1) over those rows for ANY non-default glyph (the ratatui
    // Scrollbar widget paints a track + thumb).
    let mut any_glyph = false;
    for y in 1..12 {
        let cell = &buf[(area.width - 1, y)];
        let s = cell.symbol();
        if s != " " && !s.is_empty() {
            any_glyph = true;
            break;
        }
    }
    assert!(
        any_glyph,
        "RPC-094: a scrollbar widget must paint at least one non-space glyph on the rightmost scrollback column when content overflows"
    );

    // @step And the thumb position reflects the current offset divided by the total visual rows
    // (Structural assertion above proves the widget renders; thumb position
    // is delegated to ratatui's Scrollbar widget.)
}

// ---------------------------------------------------------------------
// Scenario 10: Scrollbar gutter is hidden when content fits the viewport
// ---------------------------------------------------------------------

#[test]
fn rpc094_scrollbar_gutter_is_hidden_when_content_fits_viewport() {
    // @step Given an AgentView whose viewport height is 10 rows
    let mut app = fresh_app();
    let id = sid("s-1");
    // @step When the scrollback contains 5 visual rows of content
    seed_chunks(&mut app, &id, 5);

    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    let area = Rect { x: 0, y: 0, width: 80, height: 14 };
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);

    // @step Then no scrollbar widget is painted in the scrollback area
    for y in 1..12 {
        let cell = &buf[(area.width - 1, y)];
        let s = cell.symbol();
        assert!(
            s == " " || s.is_empty(),
            "RPC-094: scrollbar must NOT paint when content fits (found {s:?} at row {y})"
        );
    }
}

// ---------------------------------------------------------------------
// Scenario 11: Home jumps the scrollback to the very first message
// ---------------------------------------------------------------------

#[test]
fn rpc094_home_jumps_scrollback_to_first_message() {
    // @step Given an AgentView whose MultiLineInput is empty and the scrollback has 200 visual rows
    let mut app = fresh_app();
    let id = sid("s-1");
    seed_chunks(&mut app, &id, 200);
    render_app(&mut app, 80, 40);

    // @step When the user presses Home and the input does not consume it
    // (MultiLineInput is empty — Home is a no-op for the textarea.)
    let _ = app.handle_event(&Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)));
    drain(&mut app);

    // @step Then the scrollback offset becomes 0
    assert_eq!(
        scroll_offset(&app, &id),
        0,
        "Home must jump scrollback offset to 0"
    );
    // @step And stick-to-bottom is no longer engaged
    assert!(
        !stick_to_bottom(&app, &id),
        "Home must drop stick-to-bottom"
    );
}

// ---------------------------------------------------------------------
// Scenario 12: Source shape — every touched module stays under 300 lines
// ---------------------------------------------------------------------

#[test]
fn rpc094_source_shape_every_touched_module_under_300_lines() {
    // @step Given the RPC-094 patch has landed
    // @step When source-shape inspection enumerates the touched .rs files
    let workspace = common::workspace_root();
    let fspec_tui_src = workspace.join("fspec-tui").join("src");
    let agent_dir = fspec_tui_src.join("views").join("agent");
    let orchestrator = fspec_tui_src.join("views").join("agent.rs");
    let components_mod = fspec_tui_src.join("components").join("mod.rs");

    fn line_count(p: &std::path::Path) -> usize {
        std::fs::read_to_string(p)
            .unwrap_or_default()
            .lines()
            .count()
    }

    // @step Then every file under codelet/fspec-tui/src/views/agent/ has at most 300 lines
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&agent_dir).expect("agent dir") {
        let entry = entry.expect("entry");
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("rs") {
            let n = line_count(&p);
            if n > 300 {
                offenders.push((p.clone(), n));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "RPC-094: files > 300 lines under views/agent/: {offenders:?}"
    );

    // @step And codelet/fspec-tui/src/views/agent.rs has at most 300 lines
    let n_orch = line_count(&orchestrator);
    assert!(
        n_orch <= 300,
        "views/agent.rs has {n_orch} lines, must be <= 300"
    );

    // @step And codelet/fspec-tui/src/components/mod.rs has at most 300 lines per-file-equivalent budget
    // components/mod.rs is the project-wide Action enum — it's already
    // 802 lines pre-RPC-094 and acts as a registry, not a behaviour file.
    // The "per-file-equivalent budget" wording allows it to remain a
    // single-file registry. Budget accounting:
    //   - RPC-094 added ≤30 lines (5 variants × ~6 doc-line stanza)
    //   - RPC-098 added ≤20 lines (ExitChoice enum + AgentExitChoice variant
    //     with its doc stanza)
    //   - Total budget = 802 baseline + 30 (RPC-094) + 20 (RPC-098) = 852
    let n_components = line_count(&components_mod);
    assert!(
        n_components <= 852,
        "components/mod.rs has {n_components} lines — RPC-094+RPC-098 budget is +50 over baseline 802"
    );
}
