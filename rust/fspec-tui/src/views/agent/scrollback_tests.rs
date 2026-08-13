//! Unit tests for `ScrollbackList`. Lives in a sibling file via
//! `#[path = "scrollback_tests.rs"] mod tests;` so the production
//! widget file stays under the 300-LoC ceiling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use ratatui::text::{Line, Span};

fn chunk(seq: u64, body: &str) -> RenderedChunk {
    RenderedChunk {
        seq,
        lines: vec![Line::from(Span::raw(body.to_string()))],
        source: None,
    }
}

#[test]
fn default_state_is_offset_zero_stick_true() {
    let s = ScrollState::default();
    assert_eq!(s.offset, 0);
    assert!(s.stick_to_bottom);
}

#[test]
fn push_with_stick_mode_keeps_latest_chunks_visible() {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(3);
    for i in 0..10 {
        list.push(chunk(i, &format!("c{i}")));
    }
    assert!(list.scroll_state().stick_to_bottom);
    assert_eq!(list.scroll_state().offset, 7);
}

#[test]
fn scroll_up_disables_stick_and_caps_at_zero() {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(3);
    for i in 0..10 {
        list.push(chunk(i, "x"));
    }
    list.scroll_up(2);
    assert_eq!(list.scroll_state().offset, 5);
    assert!(!list.scroll_state().stick_to_bottom);
    list.scroll_up(100);
    assert_eq!(list.scroll_state().offset, 0);
}

#[test]
fn scroll_down_caps_at_max_and_re_enables_stick() {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(3);
    for i in 0..10 {
        list.push(chunk(i, "x"));
    }
    list.scroll_up(5);
    list.scroll_down(5);
    assert_eq!(list.scroll_state().offset, 7);
    assert!(list.scroll_state().stick_to_bottom);
}

// ────────────────────────────────────────────────────────────────────
// RPC-381 — turn-selection (SELECT) mode unit tests.
//
// Feature: spec/features/agentview-turn-select-mode.feature
//
// These exercise the ScrollbackList SELECT-mode surface described in
// the RPC-381 design doc (§4.2): SelectionMode { Scroll, Item }, a
// TurnDir { Up, Down }, and the methods enter_item_mode /
// exit_item_mode / select_last_turn / navigate_turn / selected_index /
// selected_seq. They are EXPECTED to fail (compile error) until the
// production surface lands — that is the correct red state.
// ────────────────────────────────────────────────────────────────────

/// Build a 3-turn list keyed by seq 0,1,2.
fn three_turn_list() -> ScrollbackList {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(10);
    for i in 0..3 {
        list.push(chunk(i, &format!("turn-{i}")));
    }
    list
}

#[test]
fn rpc381_default_selection_mode_is_scroll_with_no_selection() {
    let list = ScrollbackList::new();
    assert!(matches!(list.selection_mode(), SelectionMode::Scroll));
    assert_eq!(list.selected_index(), None);
    assert_eq!(list.selected_seq(), None);
}

#[test]
fn rpc381_enter_item_mode_selects_last_turn() {
    // Entering item mode auto-selects the most-recent (last) turn.
    let mut list = three_turn_list();
    list.enter_item_mode();
    assert!(matches!(list.selection_mode(), SelectionMode::Item));
    assert_eq!(list.selected_index(), Some(2));
    assert_eq!(list.selected_seq(), Some(2));
}

#[test]
fn rpc381_exit_item_mode_clears_selection_and_returns_to_scroll() {
    let mut list = three_turn_list();
    list.enter_item_mode();
    list.exit_item_mode();
    assert!(matches!(list.selection_mode(), SelectionMode::Scroll));
    assert_eq!(list.selected_index(), None);
    assert_eq!(list.selected_seq(), None);
}

#[test]
fn rpc381_navigate_up_selects_previous_turn() {
    let mut list = three_turn_list();
    list.enter_item_mode(); // selected = 2 (last)
    list.navigate_turn(TurnDir::Up);
    assert_eq!(list.selected_index(), Some(1));
    assert_eq!(list.selected_seq(), Some(1));
}

#[test]
fn rpc381_navigate_up_clamps_at_first_turn() {
    let mut list = three_turn_list();
    list.enter_item_mode();
    list.navigate_turn(TurnDir::Up); // 2 -> 1
    list.navigate_turn(TurnDir::Up); // 1 -> 0
    list.navigate_turn(TurnDir::Up); // 0 -> 0 (clamp)
    assert_eq!(list.selected_index(), Some(0));
}

#[test]
fn rpc381_navigate_down_clamps_at_last_turn() {
    let mut list = three_turn_list();
    list.enter_item_mode(); // selected = 2 (last)
    list.navigate_turn(TurnDir::Down); // 2 -> 2 (clamp)
    assert_eq!(list.selected_index(), Some(2));
}

#[test]
fn rpc381_selection_stays_pinned_to_seq_when_new_chunk_streams_in() {
    // Select the SECOND of three turns, then stream a new turn in.
    // The selection must stay pinned to the originally-selected seq.
    let mut list = three_turn_list();
    list.enter_item_mode(); // selected = index 2, seq 2
    list.navigate_turn(TurnDir::Up); // selected = index 1, seq 1
    assert_eq!(list.selected_seq(), Some(1));

    // Agent streams a new turn (seq 3) into the scrollback.
    list.push(chunk(3, "turn-3"));

    // The selection stays on the originally-selected seq=1 turn (still
    // index 1 because the new chunk was appended after it).
    assert_eq!(list.selected_seq(), Some(1));
    assert_eq!(list.selected_index(), Some(1));
}

#[test]
fn rpc381_item_mode_render_frames_selected_turn_with_gray_arrow_bars() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    // Select the middle turn so there is room above AND below for the
    // ▼ (top) and ▲ (bottom) arrow bars.
    let mut list = three_turn_list();
    list.enter_item_mode();
    list.navigate_turn(TurnDir::Up); // select index 1 ("turn-1")

    let area = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 10,
    };
    let mut buf = Buffer::empty(area);
    let _ = list.render_count_visited(area, &mut buf);

    // Collect rows of glyphs.
    let mut rows: Vec<String> = Vec::new();
    for y in 0..area.height {
        let mut row = String::new();
        for x in 0..area.width {
            row.push_str(buf[(x, y)].symbol());
        }
        rows.push(row);
    }

    // The selected turn's text row.
    let sel_y = rows
        .iter()
        .position(|r| r.contains("turn-1"))
        .expect("selected turn row present");

    // A gray down-arrow (▼) bar is rendered ABOVE the selected turn.
    assert!(sel_y >= 1, "need a row above the selected turn");
    let above = &rows[sel_y - 1];
    assert!(
        above.contains('\u{25BC}'),
        "expected a ▼ down-arrow bar above the selected turn; got: {above:?}"
    );
    // A gray up-arrow (▲) bar is rendered BELOW the selected turn.
    let below = &rows[sel_y + 1];
    assert!(
        below.contains('\u{25B2}'),
        "expected a ▲ up-arrow bar below the selected turn; got: {below:?}"
    );

    // Both arrow-bar rows paint on a gray background.
    let bar_above_x = above.find('\u{25BC}').expect("locate ▼ glyph") as u16;
    assert_eq!(
        buf[(bar_above_x, (sel_y - 1) as u16)].bg,
        ratatui::style::Color::Gray,
        "▼ arrow-bar must paint on a gray background"
    );
    let bar_below_x = below.find('\u{25B2}').expect("locate ▲ glyph") as u16;
    assert_eq!(
        buf[(bar_below_x, (sel_y + 1) as u16)].bg,
        ratatui::style::Color::Gray,
        "▲ arrow-bar must paint on a gray background"
    );
}
