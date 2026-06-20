//! RPC-093 + RPC-095 — unit tests for `input_transition.rs`, split into a
//! `#[path]`-included sibling so the parent stays under the 300-LoC
//! source-shape ceiling while keeping canonical rustfmt formatting.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn line_at(buf: &Buffer, area: Rect) -> String {
    let mut s = String::new();
    for x in area.x..area.x + area.width {
        s.push_str(buf[(x, area.y)].symbol());
    }
    s.trim_end().to_string()
}

#[test]
fn loading_renders_thinking_line() {
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(
        area,
        &mut buf,
        &InputTransitionState::Loading { elapsed_ms: 0 },
    );
    assert!(line_at(&buf, area).starts_with("⠋ Thinking... (Esc to stop)"));
}

#[test]
fn compacting_renders_compacting_line() {
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(
        area,
        &mut buf,
        &InputTransitionState::Compacting { elapsed_ms: 0 },
    );
    assert!(line_at(&buf, area).starts_with("⠋ Compacting... (Esc to stop)"));
}

#[test]
fn idle_is_noop_via_dispatcher() {
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(area, &mut buf, &InputTransitionState::Idle);
    assert_eq!(buf[(0, 0)].symbol(), " ");
}

#[test]
fn loading_advances_frame_per_elapsed() {
    let area = Rect::new(0, 0, 4, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(
        area,
        &mut buf,
        &InputTransitionState::Loading { elapsed_ms: 240 },
    );
    assert_eq!(buf[(0, 0)].symbol(), "⠸");
}

#[test]
fn cursor_painted_only_when_idle() {
    assert!(InputTransitionState::Idle.is_cursor_painted());
    assert!(!InputTransitionState::Loading { elapsed_ms: 0 }.is_cursor_painted());
    assert!(!InputTransitionState::Compacting { elapsed_ms: 0 }.is_cursor_painted());
}
