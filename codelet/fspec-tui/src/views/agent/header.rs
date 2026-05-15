//! SessionHeader — 1-row strip at the top of AgentView painting the
//! current model badges + per-session token deltas.
//!
//! Feature: spec/features/rpc018-agent-chrome.feature
//! Card: RPC-018.
//!
//! Mirrors the TS `src/tui/components/SessionHeader.tsx` layout:
//!
//! ```text
//!  #N: <model display name> [R] [V] [Nk] [T:<level>]  tokens: in↓ out↑ [P%]
//! ```
//!
//! Each badge is conditional:
//!   - `#N:`        — only when `session_index.0 >= 1`.
//!   - `[R]`        — only when `model.supports_reasoning`.
//!   - `[V]`        — only when `model.supports_vision`.
//!   - `[Nk]`       — only when `model.context_window > 0`.
//!   - `[T:<level>]` — only when `thinking != Off`.
//!
//! The right side always paints `tokens: in↓ out↑ [P%]` so the user can
//! tell the box is alive even when no chunks have arrived yet.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::store::TokenState;
use codelet_rpc_types::{ModelInfo, ThinkingLevel};

/// Owned snapshot of every input the SessionHeader needs to render.
/// Built fresh per frame by the AgentView orchestrator so the widget
/// itself stays free of cloned data on the painter's hot path.
pub struct SessionHeader<'a> {
    pub session_index: (usize, usize),
    pub model: Option<&'a ModelInfo>,
    pub thinking: ThinkingLevel,
    pub tokens: TokenState,
}

impl<'a> Widget for SessionHeader<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let left = build_left_text(self.session_index, self.model, self.thinking);
        let right = build_right_text(&self.tokens);
        paint_two_columns(area, buf, &left, &right);
    }
}

fn build_left_text(
    index: (usize, usize),
    model: Option<&ModelInfo>,
    thinking: ThinkingLevel,
) -> String {
    let mut out = String::new();
    if index.0 >= 1 {
        out.push('#');
        out.push_str(&index.0.to_string());
        out.push_str(": ");
    }
    let display_name = model.map(|m| m.display_name.as_str()).unwrap_or("");
    if display_name.is_empty() {
        out.push_str("Agent");
    } else {
        out.push_str(display_name);
    }
    if let Some(m) = model {
        if m.supports_reasoning {
            out.push_str(" [R]");
        }
        if m.supports_vision {
            out.push_str(" [V]");
        }
        if m.context_window > 0 {
            out.push_str(" [");
            out.push_str(&format_context_window(m.context_window));
            out.push(']');
        }
    }
    if let Some(label) = thinking_label(thinking) {
        out.push_str(" [T:");
        out.push_str(label);
        out.push(']');
    }
    out
}

fn build_right_text(tokens: &TokenState) -> String {
    format!(
        "tokens: {}↓ {}↑ [{}%]",
        tokens.input_tokens, tokens.output_tokens, tokens.context_fill_pct
    )
}

fn thinking_label(level: ThinkingLevel) -> Option<&'static str> {
    match level {
        ThinkingLevel::Off => None,
        ThinkingLevel::Low => Some("Low"),
        ThinkingLevel::Medium => Some("Med"),
        ThinkingLevel::High => Some("High"),
    }
}

/// Compact context-window display: `192000` → `192k`, `200000` → `200k`,
/// `8192` → `8k`, `1023` → `1023`. Mirrors `formatContextWindow` from
/// `src/tui/utils/sessionHeaderUtils.ts`.
pub(crate) fn format_context_window(n: u32) -> String {
    if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

/// Paint `left` aligned to the left edge of `area` and `right` aligned
/// to the right edge — with a 1-cell gap. If left + right overflow the
/// area width, the left side is truncated to fit.
fn paint_two_columns(area: Rect, buf: &mut Buffer, left: &str, right: &str) {
    let width = area.width as usize;
    let right_len = right.chars().count();
    let budget_left = width.saturating_sub(right_len).saturating_sub(1);
    let left_truncated: String = left.chars().take(budget_left).collect();
    let left_len = left_truncated.chars().count();

    // RPC-018: paint the left in dim grey so the right side dominates
    // when scanning for token deltas during long bursts.
    let left_style = Style::default().fg(Color::White);
    let right_style = Style::default().fg(Color::DarkGray);

    let left_line = Line::from(Span::styled(left_truncated, left_style));
    Paragraph::new(left_line).render(
        Rect {
            x: area.x,
            y: area.y,
            width: left_len as u16,
            height: 1,
        },
        buf,
    );

    let right_x = area.x.saturating_add(area.width.saturating_sub(right_len as u16));
    let right_line = Line::from(Span::styled(right.to_string(), right_style));
    Paragraph::new(right_line).render(
        Rect {
            x: right_x,
            y: area.y,
            width: right_len as u16,
            height: 1,
        },
        buf,
    );
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn format_context_window_compacts_thousands() {
        assert_eq!(format_context_window(192_000), "192k");
        assert_eq!(format_context_window(8_192), "8k");
        assert_eq!(format_context_window(999), "999");
        assert_eq!(format_context_window(0), "0");
    }

    #[test]
    fn build_left_text_renders_full_chrome() {
        let model = ModelInfo {
            display_name: "Claude Opus 4.7".to_string(),
            supports_reasoning: true,
            supports_vision: true,
            context_window: 192_000,
        };
        let s = build_left_text((1, 1), Some(&model), ThinkingLevel::High);
        assert!(s.contains("#1:"));
        assert!(s.contains("Claude Opus 4.7"));
        assert!(s.contains("[R]"));
        assert!(s.contains("[V]"));
        assert!(s.contains("[192k]"));
        assert!(s.contains("[T:High]"));
    }

    #[test]
    fn build_left_text_renders_placeholder_when_no_model() {
        let s = build_left_text((0, 0), None, ThinkingLevel::Off);
        assert!(s.contains("Agent"));
        assert!(!s.contains("[R]"));
        assert!(!s.contains("[V]"));
        assert!(!s.contains("[T:"));
    }
}
