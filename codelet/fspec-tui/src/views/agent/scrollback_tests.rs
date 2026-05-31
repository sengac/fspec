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
