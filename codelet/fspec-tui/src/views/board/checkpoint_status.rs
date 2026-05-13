//! Checkpoint status text widget — Rust port of the TS sub-widget
//! `src/tui/components/CheckpointStatus.tsx`.
//!
//! Feature: spec/features/rpc015-board-header.feature
//! Card: RPC-015.
//!
//! Paints either:
//!   - `Checkpoints: None` when both counts are 0, or
//!   - `Checkpoints: {manual} Manual, {auto} Auto` otherwise.

use codelet_rpc_types::CheckpointCounts;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Render the 1-row checkpoint status line into `area`. The area's
/// `width` is used as the maximum line length; `area.height` must be
/// at least 1.
pub fn render(area: Rect, buf: &mut Buffer, counts: CheckpointCounts) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text = format_text(counts);
    let line = Line::from(Span::styled(text, Style::default()));
    Paragraph::new(line).render(
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
        buf,
    );
}

/// Format the status text exactly as TS does in
/// `src/tui/components/CheckpointStatus.tsx`:
///
///   Both counts zero → `Checkpoints: None`
///   Otherwise         → `Checkpoints: {manual} Manual, {auto} Auto`
pub fn format_text(counts: CheckpointCounts) -> String {
    if counts.manual == 0 && counts.auto == 0 {
        "Checkpoints: None".to_string()
    } else {
        format!("Checkpoints: {} Manual, {} Auto", counts.manual, counts.auto)
    }
}
