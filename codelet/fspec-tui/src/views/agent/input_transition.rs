//! RPC-093 + RPC-095 — input-row dispatcher + finish-animation phases.
//! Mirrors `src/tui/components/InputTransition.tsx`.
//! INK_FRAME_TIME_MS=17, CHARS_PER_FRAME=5, ANIMATION_PHASE_DELAY_MS=34.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::multiline_input::MultiLineInput;
use super::spinner::{current_frame_glyph, DOTS_FRAMES, DOTS_INTERVAL_MS};
use super::INPUT_PLACEHOLDER_HINT;

/// Per-frame duration of Ink's render loop (ms).
pub const INK_FRAME_TIME_MS: u64 = 17;
/// Characters revealed/hidden per frame in TS InputTransition.
pub const CHARS_PER_FRAME: usize = 5;
/// Gap between Hiding-complete and Showing-start (ms).
pub const ANIMATION_PHASE_DELAY_MS: u64 = 34;

/// The five rendering states for the input row.
#[derive(Debug, Clone, Default)]
pub enum InputTransitionState {
    #[default]
    Idle,
    Loading {
        elapsed_ms: u64,
    },
    Compacting {
        elapsed_ms: u64,
    },
    /// Captured spinner line shrinks 5 chars/frame.
    Hiding {
        captured: String,
        visible_chars: usize,
        started_at: u64,
        hide_completed_at: Option<u64>,
    },
    /// Placeholder grows 5 chars/frame from 0 → placeholder.len().
    Showing {
        placeholder: String,
        visible_chars: usize,
        started_at: u64,
    },
}

impl InputTransitionState {
    /// `(elapsed_ms - phase_start) / INK_FRAME_TIME_MS` clamped at 0.
    fn frame_count(elapsed_ms_since_start: u64) -> usize {
        (elapsed_ms_since_start / INK_FRAME_TIME_MS) as usize
    }

    /// Advance the state by the absolute `clock_ms`. Idempotent: pure
    /// over (self, clock_ms). Used by tests and by the run loop each
    /// frame.
    pub fn advance(&self, clock_ms: u64) -> InputTransitionState {
        match self {
            InputTransitionState::Hiding {
                captured,
                started_at,
                hide_completed_at,
                ..
            } => {
                let total = captured.chars().count();
                let frames = Self::frame_count(clock_ms.saturating_sub(*started_at));
                let consumed = frames.saturating_mul(CHARS_PER_FRAME);
                let visible_chars = total.saturating_sub(consumed);
                if visible_chars > 0 {
                    return InputTransitionState::Hiding {
                        captured: captured.clone(),
                        visible_chars,
                        started_at: *started_at,
                        hide_completed_at: None,
                    };
                }
                // visible_chars == 0
                let completion = hide_completed_at.unwrap_or(clock_ms);
                if clock_ms >= completion.saturating_add(ANIMATION_PHASE_DELAY_MS) {
                    InputTransitionState::Showing {
                        placeholder: INPUT_PLACEHOLDER_HINT.to_string(),
                        visible_chars: 0,
                        started_at: completion + ANIMATION_PHASE_DELAY_MS,
                    }
                } else {
                    InputTransitionState::Hiding {
                        captured: captured.clone(),
                        visible_chars: 0,
                        started_at: *started_at,
                        hide_completed_at: Some(completion),
                    }
                }
            }
            InputTransitionState::Showing {
                placeholder,
                started_at,
                ..
            } => {
                let total = placeholder.chars().count();
                let frames = Self::frame_count(clock_ms.saturating_sub(*started_at));
                let revealed = frames.saturating_mul(CHARS_PER_FRAME).min(total);
                if revealed >= total {
                    InputTransitionState::Idle
                } else {
                    InputTransitionState::Showing {
                        placeholder: placeholder.clone(),
                        visible_chars: revealed,
                        started_at: *started_at,
                    }
                }
            }
            other => other.clone(),
        }
    }

    /// Status → Idle: capture the spinner text and enter Hiding.
    pub fn transition_on_idle(
        prev: &InputTransitionState,
        captured: &str,
        clock_ms: u64,
    ) -> InputTransitionState {
        let _ = prev;
        InputTransitionState::Hiding {
            captured: captured.to_string(),
            visible_chars: captured.chars().count(),
            started_at: clock_ms,
            hide_completed_at: None,
        }
    }

    /// Status → Running mid-animation: abort and resume the spinner.
    pub fn transition_on_running(
        _prev: &InputTransitionState,
        _clock_ms: u64,
    ) -> InputTransitionState {
        InputTransitionState::Loading { elapsed_ms: 0 }
    }

    /// Printable key during Hiding/Showing short-circuits to Idle and
    /// returns the buffered character for the MultiLineInput.
    pub fn on_printable_key(
        prev: &InputTransitionState,
        ch: char,
    ) -> (InputTransitionState, Option<char>) {
        match prev {
            InputTransitionState::Hiding { .. } | InputTransitionState::Showing { .. } => {
                (InputTransitionState::Idle, Some(ch))
            }
            other => (other.clone(), None),
        }
    }

    /// True iff the cursor should be painted in this phase.
    pub fn is_cursor_painted(&self) -> bool { matches!(self, InputTransitionState::Idle) }

    /// RPC-093: mid-Hiding/Showing finish animation. The run loop
    /// reads this to keep drawing every tick after `is_busy` flips
    /// false so the 5 char/17ms sweep advances.
    pub fn is_animating(&self) -> bool {
        matches!(self, Self::Hiding { .. } | Self::Showing { .. })
    }
}

/// Render either the spinner, the captured/placeholder slice, or the
/// MultiLineInput depending on `state`.
pub fn render_input_transition(area: Rect, buf: &mut Buffer, state: &InputTransitionState) {
    match state {
        InputTransitionState::Loading { elapsed_ms } => {
            paint_spinner(area, buf, *elapsed_ms, "Thinking", "(Esc to stop)");
        }
        InputTransitionState::Compacting { elapsed_ms } => {
            paint_spinner(area, buf, *elapsed_ms, "Compacting", "(Esc to stop)");
        }
        InputTransitionState::Hiding {
            captured,
            visible_chars,
            ..
        } => {
            paint_sliced_text(
                area,
                buf,
                captured,
                *visible_chars,
                Style::default().add_modifier(Modifier::DIM),
            );
        }
        InputTransitionState::Showing {
            placeholder,
            visible_chars,
            ..
        } => {
            paint_sliced_text(
                area,
                buf,
                placeholder,
                *visible_chars,
                Style::default().fg(Color::DarkGray),
            );
        }
        InputTransitionState::Idle => {
            // Caller paints the MultiLineInput.
        }
    }
}

/// Render the idle input row. Convenience helper so the AgentView
/// orchestrator can express the transition uniformly.
pub fn render_idle_input(area: Rect, buf: &mut Buffer, input: &MultiLineInput) {
    input.render_with_prompt(area, buf, INPUT_PLACEHOLDER_HINT);
}

/// Pick spinner / animation slice / input for the AgentView input row.
pub fn paint_input_or_spinner(
    area: Rect,
    buf: &mut Buffer,
    input: &MultiLineInput,
    state: &InputTransitionState,
) {
    match state {
        InputTransitionState::Idle => render_idle_input(area, buf, input),
        other => render_input_transition(area, buf, other),
    }
}

fn paint_spinner(area: Rect, buf: &mut Buffer, elapsed_ms: u64, message: &str, hint: &str) {
    let idx = (elapsed_ms / DOTS_INTERVAL_MS) as usize % DOTS_FRAMES.len();
    super::spinner::paint_spinner_line(area, buf, idx, message, hint);
    let _ = current_frame_glyph(elapsed_ms);
}

fn paint_sliced_text(area: Rect, buf: &mut Buffer, source: &str, visible_chars: usize, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let prefix: String = source.chars().take(visible_chars).collect();
    Paragraph::new(Line::from(Span::styled(prefix, style))).render(area, buf);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn line_at(buf: &Buffer, area: Rect) -> String {
        let mut s = String::new();
        for x in area.x..area.x + area.width {
            s.push_str(buf[(x, area.y)].symbol());
        }
        s.trim_end().to_string()
    }

    #[test]
    fn loading_renders_thinking_line() {
        let area = Rect::new(0, 0, 60, 1);
        let mut buf = Buffer::empty(area);
        render_input_transition(area, &mut buf, &InputTransitionState::Loading { elapsed_ms: 0 });
        assert!(line_at(&buf, area).starts_with("⠋ Thinking... (Esc to stop)"));
    }

    #[test]
    fn compacting_renders_compacting_line() {
        let area = Rect::new(0, 0, 60, 1);
        let mut buf = Buffer::empty(area);
        render_input_transition(
            area,
            &mut buf,
            &InputTransitionState::Compacting { elapsed_ms: 0 },
        );
        assert!(line_at(&buf, area).starts_with("⠋ Compacting... (Esc to stop)"));
    }

    #[test]
    fn idle_is_noop_via_dispatcher() {
        let area = Rect::new(0, 0, 60, 1);
        let mut buf = Buffer::empty(area);
        render_input_transition(area, &mut buf, &InputTransitionState::Idle);
        assert_eq!(buf[(0, 0)].symbol(), " ");
    }

    #[test]
    fn loading_advances_frame_per_elapsed() {
        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);
        render_input_transition(
            area,
            &mut buf,
            &InputTransitionState::Loading { elapsed_ms: 240 },
        );
        assert_eq!(buf[(0, 0)].symbol(), "⠸");
    }

    #[test]
    fn cursor_painted_only_when_idle() {
        assert!(InputTransitionState::Idle.is_cursor_painted());
        assert!(!InputTransitionState::Loading { elapsed_ms: 0 }.is_cursor_painted());
        assert!(!InputTransitionState::Compacting { elapsed_ms: 0 }.is_cursor_painted());
    }
}
