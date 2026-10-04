//! BUG-194 — per-session thinking-indicator transition state.
//!
//! Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
//!
//! The thinking indicator's phase machine (`InputTransitionState`) was a
//! single VIEW-LEVEL state on `AgentView` ticked only by the FOCUSED mux
//! agent pane (`pane_render.rs` → `tick_animation`). In mux mode the
//! store's current session follows mux focus, so the shared machine was
//! always driven by the focused pane's session — a running session whose
//! pane lost focus lost its spinner (an idle focused agent pane actively
//! drove the shared state `Loading → Hiding → Idle`).
//!
//! BUG-194 moves the state machine PER SESSION: each `SessionContext`
//! owns its own [`SessionTransition`] slot. Every rendered agent pane
//! ticks its OWN session's slot from its OWN `SessionStatus` (focused or
//! not), and paints the identical spinner/finish-sweep rows. Focus
//! movement never touches the slots — the only input to a session's
//! machine is that session's status.

use std::time::Instant;

use super::input_transition::InputTransitionState;

/// Per-session thinking-indicator transition slot (BUG-194).
#[derive(Debug, Clone, Default)]
pub struct SessionTransition {
    /// The phase machine (`Loading` / `Compacting` / `Hiding` /
    /// `Showing` / `Idle`). Advanced by
    /// `AgentView::tick_session_transition` from the session's own
    /// `SessionStatus`.
    pub state: InputTransitionState,
    /// Spinner start clock — `Some` while the session is Running or
    /// Compacting (feeds `elapsed_ms` for the 80ms braille cadence).
    /// `None` when idle. Focus movement NEVER resets this.
    pub spinner_started_at: Option<Instant>,
    /// The spinner line painted by this session's pane on the last
    /// frame — captured for the `Hiding` transition (the finish sweep
    /// sweeps out the EXACT line that was showing). `None` when the
    /// pane last painted a non-spinner row.
    pub last_spinner_line: Option<String>,
    /// Monotonic per-session animation clock (ms). Advances 16ms per
    /// rendered frame for this session — the `Hiding`/`Showing` frame
    /// math (`clock_ms - started_at`) is per-session, so multiple
    /// panes animating simultaneously never speed each other up.
    pub clock_ms: u64,
}

impl SessionTransition {
    /// True iff the finish sweep (Hiding/Showing) is mid-flight — the
    /// run-loop redraw-gate operand for this session.
    pub fn is_animating(&self) -> bool {
        self.state.is_animating()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn default_slot_is_idle_with_no_clock() {
        let slot = SessionTransition::default();
        assert!(matches!(slot.state, InputTransitionState::Idle));
        assert!(slot.spinner_started_at.is_none());
        assert!(slot.last_spinner_line.is_none());
        assert_eq!(slot.clock_ms, 0);
        assert!(!slot.is_animating());
    }

    #[test]
    fn is_animating_tracks_the_phase_machine() {
        let mut slot = SessionTransition::default();
        assert!(!slot.is_animating(), "Idle is not mid-sweep");
        slot.state = InputTransitionState::Loading { elapsed_ms: 100 };
        assert!(!slot.is_animating(), "Loading is not a finish sweep");
        slot.state = InputTransitionState::Hiding {
            captured: "⠋ Thinking... (Esc to stop)".to_string(),
            visible_chars: 27,
            started_at: 16,
            hide_completed_at: None,
        };
        assert!(slot.is_animating(), "Hiding is mid-sweep");
    }
}
