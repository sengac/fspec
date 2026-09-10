//! ScrollbackList — derived row math + scroll-window clamps.
//!
//! Extracted from `scrollback.rs` so the widget file stays under the
//! 300-LoC source-shape ceiling (rpc094 pin).

use super::ScrollbackList;

impl ScrollbackList {
    /// Sum of `chunk.lines.len()` across every chunk — the total visible
    /// row count once everything is unfurled.
    pub(crate) fn total_visual_rows(&self) -> usize {
        self.chunks.iter().map(|c| c.lines.len()).sum()
    }

    pub(super) fn max_offset_for_viewport(&self) -> usize {
        let total = self.total_visual_rows();
        let vh = self.viewport_height as usize;
        if vh == 0 || total <= vh {
            0
        } else {
            total.saturating_sub(vh)
        }
    }

    pub(super) fn recompute_offset_for_stick(&mut self) {
        self.scroll_state.offset = self.max_offset_for_viewport();
    }
}

