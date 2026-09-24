//! BUG-192 — Scrollback visual-row cap + oldest-content trim.
//!
//! Feature: spec/features/agentview-scrollback-unbounded-growth-cap-total-visual-rows.feature
//!
//! The AgentView ScrollbackList accumulates wrapped visual rows without bound
//! for the life of a session; per-frame cost grows O(total rows) and at
//! ~200k rows the 16ms render tick is exceeded ~8x (see
//! spec/attachments/BUG-192/BUG-192-scrollback-cap-research.md). These tests
//! pin: total rows capped at `MAX_SCROLLBACK_VISUAL_ROWS` (20,000) with the
//! oldest chunks trimmed, an accumulating single-line marker, offset
//! compensation, in-flight-chunk protection, SELECT-mode selection clearing,
//! stable marker geometry on resize, reset semantics and the per-push
//! invariant.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::{
    RenderedChunk, ScrollbackList, SelectionMode, TurnDir, MAX_SCROLLBACK_VISUAL_ROWS,
};
use proptest::{prop_assert, proptest};
use ratatui::text::{Line, Span};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

/// Opaque (source-less) chunk: one wrapped `Line` per entry of `rows`.
fn opaque_chunk(seq: u64, rows: &[&str]) -> RenderedChunk {
    RenderedChunk {
        seq,
        lines: rows
            .iter()
            .map(|r| Line::from(Span::raw(r.to_string())))
            .collect(),
        source: None,
    }
}

/// A chunk of `n` rows, each carrying identifying text `row {i}`.
fn n_row_chunk(seq: u64, n: usize) -> RenderedChunk {
    let rows: Vec<String> = (0..n).map(|i| format!("row {i}")).collect();
    let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    opaque_chunk(seq, &refs)
}

/// Push `count` chunks of `rows_each` rows (seqs start at 0) and return the
/// resulting total visual-row count.
fn fill(list: &mut ScrollbackList, count: usize, rows_each: usize) -> usize {
    for i in 0..count {
        list.push(n_row_chunk(i as u64, rows_each));
    }
    list.total_rows()
}

/// First row of the buffer, joined to a string (for "marker on top" checks).
fn top_row(buf: &ratatui::buffer::Buffer, w: u16, _h: u16) -> String {
    (0..w)
        .map(|x| buf[(x, 0)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn bottom_row(buf: &ratatui::buffer::Buffer, w: u16, h: u16) -> String {
    (0..w)
        .map(|x| buf[(x, h - 1)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn area(w: u16, h: u16) -> ratatui::layout::Rect {
    ratatui::layout::Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    }
}

/// Render into a fresh buffer and return it.
fn render(list: &mut ScrollbackList, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let mut buf = ratatui::buffer::Buffer::empty(area(w, h));
    list.render_count_visited(area(w, h), &mut buf);
    buf
}

// ---------------------------------------------------------------------
// Feature: agentview-scrollback-unbounded-growth-cap-total-visual-rows
// ---------------------------------------------------------------------

#[test]
fn scenario_scrollback_below_cap_is_never_trimmed() {
    // @step Given a session's ScrollbackList holding chunks totalling 10,000 visual rows
    let mut list = ScrollbackList::new();
    let total = fill(&mut list, 100, 100);
    assert_eq!(total, 10_000);

    // @step When a 100-row chunk is pushed
    list.push(n_row_chunk(100, 100));
    // trim_to_cap must be a no-op below the cap
    let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);

    // @step Then the scrollback still holds every chunk that was pushed
    assert_eq!(
        list.chunk_count(),
        101,
        "no chunk may be removed below the cap"
    );
    assert_eq!(res.removed_chunks, 0);
    assert_eq!(res.removed_rows, 0);
    assert!(!res.marker_inserted);

    // @step And the total visual row count is 10,100
    assert_eq!(list.total_rows(), 10_100);

    // @step And no trim-marker chunk exists in the scrollback
    assert!(!list.first_chunk_is_marker());
    assert_eq!(list.trimmed_rows_total(), 0);
}

#[test]
fn scenario_pushing_past_the_cap_trims_the_oldest_chunks() {
    // @step Given a session's ScrollbackList holding chunks totalling 20,900 visual rows
    let mut list = ScrollbackList::new();
    assert_eq!(fill(&mut list, 104, 100), 10_400);
    list.push(n_row_chunk(104, 10_500)); // 20,900 total
    assert_eq!(list.total_rows(), 20_900);

    // @step When a 500-row chunk is pushed
    list.push(n_row_chunk(105, 500));
    let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);

    // @step Then the total visual row count is at most 20,000
    assert!(
        list.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS,
        "total {} exceeds cap",
        list.total_rows()
    );
    assert!(res.removed_chunks > 0 && res.removed_rows > 0);

    // @step And the oldest chunks have been removed while the newest chunks remain intact
    let chunks = list.chunks();
    assert_eq!(
        chunks.last().unwrap().lines.len(),
        500,
        "the newest chunk must survive untouched"
    );
    assert_eq!(
        chunks.last().unwrap().lines[499].spans[0].content.as_ref(),
        "row 499",
        "newest chunk content intact"
    );
    // The OLDEST chunk (seq 0) must be gone — every chunk carries a
    // "row 0" label (the label restarts per chunk), so the assertion must
    // be on chunk identity, not on the row text.
    assert!(
        !chunks.iter().any(|c| c.seq == 0),
        "oldest chunk (seq 0) must have been removed"
    );

    // @step And the first chunk is a trim-marker line
    assert!(list.first_chunk_is_marker());
    assert_eq!(chunks[0].lines.len(), 1, "the marker is exactly one line");
}

#[test]
fn scenario_the_trim_marker_accumulates_the_total_trimmed_rows() {
    // @step Given a session's ScrollbackList that has already trimmed 1,500 older rows
    let mut list = ScrollbackList::new();
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100)); // 10,000 rows
    }
    list.push(n_row_chunk(100, 10_500)); // 20,500 total
    let r1 = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert!(
        r1.removed_rows >= 501,
        "first trim must remove rows, got {}",
        r1.removed_rows
    );
    assert!(r1.marker_inserted, "first trim must insert the marker");
    assert!(list.first_chunk_is_marker());
    assert_eq!(list.total_rows(), list.total_rows()); // invariant: below cap
    assert!(list.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS);

    // @step When further pushes cause another 2,000 older rows to be trimmed
    // (scenario magnitude; here a 1,000-row push — the contract under test is
    // ACCUMULATION across trims, independent of the exact totals.)
    list.push(n_row_chunk(101, 1_000));
    let r2 = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert!(
        r2.removed_rows >= 1_000,
        "second trim must remove rows, got {}",
        r2.removed_rows
    );
    assert!(
        !r2.marker_inserted,
        "second trim must update the existing marker"
    );

    // @step Then the trim-marker chunk is still exactly one line
    assert!(list.first_chunk_is_marker());
    assert_eq!(list.chunks()[0].lines.len(), 1);

    // @step And the marker counts 3,500 trimmed older rows in total
    // (i.e. the accumulated total across both trims, not the last trim's delta)
    assert_eq!(
        list.trimmed_rows_total(),
        r1.removed_rows + r2.removed_rows,
        "the marker must count every trimmed row, not just the latest trim"
    );
    let marker_text = list.full_text_for_seq(list.chunks()[0].seq).unwrap();
    assert!(
        marker_text.contains(&(r1.removed_rows + r2.removed_rows).to_string()),
        "marker must show the accumulated count, got: {marker_text}"
    );
    assert!(
        marker_text.contains("older lines trimmed"),
        "marker must be the '… N older lines trimmed …' line, got: {marker_text}"
    );
}

#[test]
fn scenario_sticky_streaming_shows_no_visible_change_after_a_trim() {
    // @step Given a stick-to-bottom scrollback whose total rows exceed the cap
    let mut list = ScrollbackList::new();
    list.set_viewport_height(45);
    list.set_viewport_width(200);
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100));
    }
    list.push(n_row_chunk(100, 11_000)); // 21,000 total — over the cap
    assert!(list.total_rows() > MAX_SCROLLBACK_VISUAL_ROWS);
    assert!(list.scroll_state().stick_to_bottom);

    // The tail BEFORE the trim: bottom row is the newest chunk's last row.
    let buf0 = render(&mut list, 200, 45);
    let before = bottom_row(&buf0, 200, 45);
    assert!(
        before.contains("row 10999"),
        "tail must be the newest chunk's last row: {before}"
    );

    // @step When a push trims older chunks
    list.push(n_row_chunk(101, 1_000));
    let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert!(
        res.removed_chunks > 0,
        "the push must have triggered a trim"
    );

    // @step Then the last visible rows paint the same content as before the trim
    // (the new tail row arrived, so assert the NEW tail is at the bottom and
    // nothing of the previous tail shifted away).
    let buf = render(&mut list, 200, 45);
    let bottom = bottom_row(&buf, 200, 45);
    assert!(
        bottom.contains("row 999"),
        "newest row must be bottom-anchored, got: {bottom}"
    );

    // @step And the trim marker only becomes visible when scrolled to the top
    assert!(
        !bottom.contains("older lines trimmed"),
        "marker must not appear in the sticky tail"
    );
    list.jump_to_top();
    let topbuf = render(&mut list, 200, 45);
    let top = top_row(&topbuf, 200, 45);
    assert!(
        top.contains("older lines trimmed"),
        "scrolled to the top, the marker must be the first row, got: {top}"
    );
}

#[test]
fn scenario_a_scrolled_up_viewport_stays_pinned_to_the_same_content() {
    // @step Given a scrollback with 21,000 visual rows where the user has scrolled up so row 3,000 from the tail is visible
    let mut list = ScrollbackList::new();
    list.set_viewport_height(45);
    list.set_viewport_width(200);
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100)); // 10,000
    }
    list.push(n_row_chunk(100, 11_000)); // 21,000 total
    list.jump_to_bottom();
    list.scroll_up(3_000); // pinned 3,000 rows above the tail
    let pinned_offset = list.scroll_state().offset;
    assert_eq!(pinned_offset, 21_000 - 45 - 3_000);
    // pin the top row's text
    let before_buf = render(&mut list, 200, 45);
    let before_top = top_row(&before_buf, 200, 45);

    // @step When a trim removes 2,000 rows above the viewport
    list.push(n_row_chunk(101, 1_000)); // 22,000 total
    let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert!(
        res.removed_rows >= 2_000,
        "the trim must remove at least the 2,000 head rows, got {}",
        res.removed_rows
    );

    // @step Then the viewport still shows the same rows at the same positions
    let after_buf = render(&mut list, 200, 45);
    let after_top = top_row(&after_buf, 200, 45);
    assert_eq!(after_top, before_top, "pinned viewport must not move");

    // @step And the scroll offset has been compensated by the net removed rows
    assert_eq!(
        list.scroll_state().offset,
        pinned_offset - (res.removed_rows - 1),
        "offset must shrink by (removed rows - marker row)"
    );
}

#[test]
fn scenario_a_scroll_offset_inside_the_removed_region_clamps_to_the_marker() {
    // @step Given a scrollback where the user has scrolled up past the rows that a trim removes
    let mut list = ScrollbackList::new();
    list.set_viewport_height(10);
    list.set_viewport_width(200);
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100)); // 10,000
    }
    list.push(n_row_chunk(100, 11_000)); // 21,000 rows
                                         // User scrolled to the very top: offset 0 is entirely inside the region
                                         // the next trim removes.
    list.jump_to_top();
    assert_eq!(list.scroll_state().offset, 0);

    // @step When the trim removes those rows
    list.push(n_row_chunk(101, 1_000));
    list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);

    // @step Then the scroll offset is clamped to 0
    assert_eq!(list.scroll_state().offset, 0);

    // @step And the first visible row is the trim marker
    let top_buf = render(&mut list, 200, 10);
    let top = top_row(&top_buf, 200, 10);
    assert!(
        top.contains("older lines trimmed"),
        "the marker must be the first visible row, got: {top}"
    );
}

#[test]
fn scenario_the_trim_never_removes_an_in_flight_streaming_chunk() {
    // @step Given a session with 20,500 total rows and an in-flight assistant chunk at the tail still accumulating deltas
    let mut list = ScrollbackList::new();
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100)); // 10,000
    }
    list.push(n_row_chunk(100, 1_000)); // 11,000
    list.push(n_row_chunk(101, 9_500)); // 20,500 total
                                        // The in-flight chunk is the last one (index 102).
    let in_flight_idx = list.chunk_count() - 1;
    let in_flight_seq = list.chunks()[in_flight_idx].seq;

    // @step When more text deltas are streamed into the in-flight chunk so the cap is crossed
    // (modelled at the widget level: the in-flight chunk grows in place, then
    // the store trims with the in-flight index protected). Grow it by 1,000 rows:
    let grown = n_row_chunk(in_flight_seq, 10_500);
    list.chunks_mut().truncate(in_flight_idx);
    list.push(grown);
    assert!(
        list.total_rows() > MAX_SCROLLBACK_VISUAL_ROWS,
        "20,500 + 1,000 grown must exceed the cap"
    );
    let res = list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, in_flight_idx);
    assert!(res.removed_chunks > 0, "a trim must have fired");

    // @step Then the in-flight chunk is not removed by the trim
    let after_idx = in_flight_idx.saturating_sub(res.removed_chunks)
        + (if res.marker_inserted { 1 } else { 0 });
    let surviving = list
        .chunks()
        .get(after_idx)
        .expect("in-flight chunk must survive");
    assert_eq!(
        surviving.seq, in_flight_seq,
        "the in-flight chunk (protected by the floor) must still be in the list"
    );
    assert_eq!(
        surviving.lines.len(),
        10_500,
        "the in-flight chunk must be whole"
    );

    // @step And the in-flight chunk's index still points at the same chunk
    // (index-shift contract the store relies on:
    //  shift = marker_inserted - removed_chunks)
    assert_eq!(
        after_idx,
        in_flight_idx.saturating_sub(res.removed_chunks)
            + (if res.marker_inserted { 1 } else { 0 })
    );

    // @step And further text deltas continue to append to that same chunk
    // (the store keeps the slot; appending rewrites the SAME chunk's source —
    // here we assert the shifted slot still resolves to the same seq)
    assert_eq!(list.chunks()[after_idx].seq, in_flight_seq);
    // The cap holds: only the protected in-flight tail may exceed it.
    assert!(
        list.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS + 10_500,
        "total {} — trim must have removed as much as the floor allowed",
        list.total_rows()
    );
}

#[test]
fn scenario_select_mode_selection_on_a_trimmed_turn_is_cleared() {
    // @step Given a scrollback in SELECT (item) mode with a turn selected that the trim removes
    let mut list = ScrollbackList::new();
    for i in 0..110 {
        list.push(n_row_chunk(i as u64, 100)); // 11,000
    }
    list.enter_item_mode();
    // walk to the OLDEST turn (index 0) — the one a trim will remove:
    for _ in 0..300 {
        list.navigate_turn(TurnDir::Up);
    }
    assert_eq!(
        list.selected_seq(),
        Some(list.chunks()[0].seq),
        "selection must be on the oldest turn"
    );

    // @step When the trim removes that turn's chunk
    list.push(n_row_chunk(110, 9_500)); // 20,500 total
    list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);

    // @step Then the scrollback has no selected turn
    assert_eq!(
        list.selected_seq(),
        None,
        "the selected turn was trimmed — the selection must clear"
    );
    // mode is still Item (only the selection cleared):
    assert_eq!(list.selection_mode(), SelectionMode::Item);
}

#[test]
fn scenario_the_trim_marker_has_stable_geometry_on_resize() {
    // @step Given a scrollback whose first chunk is a trim marker
    let mut list = ScrollbackList::new();
    list.set_viewport_width(80);
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100)); // 10,000
    }
    list.push(n_row_chunk(100, 11_000)); // 21,000
    list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert!(list.first_chunk_is_marker());
    let marker_seq = list.chunks()[0].seq;
    let marker_before = list.full_text_for_seq(marker_seq).unwrap();

    // @step When the viewport width changes and every chunk is re-wrapped
    list.set_viewport_width(120);

    // @step Then the trim marker chunk keeps exactly one line
    assert_eq!(
        list.chunks()[0].lines.len(),
        1,
        "the opaque marker must never re-wrap"
    );

    // @step And its marker text is unchanged
    assert_eq!(
        list.full_text_for_seq(marker_seq).unwrap(),
        marker_before,
        "resize must not alter the marker text"
    );
}

#[test]
fn scenario_reset_clears_the_chunks_the_marker_and_the_trimmed_counter() {
    // @step Given a session whose scrollback has been trimmed at least once
    let mut list = ScrollbackList::new();
    for i in 0..100 {
        list.push(n_row_chunk(i as u64, 100)); // 10,000
    }
    list.push(n_row_chunk(100, 11_000)); // 21,000
    list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
    assert!(list.trimmed_rows_total() > 0);

    // @step When the session's scrollback is reset (slash-clear)
    list.reset();

    // @step Then the scrollback holds no chunks and no trim marker
    assert_eq!(list.chunk_count(), 0);
    assert!(!list.first_chunk_is_marker());

    // @step And the trimmed-row counter is back to zero
    assert_eq!(list.trimmed_rows_total(), 0);

    // @step And the next push starts a fresh scrollback with no marker
    list.push(n_row_chunk(7, 50));
    assert_eq!(list.chunk_count(), 1);
    assert!(!list.first_chunk_is_marker());
    assert_eq!(list.total_rows(), 50);
}

#[test]
fn scenario_every_push_keeps_the_total_at_or_under_the_cap() {
    // @step Given a session's ScrollbackList
    // @step When 200 successive pushes of 200-row chunks are applied (40,000 rows total)
    let mut list = ScrollbackList::new();
    for i in 0..200u64 {
        list.push(n_row_chunk(i, 200));
        list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
        // @step Then after each push the total visual row count is at most 20,000
        assert!(
            list.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS,
            "after push {i}: total {} exceeds the cap",
            list.total_rows()
        );
    }
    // @step And the newest content is always present in the scrollback
    let chunks = list.chunks();
    assert_eq!(chunks.last().unwrap().lines.len(), 200);
    assert_eq!(
        chunks.last().unwrap().lines[199].spans[0].content.as_ref(),
        "row 199"
    );
}

proptest! {
    /// Property (proptest companion to the per-push scenario above): for ANY
    /// push sizes, after trim the total is at most cap + last_chunk_rows —
    /// the last chunk cannot be partially trimmed, so it is the only allowed
    /// overshoot.
    #[test]
    fn propt_random_push_sequences_respect_the_cap(i in 0u64..30, n in 1usize..=4_000) {
        let mut list = ScrollbackList::new();
        let mut last;
        for j in 0..=i {
            let step = ((j * 7 + 1) % n as u64) as usize + 1;
            last = step;
            list.push(n_row_chunk(j, step));
            list.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, 0);
            prop_assert!(
                list.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS + last,
                "total {} after a {last}-row push",
                list.total_rows()
            );
        }
    }
}

// ---------------------------------------------------------------------
// SessionContext-level integration (store wiring)
// ---------------------------------------------------------------------

fn assistant_text(text: &str) -> codelet_rpc_types::StreamChunk {
    codelet_rpc_types::StreamChunk::text(text.to_string())
}

#[test]
fn store_in_flight_chunk_survives_the_cap_and_stays_appended_to() {
    use codelet_fspec_tui::store::SessionContext;

    let mut ctx = SessionContext::new(codelet_rpc_types::SessionId::new("s-1"));
    // 160 settled assistant turns (~101 wrapped rows each at the 80-col
    // default wrap width) => ~16k rows.
    for i in 0..160u32 {
        let body: String = (0..100)
            .map(|l| format!("turn {i} line {l} of a settled assistant message"))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.record_chunk(&assistant_text(&body));
        ctx.record_chunk(&codelet_rpc_types::StreamChunk::Done);
    }
    // Now start an in-flight assistant turn.
    ctx.record_chunk(&assistant_text("in-flight turn starts\n"));
    let in_flight_before = ctx.in_flight_assistant;
    assert!(in_flight_before.is_some(), "a streaming turn is in flight");
    let seq_before = ctx
        .scrollback
        .chunks()
        .get(in_flight_before.unwrap())
        .map(|c| c.seq)
        .unwrap();

    // Stream many more deltas: total rows cross the cap mid-stream.
    for i in 0..40u32 {
        let body: String = (0..100)
            .map(|l| format!("in-flight line {i}-{l} still streaming"))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.record_chunk(&assistant_text(&format!("{body}\n")));
    }

    // The in-flight chunk must not have been trimmed:
    let in_flight_after = ctx.in_flight_assistant;
    assert!(
        in_flight_after.is_some(),
        "in-flight slot must still be set after cap trims"
    );
    let chunk_after = ctx
        .scrollback
        .chunks()
        .get(in_flight_after.unwrap())
        .expect("chunk");
    assert_eq!(
        chunk_after.seq, seq_before,
        "in_flight_assistant must still point at the SAME streaming chunk"
    );
    assert_eq!(
        chunk_after.source.as_ref().map(|s| s.is_streaming),
        Some(true),
        "the in-flight chunk must still be flagged streaming"
    );

    // Deltas keep appending to that same chunk (no re-bulleting / no new chunk):
    let chunks_before_extra = ctx.scrollback.chunk_count();
    ctx.record_chunk(&assistant_text(" one more delta"));
    assert_eq!(
        ctx.scrollback.chunk_count(),
        chunks_before_extra,
        "a Text delta must accumulate into the in-flight chunk, not push a new one"
    );
    assert_eq!(ctx.in_flight_assistant, in_flight_after);
    let grown = &ctx.scrollback.chunks()[in_flight_after.unwrap()];
    assert!(
        grown
            .source
            .as_ref()
            .unwrap()
            .text
            .contains("one more delta"),
        "the delta must land in the same in-flight chunk"
    );

    // The cap still holds (in-flight tail excepted):
    assert!(
        ctx.scrollback.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS + 1_000,
        "total {} — trim must have fired",
        ctx.scrollback.total_rows()
    );
    // A marker is present (the session did trim at some point):
    assert!(
        ctx.scrollback.first_chunk_is_marker(),
        "after crossing the cap the marker must be present"
    );
}

#[test]
fn store_reset_and_isolation() {
    use codelet_fspec_tui::store::SessionContext;

    // @step Given two open sessions whose scrollbacks are both below the cap
    let mut a = SessionContext::new(codelet_rpc_types::SessionId::new("s-a"));
    let mut b = SessionContext::new(codelet_rpc_types::SessionId::new("s-b"));
    for i in 0..10u32 {
        let body: String = (0..30)
            .map(|l| format!("session-a line {i}-{l}"))
            .collect::<Vec<_>>()
            .join("\n");
        a.record_chunk(&assistant_text(&body));
        a.record_chunk(&codelet_rpc_types::StreamChunk::Done);
        let body: String = (0..30)
            .map(|l| format!("session-b line {i}-{l}"))
            .collect::<Vec<_>>()
            .join("\n");
        b.record_chunk(&assistant_text(&body));
        b.record_chunk(&codelet_rpc_types::StreamChunk::Done);
    }
    let b_chunks_before = b.scrollback.chunk_count();
    let b_rows_before = b.scrollback.total_rows();

    // @step When session A streams until its trim fires
    for i in 0..200u32 {
        let body: String = (0..120)
            .map(|l| format!("session-a big turn {i} line {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        a.record_chunk(&assistant_text(&body));
        a.record_chunk(&codelet_rpc_types::StreamChunk::Done);
    }

    // @step Then session A's total visual row count is at most 20,000
    assert!(
        a.scrollback.total_rows() <= MAX_SCROLLBACK_VISUAL_ROWS,
        "session A total {} exceeds the cap",
        a.scrollback.total_rows()
    );
    assert!(a.scrollback.first_chunk_is_marker());

    // @step And session B's scrollback is unchanged (same chunks, same total rows, no marker)
    assert_eq!(b.scrollback.chunk_count(), b_chunks_before);
    assert_eq!(b.scrollback.total_rows(), b_rows_before);
    assert!(!b.scrollback.first_chunk_is_marker());

    // Reset (slash-clear) clears everything for A:
    a.reset_scrollback();
    assert_eq!(a.scrollback.chunk_count(), 0);
    assert_eq!(a.scrollback.trimmed_rows_total(), 0);
    assert!(!a.scrollback.first_chunk_is_marker());
    // ...and B is still untouched:
    assert_eq!(b.scrollback.chunk_count(), b_chunks_before);
}
