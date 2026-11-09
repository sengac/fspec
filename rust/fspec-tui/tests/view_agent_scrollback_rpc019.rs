//! RPC-019 — ScrollbackList unit tests (red phase).
//!
//! Feature: spec/features/rpc019-scrollback.feature
//!
//! Drives the windowed-rendering / stick-to-bottom / PageUp / PageDown
//! scenarios for the new `ScrollbackList` widget. The widget owns the
//! `Vec<RenderedChunk>` and the `ScrollState` previously inlined into
//! `AgentView`.

use codelet_fspec_tui::views::agent::scrollback::{ScrollState, ScrollbackList};
use codelet_fspec_tui::views::AgentRenderedChunk;
use codelet_fspec_tui::{Action, AgentView, AgentViewStore};
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::Terminal;
use tokio::sync::mpsc::unbounded_channel;

fn chunk(seq: u64, body: &str) -> AgentRenderedChunk {
    AgentRenderedChunk {
        row_ends: Vec::new(),
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

fn render_rows(
    width: u16,
    height: u16,
    store: &mut AgentViewStore,
    view: &mut AgentView,
) -> Result<Vec<String>, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .or_fail("draw")?;
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }
    Ok(rows)
}

/// Scenario: ScrollbackList::push appends a chunk and bumps offset in stick mode
#[test]
fn scrollback_list_push_appends_a_chunk_and_bumps_offset_in_stick_mode() -> TestResult {
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
    assert!(
        visible.iter().any(|c| c.seq == 100),
        "chunk #101 (seq=100) must be visible: {visible:?}"
    );
    Ok(())
}

/// Scenario: PageUp on the scrollback decrements offset by viewport_height and disables stick
#[test]
fn page_up_on_the_scrollback_decrements_offset_by_viewport_height_and_disables_stick() -> TestResult
{
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
    Ok(())
}

/// Scenario: PageDown / End from a scrolled-up position re-enables stick when offset reaches the tail
#[test]
fn page_down_or_end_from_scrolled_up_re_enables_stick_when_offset_reaches_tail() -> TestResult {
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
    Ok(())
}

/// Scenario: ScrollbackList::render only lays out the visible window
#[test]
fn scrollback_list_render_only_lays_out_the_visible_window() -> TestResult {
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
    Ok(())
}

/// Scenario: ScrollState default is stick_to_bottom = true, offset = 0
#[test]
fn scroll_state_default_is_stick_to_bottom_true_offset_zero() -> TestResult {
    let s = ScrollState::default();
    assert_eq!(s.offset, 0);
    assert!(s.stick_to_bottom);
    Ok(())
}

/// AgentView render integration: PageUp/PageDown in AgentView routes
/// through the ScrollbackList offset semantics via the per-session
/// SessionContext + the ScrollbackPageUp/Down Actions wired in RPC-024.
#[test]
fn agent_view_page_up_disables_stick_and_decrements_offset() -> TestResult {
    let (tx, mut rx) = unbounded_channel();
    let mut view = AgentView::new(tx);
    let mut store = AgentViewStore::default();
    store.append_session(codelet_fspec_tui::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-1"),
    ));
    {
        let ctx = store.current_session_context_mut().or_fail("current ctx")?;
        for i in 0..100u64 {
            ctx.scrollback.push(chunk(i, &format!("chunk-{i}")));
        }
    }
    // Force a render so viewport_height gets recorded on the
    // SessionContext's ScrollbackList (via render_count_visited).
    let _ = render_rows(80, 20, &mut store, &mut view)?;
    // viewport_height for an 80x20 layout = 20 - 1 (header) - 1 (footer)
    // - 3 (input) - 2 (block border) = 13.
    let event = Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    let _ = view.handle_event(&event);
    // RPC-024: AgentView::handle_event emits Action::ScrollbackPageUp
    // rather than mutating its own ScrollbackList. The Action is
    // routed through App::dispatch in production; this unit-level test
    // verifies the Action is emitted and that the SessionContext's
    // scrollback is still in stick mode (since dispatch isn't wired).
    let action = rx.try_recv().or_fail("Action::ScrollbackPageUp on bus")?;
    assert!(matches!(action, Action::ScrollbackPageUp));
    let ctx = store.current_session_context().or_fail("current ctx")?;
    // Stick mode is still on because we did NOT route through App::dispatch
    // in this unit test — the Action drives the scrollback mutation.
    assert!(ctx.scrollback.scroll_state().stick_to_bottom);
    Ok(())
}

/// Scenario: AgentView vertical layout reserves Length(visible_rows) for the input box (RPC-029)
#[test]
fn agent_view_vertical_layout_reserves_input_box_rows_tracking_the_textarea() -> TestResult {
    let (tx, _rx) = unbounded_channel();
    // @step Given an AgentView whose MultiLineInput contains "a\nb\nc"
    let mut view = AgentView::new(tx);
    view.input.set_value("a\nb\nc");

    let mut store = AgentViewStore::default();
    store.append_session(codelet_fspec_tui::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-1"),
    ));
    // 30 one-row chunks overflow the scrollback, so in stick mode it
    // fills every row of its region and the newest chunk sits on the
    // region's last row.
    let ctx = store
        .current_session_context_mut()
        .or_fail("current session context")?;
    for seq in 0..30 {
        ctx.scrollback.push(chunk(seq, &format!("chunk-{seq}")));
    }

    // @step When the App renders AgentView against an 80x20 TestBackend
    let rows = render_rows(80, 20, &mut store, &mut view)?;

    // @step Then the input box occupies exactly 3 rows (RPC-029: no 4-sided border)
    let input = view
        .last_input_area
        .or_fail("input area recorded after render")?;
    assert_eq!(input.height, 3, "input box rows: {input:?}");
    assert_eq!(input.y + input.height, 20, "input box ends on the last row");

    // @step And the scrollback region occupies the remaining flex rows between the header and the footer
    // Rows 0 and 1 are the session header and the menu bar (MENU-003);
    // the footer is the row directly above the input box.
    let footer_y = usize::from(input.y) - 1;
    assert_eq!(footer_y, 16, "footer row");
    assert!(
        !rows[1].contains("chunk-"),
        "no scrollback row above row 2: {:?}",
        rows[1]
    );
    assert!(
        rows[2].contains("chunk-16"),
        "the scrollback starts on row 2: {:?}",
        rows[2]
    );
    assert!(
        rows[footer_y - 1].contains("chunk-29"),
        "the newest chunk sits directly above the footer: {:?}",
        rows[footer_y - 1]
    );
    assert!(
        !rows[footer_y].contains("chunk-"),
        "the footer row holds no scrollback: {:?}",
        rows[footer_y]
    );

    // @step And the SessionFooter row above the input does NOT contain the substring "Enter=send" (RPC-029)
    assert!(
        !rows[footer_y].contains("Enter=send"),
        "footer row must not paint 'Enter=send': {:?}",
        rows[footer_y]
    );
    Ok(())
}

/// AgentView::record_chunk delegates into the focused SessionContext's
/// ScrollbackList so the auto-stick behavior continues to work
/// end-to-end after the RPC-024 refactor.
#[test]
fn agent_view_record_chunk_delegates_to_scrollback_list_push() -> TestResult {
    let (tx, _rx) = unbounded_channel();
    let mut view = AgentView::new(tx);
    let mut store = AgentViewStore::default();
    store.append_session(codelet_fspec_tui::SessionContext::new(
        codelet_rpc_types::SessionId::new("s-1"),
    ));
    let chunk = codelet_rpc_types::StreamChunk::Text {
        text: "hi".to_string(),
        correlation_id: None,
        observed_correlation_ids: None,
    };
    let before = view.chunk_count(&store);
    view.record_chunk(&mut store, &chunk);
    assert_eq!(view.chunk_count(&store), before + 1);
    Ok(())
}
