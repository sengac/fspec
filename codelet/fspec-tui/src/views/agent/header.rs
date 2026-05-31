//! SessionHeader — 1-row strip at the top of AgentView painting the
//! current model badges + per-session token deltas.
//!
//! Feature files:
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc029-agent-structure-alignment.feature
//!
//! Mirrors the TS `src/tui/components/SessionHeader.tsx` layout:
//!
//! ```text
//!  #N (WU-ID: status): <model> [ISOLATED] [R] [V] [Nk] [DEBUG] [SELECT] [T:<level>]   tokens: in↓ out↑ [P%]
//! ```
//!
//! Each badge is conditional (see the per-segment list inside
//! `build_left_line`). The right side always paints
//! `tokens: in↓ out↑ [P%]` so the user can tell the box is alive even
//! when no chunks have arrived yet — RPC-029 splits the right line
//! into two spans (dark-grey delta block + context-fill coloured
//! percent bracket).
//!
//! RPC-029: the whole row paints a dark-grey `#333333` background and
//! is horizontally padded by 1 column on both sides to mirror the TS
//! `paddingX={1}` + `backgroundColor="#333333"` on `SessionHeader.tsx`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::store::TokenState;
use codelet_rpc_types::{ModelInfo, ThinkingLevel};

use super::chrome::{horizontal_pad, line_width};
use super::header_build::{build_left_line, build_right_line};
use super::paint_row_bg;

/// RPC-029: dark-grey (`#333333`) row background painted on every cell
/// of the header strip.
pub(crate) const HEADER_BG: Color = Color::Rgb(0x33, 0x33, 0x33);

/// Owned snapshot of every input the SessionHeader needs to render.
/// Built fresh per frame by the AgentView orchestrator so the widget
/// itself stays free of cloned data on the painter's hot path.
pub struct SessionHeader<'a> {
    pub session_index: (usize, usize),
    pub model: Option<&'a ModelInfo>,
    pub thinking: ThinkingLevel,
    pub tokens: TokenState,
    /// RPC-029: current work-unit id (e.g. `"RPC-029"`). When `Some`,
    /// inserted between the `#N` session prefix and the model name as
    /// `#N (ID[: status]): model`. When `None`, the header collapses
    /// to `#N: model`.
    pub work_unit_id: Option<&'a str>,
    /// RPC-029: current work-unit status (e.g. `"implementing"`).
    /// Only painted when `work_unit_id` is also `Some`.
    pub work_unit_status: Option<&'a str>,
    /// RPC-029: paint `[ISOLATED]` (green) when true.
    pub is_isolated: bool,
    /// RPC-029: paint `[DEBUG]` (red-bold) when true.
    pub is_debug_enabled: bool,
    /// RPC-029: paint `[SELECT]` (cyan) when true.
    pub is_select_mode: bool,
    /// RPC-029: paint `N.N tok/s` magenta on the right side when
    /// `is_loading` is true AND this is `Some`.
    pub tokens_per_second: Option<f32>,
    /// RPC-029: paint ` <n>🧠` inside the tokens delta block when > 0.
    pub reasoning_tokens: u64,
    /// RPC-029: paint `[P%: COMPACTED N%]` in the percent bracket when
    /// `Some`. Otherwise paint `[P%]`.
    pub compaction_reduction: Option<i32>,
    /// RPC-029: gates the tokens-per-second magenta segment.
    pub is_loading: bool,
    /// RPC-061: optional subordinate-of label, e.g. `"abcdef12"` or
    /// `"abcdef12+2"`. When `Some`, the SessionHeader renders
    /// `[Subordinate of: <label>]` (cyan, no modifier) directly after
    /// the model/badges block. When `None`, no badge is painted.
    pub subordinate_label: Option<&'a str>,
}

impl<'a> Widget for SessionHeader<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // RPC-029: dark-grey row background on every cell of the strip.
        paint_row_bg(area, buf, HEADER_BG);
        // RPC-029: horizontal padding of 1 column on both sides.
        let inner = horizontal_pad(area, 1);
        if inner.width == 0 {
            return;
        }
        let left = build_left_line(
            self.session_index,
            self.model,
            self.thinking,
            self.work_unit_id,
            self.work_unit_status,
            self.is_isolated,
            self.is_debug_enabled,
            self.is_select_mode,
            self.subordinate_label,
        );
        let right = build_right_line(
            &self.tokens,
            self.tokens_per_second,
            self.reasoning_tokens,
            self.compaction_reduction,
            self.is_loading,
        );
        paint_two_columns(inner, buf, left, right);
    }
}

/// Paint `left` aligned to the left edge of `inner` and `right`
/// aligned to the right edge — with a 1-cell gap. If the left line's
/// total width exceeds the budget after the right line is reserved,
/// the trailing spans are dropped one at a time (and the last
/// remaining span is truncated to fit). The row background is already
/// painted by `paint_row_bg`, so we can render the styled lines
/// directly and ratatui's cell-merge preserves the bg.
fn paint_two_columns(
    inner: Rect,
    buf: &mut Buffer,
    left: Line<'static>,
    right: Line<'static>,
) {
    let width = inner.width as usize;
    let right_len = line_width(&right);
    // Budget includes one cell of gap when both columns are non-empty.
    let budget_left = width.saturating_sub(right_len).saturating_sub(1);
    let left_truncated = truncate_line(left, budget_left);
    let left_len = line_width(&left_truncated);
    if left_len > 0 {
        Paragraph::new(left_truncated).render(
            Rect {
                x: inner.x,
                y: inner.y,
                width: left_len as u16,
                height: 1,
            },
            buf,
        );
    }
    if right_len > 0 && (right_len as u16) <= inner.width {
        let right_x = inner.x.saturating_add(inner.width - right_len as u16);
        Paragraph::new(right).render(
            Rect {
                x: right_x,
                y: inner.y,
                width: right_len as u16,
                height: 1,
            },
            buf,
        );
    }
}

fn truncate_line(line: Line<'static>, budget: usize) -> Line<'static> {
    let mut remaining = budget;
    let mut out: Vec<Span<'static>> = Vec::new();
    for span in line.spans {
        let span_len = span.content.chars().count();
        if span_len <= remaining {
            remaining -= span_len;
            out.push(span);
        } else {
            if remaining > 0 {
                let style = span.style;
                let s: String = span.content.chars().take(remaining).collect();
                out.push(Span::styled(s, style));
            }
            break;
        }
    }
    Line::from(out)
}

