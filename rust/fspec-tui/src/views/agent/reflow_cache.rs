//! BUG-190 — per-row reflow cache for the AgentView scrollback.
//!
//! Feature: spec/features/scrollback-row-reflow-cache.feature
//!
//! While a session is busy, the 16ms render tick draws full frames at
//! ~60fps. The old paint path rebuilt a ratatui `Paragraph` per visible
//! row per frame, so every frame re-wrapped + re-grapheme-scanned +
//! re-width-measured every visible row even though a busy turn's steady
//! state only changes the tail (newly-appended lines + scroll offset).
//!
//! This cache memoizes the REFLECTED result of each row — the exact
//! `Cell`s the ratatui path would paint — keyed by
//! (chunk seq, chunk content fingerprint, content width):
//!
//! - HIT (fingerprint + width unchanged): blit the stored cells into the
//!   frame buffer — no wrapping, no grapheme scan, no width measurement.
//! - MISS: reflect the row once through the SAME ratatui path the
//!   uncached painter uses (`Paragraph::new(line).render` into a
//!   1-row scratch buffer) and store the trimmed result.
//!
//! The cached path is byte-identical to the uncached path (R2) because
//! both paint through ratatui's `Buffer::set_line` semantics on the same
//! 1-row geometry. Scrollbar / arrow-bar / selection-highlight painting
//! stays per-frame and untouched (R5).
//!
//! Invalidation needs NO call-site changes (R4): every production
//! mutation funnels through `ScrollbackList` rewrap paths, and any
//! `chunks_mut()` direct write changes the stored content — which the
//! per-frame fingerprint detects. `ScrollbackList::reset` clears the
//! cache entirely. The cache is bounded to `MAX_CHUNK_ENTRIES` with
//! head-eviction (insertion order = arrival order).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};

use super::RenderedChunk;

/// Bounded chunk entries: long sessions drop the OLDEST chunks' rows
/// (head eviction — insertion order matches per-session monotonic seqs).
pub(crate) const MAX_CHUNK_ENTRIES: usize = 512;

/// BUG-190 read-only counters for tests + profiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowReflowCounter {
    reflow_calls: usize,
    blit_hits: usize,
    cached_chunks: usize,
}

impl RowReflowCounter {
    /// Rows re-reflected (cache miss) since the cache's last reset (R1).
    pub fn reflow_calls(&self) -> usize {
        self.reflow_calls
    }

    /// Rows blitted from cache (cache hit) since the last reset (R1).
    pub fn blit_hits(&self) -> usize {
        self.blit_hits
    }

    /// Cached chunk entries (bounded at `MAX_CHUNK_ENTRIES`).
    pub fn chunk_count(&self) -> usize {
        self.cached_chunks
    }
}

/// One cached chunk's reflected rows.
#[derive(Debug)]
struct CachedChunk {
    seq: u64,
    /// Fingerprint of `chunk.lines` at store time (R3).
    fingerprint: u64,
    /// Content width the rows were reflected at (part of the cache key).
    width: u16,
    /// Reflected cells per row — exactly the extent the uncached path
    /// writes (columns `0..content_width`). `None` = not yet reflected
    /// this key; `Some(empty)` = blank row (a hit that blits nothing).
    rows: Vec<Option<Box<[Cell]>>>,
}

/// The cache key for one chunk's rows: its stable identity (`seq`),
/// the content fingerprint of `chunk.lines` (R3), and the content
/// width the rows are reflected at. A row is cached exactly while this
/// key is unchanged — steady-state frames then blit without re-wrapping
/// or grapheme-scanning (R1).
#[derive(Debug, Clone, Copy)]
pub struct RowKey {
    /// Stable chunk identity (per-session monotonic `RenderedChunk::seq`).
    pub seq: u64,
    /// Fingerprint of the chunk's stored lines (R3).
    pub fingerprint: u64,
    /// Content width (viewport minus the RPC-094 2-col gutter when
    /// reserved — the width `paint_chunk_rows` passes to each row).
    pub width: u16,
}

impl RowKey {
    pub fn new(chunk: &RenderedChunk, width: u16) -> Self {
        Self {
            seq: chunk.seq,
            fingerprint: fingerprint_lines(&chunk.lines),
            width,
        }
    }
}

/// Per-chunk reflected-row cache owned by `ScrollbackList`.
#[derive(Debug, Default)]
pub(crate) struct RowReflowCache {
    /// Insertion-ordered (arrival order). Bounded at `MAX_CHUNK_ENTRIES`.
    chunks: Vec<CachedChunk>,
    /// Rows re-reflected (miss) since the last `reset` (R1 counter).
    reflow_calls: usize,
    /// Rows blitted from cache since the last reset (R1 counter).
    blit_hits: usize,
}

impl RowReflowCache {
    /// Read-only counters for tests + profiling (R1).
    pub fn counters(&self) -> RowReflowCounter {
        RowReflowCounter {
            reflow_calls: self.reflow_calls,
            blit_hits: self.blit_hits,
            cached_chunks: self.chunks.len(),
        }
    }

    /// Drop every cached chunk (called from `ScrollbackList::reset`).
    /// Counters are CUMULATIVE by design (acceptance accounting).
    pub fn reset(&mut self) {
        self.chunks.clear();
    }

    /// Paint one chunk row (`row` = chunk-local index into
    /// `chunk.lines`) at `(x, y)`: blit the cached cells on a
    /// (fingerprint, width) hit; reflect through the uncached ratatui
    /// path on a miss and store the result.
    ///
    /// The cache row index is the CHUNK-LOCAL line index (NOT the screen
    /// `y`), so scrolling — which moves rows to different screen rows —
    /// never causes a spurious miss.
    ///
    /// Precondition: `(x, y)` lies inside `buf.area` (the caller walks
    /// the same windowed layout as the old uncached painter).
    pub fn paint_row(
        &mut self,
        key: RowKey,
        line: &Line<'static>,
        x: u16,
        y: u16,
        row: usize,
        buf: &mut Buffer,
    ) {
        // Single mutable lookup; `cells` borrows only `self.chunks` so the
        // disjoint-field `blit_hits` write below stays legal.
        let cells = self
            .chunks
            .iter_mut()
            .find(|c| c.seq == key.seq)
            .filter(|c| c.fingerprint == key.fingerprint && c.width == key.width)
            .and_then(|c| c.rows.get(row))
            .and_then(|c| c.as_deref());
        if let Some(cells) = cells {
            blit_cells(x, y, cells, buf);
            self.blit_hits = self.blit_hits.saturating_add(1);
        } else {
            let reflected = reflect_row_cells(line, key.width);
            upsert_row(&mut self.chunks, key, row, reflected.clone());
            if let Some(cells) = &reflected {
                blit_cells(x, y, cells, buf);
            }
            self.reflow_calls = self.reflow_calls.saturating_add(1);
        }
    }
}

/// Blit stored cells into `buf` starting at `(x, y)`.
fn blit_cells(x: u16, y: u16, cells: &[Cell], buf: &mut Buffer) {
    for (i, cell) in cells.iter().enumerate() {
        let cx = x.saturating_add(i as u16);
        if cx >= buf.area.right() {
            break;
        }
        buf[(cx, y)] = cell.clone();
    }
}

/// Insert or update one row inside a chunk entry, evicting the head
/// when the bound is exceeded (head = oldest arrival).
fn upsert_row(chunks: &mut Vec<CachedChunk>, key: RowKey, row: usize, cells: Option<Box<[Cell]>>) {
    if let Some(entry) = chunks.iter_mut().find(|c| c.seq == key.seq) {
        if entry.fingerprint == key.fingerprint && entry.width == key.width {
            if entry.rows.len() < row + 1 {
                entry.rows.resize(row + 1, None);
            }
            entry.rows[row] = cells;
            return;
        }
        // Fingerprint or width changed — stale rows are dropped; this row
        // is stored now, and the remaining rows re-reflect on their own
        // misses (R1/R3: each row re-flows exactly once).
        entry.fingerprint = key.fingerprint;
        entry.width = key.width;
        entry.rows = vec![None; row + 1];
        entry.rows[row] = cells;
        return;
    }
    if chunks.len() >= MAX_CHUNK_ENTRIES {
        chunks.remove(0);
    }
    let mut rows = vec![None; row + 1];
    rows[row] = cells;
    chunks.push(CachedChunk {
        seq: key.seq,
        fingerprint: key.fingerprint,
        width: key.width,
        rows,
    });
}

/// Content fingerprint for `chunk.lines` (R3): row count + every
/// span's content and style. O(working set); a direct `chunks_mut()`
/// write that changes any stored span is detected on the next frame.
/// Per-line alignment is intentionally NOT part of the key — the
/// scrollback paint path ignores it (unstyled Paragraph, no wrap),
/// mirroring the uncached path exactly.
pub fn fingerprint_lines(lines: &[Line<'_>]) -> u64 {
    let mut hasher = DefaultHasher::new();
    lines.len().hash(&mut hasher);
    for line in lines {
        for span in line.iter() {
            span.content.hash(&mut hasher);
            span.style.hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Reflect one row through the SAME ratatui path the uncached painter
/// uses: `Paragraph::new(line)` rendered into a 1-row scratch buffer
/// (identical geometry to `paint_chunk_rows`).
///
/// The stored cells are EXACTLY the extent the uncached path writes —
/// columns `0..extent` verbatim (glyphs, wide-grapheme continuation
/// blanks, trailing blanks within the extent). Ratatui's
/// `set_stringn` writes every column up to the content width (a space
/// grapheme or a wide-grapheme reset is still a write), and never a
/// column beyond it, so blitting this whole extent is byte-identical to
/// the uncached render (R2) — no trimming, no divergence on cells that
/// a shorter previous frame left behind. An empty row (extent 0) stores
/// an EMPTY cell list — a valid cache hit that blits nothing, so blank
/// separator rows never re-reflect in steady state.
fn reflect_row_cells(line: &Line<'static>, width: u16) -> Option<Box<[Cell]>> {
    let width = width as usize;
    if width == 0 {
        return None;
    }
    let mut scratch = Buffer::empty(Rect {
        x: 0,
        y: 0,
        width: width as u16,
        height: 1,
    });
    let area = Rect {
        x: 0,
        y: 0,
        width: width as u16,
        height: 1,
    };
    Paragraph::new(line.clone()).render(area, &mut scratch);
    let content_width = line.width();
    let extent = content_width.min(width);
    if extent == 0 {
        return Some(Box::new([]));
    }
    let mut cells: Vec<Cell> = Vec::with_capacity(extent);
    for col in 0..extent {
        cells.push(scratch[(col as u16, 0)].clone());
    }
    Some(cells.into_boxed_slice())
}

#[cfg(test)]
#[path = "reflow_cache_tests.rs"]
mod tests;
