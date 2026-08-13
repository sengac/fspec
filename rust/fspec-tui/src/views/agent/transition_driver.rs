//! RPC-093 — phase-machine driver for `AgentView::render_with_store`.
//!
//! Extracted from `views/agent.rs` to keep the orchestrator under its
//! 300-LoC ceiling. Pure function: takes (busy, prev state, optional
//! captured spinner line, clock) and returns the next state.

use codelet_rpc_types::SessionStatus;

use super::input_transition::InputTransitionState;
use super::spinner::current_frame_glyph;

/// RPC-093: re-derive the spinner line painted on the current frame
/// from the active transition state. Returns `None` when not
/// painting a spinner (Idle / Hiding / Showing).
pub fn cached_spinner_line(state: &InputTransitionState) -> Option<String> {
    match state {
        InputTransitionState::Loading { elapsed_ms } => Some(format!(
            "{} Thinking... (Esc to stop)",
            current_frame_glyph(*elapsed_ms)
        )),
        InputTransitionState::Compacting { elapsed_ms } => Some(format!(
            "{} Compacting... (Esc to stop)",
            current_frame_glyph(*elapsed_ms)
        )),
        _ => None,
    }
}

/// Per-render driver. Given the current session status, the previous
/// transition state, an optional cached spinner line (set whenever
/// the spinner painted on the last frame), and the monotonic
/// animation clock in milliseconds, return the next state.
pub fn advance_transition(
    session_status: Option<SessionStatus>,
    prev: &InputTransitionState,
    last_spinner_line: Option<&str>,
    elapsed_ms: u64,
    clock_ms: u64,
) -> InputTransitionState {
    let is_busy = matches!(
        session_status,
        Some(SessionStatus::Running) | Some(SessionStatus::Compacting)
    );
    let is_loading = matches!(session_status, Some(SessionStatus::Running));

    match (is_busy, prev) {
        (true, InputTransitionState::Hiding { .. })
        | (true, InputTransitionState::Showing { .. }) => {
            // Rule [9]: abort finish animation, resume spinner at 0.
            InputTransitionState::transition_on_running(prev, clock_ms)
        }
        (true, _) if is_loading => InputTransitionState::Loading { elapsed_ms },
        (true, _) => InputTransitionState::Compacting { elapsed_ms },
        (false, InputTransitionState::Loading { .. })
        | (false, InputTransitionState::Compacting { .. }) => {
            // Busy → Idle: capture the last spinner line.
            let captured = last_spinner_line
                .map(ToString::to_string)
                .unwrap_or_else(|| "⠋ Thinking... (Esc to stop)".to_string());
            InputTransitionState::transition_on_idle(prev, &captured, clock_ms)
        }
        (false, other) => other.advance(clock_ms),
    }
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn idle_stays_idle() {
        let next = advance_transition(
            Some(SessionStatus::Idle),
            &InputTransitionState::Idle,
            None,
            0,
            0,
        );
        assert!(matches!(next, InputTransitionState::Idle));
    }

    #[test]
    fn running_enters_loading() {
        let next = advance_transition(
            Some(SessionStatus::Running),
            &InputTransitionState::Idle,
            None,
            240,
            1_000,
        );
        match next {
            InputTransitionState::Loading { elapsed_ms } => assert_eq!(elapsed_ms, 240),
            other => panic!("expected Loading, got {other:?}"),
        }
    }

    #[test]
    fn loading_to_idle_enters_hiding_with_captured() {
        let prev = InputTransitionState::Loading { elapsed_ms: 240 };
        let next = advance_transition(
            Some(SessionStatus::Idle),
            &prev,
            Some("⠸ Thinking... (Esc to stop)"),
            0,
            5_000,
        );
        match next {
            InputTransitionState::Hiding {
                captured,
                started_at,
                ..
            } => {
                assert_eq!(captured, "⠸ Thinking... (Esc to stop)");
                assert_eq!(started_at, 5_000);
            }
            other => panic!("expected Hiding, got {other:?}"),
        }
    }

    #[test]
    fn hiding_to_running_aborts_and_resets_to_loading_zero() {
        let prev = InputTransitionState::Hiding {
            captured: "⠸ Thinking... (Esc to stop)".to_string(),
            visible_chars: 12,
            started_at: 1_000,
            hide_completed_at: None,
        };
        let next = advance_transition(Some(SessionStatus::Running), &prev, None, 999_999, 5_000);
        match next {
            InputTransitionState::Loading { elapsed_ms } => assert_eq!(elapsed_ms, 0),
            other => panic!("expected Loading{{0}}, got {other:?}"),
        }
    }
}
