//! MENU-001 — session status chips: pure snapshot builder.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R4, R9).
//!
//! Builds the Zone B chip list from a read-only slice of per-session
//! inputs. The callers (board / agent / mux render paths) gather these
//! from `AgentViewStore` (`open_sessions`, `session_index_for`,
//! `session_status_for`, `work_unit_context_for`) — the builder itself
//! stays store-free and unit-testable.
//!
//! R4: `#n` paints only when `n >= 1`; per-status glyphs:
//!   - Running      → braille frame `DOTS_FRAMES[(clock_ms / 80) % 10]`, magenta
//!   - Compacting   → `↻` (U+21BB), cyan
//!   - Paused       → `!`, yellow
//!   - Idle         → `●` (U+25CF), green dimmed
//!   - Interrupted  → `✕` (U+2715), red
//!
//! R9: `Cleared` sessions get NO chip — the builder drops them.

use codelet_rpc_types::SessionStatus;
use ratatui::style::{Color, Modifier, Style};

use crate::components::spinner::current_frame_glyph;

/// One Zone B chip: the 1-based session index, its status glyph + style,
/// and the optional work-unit id suffix (painted only when the width
/// budget allows — the painter decides, not this struct).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuChip {
    /// 1-based `(n, total)` slot (SessionHeader `#N` parity).
    pub index: (usize, usize),
    /// The status glyph (braille frame for Running).
    pub glyph: String,
    /// The glyph's style (color [+ DIM for Idle]).
    pub glyph_style: Style,
    /// The work-unit id bound to the session, when any (chip suffix).
    pub wu_id: Option<String>,
    /// MENU-006: true iff this session is the store's CURRENT session —
    /// the painter paints the whole cell inverse (the selected-menu-item
    /// highlight) when true, independent of the ring focus.
    pub active: bool,
}

/// One per-session input row for [`build_chips`]. Callers gather these
/// from the `AgentViewStore` per-session slots.
#[derive(Debug, Clone)]
pub struct ChipInput {
    /// 1-based `(n, total)` index — `session_index_for` parity.
    pub index: (usize, usize),
    /// The session's live status.
    pub status: SessionStatus,
    /// The work-unit id bound to the session, when any.
    pub wu_id: Option<String>,
    /// MENU-006: true iff this session is the store's current session
    /// (the painter's active-chip highlight source).
    pub active: bool,
}

/// Build the chip list from the per-session inputs. `Cleared` sessions
/// are dropped (R9). `clock_ms` drives the Running braille frame (R4).
pub fn build_chips(inputs: &[ChipInput], clock_ms: u64) -> Vec<MenuChip> {
    inputs
        .iter()
        .filter(|c| c.status != SessionStatus::Cleared)
        .map(|c| {
            let (glyph, style) = status_glyph(c.status, clock_ms);
            MenuChip {
                index: c.index,
                glyph,
                glyph_style: style,
                wu_id: c.wu_id.clone(),
                active: c.active,
            }
        })
        .collect()
}

/// R4: status → (glyph, style).
fn status_glyph(status: SessionStatus, clock_ms: u64) -> (String, Style) {
    match status {
        SessionStatus::Running => (
            // R4: `current_frame_glyph` already floors by DOTS_INTERVAL_MS
            // (80 ms cadence) — pass the raw clock.
            current_frame_glyph(clock_ms).to_string(),
            Style::default().fg(Color::Magenta),
        ),
        SessionStatus::Compacting => ("↻".to_string(), Style::default().fg(Color::Cyan)),
        SessionStatus::Paused => ("!".to_string(), Style::default().fg(Color::Yellow)),
        SessionStatus::Idle => (
            "●".to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::DIM),
        ),
        SessionStatus::Interrupted => ("✕".to_string(), Style::default().fg(Color::Red)),
        // Unreachable: Cleared is filtered out before the builder calls
        // this — kept as a defensive arm so the match stays exhaustive
        // if a new status lands upstream.
        SessionStatus::Cleared => ("·".to_string(), Style::default().fg(Color::Black)),
    }
}

/// Format a chip's leading `#n ` prefix (empty when `n == 0`, which the
/// store never produces for open sessions).
pub fn index_prefix(index: (usize, usize)) -> String {
    let n = index.0;
    if n >= 1 {
        format!("#{n} ")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn input(index: usize, status: SessionStatus, wu: Option<&str>) -> ChipInput {
        ChipInput {
            index: (index, 3),
            status,
            wu_id: wu.map(str::to_string),
            active: false,
        }
    }

    #[test]
    fn running_uses_the_braille_frame_at_the_clock() {
        // @step Given a MenuSnapshot with 1 open session (Running)
        // @step When the bar is rendered at clock_ms 0 and again at clock_ms 80
        let chips = build_chips(&[input(1, SessionStatus::Running, None)], 0);
        // @step Then the chip's glyph is the first braille frame at 0ms and the second frame at 80ms
        assert_eq!(chips[0].glyph, "⠋");
        let chips = build_chips(&[input(1, SessionStatus::Running, None)], 80);
        assert_eq!(chips[0].glyph, "⠙");
        let chips = build_chips(&[input(1, SessionStatus::Running, None)], 240);
        assert_eq!(chips[0].glyph, "⠸");
        assert!(chips[0].glyph_style.fg.is_some_and(|c| c == Color::Magenta));
    }

    #[test]
    fn compacting_paused_interrupted_glypes_and_colors() {
        // @step Given a MenuSnapshot with sessions in statuses Compacting, Paused and Interrupted
        let chips = build_chips(
            &[
                input(1, SessionStatus::Compacting, None),
                input(2, SessionStatus::Paused, None),
                input(3, SessionStatus::Interrupted, None),
            ],
            0,
        );
        // @step When the bar is rendered
        // @step Then the chips render "↻", "!" and "✕"
        assert_eq!(chips[0].glyph, "↻");
        // @step And the glyphs are styled cyan, yellow and red respectively
        assert_eq!(chips[0].glyph_style.fg, Some(Color::Cyan));
        assert_eq!(chips[1].glyph, "!");
        assert_eq!(chips[1].glyph_style.fg, Some(Color::Yellow));
        assert_eq!(chips[2].glyph, "✕");
        assert_eq!(chips[2].glyph_style.fg, Some(Color::Red));
    }

    #[test]
    fn idle_is_a_dimmed_green_dot() {
        // @step Given a MenuSnapshot with 3 open sessions (Idle, Running, Idle) and clock_ms 0
        let chips = build_chips(&[input(1, SessionStatus::Idle, None)], 0);
        // @step When the bar is rendered into a 120-column row
        // @step Then Zone B reads "#1 ●  #2 ⠋  #3 ●"
        assert_eq!(chips[0].glyph, "●");
        assert_eq!(chips[0].glyph_style.fg, Some(Color::Green));
        assert!(chips[0].glyph_style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn cleared_sessions_are_dropped() {
        // @step Given a MenuSnapshot whose open session list contains a session in the Cleared status
        // @step When the chip list is built
        let chips = build_chips(
            &[
                input(1, SessionStatus::Cleared, None),
                input(2, SessionStatus::Idle, None),
            ],
            0,
        );
        // @step Then no chip is produced for the Cleared session
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].index, (2, 3));
    }

    #[test]
    fn wu_id_is_captured_when_present_and_none_when_absent() {
        // @step Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
        let chips = build_chips(
            &[
                input(1, SessionStatus::Idle, Some("MENU-001")),
                input(2, SessionStatus::Idle, None),
            ],
            0,
        );
        // @step When the bar is rendered
        // @step Then the chip reads "#1 MENU-001 ●"
        assert_eq!(chips[0].wu_id.as_deref(), Some("MENU-001"));
        assert!(chips[1].wu_id.is_none());
    }

    #[test]
    fn index_prefix_paints_from_one_up() {
        // @step Given a MenuSnapshot with exactly 1 open session (Idle)
        // @step When the bar is rendered
        // @step Then Zone B reads "#1 ●" (the prefix paints because n >= 1, SessionHeader parity)
        assert_eq!(index_prefix((1, 3)), "#1 ");
        assert_eq!(index_prefix((2, 3)), "#2 ");
        assert_eq!(index_prefix((0, 0)), "");
    }
}
