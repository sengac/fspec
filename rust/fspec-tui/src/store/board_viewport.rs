//! BoardStore RPC-016 extensions — per-column scroll viewport math.
//!
//! Feature: spec/features/rpc016-board-store-viewport.feature
//!
//! Lives in a sibling file so `store/board.rs` stays under the 300 LoC
//! file-size invariant. Operates on the `pub(super)` field surface of
//! [`BoardStore`] declared in `board.rs`.

use codelet_rpc_types::WorkUnitInfo;

use super::board::BoardStore;

impl BoardStore {
    /// RPC-016: read the current `scroll_offset` for `column`. Defaults
    /// to 0 — mirrors the TS UnifiedBoardLayout `scrollOffsets[column] ?? 0`.
    pub fn scroll_offset_for(&self, column: &str) -> usize {
        self.scroll_offsets.get(column).copied().unwrap_or(0)
    }

    /// RPC-016: set the `scroll_offset` for `column`. Called from
    /// App::dispatch on Action::ScrollFocusedColumnUp/Down + by the
    /// auto-scroll helpers below.
    pub fn set_scroll_offset_for(&mut self, column: &str, offset: usize) {
        self.scroll_offsets.insert(column.to_string(), offset);
    }

    /// RPC-016: move the focused column's `selected_index` by `delta`
    /// rows, wrapping unconditionally at both ends, and auto-scroll so
    /// the selection stays visible. Mirrors TS `onWorkUnitChange` in
    /// `src/tui/components/BoardView.tsx:535-540` — going below 0 wraps
    /// to the last unit, going past the last unit wraps to 0. Wrap
    /// happens regardless of whether the column is scrollable.
    pub fn move_selection(&mut self, delta: i32, viewport_height: usize) {
        let column = self.focused_column().to_string();
        let len = self.by_column.get(&column).map(Vec::len).unwrap_or(0);
        if len == 0 || viewport_height == 0 {
            return;
        }
        let current = self.selected_index_for(&column) as i32;
        let proposed = current.saturating_add(delta);
        let len_i = len as i32;
        // Unconditional wrap-around — euclidean modulo handles both
        // directions even when |delta| > len.
        let wrapped = proposed.rem_euclid(len_i) as usize;
        self.selected_index_per_column
            .insert(column.clone(), wrapped);
        adjust_scroll_offset(self, &column, wrapped, viewport_height, len);
    }

    /// RPC-016: PageUp/PageDown — advance the focused column's
    /// selection by `delta * viewport_height` rows.
    pub fn scroll_focused_column(&mut self, delta: i32, viewport_height: usize) {
        let step = (delta as i64).saturating_mul(viewport_height as i64);
        let clipped = step.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        self.move_selection(clipped, viewport_height);
    }

    /// RPC-016: Home — reset focused column to selected_index=0, offset=0.
    pub fn select_first_in_focused(&mut self) {
        let column = self.focused_column().to_string();
        self.selected_index_per_column.insert(column.clone(), 0);
        self.scroll_offsets.insert(column, 0);
    }

    /// RPC-016: End — jump to the LAST work unit of the focused column.
    ///
    /// Auto-scrolling for End is deferred until the next render +
    /// arrow-key dispatch — this method has no viewport_height context
    /// (mirrors architecture note [4] which keeps the Action variant
    /// argument-free). Setting `selected_index` alone is sufficient for
    /// the spec scenario; if the cell falls outside the viewport, the
    /// next `move_selection` call (or any explicit scroll action) will
    /// pull it back into view.
    pub fn select_last_in_focused(&mut self) {
        let column = self.focused_column().to_string();
        let len = self.by_column.get(&column).map(Vec::len).unwrap_or(0);
        if len == 0 {
            return;
        }
        let last = len - 1;
        self.selected_index_per_column.insert(column, last);
    }

    /// RPC-016: borrow the work unit with the largest
    /// `last_state_change_at` timestamp. `None` when none carry one.
    pub fn last_changed_unit(&self) -> Option<&WorkUnitInfo> {
        self.work_units
            .iter()
            .filter(|u| u.last_state_change_at.is_some())
            .max_by(|a, b| {
                a.last_state_change_at
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.last_state_change_at.as_deref().unwrap_or(""))
            })
    }

    /// RPC-023: set the focused column's `selected_index` to `index`
    /// (clamped to the column length) and re-run the same
    /// `adjust_scroll_offset` helper `move_selection` uses. Mirrors the
    /// TS UnifiedBoardLayout's behaviour when a user clicks on a
    /// content row: the click both targets the row AND scrolls it into
    /// view if it falls outside the current viewport.
    pub fn select_index_in_focused(&mut self, index: usize, viewport_height: usize) {
        let column = self.focused_column().to_string();
        let len = self.by_column.get(&column).map(Vec::len).unwrap_or(0);
        if len == 0 {
            return;
        }
        let clamped = index.min(len.saturating_sub(1));
        self.selected_index_per_column
            .insert(column.clone(), clamped);
        if viewport_height == 0 {
            return;
        }
        adjust_scroll_offset(self, &column, clamped, viewport_height, len);
    }
}

/// Free-function helper invoked by `move_selection` and
/// `select_last_in_focused` to compute the new scroll offset for the
/// focused column so the selection stays visible. Pure function in the
/// arithmetic sense — operates on `BoardStore`'s `scroll_offsets` map
/// only.
fn adjust_scroll_offset(
    store: &mut BoardStore,
    column: &str,
    selected: usize,
    viewport_height: usize,
    len: usize,
) {
    if viewport_height == 0 || len == 0 {
        return;
    }
    let mut offset = store.scroll_offset_for(column);
    let down_arrow = offset.saturating_add(viewport_height) < len;
    let last_visible = offset
        .saturating_add(viewport_height)
        .saturating_sub(if down_arrow { 1 } else { 0 })
        .saturating_sub(1);
    let up_arrow = offset > 0;
    let first_visible = offset + if up_arrow { 1 } else { 0 };

    if selected < first_visible {
        // Scroll up: position selected near the top, accounting for the
        // ↑ arrow that consumes one viewport row. Mirrors TS
        // src/tui/components/UnifiedBoardLayout.tsx:206-211.
        offset = selected.saturating_sub(1);
    } else if selected > last_visible {
        // Scroll down: two-pass algorithm matching TS
        // src/tui/components/UnifiedBoardLayout.tsx:212-222. The first
        // pass assumes BOTH arrows are visible (effective height =
        // viewport_height - 2); the second pass recomputes which arrows
        // would actually appear at the first-pass offset and shifts the
        // offset by `-1` when no ↑ arrow will be drawn (so the selected
        // item ends up on the last content row, not one row above it).
        let estimated_effective = viewport_height.saturating_sub(2);
        let first_pass = (selected + 1).saturating_sub(estimated_effective);
        let test_up_arrow = first_pass > 0;
        let test_down_arrow = first_pass.saturating_add(viewport_height) < len;
        let test_arrows = usize::from(test_up_arrow) + usize::from(test_down_arrow);
        let test_effective = viewport_height.saturating_sub(test_arrows);
        let tail_correction = usize::from(!test_up_arrow);
        let mut new_offset = (selected + tail_correction).saturating_sub(test_effective);
        // Clamp to [0, len - viewport_height] — TS uses
        // `Math.max(0, columnUnits.length - VIEWPORT_HEIGHT)`.
        let max_offset = len.saturating_sub(viewport_height);
        if new_offset > max_offset {
            new_offset = max_offset;
        }
        offset = new_offset;
    }
    store.scroll_offsets.insert(column.to_string(), offset);
}
