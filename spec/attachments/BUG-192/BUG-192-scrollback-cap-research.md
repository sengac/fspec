# BUG-192 — Research: capping AgentView scrollback growth

**Date:** 2026-09-22
**Scope:** `rust/fspec-tui` — `ScrollbackList` (AgentView scrollback widget)
**Reported symptom:** once a session's scrollback exceeds ~200,000 visual rows the AgentView becomes extraordinarily slow (dropped frames, sluggish input/scroll response).

---

## 1. Problem statement

The AgentView scrollback (`ScrollbackList`) accumulates `RenderedChunk`s (pre-wrapped visual
rows) **without any upper bound** for the life of a session. Every streamed assistant delta,
tool card, thinking block and user input adds more wrapped lines; nothing is ever removed
except `/clear` (`reset_scrollback`) and the in-place reconnect-notice replacement.

The BUG-190 row reflow cache fixed the *per-row repaint* cost (steady-state frames blit cached
cells instead of re-wrapping), but it does **not** bound the chunk list itself. Two per-frame
passes still scale with the **total** row count:

1. `paint_chunk_rows` (`views/agent/scrollback_paint.rs:77-106`) walks *every* chunk and every
   row to count past `skip_rows` — O(total rows) per frame even when only the tail is visible.
2. `RowKey::new` is built **per chunk, per frame** (`scrollback_paint.rs:86`), and
   `fingerprint_lines` (`reflow_cache.rs:232-242`) hashes every span of every line of that
   chunk — another O(total rows) per frame.
3. `total_visual_rows()` (`scrollback_tail.rs:11-13`) is summed per frame in
   `scrollback_render.rs` (twice: gutter decision + total) — O(chunk count), cheap, but the
   scrollbar/`jump_to_offset` clamps also use it.

In the busy steady state the run loop redraws at the 16 ms tick
(`app/run_loop.rs:17` — `RENDER_TICK`, `is_session_busy` keeps the tick drawing), so an
O(total rows) frame cost translates directly into a frozen-feeling TUI the longer a session
lives. In **mux mode** every agent pane paints its own session's scrollback
(`pane_render.rs:145-159`), multiplying the cost by the number of visible agent panes.

---

## 2. Architecture summary (where the state lives)

| Concern | Location |
|---|---|
| Chunk list + scroll state + reflow cache | `views/agent/scrollback.rs` — `ScrollbackList` (`chunks: Vec<RenderedChunk>`, `scroll_state`, `reflow_cache`) |
| Chunk shape | `views/agent/rendered_chunk.rs` — `RenderedChunk { seq, lines: Vec<Line>, source: Option<ChunkSource> }` |
| Wrap + collapse rules | `store/agent_view/chunk_wrap.rs` — `wrap_source` (tool cards self-collapse to 8–10 visible lines, RPC-389/399) |
| Chunk accumulation (streaming) | `store/agent_view/chunk_processor.rs` / `chunk_tool_result.rs` — index-based in-flight slots `in_flight_assistant`, `in_flight_thinking` (indices into `scrollback.chunks`) |
| Ingress (sanitization + routing) | `store/agent_view/record_chunk.rs` — `SessionContext::record_chunk` |
| Push/insert choke points | `store/agent_view/session_context.rs:118-147` — `push_source` / `insert_source_at` (the only producers of new chunks) |
| Render pass | `views/agent/scrollback_render.rs` → `paint_chunk_rows` (`scrollback_paint.rs`) |
| Scroll offset math | `scrollback_tail.rs` — `total_visual_rows`, `max_offset_for_viewport`, `recompute_offset_for_stick` |
| SELECT-mode selection | `scrollback_select.rs` — selection pinned by stable `seq` (auto-clears when the chunk disappears — `resolve_selection_from_seq`) |
| Turn modal / copy | read chunks by `seq` (`full_text_for_seq`, `kind_for_seq`) — return `None` for absent chunks |
| Reset | `ScrollbackList::reset` (`/clear`, `SessionContext::reset_scrollback`) |

Key invariant: **chunk `seq`s are per-session monotonic** and every mutation funnels through
`push_source` / `insert_source_at` (store side) or direct `chunks_mut()` rewrites of *existing*
chunks (reconnect notice, tool result — no list-length change). Only `push` / `insert` /
`remove(idx)` / `reset` change the list length.

---

## 3. Simulation (measured, real production surface)

Simulator: `rust/fspec-tui/examples/scrollback_perf.rs` (diagnostic scratch, mirrors
`probe_pause.rs`). It drives the **real** `ScrollbackList::push` + `render_count_visited`
path (BUG-190 reflow cache included) with a realistic agent-turn mix at a 200×45 viewport
(typical AgentView pane): user input + thinking block + ~4–18 KB assistant prose + a tool
card (collapsed per RPC-389/399) + trailing prose. Run in `--release` on the dev box
(aarch64 Linux, 20 cores).

### Steady-state frame (stick-to-bottom, cache warm — the busy 60 fps repaint)

| total rows | chunks | avg frame | vs 16 ms budget | est. memory |
|---:|---:|---:|---:|---:|
| 1,000  | 20   | 0.6 ms   | 4%    | 0.2 MB |
| 5,000  | 63   | 2.8 ms   | 17%   | 0.9 MB |
| 20,000 | 217  | 10.4 ms  | 65%   | 3.3 MB |
| 50,000 | 515  | 27.4 ms  | 171%  | 8.2 MB |
| 100,000| 1,025| 58.8 ms  | 368%  | 16.3 MB |
| 200,000| 2,048| 127.6 ms | 800%  | 32.6 MB |

### Scrolling frame (wheel tick to a fresh offset, rows re-reflect once)

| total rows | avg frame | vs 16 ms budget |
|---:|---:|---:|
| 1,000  | 0.6 ms   | 4%   |
| 5,000  | 3.0 ms   | 19%  |
| 20,000 | 10.5 ms  | 66%  |
| 50,000 | 26.4 ms  | 165% |
| 100,000| 54.8 ms  | 343% |
| 200,000| 116.7 ms | 729% |

### Interpretation

* Cost scales **linearly** in total rows (≈0.6 µs/row/frame in the steady state) — consistent
  with the two O(total) passes identified in §1. There is no plateau: the widget is
  *designed* to virtualize painting, but fingerprinting + row counting are not virtualized.
* **20,000 rows ≈ 10 ms/frame** — fits the 16 ms tick with headroom for header/footer/
  composer/other panes; already the *worst acceptable* single-pane number.
* **50,000 rows ≈ 27 ms/frame** — the session can no longer render at 60 fps; the busy
  spinner visibly stutters. This is where users start feeling it.
* **200,000 rows ≈ 117–128 ms/frame** — matches the reported "extraordinarily slow": the main
  (UI) thread spends ~8× the frame budget on the scrollback alone, so input events, scroll
  events and the LLM chunk stream all queue behind 8 fps renders. (The user's 200k figure is
  exactly the measured cliff.)
* Memory is bounded-but-growing: ~0.16 KB/row + ~0.15 KB/chunk (lines, `ChunkSource` text,
  reflow cache). At 200k rows ≈ 33 MB *per session* — multiplied by open sessions in mux mode.

### Reproducing the numbers

```bash
cargo build --release -p codelet-fspec-tui --example scrollback_perf
./rust/target/release/examples/scrollback_perf
```

---

## 4. Cap decision

**Cap the scrollback at 20,000 visual rows per session.**

Rationale:

* Keeps the steady-state scrollback frame cost at ≤ ~10 ms on the reference machine (65% of
  the 16 ms tick), leaving ≥ 6 ms for the rest of the frame (header, footer, composer, other
  mux panes). The next tier down (10k rows ≈ 6 ms) would be safer but needlessly short:
  20k rows ≈ **~400 pages** of scrollback at a 45-row viewport and ≈ **40+ heavy agent
  turns** (the simulator's turns are ~500 rows each including thinking + tool output).
* The per-row cost (~0.6 µs) is on a modern aarch64 box; slower terminals/CPUs shift the
  budget, and 20k leaves margin for 2× regression before hitting the 50k cliff.
* Bounds per-session memory at ~3.3 MB regardless of session length (vs 32.6 MB unbounded).
* The cap is a **const**, not a setting, for v1: `pub const MAX_SCROLLBACK_VISUAL_ROWS: usize
  = 20_000;`. (A config knob can be added later; nothing here depends on it being dynamic.)

Alternative considered: caching `RowKey` fingerprints per chunk (recompute only on
rewrap) would cut the per-frame cost to O(visible rows) and could push the acceptable size
to ~100k rows. That is a **separate follow-up optimization**, not a substitute — the cap is
still needed to bound memory and the (non-virtualizable) row-count walk.

---

## 5. Design: how the code cuts off older conversation

### 5.1 Core trim operation (widget)

New module `views/agent/scrollback_trim.rs` (child of `scrollback` via `#[path]`, matching
the existing `scrollback_*` pattern; `scrollback.rs` is already near the 300-LoC ceiling):

```rust
/// Trim oldest complete chunks until total visual rows <= `cap`.
/// `protected_floor` = lowest chunk index that must NEVER be removed
/// (in-flight streaming chunks); the trim stops just below it.
/// `removed` = chunks dropped, `rows` = visual rows removed net of the
/// marker row they are replaced with.
pub(super) fn trim_to_cap(&mut self, cap: usize, protected_floor: usize)
    -> (usize removed_chunks, usize removed_rows)
```

Algorithm:

1. `total = total_visual_rows()`; if `total <= cap` or `chunks.len() <= 1` → no-op `(0, 0)`.
2. Determine marker state: if `chunks[0]` is the trim-marker chunk (opaque,
   `source: None`, tagged via a small `is_trim_marker` convention — e.g. a well-known
   `seq` sentinel `u64::MAX` or a `TrimMarker` flag on `RenderedChunk`), `oldest_index = 1`,
   else `oldest_index = 0`.
3. While `total > cap` and `oldest_index < protected_floor` and `chunks.len() - oldest_index > 1`
   (never drop the last real chunk):
   * `rows = chunks[oldest_index].lines.len()`; remove it (`Vec::remove(oldest_index)` —
     O(n) per removal, but trim runs only on push and removes a handful of chunks at a time;
     a drain-collect rewrite can be used instead: build the retained slice once).
   * accumulate `removed_chunks += 1; removed_rows += rows`.
4. Marker:
   * **no marker yet** → insert at index 0 a single-row *opaque* chunk
     (`source: None`, one dim `Line`: `"… {trimmed_total} older lines trimmed …"`).
     Use `ChunkKind`-agnostic opaque lines so it never re-wraps on resize (stable geometry).
   * **marker exists** → update its text in place (`chunks_mut().get_mut(0)`), accumulating
     the count: `trimmed_total += removed_rows`.
5. Scroll offset compensation (the trimmed rows were *above* the viewport):
   * net upward shift of remaining content = `removed_rows − 1` (marker is 1 row).
   * `stick_to_bottom = true` → just `recompute_offset_for_stick()` (no visible change).
   * else `offset = offset.saturating_sub(removed_rows − 1)`; if the old offset pointed into
     the removed region, clamp to `0` (marker + start of surviving content shown).
6. `resolve_selection_from_seq()` — if the SELECT-mode selected turn was trimmed, the seq
   lookup fails and the selection clears automatically (existing behaviour, `scrollback_select.rs:193`).
7. Store `trimmed_rows_total: usize` on `ScrollbackList` (persisted only in-memory;
   `/clear` resets it via `reset()`).

Notes:

* `Vec::remove(0)`-style front removal is O(n) per chunk, but trim executes only on chunk
  insert and removes O(chunks trimmed per push) ≈ 1–5 chunks; n ≈ ≤ 20k/avg-rows-per-chunk
  ≈ a few hundred chunks. Worst case ~10⁵ element moves = sub-microsecond-class; acceptable
  (and the alternative is the same complexity as today's `insert_source_at` which already
  does `Vec::insert(idx, …)`).
* The reflow cache keeps stale entries for removed chunks (bounded at 512 chunks, head
  eviction on new-chunk upsert) — no action needed; they self-evict.
* Opaque marker chunk: `full_text_for_seq`/turn modal for the marker seq returns the marker
  text — harmless (SELECT mode may "select" it; Enter shows a modal with the one-line
  message, which is self-explanatory).

### 5.2 Store-side wiring (index integrity)

`SessionContext` tracks `in_flight_assistant` / `in_flight_thinking` as **indices** into
`scrollback.chunks`. Trim shifts those indices, so the trim must be coordinated at the store:

`store/agent_view/session_context.rs` (new helper, ~30 LoC; keep file under the 300-LoC
ceiling by placing it in a sibling module if needed, e.g. `scrollback_trim_state.rs`):

```rust
pub(crate) fn trim_scrollback_to_cap(&mut self) {
    // protect the in-flight (streaming) chunks from removal:
    let floor = self
        .in_flight_assistant
        .iter()
        .chain(self.in_flight_thinking.iter())
        .copied()
        .min()
        .unwrap_or(usize::MAX);
    let (removed, _rows) =
        self.scrollback.trim_to_cap(MAX_SCROLLBACK_VISUAL_ROWS, floor);
    // Net index shift: -removed (dropped) +1 if a new marker was inserted
    // (trim returns marker-inserted flag), i.e. shift = (marker_inserted as usize) - removed.
    let shift = self.scrollback.marker_inserted_this_trim() as isize - removed as isize;
    if shift != 0 {
        self.in_flight_assistant =
            self.in_flight_assistant.map(|i| (i as isize + shift).max(0) as usize);
        self.in_flight_thinking =
            self.in_flight_thinking.map(|i| (i as isize + shift).max(0) as usize);
    }
}
```

Call sites — exactly the two chunk-producing choke points:

* `SessionContext::push_source` (after `scrollback.push`)
* `SessionContext::insert_source_at` (after `scrollback.insert`)

This covers **every** producer: `record_chunk` (all StreamChunk variants), `push_line`
(notices), `append_assistant_text`, `append_thinking`, tool call/result/progress,
reconnect notices. Direct `chunks_mut()` rewrites (tool result body, reconnect
replacement) don't change length → no re-trim needed.

**Accepted edge case:** `set_viewport_width` re-wraps all chunks on resize and can *increase*
total rows past the cap (narrowing the terminal). The overshoot persists until the next
content mutation (a few µs–ms of extra work per frame, bounded by the rewrap amplification
factor, and the user is mid-resize, not mid-stream). Documented as accepted; a future
follow-up can re-trim from the render path once in-flight tracking is seq-based.

### 5.3 Behaviour summary (what the user sees)

* After a session exceeds 20k rows, the oldest chunks vanish and the first visible row when
  scrolled to the top becomes a dim one-liner:
  `… 12,483 older lines trimmed …` (count accumulates as more is trimmed).
* Stick-to-bottom streaming is unaffected (the tail never moves).
* Scrolled-up users: the viewport stays pinned to the *same content* (offset is
  compensated); the only visible change is the marker appearing at the very top.
* SELECT mode: a trimmed selected turn clears its selection (existing seq-resolution).
* Turn modal for a trimmed turn: `full_text_for_seq` → `None` → modal stays closed
  (existing handling in `turn_modal_select.rs:68`).
* `/clear` and session removal reset everything as today.
* Mux mode: each pane's session trims independently.

### 5.4 Out of scope

* LLM context compaction (`/compact`, LIMITS/CMPCT systems) — unrelated to UI scrollback.
* The O(visible) fingerprint-caching optimization (separate follow-up card if we ever want
  to raise the cap or remove the row-count walk).
* Persisting trimmed content (the durable session history on the backend is untouched —
  this is purely a UI-side buffer; `/resume` replays the last 1000 chunks regardless).

---

## 6. Test plan (ACDD)

Feature file (new): `spec/features/agentview-scrollback-cap.feature`, tagged
`@BUG-192` (+ existing component tags). Scenarios:

1. **Below cap:** N pushes totalling ≤ 20k rows → `chunk_count` unchanged, no marker,
   `total_visual_rows` equals the pushed sum.
2. **Trim at cap:** pushes totalling > 20k rows → `total_visual_rows ≤ cap`; oldest chunks
   gone; newest chunks intact (seqs preserved).
3. **Marker content:** after trimming, chunk 0 is the dim marker line
   `… {n} older lines trimmed …` with n = exact rows removed (accumulate across two trims).
4. **Sticky tail unchanged:** while `stick_to_bottom`, the last viewport rows paint the same
   content before/after trim.
5. **Scrolled-up offset stable:** with `stick_to_bottom=false` and a fixed offset, the row
   at a given viewport position after trim carries the same text as before (offset
   compensation).
6. **Offset clamped when scrolled above trim point:** offset inside the removed region →
   0, marker visible at the top of the viewport.
7. **In-flight integrity:** an in-progress assistant chunk is never removed and its
   `in_flight_assistant` index still points at the same chunk after a trim (regression:
   streaming continues appending to the right chunk).
8. **Selection cleared when selected turn trimmed:** SELECT mode with selection on an old
   turn → after trim, `selected_seq()` is `None`.
9. **Reset:** `reset_scrollback` clears chunks, marker and trimmed counter.
10. **Per-session isolation:** trimming session A's scrollback never touches session B's.

Test locations: unit tests in `scrollback_trim.rs` / `session_context.rs` inline `#[cfg(test)]`
modules; one integration file `tests/agentview_scrollback_cap_bug192.rs` using the real
`SessionContext::record_chunk` path with a synthetic chunk stream (reuse the
`source_chunk`/`opaque_chunk` helper pattern from
`tests/scrollback_row_reflow_cache_bug190.rs`). Property test (proptest): for random push
sequences, the invariant `total_visual_rows ≤ cap` (or a single-chunk-only list) always
holds.

---

## 7. File-level change list (expected)

| File | Change |
|---|---|
| `views/agent/scrollback_trim.rs` (new) | `trim_to_cap`, marker build/update, offset compensation, `trimmed_rows_total` accessor; unit tests |
| `views/agent/scrollback.rs` | `#[path] mod trim;` + field `trimmed_rows_total` + `MAX_SCROLLBACK_VISUAL_ROWS` const + `marker_inserted_this_trim` + `reset` clears new state |
| `views/agent/rendered_chunk.rs` | optional `is_trim_marker: bool` flag (or well-known seq sentinel) on `RenderedChunk` |
| `store/agent_view/session_context.rs` | `trim_scrollback_to_cap` + call in `push_source` / `insert_source_at` |
| `store/agent_view.rs` | (no change — or re-export) |
| `tests/agentview_scrollback_cap_bug192.rs` (new) | integration scenarios above |
| `spec/features/agentview-scrollback-cap.feature` (new) | acceptance criteria |

Estimate: **5 points** (multiple files, clear integration, offset/selection index
integrity subtleties, new tests; no architecture change).

---

## 8. Key file references

* `rust/fspec-tui/src/views/agent/scrollback.rs:41-65` — `ScrollbackList` fields
* `rust/fspec-tui/src/views/agent/scrollback_paint.rs:69-108` — `paint_chunk_rows` (O(total) row walk + per-chunk `RowKey::new`)
* `rust/fspec-tui/src/views/agent/reflow_cache.rs:150-179,232-242` — `paint_row` + `fingerprint_lines` (O(chunk rows) per chunk per frame)
* `rust/fspec-tui/src/views/agent/scrollback_tail.rs:11-27` — total-row math
* `rust/fspec-tui/src/views/agent/scrollback_render.rs:31-63` — per-frame gutter/total passes
* `rust/fspec-tui/src/store/agent_view/session_context.rs:118-147` — `push_source` / `insert_source_at` (trim hook points)
* `rust/fspec-tui/src/store/agent_view/chunk_processor.rs` — index-based in-flight slots (shift hazard)
* `rust/fspec-tui/src/views/agent/scrollback_select.rs:193-201` — seq-based selection resolution (trim-safe)
* `rust/fspec-tui/src/app/run_loop.rs:16-17,84-109` — 16 ms render tick + busy redraw
* `rust/fspec-tui/src/views/agent/pane_render.rs:145-159` — per-mux-pane scrollback render
