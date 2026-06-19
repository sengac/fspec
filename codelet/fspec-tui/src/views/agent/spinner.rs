//! RPC-095 — braille-dots spinner painter.
//!
//! Mirrors `src/tui/components/ThinkingIndicator.tsx:19-22` (the
//! `'dots'` spinner style) byte-for-byte:
//!
//! - 10 frames: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`
//! - 80 ms per frame
//!
//! The painter is a pure function over `(area, buf, frame_index,
//! message, hint)` — no internal timer. The 60 fps render tick in
//! `app/events.rs` (RPC-008 rule [11]) already drives repaints; the
//! orchestrator computes `elapsed_ms = now - spinner_started_at`
//! and forwards it here.
//!
//! Style: every painted cell carries `Modifier::DIM` to match
//! `ThinkingIndicator.tsx:132-136` (`<Text dimColor>{...}</Text>`).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// 10-frame braille-dots set, identical to the TS Ink
/// `ThinkingIndicator` spinner style `'dots'`.
pub const DOTS_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Frame interval in milliseconds (TS uses 80 ms).
pub const DOTS_INTERVAL_MS: u64 = 80;

/// Pure helper — picks the current frame glyph from `DOTS_FRAMES`
/// given elapsed milliseconds since the spinner started. Matches the
/// TS `useEffect`-driven `(prev + 1) % frames.length` scheme: at
/// `elapsed_ms = 0` the glyph is `DOTS_FRAMES[0]`; at
/// `elapsed_ms = 80` it is `DOTS_FRAMES[1]`; at `240` it is
/// `DOTS_FRAMES[3]`; etc.
#[must_use]
pub fn current_frame_glyph(elapsed_ms: u64) -> &'static str {
    let idx = (elapsed_ms / DOTS_INTERVAL_MS) as usize % DOTS_FRAMES.len();
    DOTS_FRAMES[idx]
}

/// Paint one row consisting of `"{spinner_glyph} {message}... {hint}"`
/// into `area`, all dim-styled. `frame_index` selects from
/// `DOTS_FRAMES` (callers typically pass the result of
/// `current_frame_glyph(elapsed_ms)` already resolved — but to keep
/// the helper testable in isolation we accept a usize index).
pub fn paint_spinner_line(
    area: Rect,
    buf: &mut Buffer,
    frame_index: usize,
    message: &str,
    hint: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let glyph = DOTS_FRAMES[frame_index % DOTS_FRAMES.len()];
    let style = Style::default().add_modifier(Modifier::DIM);
    let text = format!("{glyph} {message}... {hint}");
    Paragraph::new(Line::from(Span::styled(text, style))).render(area, buf);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn frames_count_is_ten() {
        assert_eq!(DOTS_FRAMES.len(), 10);
    }

    #[test]
    fn interval_is_eighty_ms() {
        assert_eq!(DOTS_INTERVAL_MS, 80);
    }

    #[test]
    fn frame_picker_modulus_wraps() {
        assert_eq!(current_frame_glyph(0), "⠋");
        assert_eq!(current_frame_glyph(80), "⠙");
        assert_eq!(current_frame_glyph(240), "⠸");
        // 10 frames * 80ms = 800ms full cycle.
        assert_eq!(current_frame_glyph(800), "⠋");
        assert_eq!(current_frame_glyph(880), "⠙");
    }

    #[test]
    fn painter_writes_spinner_glyph_at_origin() {
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        paint_spinner_line(area, &mut buf, 0, "Thinking", "(Esc to stop)");
        assert_eq!(buf[(0, 0)].symbol(), "⠋");
    }

    #[test]
    fn painter_applies_dim_modifier() {
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        paint_spinner_line(area, &mut buf, 0, "Thinking", "(Esc to stop)");
        assert!(buf[(0, 0)].modifier.contains(Modifier::DIM));
    }

    #[test]
    fn painter_respects_area_origin() {
        let area = Rect::new(5, 3, 30, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 60, 10));
        paint_spinner_line(area, &mut buf, 0, "Compacting", "(Esc to stop)");
        // Cell at area.x must be the spinner glyph.
        assert_eq!(buf[(5, 3)].symbol(), "⠋");
        // Cell BEFORE the area must be untouched.
        assert_eq!(buf[(4, 3)].symbol(), " ");
    }
}
