//! Unit tests for the BUG-192 scrollback visual-row cap + trim.
//!
//! Feature: spec/features/agentview-scrollback-unbounded-growth-cap-total-visual-rows.feature
//!
//! Widget-level contracts (the store-level `record_chunk` integration lives in
//! `tests/agentview_scrollback_cap_bug192.rs`): no-op below the cap, oldest
//! chunks trimmed first, marker accumulation, protected-floor (in-flight)
//! integrity, last-chunk-never-removed, offset compensation + clamp, marker
//! stability on resize, reset semantics and the per-push invariant.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ratatui::text::{Line, Span};

use super::super::RenderedChunk;
use super::*;
use crate::views::agent::ScrollbackList;

/// `n` rows, each carrying identifying text `row {i}`.
fn n_row_lines(n: usize) -> Vec<Line<'static>> {
    (0..n)
        .map(|i| Line::from(Span::raw(format!("row {i}"))))
        .collect()
}

fn chunk_with(seq: u64, lines: Vec<Line<'static>>) -> RenderedChunk {
    RenderedChunk {
        seq,
        lines,
        source: None,
    }
}

/// Fill a fresh list with `count` chunks of `rows_each` rows.
fn fill(list: &mut ScrollbackList, count: usize, rows_each: usize) {
    for i in 0..count {
        let rows = n_row_lines(rows_each);
        list.push(chunk_with(i as u64, rows));
    }
}

#[test]
fn trim_is_a_no_op_below_the_cap() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 10, 100); // 1,000 rows
    let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert_eq!(res, TrimResult::default());
    assert_eq!(list.chunk_count(), 10);
    assert_eq!(list.total_rows(), 1_000);
    assert!(!list.first_chunk_is_marker());
    assert_eq!(list.trimmed_rows_total(), 0);
}

#[test]
fn trim_removes_oldest_chunks_first_and_keeps_the_newest_intact() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 50, 600); // 30,000 rows
    let res = list.trim_to_cap(1_000, 0);

    assert!(res.removed_chunks > 0);
    assert!(res.removed_rows > 0);
    assert!(list.total_rows() <= 1_000);
    assert!(list.first_chunk_is_marker());
    // The newest chunk survives whole.
    let chunks = list.chunks();
    assert_eq!(chunks.last().unwrap().lines.len(), 600);
    assert_eq!(
        chunks.last().unwrap().lines[599].spans[0].content.as_ref(),
        "row 599"
    );
    // The oldest chunk is gone.
    assert!(!chunks.iter().any(|c| c.seq == 0));
    // removed_rows accounts for every dropped chunk.
    assert_eq!(res.removed_rows + list.total_rows() - 1, 30_000);
}

#[test]
fn trim_never_removes_the_last_real_chunk() {
    let mut list = ScrollbackList::new();
    // A single chunk already bigger than the cap.
    list.push(chunk_with(0, n_row_lines(5_000)));
    let res = list.trim_to_cap(1_000, 0);
    assert_eq!(res, TrimResult::default());
    assert_eq!(list.chunk_count(), 1);
    assert_eq!(list.total_rows(), 5_000);
    assert!(!list.first_chunk_is_marker());
}

#[test]
fn trim_stops_at_the_protected_floor() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 20, 1_000); // 20,000 rows
                                // The in-flight chunk is index 10 (MIDDLE, not the tail — the tail is
                                // always protected by the last-chunk rule). cap 10,000 (no marker →
                                // budget 9,999): reaching the budget would need 11 removals (→ 9,000),
                                // but the floor allows only 10 — so the trim MUST stop at exactly 10
                                // removed chunks with content still over the budget.
    let res = list.trim_to_cap(10_000, 10);
    assert_eq!(
        res.removed_chunks, 10,
        "the floor must stop the trim at chunk 10"
    );
    let chunks = list.chunks();
    // The protected chunk (seq 10) survives in place (index 1 behind the
    // marker); the 10 oldest are gone.
    assert!(chunks.iter().any(|c| c.seq == 10));
    assert!(!chunks.iter().any(|c| c.seq < 10));
    // Content (10,000 rows) + marker row — the floor-bound overshoot.
    assert_eq!(list.total_rows(), 10_001);
}

#[test]
fn trim_updates_an_existing_marker_and_accumulates_the_count() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 20, 1_000); // 20,000 rows
    let r1 = list.trim_to_cap(10_000, 0);
    assert!(r1.marker_inserted);
    assert_eq!(list.trimmed_rows_total(), r1.removed_rows);

    // Grow past the cap again (the marker occupies index 0).
    for i in 100..120 {
        list.push(chunk_with(i as u64, n_row_lines(1_000)));
    }
    let r2 = list.trim_to_cap(10_000, 0);
    assert!(
        !r2.marker_inserted,
        "the existing marker must be updated, not re-inserted"
    );
    assert_eq!(
        list.trimmed_rows_total(),
        r1.removed_rows + r2.removed_rows,
        "the counter must accumulate across trims"
    );
    // The marker is still exactly one line and shows the accumulated count.
    assert_eq!(list.chunks()[0].lines.len(), 1);
    let marker_text = list.full_text_for_seq(list.chunks()[0].seq).unwrap();
    assert!(
        marker_text.contains(&(r1.removed_rows + r2.removed_rows).to_string()),
        "marker must show the accumulated count, got: {marker_text}"
    );
    assert!(marker_text.contains("older lines trimmed"));
}

#[test]
fn trim_marker_text_matches_the_rule_format() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 5, 1_000); // 5,000 rows
    let res = list.trim_to_cap(1_000, 0);
    assert_eq!(res.removed_chunks, 4);
    assert_eq!(res.removed_rows, 4_000);
    let marker_text = list.full_text_for_seq(list.chunks()[0].seq).unwrap();
    assert_eq!(
        marker_text.trim(),
        "\u{2026} 4000 older lines trimmed \u{2026}"
    );
}

#[test]
fn trim_keeps_the_offset_pinned_when_scrolled_up() {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(10);
    fill(&mut list, 20, 1_000); // 20,000 rows
    list.jump_to_bottom();
    list.scroll_up(3_000);
    let pinned = list.scroll_state().offset;
    assert_eq!(pinned, 20_000 - 10 - 3_000);

    let res = list.trim_to_cap(5_000, 0);
    assert!(res.removed_rows > 3_000 + 10); // all above the viewport
    assert_eq!(
        list.scroll_state().offset,
        pinned - (res.removed_rows - 1),
        "the offset must shrink by (removed rows - marker row)"
    );
    assert!(!list.scroll_state().stick_to_bottom);
}

#[test]
fn trim_clamps_an_offset_inside_the_removed_region_to_zero() {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(10);
    fill(&mut list, 20, 1_000);
    list.jump_to_top(); // offset 0 — inside whatever the trim removes
    let res = list.trim_to_cap(5_000, 0);
    assert!(res.removed_chunks > 0);
    assert_eq!(list.scroll_state().offset, 0);
    assert!(list.first_chunk_is_marker());
}

#[test]
fn trim_stick_to_bottom_reanchors_with_no_offset_left_behind() {
    let mut list = ScrollbackList::new();
    list.set_viewport_height(10);
    fill(&mut list, 20, 1_000);
    assert!(list.scroll_state().stick_to_bottom);
    let res = list.trim_to_cap(5_000, 0);
    assert!(res.removed_chunks > 0);
    // Re-anchored to the (new) tail.
    let max_off = list.total_rows() - 10;
    assert_eq!(list.scroll_state().offset, max_off);
}

#[test]
fn trim_clears_a_select_mode_selection_on_a_trimmed_turn() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 10, 500); // 5,000 rows
    list.enter_item_mode(); // selects the last turn
    for _ in 0..9 {
        list.navigate_turn(crate::views::agent::TurnDir::Up);
    }
    assert_eq!(list.selected_seq(), Some(0));
    let res = list.trim_to_cap(1_000, 0);
    assert!(res.removed_chunks >= 1);
    assert_eq!(
        list.selected_seq(),
        None,
        "trimmed turn must clear the selection"
    );
    assert_eq!(
        list.selection_mode(),
        crate::views::agent::SelectionMode::Item
    );
}

#[test]
fn the_marker_keeps_stable_geometry_on_resize() {
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    fill(&mut list, 10, 1_000);
    list.trim_to_cap(5_000, 0);
    assert!(list.first_chunk_is_marker());
    let marker_seq = list.chunks()[0].seq;
    let before = list.full_text_for_seq(marker_seq).unwrap();

    // Resize: opaque chunks (source: None) never re-wrap.
    list.set_viewport_width(120);
    assert_eq!(list.chunks()[0].lines.len(), 1);
    assert_eq!(
        list.full_text_for_seq(marker_seq).unwrap(),
        before,
        "resize must not alter the marker text"
    );
}

#[test]
fn reset_clears_the_chunks_the_marker_and_the_counter() {
    let mut list = ScrollbackList::new();
    fill(&mut list, 20, 1_000);
    list.trim_to_cap(5_000, 0);
    assert!(list.trimmed_rows_total() > 0);

    list.reset();
    assert_eq!(list.chunk_count(), 0);
    assert!(!list.first_chunk_is_marker());
    assert_eq!(list.trimmed_rows_total(), 0);

    // A fresh push starts without a marker.
    list.push(chunk_with(7, n_row_lines(50)));
    assert_eq!(list.chunk_count(), 1);
    assert!(!list.first_chunk_is_marker());
    assert_eq!(list.total_rows(), 50);
}

#[test]
fn per_push_the_total_stays_at_or_under_the_cap() {
    let mut list = ScrollbackList::new();
    for i in 0..200u64 {
        list.push(chunk_with(i, n_row_lines(200)));
        let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
        let _ = res;
        assert!(
            list.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS,
            "after push {i}: total {} exceeds the cap",
            list.total_rows()
        );
    }
    let chunks = list.chunks();
    assert_eq!(chunks.last().unwrap().lines.len(), 200);
}
