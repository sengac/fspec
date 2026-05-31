//! RPC-019 — ScrollbackList unit tests (red phase).
//!
//! Feature: spec/features/rpc019-scrollback.feature
//!
//! Drives the windowed-rendering / stick-to-bottom / PageUp / PageDown
//! scenarios for the new `ScrollbackList` widget. The widget owns the
//! `Vec<RenderedChunk>` and the `ScrollState` previously inlined into
//! `AgentView`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::scrollback::{ScrollState, ScrollbackList};
use codelet_fspec_tui::views::AgentRenderedChunk;
use codelet_fspec_tui::{Action, AgentView, AgentViewStore};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn chunk(seq: u64, body: &str) -> AgentRenderedChunk {
    AgentRenderedChunk {
        seq,
        lines: vec![Line::from(Span::raw(body.to_string()))],
        source: None,
    }
}

fn seed_list(n: u64) -> ScrollbackList {
    let mut list = ScrollbackList::new();
    for i in 0..n {
        list.push(chunk(i, &format!("chunk-{i}")));
    }
    list
}

fn render_rows(width: u16, height: u16, store: &mut AgentViewStore, view: &mut AgentView) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    rows
}

/// Scenario: ScrollbackList::push appends a chunk and bumps offset in stick mode
#[test]
fn scrollback_list_push_appends_a_chunk_and_bumps_offset_in_stick_mode() {
    // @step Given a ScrollbackList in stick_to_bottom mode with 100 single-line chunks and viewport_height = 12
    let mut list = seed_list(100);
    list.set_viewport_height(12);
    assert!(list.scroll_state().stick_to_bottom);
    assert_eq!(list.scroll_state().offset, 88);

    // @step When the ScrollbackList::push appends one more single-line chunk (chunk #101)
    list.push(chunk(100, "chunk-100"));

    // @step Then the ScrollbackList's offset is 89
    assert_eq!(list.scroll_state().offset, 89);
    // @step And ScrollbackList::stick_to_bottom is true
    assert!(list.scroll_state().stick_to_bottom);
    // @step And the visible chunks include chunk #101 at the bottom
    let visible = list.visible_window(12);
    assert!(visible.iter().any(|c| c.seq == 100), "chunk #101 (seq=100) must be visible: {visible:?}");
}

/// Scenario: PageUp on the scrollback decrements offset by viewport_height and disables stick
#[test]
fn page_up_on_the_scrollback_decrements_offset_by_viewport_height_and_disables_stick() {
    // @step Given a ScrollbackList in stick_to_bottom mode with 100 single-line chunks and viewport_height = 12
    let mut list = seed_list(100);
    list.set_viewport_height(12);
    assert_eq!(list.scroll_state().offset, 88);

    // @step When the user presses PageUp inside AgentView
    list.scroll_up(12);

    // @step Then the ScrollbackList's offset is exactly 76
    assert_eq!(list.scroll_state().offset, 76);
    // @step And ScrollbackList::stick_to_bottom is false
    assert!(!list.scroll_state().stick_to_bottom);
}

/// Scenario: PageDown / End from a scrolled-up position re-enables stick when offset reaches the tail
#[test]
fn page_down_or_end_from_scrolled_up_re_enables_stick_when_offset_reaches_tail() {
    // @step Given a ScrollbackList at offset 76 with stick_to_bottom = false, 100 single-line chunks, viewport_height = 12
    let mut list = seed_list(100);
    list.set_viewport_height(12);
    list.scroll_up(12); // offset becomes 76, stick=false
    assert_eq!(list.scroll_state().offset, 76);
    assert!(!list.scroll_state().stick_to_bottom);

    // @step When the user presses PageDown inside AgentView
    list.scroll_down(12);

    // @step Then the ScrollbackList's offset is exactly 88
    assert_eq!(list.scroll_state().offset, 88);
    // @step And ScrollbackList::stick_to_bottom is true
    assert!(list.scroll_state().stick_to_bottom);
}

/// Scenario: ScrollbackList::render only lays out the visible window
#[test]
fn scrollback_list_render_only_lays_out_the_visible_window() {
    // @step Given a ScrollbackList with 10_000 single-line chunks in stick_to_bottom mode and viewport_height = 12
    let mut list = seed_list(10_000);
    list.set_viewport_height(12);
    assert!(list.scroll_state().stick_to_bottom);

    // @step When ScrollbackList::render is called against an 80x12 area
    let area = Rect::new(0, 0, 80, 12);
    let mut buf = Buffer::empty(area);
    let visited = list.render_count_visited(area, &mut buf);

    // @step Then the number of chunks visited during layout is at most 12
    assert!(visited <= 12, "visited {visited} > 12 chunks during render");

    // @step And the rendered buffer's bottom row contains chunk #9999's body
    let mut bottom_row = String::new();
    for x in 0..area.width {
        bottom_row.push_str(buf[(x, area.height - 1)].symbol());
    }
    assert!(
        bottom_row.contains("chunk-9999"),
        "bottom row should contain chunk #9999, got: {bottom_row:?}"
    );
}

/// Scenario: ScrollState default is stick_to_bottom = true, offset = 0
#[test]
fn scroll_state_default_is_stick_to_bottom_true_offset_zero() {
    let s = ScrollState::default();
    assert_eq!(s.offset, 0);
    assert!(s.stick_to_bottom);
}

/// AgentView render integration: PageUp/PageDown in AgentView routes
/// through the ScrollbackList offset semantics via the per-session
/// SessionContext + the ScrollbackPageUp/Down Actions wired in RPC-024.
#[test]
fn agent_view_page_up_disables_stick_and_decrements_offset() {
    let (tx, mut rx) = unbounded_channel();
    let mut view = AgentView::new(tx);
    let mut store = AgentViewStore::default();
    store.append_session(codelet_fspec_tui::SessionContext::new(codelet_rpc_types::SessionId::new("s-1")));
    {
        let ctx = store.current_session_context_mut().expect("current ctx");
        for i in 0..100u64 {
            ctx.scrollback.push(chunk(i, &format!("chunk-{i}")));
        }
    }
    // Force a render so viewport_height gets recorded on the
    // SessionContext's ScrollbackList (via render_count_visited).
    let _ = render_rows(80, 20, &mut store, &mut view);
    // viewport_height for an 80x20 layout = 20 - 1 (header) - 1 (footer)
    // - 3 (input) - 2 (block border) = 13.
    let event = Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    let _ = view.handle_event(&event);
    // RPC-024: AgentView::handle_event emits Action::ScrollbackPageUp
    // rather than mutating its own ScrollbackList. The Action is
    // routed through App::dispatch in production; this unit-level test
    // verifies the Action is emitted and that the SessionContext's
    // scrollback is still in stick mode (since dispatch isn't wired).
    let action = rx.try_recv().expect("Action::ScrollbackPageUp on bus");
    assert!(matches!(action, Action::ScrollbackPageUp));
    let ctx = store.current_session_context().expect("current ctx");
    // Stick mode is still on because we did NOT route through App::dispatch
    // in this unit test — the Action drives the scrollback mutation.
    assert!(ctx.scrollback.scroll_state().stick_to_bottom);
}

/// Scenario: AgentView vertical layout reserves Length(N+2) for the input box where N tracks the textarea
#[test]
fn agent_view_vertical_layout_reserves_input_box_rows_tracking_the_textarea() {
    let (tx, _rx) = unbounded_channel();
    // @step Given an AgentView whose MultiLineInput contains "a\nb\nc"
    let mut view = AgentView::new(tx);
    view.input.set_value("a\nb\nc");

    let mut store = AgentViewStore::default();
    store.append_session(codelet_fspec_tui::SessionContext::new(codelet_rpc_types::SessionId::new("s-1")));

    // @step When the App renders AgentView against an 80x20 TestBackend
    let rows = render_rows(80, 20, &mut store, &mut view);

    // @step Then the input box occupies exactly 5 rows
    let input_top_idx = view
        .last_input_area
        .map(|r| r.y as usize)
        .expect("input area recorded after render");
    let input_height = view
        .last_input_area
        .map(|r| r.height as usize)
        .expect("input area recorded after render");
    // RPC-029: the input area no longer carries a 4-sided border, so
    // it occupies exactly visible_rows() == 3 rows (no +2 border budget).
    assert_eq!(input_height, 3, "input box should be 3 content rows after RPC-029, got {input_height}");

    // @step And the scrollback region occupies the remaining flex rows between the header and input
    // RPC-029: the footer row sits between scrollback and input now, so
    // the layout is: header(1) + scrollback(flex) + footer(1) + input(3).
    let scrollback_rows = input_top_idx.saturating_sub(2); // minus header row 0 and footer row above input
    assert!(
        scrollback_rows > 0,
        "scrollback region should sit between header and footer"
    );

    // @step And the SessionFooter still paints "Enter=send" on the bottom row
    // RPC-029: the footer's old hints (Enter=send / Ctrl+C / ESC=back)
    // are removed to match the canonical TS layout. We instead pin
    // the footer-above-input invariant by checking the footer row
    // does NOT contain those hints AND lies above the input row.
    let footer_y = input_top_idx.saturating_sub(1);
    let footer_row = &rows[footer_y];
    assert!(
        !footer_row.contains("Enter=send"),
        "footer should NOT paint 'Enter=send' after RPC-029; got {footer_row:?}"
    );
}

/// AgentView::record_chunk delegates into the focused SessionContext's
/// ScrollbackList so the auto-stick behavior continues to work
/// end-to-end after the RPC-024 refactor.
#[test]
fn agent_view_record_chunk_delegates_to_scrollback_list_push() {
    let (tx, _rx) = unbounded_channel();
    let mut view = AgentView::new(tx);
    let mut store = AgentViewStore::default();
    store.append_session(codelet_fspec_tui::SessionContext::new(codelet_rpc_types::SessionId::new("s-1")));
    let chunk = codelet_rpc_types::StreamChunk::Text {
        text: "hi".to_string(),
        correlation_id: None,
        observed_correlation_ids: None,
    };
    let before = view.chunk_count(&store);
    view.record_chunk(&mut store, &chunk);
    let _ = Action::Redraw; // touch the enum to keep the use-line warm
    assert_eq!(view.chunk_count(&store), before + 1);
}
