//! **BUG-192** — scrollback push/insert + cap-trim machinery for
//! `SessionContext`, extracted from `session_context.rs` to keep that
//! file under the 300-LoC ceiling pinned by
//! `spec/features/rpc024-multi-session-cycling.feature` (and re-pinned by
//! the RPC-049 / RPC-050 source-shape cards over
//! `rust/fspec-tui/src/store/agent_view/`).
//!
//! Every store-side mutation of `SessionContext::scrollback` (chunk push,
//! chunk insert, in-place re-wrap) funnels through these helpers so the
//! in-flight slot indices (`in_flight_assistant` / `in_flight_thinking`)
//! always keep pointing at the SAME chunks across trims.
//!
//! `SessionContext` still exposes the one-line method delegates
//! (`push_chunk`, `push_source`, `insert_source_at`,
//! `rewrap_and_trim_at`, `trim_scrollback_to_cap`) so existing call sites
//! in `chunk_processor`, `chunk_tool_result`, `reconnect_notice` and
//! `record_chunk` are untouched.

use super::chunk_wrap::{wrap_source, DEFAULT_WRAP_WIDTH};
use super::session_context::SessionContext;
use crate::views::agent::{ChunkSource, RenderedChunk, TrimResult, MAX_SCROLLBACK_VISUAL_ROWS};

/// **BUG-192**: shift the in-flight slot indices by the net chunk-count
/// change from a trim (plus the insert shift) so they keep pointing at
/// the SAME chunks.
///
/// `inserted_at`: when a chunk was just INSERTED at `idx`, slots at or
/// beyond `idx` also move right by 1 (the insert itself); a `push`
/// (append at the tail) passes `None`. `trim_shift` is the net chunk
/// shift from the trim itself (`-removed + marker`); 0 when no trim
/// fired.
///
/// Invariants that keep this safe:
/// - the trim removes a PREFIX of chunks strictly below the protected
///   floor (the in-flight slots' minimum), so no slot ever points at a
///   removed chunk;
/// - a slot at index 0 (in-flight chunk IS the oldest) defers the trim
///   (the marker would occupy its index); the insert shift still
///   applies and the trim self-heals on the next mutation.
pub(crate) fn shift_in_flight_slots(
    ctx: &mut SessionContext,
    inserted_at: Option<usize>,
    trim_shift: isize,
) {
    for slot in [&mut ctx.in_flight_assistant, &mut ctx.in_flight_thinking] {
        if let Some(i) = slot {
            let mut new = *i as isize + trim_shift;
            if let Some(inserted_at) = inserted_at {
                if *i >= inserted_at {
                    new += 1;
                }
            }
            *slot = Some(new.max(0) as usize);
        }
    }
}

/// **BUG-192**: the in-flight slots' minimum index — the protected floor
/// the widget trim must not remove. `None` when no in-flight slot is
/// armed (a plain tail push).
fn in_flight_floor(ctx: &SessionContext) -> Option<usize> {
    ctx.in_flight_assistant
        .iter()
        .chain(ctx.in_flight_thinking.iter())
        .copied()
        .min()
}

/// **BUG-192**: run the widget trim (protecting the in-flight slots) and
/// shift the slot indices by the net chunk-count change so they keep
/// pointing at the SAME chunks.
///
/// Called after every chunk-producing `push_source` /
/// `insert_source_at` and after every in-place in-flight growth
/// (`rewrap_and_trim_at`) — the only two ways the total visual-row count
/// grows.
///
/// `inserted_at` carries the insert-index shift for `insert_source_at`
/// call sites (see [`shift_in_flight_slots`]). Returns the
/// [`crate::views::agent::TrimResult`] of the widget trim — `Default`
/// when the trim was deferred (in-flight chunk at index 0) or did not
/// fire.
pub(crate) fn trim_to_cap(ctx: &mut SessionContext, inserted_at: Option<usize>) -> TrimResult {
    let floor = in_flight_floor(ctx);
    if floor == Some(0) {
        // In-flight chunk at index 0 — the trim would remove the
        // marker's slot; defer (the insert shift still applies below).
        shift_in_flight_slots(ctx, inserted_at, 0);
        return TrimResult::default();
    }
    let res = ctx
        .scrollback
        .trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, floor.unwrap_or(0));
    let trim_shift = res.marker_inserted as isize - res.removed_chunks as isize;
    shift_in_flight_slots(ctx, inserted_at, trim_shift);
    res
}

/// **BUG-192**: re-wrap a single (growing) chunk and then trim to the
/// cap, shifting the in-flight slots. All store-side in-place growth
/// (streaming deltas, tool-card progress, settle re-wraps) funnels
/// through here so the cap holds even between chunk pushes.
pub(crate) fn rewrap_and_trim_at(ctx: &mut SessionContext, idx: usize) {
    ctx.scrollback.rewrap_at(idx);
    trim_to_cap(ctx, None);
}

/// Lower-level push that allocates the seq cursor and performs the
/// initial wrap. **RPC-091**.
///
/// **BUG-192**: trims after the push (see [`trim_to_cap`]); existing
/// in-flight slots are shifted by the trim's net change (the pushed
/// chunk is always the tail and is never removed by the trim, so callers
/// that adopt it as a new in-flight slot read `chunk_count() - 1`
/// afterwards).
pub(crate) fn push_source(ctx: &mut SessionContext, source: ChunkSource) {
    let seq = ctx.scrollback_next_seq;
    ctx.scrollback_next_seq = ctx.scrollback_next_seq.saturating_add(1);
    let lines = wrap_source(&source, DEFAULT_WRAP_WIDTH);
    ctx.scrollback.push(RenderedChunk {
        seq,
        lines,
        source: Some(source),
    });
    trim_to_cap(ctx, None);
}

/// Insert a chunk at `idx`, shifting subsequent chunks right. Mirrors
/// [`push_source`] but uses [`crate::views::agent::ScrollbackList::insert`].
/// **RPC-093**: used by `chunk_processor::append_thinking` to splice a
/// new thinking chunk BEFORE an in-flight assistant chunk (TS parity
/// with the `appendThinking` splice-before-streaming-assistant rule).
///
/// Returns the allocated `seq`.
///
/// **BUG-192**: trims after the insert and shifts the in-flight slots by
/// BOTH the insert (+1 for slots at or beyond `idx`) and the trim's net
/// change, so pre-existing slots keep pointing at the same chunks.
pub(crate) fn insert_source_at(ctx: &mut SessionContext, idx: usize, source: ChunkSource) -> u64 {
    let seq = ctx.scrollback_next_seq;
    ctx.scrollback_next_seq = ctx.scrollback_next_seq.saturating_add(1);
    let lines = wrap_source(&source, DEFAULT_WRAP_WIDTH);
    ctx.scrollback.insert(
        idx,
        RenderedChunk {
            seq,
            lines,
            source: Some(source),
        },
    );
    trim_to_cap(ctx, Some(idx));
    seq
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use codelet_rpc_types::SessionId;

    use super::*;
    use crate::views::agent::{ChunkKind, MAX_SCROLLBACK_VISUAL_ROWS};

    fn make_source(text: &str) -> ChunkSource {
        ChunkSource {
            text: text.to_string(),
            color: ratatui::style::Color::White,
            kind: ChunkKind::Notification,
            is_streaming: false,
            full_text: None,
        }
    }

    /// BUG-192 rule: an insert shifts the in-flight slots AT OR BEYOND
    /// the insert index right by 1; slots below it stay put.
    #[test]
    fn shift_in_flight_slots_moves_slots_at_or_beyond_the_insert_index() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        ctx.in_flight_assistant = Some(1);
        ctx.in_flight_thinking = Some(0);
        shift_in_flight_slots(&mut ctx, Some(1), 0);
        assert_eq!(ctx.in_flight_assistant, Some(2));
        assert_eq!(ctx.in_flight_thinking, Some(0));
    }

    /// BUG-192 rule: the trim's net shift (`-removed + marker`) is
    /// applied to every armed slot, floored at 0.
    #[test]
    fn shift_in_flight_slots_applies_the_trim_shift_floored_at_zero() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        ctx.in_flight_assistant = Some(3);
        shift_in_flight_slots(&mut ctx, None, -2);
        assert_eq!(ctx.in_flight_assistant, Some(1));
        ctx.in_flight_assistant = Some(1);
        shift_in_flight_slots(&mut ctx, None, -2);
        assert_eq!(ctx.in_flight_assistant, Some(0));
    }

    /// BUG-192 rule: `trim_to_cap` defers the trim (returns the default
    /// `TrimResult`) when an in-flight chunk sits at index 0 — the
    /// marker would otherwise occupy that slot.
    #[test]
    fn trim_to_cap_defers_when_an_in_flight_chunk_sits_at_index_zero() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        // One oversized chunk (20_001 lines > cap 20_000) so the cap
        // math would fire if the trim ran.
        let huge = "x ".repeat(MAX_SCROLLBACK_VISUAL_ROWS + 1);
        push_source(&mut ctx, make_source(&huge));
        ctx.in_flight_assistant = Some(0);
        let res = trim_to_cap(&mut ctx, None);
        assert_eq!(
            res.removed_chunks, 0,
            "no chunk may be removed while deferred"
        );
        assert!(
            !res.marker_inserted,
            "no marker may be inserted while deferred"
        );
        assert_eq!(
            ctx.scrollback.chunk_count(),
            1,
            "deferral must not trim the oversized in-flight chunk"
        );
        assert_eq!(ctx.in_flight_assistant, Some(0));
    }

    /// BUG-192 rule: with no armed in-flight slot a plain tail push
    /// trims to the cap and the seq cursor keeps advancing.
    #[test]
    fn push_source_trims_to_cap_when_no_in_flight_slot_is_armed() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        for _ in 0..(MAX_SCROLLBACK_VISUAL_ROWS / 100 + 4) {
            push_source(&mut ctx, make_source("x"));
        }
        let after_count = ctx.scrollback.chunk_count();
        // Each pushed chunk is one visual row; the trim keeps the total
        // (marker row included) at or under MAX_SCROLLBACK_VISUAL_ROWS.
        assert!(
            after_count <= MAX_SCROLLBACK_VISUAL_ROWS + 1,
            "expected trim to fire, got {after_count} chunks"
        );
        assert!(
            ctx.scrollback_next_seq == after_count as u64,
            "seq cursor must equal the number of pushed content chunks"
        );
    }

    /// BUG-192 rule: an `insert_source_at` at a mid index shifts the
    /// in-flight slots at or beyond the insert index right by 1.
    #[test]
    fn insert_source_at_shifts_in_flight_slots_at_or_beyond_the_insert_index() {
        let mut ctx = SessionContext::new(SessionId::new("s-1"));
        // Seed three chunks so index 2 is a valid insert target.
        for _ in 0..3 {
            push_source(&mut ctx, make_source("seed"));
        }
        ctx.in_flight_assistant = Some(2);
        let seq = insert_source_at(&mut ctx, 2, make_source("spliced"));
        assert_eq!(seq, ctx.scrollback_next_seq - 1);
        assert_eq!(ctx.in_flight_assistant, Some(3));
    }
}
