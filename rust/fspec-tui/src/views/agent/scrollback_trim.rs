//! BUG-192 — scrollback visual-row cap + oldest-content trim.
//!
//! Feature: spec/features/agentview-scrollback-unbounded-growth-cap-total-visual-rows.feature
//!
//! `ScrollbackList` accumulates `RenderedChunk`s without bound for the life
//! of a session, so steady-state frames cost O(total rows) (row
//! fingerprinting + row-count walk + offset math). This module bounds that:
//! [`ScrollbackList::trim_to_cap`] drops the OLDEST complete chunks until the
//! total visual-row count is back at or under `cap`, replacing them with a
//! single dim opaque (source-less, non-rewrapping) marker line whose count
//! accumulates across trims. Trim never removes the last real chunk nor a
//! protected in-flight (streaming) chunk, compensates the scroll offset so a
//! scrolled-up viewport stays pinned, and re-resolves the SELECT-mode
//! selection from seq (clearing it if the trimmed turn is gone).
//!
//! Driven from the store's `push_source` / `insert_source_at` / in-flight
//! growth paths so every chunk producer is covered; in-flight slot indices
//! are shifted by the net chunk-count change so streaming chunks survive.
//!
//! Extracted from `scrollback.rs` (300-LoC ceiling) matching the existing
//! `scrollback_*` sibling-module pattern.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::views::agent::RenderedChunk;

use super::ScrollbackList;

/// BUG-192: per-session cap on total scrollback visual rows.
///
/// Chosen by simulation (`examples/scrollback_perf.rs`, 200×45 viewport,
/// release): 20k rows ≈ 10.4 ms/frame (65% of the 16 ms render tick) vs 50k
/// ≈ 27 ms (171% — stutter threshold) and 200k ≈ 128 ms (the reported
/// cliff). ≈ 400 pages of scrollback; bounds per-session memory at ~3.3 MB.
/// Lower to 10_000 if post-ship profiling shows 20k is tight on low-end
/// machines (one-line change).
pub const MAX_SCROLLBACK_VISUAL_ROWS: usize = 20_000;

/// The stable `seq` of the trim-marker chunk. `scrollback_next_seq` starts at
/// 0 and grows by 1 per session content chunk, so `u64::MAX` can never be
/// allocated for real content.
const TRIM_MARKER_SEQ: u64 = u64::MAX;

/// Outcome of a [`ScrollbackList::trim_to_cap`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrimResult {
    /// Chunks removed by this trim (the marker is not counted).
    pub removed_chunks: usize,
    /// Visual rows removed by this trim (the marker's single row is NOT
    /// counted — this is the raw number of content rows dropped).
    pub removed_rows: usize,
    /// True when this trim INSERTED the marker chunk (as opposed to updating
    /// a marker that already existed).
    pub marker_inserted: bool,
}

impl ScrollbackList {
    /// BUG-192: drop the OLDEST complete chunks until the total visual-row
    /// count is at or under `cap`, replacing them with a single dim opaque
    /// marker line whose count accumulates across trims.
    ///
    /// `protected_floor` is the lowest chunk index (pre-trim) that must
    /// never be removed (in-flight streaming chunks); the trim stops just
    /// below it. **`protected_floor == 0` is the "no protection" sentinel**
    /// — the store passes 0 when no in-flight chunk exists, and a real
    /// in-flight chunk can never sit at index 0 after a trim (the marker
    /// occupies 0 and trims never remove the in-flight chunk). The last
    /// real chunk is never removed, so an oversized single chunk survives
    /// — the only permitted overshoot over `cap`.
    ///
    /// Budget: the post-trim total (marker row included) must be at or under
    /// `cap`. When a marker already exists it occupies 1 of those rows, so
    /// the loop budget is `cap`; when it is about to be inserted, the
    /// content must land at `cap - 1` so the inserted marker row fits —
    /// both cases stop at the same content level (`cap - 1`).
    ///
    /// Scroll-offset compensation: the removed rows sit ABOVE the viewport,
    /// so a non-stick offset is reduced by `removed_rows - 1` (the marker
    /// row they are replaced by); offsets that pointed inside the removed
    /// region clamp to 0 (the marker). Stick-to-bottom sessions just
    /// re-anchor (no visible change). The SELECT-mode selection is
    /// re-resolved from seq, clearing automatically if the selected turn
    /// was trimmed.
    pub fn trim_to_cap(&mut self, cap: usize, protected_floor: usize) -> TrimResult {
        // The marker must be chunk 0 — if a widget-level fixture pushed
        // opaque chunks after the marker was inserted (displacing it), move
        // it back to the front first. The store never produces this shape.
        if let Some(i) = self.is_trim_marker() {
            if i != 0 {
                let marker = self.chunks.remove(i);
                self.chunks.insert(0, marker);
            }
        }
        let marker_present = self.is_trim_marker().is_some();

        // Removal budget: `protected_floor == 0` is the "no protection"
        // sentinel (the store passes 0 when no in-flight chunk exists).
        // Otherwise the floor chunk must survive: each removal shifts it
        // left by one, so at most `floor - marker_present` chunks can be
        // dropped (the floor lands at index 0 without a marker, or index 1
        // — right after the marker — with one).
        let mut removable = if protected_floor == 0 {
            usize::MAX
        } else if marker_present {
            protected_floor.saturating_sub(1)
        } else {
            protected_floor
        };

        // Budget: the post-trim total (marker row included) must be at or
        // under `cap`. When a marker already exists it occupies 1 of those
        // rows, so the loop budget is `cap`; when it is about to be
        // inserted, the content must land at `cap - 1` so the inserted
        // marker row fits — both cases stop at the same content level
        // (`cap - 1`).
        let budget = if marker_present {
            cap
        } else {
            cap.saturating_sub(1)
        };
        let mut running_total = self.total_visual_rows();
        if running_total <= budget {
            return TrimResult::default();
        }

        // Drop the oldest complete chunks first (the marker, if present, is
        // chunk 0 and is never a removal candidate). Stop at the protected
        // floor and never at the last real chunk. `Vec::remove(idx)` keeps
        // the next oldest chunk at the SAME index, so `idx` never advances;
        // `removable` counts how many chunks the floor still allows.
        let mut removed_chunks = 0usize;
        let mut removed_rows = 0usize;
        let idx = usize::from(marker_present);
        while removable > 0 && idx + 1 < self.chunks.len() && running_total > budget {
            running_total -= self.chunks[idx].lines.len();
            removed_rows += self.chunks[idx].lines.len();
            self.chunks.remove(idx);
            removed_chunks += 1;
            removable -= 1;
        }

        if removed_rows == 0 {
            return TrimResult::default();
        }

        // Accumulate the trimmed counter and build (or update) the marker.
        self.trimmed_rows_total += removed_rows;
        let marker_inserted = if marker_present {
            if let Some(chunk) = self.chunks.first_mut() {
                chunk.lines = vec![marker_line(self.trimmed_rows_total)];
            }
            false
        } else {
            self.chunks.insert(
                0,
                RenderedChunk {
                    seq: TRIM_MARKER_SEQ,
                    lines: vec![marker_line(self.trimmed_rows_total)],
                    source: None,
                },
            );
            true
        };

        // Scroll-offset compensation: the removed rows were above the
        // viewport and are now replaced by the 1-row marker, so surviving
        // content shifted up by `removed_rows - 1`. Offsets that pointed
        // into the removed region saturate to 0 (the marker row).
        let net_shift = removed_rows - 1;
        if self.scroll_state.stick_to_bottom {
            self.recompute_offset_for_stick();
        } else {
            self.scroll_state.offset = self.scroll_state.offset.saturating_sub(net_shift);
        }

        // Re-pin the SELECT-mode selection from its remembered seq — if the
        // selected turn was trimmed, the seq lookup fails and it clears.
        self.resolve_selection_from_seq();

        TrimResult {
            removed_chunks,
            removed_rows,
            marker_inserted,
        }
    }

    /// BUG-192: the index of the trim-marker chunk, if present. The marker
    /// is normally chunk 0 (inserted at the front, never moves); the scan
    /// keeps the lookup correct even for widget-level fixtures that push
    /// opaque chunks directly (which may have displaced it).
    fn is_trim_marker(&self) -> Option<usize> {
        self.chunks
            .iter()
            .position(|c| c.seq == TRIM_MARKER_SEQ && c.source.is_none())
    }

    /// BUG-192: true when the first chunk is the trim marker (a stable
    /// one-row opaque line that never re-wraps on resize).
    pub fn first_chunk_is_marker(&self) -> bool {
        self.is_trim_marker().is_some_and(|i| i == 0)
    }

    /// BUG-192: total trimmed rows accumulated across all trims since the
    /// last `reset` — the count shown in the marker.
    pub fn trimmed_rows_total(&self) -> usize {
        self.trimmed_rows_total
    }
}

/// The single dim marker line for a given accumulated trimmed-row count.
/// Opaque (`source: None`) so resize never re-wraps it — stable geometry.
fn marker_line(trimmed_total: usize) -> Line<'static> {
    let dim = Style::default().add_modifier(Modifier::DIM);
    let text = format!("\u{2026} {trimmed_total} older lines trimmed \u{2026}");
    Line::from(Span::styled(text, dim))
}

#[cfg(test)]
#[path = "scrollback_trim_tests.rs"]
mod tests;
