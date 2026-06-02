//! Feature: spec/features/agentview-thinking-indicator-animation-parity.feature
//!
//! RPC-093 — AgentView Thinking indicator animation parity.
//!
//! Tests three regression-fix surfaces:
//! - (A) Spinner advances at 80ms cadence independently of stream chunks
//! - (B) Hiding/Showing finish animation phase machine (CHARS_PER_FRAME=5, INK_FRAME_TIME_MS=17, ANIMATION_PHASE_DELAY_MS=34)
//! - (C) Terminal cursor suppression while session is Running/Compacting or animating
//!
//! Red-phase: these tests reference Rust API surfaces that do NOT yet exist
//! (`InputTransitionState::Hiding`, `InputTransitionState::Showing`, the
//! `should_render` busy-bypass, `AgentView::is_cursor_visible`, etc.). They
//! must FAIL to compile or to assert before implementation lands.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use codelet_fspec_tui::views::agent::input_transition::{
    render_input_transition, InputTransitionState,
};
use codelet_fspec_tui::views::agent::spinner::current_frame_glyph;
use codelet_fspec_tui::views::agent::INPUT_PLACEHOLDER_HINT;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

// Animation timing constants (parity with TS InputTransition.tsx).
const INK_FRAME_TIME_MS: u64 = 17;
const CHARS_PER_FRAME: usize = 5;
const ANIMATION_PHASE_DELAY_MS: u64 = 34;

fn buf_line(buf: &Buffer, area: Rect) -> String {
    let mut s = String::new();
    for x in area.x..area.x + area.width {
        s.push_str(buf[(x, area.y)].symbol());
    }
    s.trim_end().to_string()
}

// --------------------------------------------------------------------
// Scenario: Spinner advances at 80ms cadence even when no stream chunks arrive
// --------------------------------------------------------------------
#[test]
fn spinner_advances_at_80ms_cadence_independent_of_chunks() {
    // @step Given an AgentView with session s-1 in SessionStatus::Running and spinner_started_at set to the test clock origin
    let area = Rect::new(0, 0, 4, 1);

    // @step When the App run loop advances the tokio clock to 80ms, 160ms, 240ms, ..., 960ms in 80ms steps
    let ticks_ms: [u64; 12] = [0, 80, 160, 240, 320, 400, 480, 560, 640, 720, 800, 880];
    let expected = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙"];

    // @step Then a render is performed at each tick because the session is busy (should_render gate is bypassed while Running or Compacting)
    //   (Compile-time assertion: the AgentView exposes a way to mark "busy".)
    use codelet_fspec_tui::views::agent::AgentView;
    let _is_busy_fn: fn(&AgentView) -> bool = AgentView::is_busy;

    // @step And the cell at the input row column 0 cycles through the symbols "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙" in that order across the twelve captured frames
    for (i, ms) in ticks_ms.iter().enumerate() {
        let mut buf = Buffer::empty(area);
        render_input_transition(
            area,
            &mut buf,
            &InputTransitionState::Loading { elapsed_ms: *ms },
        );
        assert_eq!(buf[(0, 0)].symbol(), expected[i], "tick {ms}ms");
    }
}

// --------------------------------------------------------------------
// Scenario: Render loop redraws while busy without waiting on stream chunks
// --------------------------------------------------------------------
#[test]
fn render_loop_redraws_while_busy_independent_of_should_render_flag() {
    // @step Given an AgentView with session s-1 in SessionStatus::Running
    //   (Verified by AgentView::is_busy returning true when session is Running.)
    // @step When a 16ms RENDER_TICK fires
    //   (Modelled by the helper App::tick_should_draw which encodes the
    //   run-loop guard: should_draw <=> should_render || is_busy.)
    use codelet_fspec_tui::app::tick_should_draw;
    // @step Then the terminal is drawn (terminal.draw is called once for that tick)
    assert!(tick_should_draw(false, true, false), "busy must bypass should_render");
    // @step And the App should_render flag is false
    //   (encoded in the first arg above)
}

// --------------------------------------------------------------------
// Scenario: Render loop stays idle when nothing is busy and no events are pending
// --------------------------------------------------------------------
#[test]
fn render_loop_stays_idle_when_not_busy_and_no_events() {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle
    // @step When a 16ms RENDER_TICK fires
    use codelet_fspec_tui::app::tick_should_draw;
    // @step Then the terminal is NOT drawn for that tick (terminal.draw is not called)
    assert!(!tick_should_draw(false, false, false), "idle + no event must skip draw");
    // @step And no input-transition finish animation is in progress (phase is Idle)
    //   (Compile-time: Idle variant exists and is the default.)
    let _ = InputTransitionState::Idle;
    // @step And the App should_render flag is false
}

// --------------------------------------------------------------------
// Scenario: Terminal cursor is suppressed while session is Running
// --------------------------------------------------------------------
#[test]
fn cursor_suppressed_during_running() {
    // @step Given an AgentView with session s-1 in SessionStatus::Running rendered into an 80x24 buffer
    use codelet_fspec_tui::views::agent::AgentView;
    // @step When the App performs a render frame
    let visible = AgentView::is_cursor_visible_for(
        Some(codelet_rpc_types::SessionStatus::Running),
        &InputTransitionState::Loading { elapsed_ms: 0 },
    );
    // @step Then AgentView::is_cursor_visible returns false
    assert!(!visible);
    // @step And frame.set_cursor_position is NOT called for that frame
    //   (App::run gates `frame.set_cursor_position` on this predicate.)
}

// --------------------------------------------------------------------
// Scenario: Terminal cursor is suppressed while session is Compacting
// --------------------------------------------------------------------
#[test]
fn cursor_suppressed_during_compacting() {
    // @step Given an AgentView with session s-1 in SessionStatus::Compacting rendered into an 80x24 buffer
    use codelet_fspec_tui::views::agent::AgentView;
    // @step When the App performs a render frame
    let visible = AgentView::is_cursor_visible_for(
        Some(codelet_rpc_types::SessionStatus::Compacting),
        &InputTransitionState::Compacting { elapsed_ms: 0 },
    );
    // @step Then AgentView::is_cursor_visible returns false
    assert!(!visible);
    // @step And frame.set_cursor_position is NOT called for that frame
}

// --------------------------------------------------------------------
// Scenario: Terminal cursor is visible when session is Idle and MultiLineInput is mounted
// --------------------------------------------------------------------
#[test]
fn cursor_visible_when_idle_and_input_mounted() {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle and the input-transition phase is Idle
    use codelet_fspec_tui::views::agent::AgentView;
    // @step When the App performs a render frame
    let visible = AgentView::is_cursor_visible_for(
        Some(codelet_rpc_types::SessionStatus::Idle),
        &InputTransitionState::Idle,
    );
    // @step Then AgentView::is_cursor_visible returns true
    assert!(visible);
    // @step And frame.set_cursor_position IS called with the input-area-relative cursor (x, y)
    //   (App::run gates `frame.set_cursor_position` on this predicate.)
}

// --------------------------------------------------------------------
// Scenario: Busy-to-idle transition captures the spinner text and enters Hiding phase
// --------------------------------------------------------------------
#[test]
fn busy_to_idle_captures_spinner_text_and_enters_hiding() {
    // @step Given an AgentView whose session has been in SessionStatus::Running for 240ms and the spinner is painting "⠸ Thinking... (Esc to stop)"
    let captured = format!("{} Thinking... (Esc to stop)", current_frame_glyph(240));
    assert!(captured.starts_with("⠸"));

    // @step When the session transitions to SessionStatus::Idle at test clock t=240ms
    let next = InputTransitionState::transition_on_idle(
        &InputTransitionState::Loading { elapsed_ms: 240 },
        &captured,
        240,
    );

    // @step Then the InputTransitionState becomes Hiding with captured text "⠸ Thinking... (Esc to stop)" and visible_chars equal to the captured text length
    match &next {
        InputTransitionState::Hiding {
            captured: cap,
            visible_chars,
            started_at,
            hide_completed_at,
        } => {
            assert_eq!(cap, &captured);
            assert_eq!(*visible_chars, captured.chars().count());
            // @step And started_at is set to the current test clock value
            assert_eq!(*started_at, 240);
            assert!(hide_completed_at.is_none());
        }
        other => panic!("expected Hiding, got {other:?}"),
    }
}

// --------------------------------------------------------------------
// Scenario: Hiding phase advances at 5 chars per 17ms frame and renders captured prefix
// --------------------------------------------------------------------
#[test]
fn hiding_phase_advances_5_chars_per_17ms_and_renders_prefix() {
    // @step Given an AgentView in InputTransitionState::Hiding with captured "⠸ Thinking... (Esc to stop)" (28 chars) and visible_chars 28 at started_at t0
    let captured = String::from("⠸ Thinking... (Esc to stop)");
    let total = captured.chars().count();
    let t0: u64 = 1_000;

    // @step When the App run loop advances the test clock to t0+17ms, t0+34ms, t0+51ms, t0+68ms, t0+85ms, t0+102ms
    let elapsed_steps: [u64; 6] = [17, 34, 51, 68, 85, 102];
    let expected_visible: [usize; 6] = [
        total.saturating_sub(CHARS_PER_FRAME),
        total.saturating_sub(CHARS_PER_FRAME * 2),
        total.saturating_sub(CHARS_PER_FRAME * 3),
        total.saturating_sub(CHARS_PER_FRAME * 4),
        total.saturating_sub(CHARS_PER_FRAME * 5),
        0,
    ];

    // @step Then visible_chars at each step equals 23, 18, 13, 8, 3, 0 respectively
    //   (NOTE: the Gherkin scenario records the canonical sequence 23,
    //   18, 13, 8, 3, 0 assuming a 28-char captured string. The literal
    //   TS string "⠸ Thinking... (Esc to stop)" is actually 27 chars,
    //   so the Rust-side sequence is 22, 17, 12, 7, 2, 0 — same shape,
    //   shifted by 1. Both are derived from the same 5-char/frame law,
    //   the count just depends on the input length.)
    assert_eq!(expected_visible, [22, 17, 12, 7, 2, 0]);
    for (i, dms) in elapsed_steps.iter().enumerate() {
        let state = InputTransitionState::Hiding {
            captured: captured.clone(),
            visible_chars: total,
            started_at: t0,
            hide_completed_at: None,
        };
        let advanced = state.advance(t0 + *dms);
        let actual_visible = match &advanced {
            InputTransitionState::Hiding { visible_chars, .. } => *visible_chars,
            other => panic!("expected Hiding at +{dms}ms, got {other:?}"),
        };
        assert_eq!(actual_visible, expected_visible[i], "step {dms}ms");

        // @step And the rendered input row at each step is the captured text sliced 0..visible_chars, dim-styled, with no other glyphs to its right
        let area = Rect::new(0, 0, 40, 1);
        let mut buf = Buffer::empty(area);
        render_input_transition(area, &mut buf, &advanced);
        let line = buf_line(&buf, area);
        let expected_prefix: String = captured.chars().take(actual_visible).collect();
        assert_eq!(line, expected_prefix.trim_end(), "rendered prefix at +{dms}ms");
    }
}

// --------------------------------------------------------------------
// Scenario: Hiding holds for 34ms after visible_chars hits zero before entering Showing
// --------------------------------------------------------------------
#[test]
fn hiding_holds_34ms_at_zero_before_entering_showing() {
    let captured = String::from("⠸ Thinking... (Esc to stop)");
    let t1: u64 = 2_000;
    // @step Given an AgentView in InputTransitionState::Hiding that just reached visible_chars=0 at test clock t1 (hide_completed_at=t1)
    let zero = InputTransitionState::Hiding {
        captured,
        visible_chars: 0,
        started_at: t1 - 102, // arbitrary past start
        hide_completed_at: Some(t1),
    };

    // @step When the App advances the test clock to t1+33ms
    let still = zero.advance(t1 + ANIMATION_PHASE_DELAY_MS - 1);
    // @step Then the InputTransitionState remains Hiding with visible_chars=0
    match &still {
        InputTransitionState::Hiding { visible_chars, .. } => assert_eq!(*visible_chars, 0),
        other => panic!("expected Hiding at +33ms, got {other:?}"),
    }

    // @step When the App advances the test clock to t1+34ms
    let next = zero.advance(t1 + ANIMATION_PHASE_DELAY_MS);
    // @step Then the InputTransitionState becomes Showing with visible_chars=0 and started_at=t1+34ms and the placeholder string equal to AgentView's INPUT_PLACEHOLDER_HINT
    match &next {
        InputTransitionState::Showing {
            placeholder,
            visible_chars,
            started_at,
        } => {
            assert_eq!(placeholder.as_str(), INPUT_PLACEHOLDER_HINT);
            assert_eq!(*visible_chars, 0);
            assert_eq!(*started_at, t1 + ANIMATION_PHASE_DELAY_MS);
        }
        other => panic!("expected Showing at +34ms, got {other:?}"),
    }
}

// --------------------------------------------------------------------
// Scenario: Showing reveals placeholder at 5 chars per 17ms frame then enters Idle
// --------------------------------------------------------------------
#[test]
fn showing_reveals_placeholder_at_5_chars_per_17ms_then_idle() {
    // @step Given an AgentView in InputTransitionState::Showing with placeholder "Type a message" (14 chars) and visible_chars 0 at started_at t2
    let placeholder = String::from("Type a message");
    let total = placeholder.chars().count();
    assert_eq!(total, 14);
    let t2: u64 = 3_000;

    // @step When the App run loop advances the test clock to t2+17ms, t2+34ms, t2+51ms
    let steps: [(u64, usize); 3] = [(17, 5), (34, 10), (51, 14)];

    for (dms, expect_visible) in steps {
        let state = InputTransitionState::Showing {
            placeholder: placeholder.clone(),
            visible_chars: 0,
            started_at: t2,
        };
        let advanced = state.advance(t2 + dms);
        // @step Then visible_chars at each step equals 5, 10, 14 respectively (clamped to placeholder length)
        let actual = match &advanced {
            InputTransitionState::Showing { visible_chars, .. } => *visible_chars,
            // After reaching length the next state may collapse to Idle in the
            // SAME tick; treat that as "fully revealed" too.
            InputTransitionState::Idle => total,
            other => panic!("expected Showing/Idle at +{dms}ms, got {other:?}"),
        };
        assert_eq!(actual, expect_visible, "+{dms}ms");

        // @step And the rendered input row at each step is the placeholder sliced 0..visible_chars in DarkGray, with no other glyphs to its right
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        render_input_transition(area, &mut buf, &advanced);
        let line = buf_line(&buf, area);
        if matches!(advanced, InputTransitionState::Showing { .. }) {
            let expected_prefix: String = placeholder.chars().take(actual).collect();
            assert_eq!(line, expected_prefix.trim_end());
        }
    }

    // @step And after the frame that reaches placeholder length the InputTransitionState transitions to Idle and MultiLineInput is mounted on the next render
    //   total=14, CHARS_PER_FRAME=5 → needs 3 frames (15 chars saturated)
    //   so 3 * INK_FRAME_TIME_MS = 51ms is the first frame that has Idle.
    let saturated = InputTransitionState::Showing {
        placeholder,
        visible_chars: total,
        started_at: t2,
    };
    let next = saturated.advance(t2 + 3 * INK_FRAME_TIME_MS);
    assert!(
        matches!(next, InputTransitionState::Idle),
        "expected Idle after saturated frame, got {next:?}"
    );
}

// --------------------------------------------------------------------
// Scenario: Cursor stays suppressed during Hiding and Showing finish phases
// --------------------------------------------------------------------
#[test]
fn cursor_suppressed_during_hiding_and_showing() {
    use codelet_fspec_tui::views::agent::AgentView;
    let hiding = InputTransitionState::Hiding {
        captured: "⠸ Thinking...".to_string(),
        visible_chars: 5,
        started_at: 0,
        hide_completed_at: None,
    };
    // @step Given an AgentView whose InputTransitionState is Hiding
    // @step When the App performs a render frame
    // @step Then AgentView::is_cursor_visible returns false
    assert!(!AgentView::is_cursor_visible_for(
        Some(codelet_rpc_types::SessionStatus::Idle),
        &hiding
    ));

    // @step Given the InputTransitionState transitions to Showing
    let showing = InputTransitionState::Showing {
        placeholder: INPUT_PLACEHOLDER_HINT.to_string(),
        visible_chars: 5,
        started_at: 0,
    };
    // @step When the App performs a render frame
    // @step Then AgentView::is_cursor_visible returns false
    assert!(!AgentView::is_cursor_visible_for(
        Some(codelet_rpc_types::SessionStatus::Idle),
        &showing
    ));
}

// --------------------------------------------------------------------
// Scenario: Running state in the middle of a finish animation aborts the animation and resumes the spinner
// --------------------------------------------------------------------
#[test]
fn running_during_finish_animation_aborts_and_resumes_spinner() {
    // @step Given an AgentView in InputTransitionState::Hiding with visible_chars=13 at test clock t3
    let t3: u64 = 4_000;
    let hiding = InputTransitionState::Hiding {
        captured: "⠸ Thinking... (Esc to stop)".to_string(),
        visible_chars: 13,
        started_at: t3 - 51,
        hide_completed_at: None,
    };

    // @step When the session transitions back to SessionStatus::Running at test clock t3
    let next = InputTransitionState::transition_on_running(&hiding, t3);

    // @step Then the InputTransitionState becomes Loading with elapsed_ms=0 and spinner_started_at=t3
    match &next {
        InputTransitionState::Loading { elapsed_ms } => assert_eq!(*elapsed_ms, 0),
        other => panic!("expected Loading, got {other:?}"),
    }

    // @step And the next render paints "⠋ Thinking... (Esc to stop)" starting at frame 0
    let area = Rect::new(0, 0, 60, 1);
    let mut buf = Buffer::empty(area);
    render_input_transition(area, &mut buf, &next);
    assert!(buf_line(&buf, area).starts_with("⠋ Thinking... (Esc to stop)"));
}

// --------------------------------------------------------------------
// Scenario: Printable keystroke during Hiding short-circuits to Idle and enters the buffer
// --------------------------------------------------------------------
#[test]
fn printable_key_during_hiding_short_circuits_to_idle_and_buffers_key() {
    // @step Given an AgentView in InputTransitionState::Hiding with visible_chars=8
    let hiding = InputTransitionState::Hiding {
        captured: "⠸ Thinking... (Esc to stop)".to_string(),
        visible_chars: 8,
        started_at: 0,
        hide_completed_at: None,
    };

    // @step When the user presses the printable key "h"
    let (next, buffered) = InputTransitionState::on_printable_key(&hiding, 'h');

    // @step Then the InputTransitionState becomes Idle
    assert!(matches!(next, InputTransitionState::Idle));
    // @step And MultiLineInput is mounted and contains the buffer "h" with the cursor positioned after it
    assert_eq!(buffered, Some('h'));
}

// --------------------------------------------------------------------
// Scenario: Printable keystroke during Showing short-circuits to Idle and enters the buffer
// --------------------------------------------------------------------
#[test]
fn printable_key_during_showing_short_circuits_to_idle_and_buffers_key() {
    // @step Given an AgentView in InputTransitionState::Showing with visible_chars=10
    let showing = InputTransitionState::Showing {
        placeholder: INPUT_PLACEHOLDER_HINT.to_string(),
        visible_chars: 10,
        started_at: 0,
    };

    // @step When the user presses the printable key "x"
    let (next, buffered) = InputTransitionState::on_printable_key(&showing, 'x');

    // @step Then the InputTransitionState becomes Idle
    assert!(matches!(next, InputTransitionState::Idle));
    // @step And MultiLineInput is mounted and contains the buffer "x" with the cursor positioned after it
    assert_eq!(buffered, Some('x'));
}

// --------------------------------------------------------------------
// Scenario: Source-shape ceiling stays under 300 LoC for input_transition.rs
// --------------------------------------------------------------------
#[test]
fn source_shape_input_transition_under_300_loc() {
    // @step Given the source file codelet/fspec-tui/src/views/agent/input_transition.rs after the animation state machine has been implemented
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/views/agent/input_transition.rs"
    );
    let contents = std::fs::read_to_string(path).unwrap();

    // @step When the source-shape test reads the file's line count
    let lines = contents.lines().count();

    // @step Then the line count is strictly less than 300
    assert!(lines < 300, "input_transition.rs has {lines} lines (>= 300)");
}

// Suppress 'unused' for the constant — referenced in expected_visible math.
#[allow(dead_code)]
const _USE_CHARS_PER_FRAME: usize = CHARS_PER_FRAME;

// --------------------------------------------------------------------
// Scenario: Render loop continues to tick while the finish animation is mid-Hiding even after session is Idle
// --------------------------------------------------------------------
#[test]
fn render_loop_ticks_during_hiding_even_when_session_idle() {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle and the InputTransitionState is Hiding with visible_chars=28
    let hiding = InputTransitionState::Hiding {
        captured: "⠸ Thinking... (Esc to stop)".to_string(),
        visible_chars: 28,
        started_at: 0,
        hide_completed_at: None,
    };
    let is_animating = matches!(
        &hiding,
        InputTransitionState::Hiding { .. } | InputTransitionState::Showing { .. }
    );
    assert!(is_animating);

    // @step When a 16ms RENDER_TICK fires and App should_render is false
    use codelet_fspec_tui::app::tick_should_draw;

    // @step Then the terminal IS drawn for that tick (terminal.draw is called once)
    // @step And tick_should_draw(false, is_busy=false, is_animating=true) returns true
    assert!(
        tick_should_draw(false, false, true),
        "tick_should_draw must accept is_animating and return true when animating"
    );
}

// --------------------------------------------------------------------
// Scenario: Render loop continues to tick while the finish animation is mid-Showing even after session is Idle
// --------------------------------------------------------------------
#[test]
fn render_loop_ticks_during_showing_even_when_session_idle() {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle and the InputTransitionState is Showing with visible_chars=5
    let showing = InputTransitionState::Showing {
        placeholder: INPUT_PLACEHOLDER_HINT.to_string(),
        visible_chars: 5,
        started_at: 0,
    };
    let is_animating = matches!(
        &showing,
        InputTransitionState::Hiding { .. } | InputTransitionState::Showing { .. }
    );
    assert!(is_animating);

    // @step When a 16ms RENDER_TICK fires and App should_render is false
    use codelet_fspec_tui::app::tick_should_draw;

    // @step Then the terminal IS drawn for that tick (terminal.draw is called once)
    // @step And tick_should_draw(false, is_busy=false, is_animating=true) returns true
    assert!(
        tick_should_draw(false, false, true),
        "tick_should_draw must accept is_animating and return true when animating"
    );
}
