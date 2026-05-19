//! Pure-data span-building helpers for SessionHeader. Extracted from
//! `header/mod.rs` to keep that module under the 300-LoC RPC source-
//! shape ceiling — no buffer access lives here.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::store::TokenState;
use codelet_rpc_types::{ModelInfo, ThinkingLevel};

/// RPC-029 — build the multi-span left line. Order mirrors
/// `SessionHeader.tsx` L141–177: prefix+wu+model run in cyan-bold,
/// then conditional badges with per-segment colours.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_left_line(
    index: (usize, usize),
    model: Option<&ModelInfo>,
    thinking: ThinkingLevel,
    work_unit_id: Option<&str>,
    work_unit_status: Option<&str>,
    is_isolated: bool,
    is_debug_enabled: bool,
    is_select_mode: bool,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();

    let session_prefix = if index.0 >= 1 {
        format!("#{}", index.0)
    } else {
        String::new()
    };
    let work_unit_text = match (work_unit_id, work_unit_status) {
        (Some(id), Some(status)) => format!(" ({id}: {status})"),
        (Some(id), None) => format!(" ({id})"),
        _ => String::new(),
    };
    let separator = if !session_prefix.is_empty() || work_unit_id.is_some() {
        ": "
    } else {
        ""
    };
    let model_name = model
        .map(|m| m.display_name.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Agent");

    spans.push(Span::styled(
        format!("{session_prefix}{work_unit_text}{separator}{model_name}"),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ));

    if is_isolated {
        spans.push(Span::styled(
            " [ISOLATED]".to_string(),
            Style::default().fg(Color::Green),
        ));
    }
    if let Some(m) = model {
        if m.supports_reasoning {
            spans.push(Span::styled(
                " [R]".to_string(),
                Style::default().fg(Color::Magenta),
            ));
        }
        if m.supports_vision {
            spans.push(Span::styled(
                " [V]".to_string(),
                Style::default().fg(Color::Blue),
            ));
        }
        if m.context_window > 0 {
            spans.push(Span::styled(
                format!(" [{}]", format_context_window(m.context_window)),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
    if is_debug_enabled {
        spans.push(Span::styled(
            " [DEBUG]".to_string(),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    if is_select_mode {
        spans.push(Span::styled(
            " [SELECT]".to_string(),
            Style::default().fg(Color::Cyan),
        ));
    }
    if let Some(label) = thinking_label(thinking) {
        spans.push(Span::styled(
            format!(" [T:{label}]"),
            Style::default().fg(Color::Yellow),
        ));
    }

    Line::from(spans)
}

/// RPC-029 — build the multi-span right line. Two spans: the dark-grey
/// `tokens: in↓ out↑[ N🧠]` block, then the context-fill coloured
/// percent bracket. When `is_loading` is true AND `tokens_per_second`
/// is `Some`, a magenta `N.N tok/s ` segment is prepended.
pub(super) fn build_right_line(
    tokens: &TokenState,
    tokens_per_second: Option<f32>,
    reasoning_tokens: u64,
    compaction_reduction: Option<i32>,
    is_loading: bool,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if is_loading {
        if let Some(tps) = tokens_per_second {
            spans.push(Span::styled(
                format!("{tps:.1} tok/s  "),
                Style::default().fg(Color::Magenta),
            ));
        }
    }
    let reasoning_part = if reasoning_tokens > 0 {
        format!(" {reasoning_tokens}🧠")
    } else {
        String::new()
    };
    spans.push(Span::styled(
        format!(
            "tokens: {}↓ {}↑{reasoning_part} ",
            tokens.input_tokens, tokens.output_tokens
        ),
        Style::default().fg(Color::DarkGray),
    ));
    let pct_text = match compaction_reduction {
        Some(r) => format!(
            "[{}%: COMPACTED {}%]",
            tokens.context_fill_pct,
            r.abs()
        ),
        None => format!("[{}%]", tokens.context_fill_pct),
    };
    spans.push(Span::styled(
        pct_text,
        Style::default().fg(context_fill_color(tokens.context_fill_pct)),
    ));
    Line::from(spans)
}

fn thinking_label(level: ThinkingLevel) -> Option<&'static str> {
    match level {
        ThinkingLevel::Off => None,
        ThinkingLevel::Low => Some("Low"),
        ThinkingLevel::Medium => Some("Med"),
        ThinkingLevel::High => Some("High"),
    }
}

/// RPC-029: mirrors `getContextFillColor` from
/// `src/tui/utils/sessionHeaderUtils.ts:37–42` — note the FOUR
/// thresholds (the magenta band between 70% and 85% is easy to miss).
pub(super) fn context_fill_color(pct: u8) -> Color {
    if pct < 50 {
        Color::Green
    } else if pct < 70 {
        Color::Yellow
    } else if pct < 85 {
        Color::Magenta
    } else {
        Color::Red
    }
}

/// Compact context-window display: `192000` → `192k`. Mirrors
/// `formatContextWindow` from `src/tui/utils/sessionHeaderUtils.ts`.
pub(crate) fn format_context_window(n: u32) -> String {
    if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}


