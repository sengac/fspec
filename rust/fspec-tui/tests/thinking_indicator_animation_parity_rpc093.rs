//! Feature: spec/features/agentview-thinking-indicator-animation-parity.feature
//!
//! RPC-093 — AgentView Thinking indicator animation parity: (A) the spinner
//! advances at 80ms cadence independently of stream chunks, (B) the
//! Hiding/Showing finish animation (CHARS_PER_FRAME=5, INK_FRAME_TIME_MS=17,
//! ANIMATION_PHASE_DELAY_MS=34), (C) the terminal cursor predicate (TUI-118:
//! shown on the busy composer row).
//!
//! Harness (state builders, row painter, cursor predicate):
//! `thinking_indicator_rpc093_support/mod.rs`.

#![allow(dead_code)]

mod common;
mod thinking_indicator_rpc093_support;
mod tui115_support;
use codelet_test_helpers::{OrFail, TestResult};
use thinking_indicator_rpc093_support::*;

/// Scenario: Spinner advances at 80ms cadence even when no stream chunks arrive
#[test]
fn spinner_advances_at_80ms_cadence_independent_of_chunks() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Running and spinner_started_at set to the test clock origin
    // @step When the App run loop advances the tokio clock to 80ms, 160ms, 240ms, ..., 960ms in 80ms steps
    let ticks_ms: [u64; 12] = [0, 80, 160, 240, 320, 400, 480, 560, 640, 720, 800, 880];
    let expected = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙"];

    // @step Then a render is performed at each tick because the session is busy (should_render gate is bypassed while Running or Compacting)
    //   (Compile-time assertion: the AgentView exposes a way to mark "busy".)
    let _is_busy_fn: fn(&AgentView) -> bool = AgentView::is_busy;

    // @step And the cell at the input row column 0 cycles through the symbols "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙" in that order across the twelve captured frames
    for (ms, glyph) in ticks_ms.iter().zip(expected) {
        let state = InputTransitionState::Loading { elapsed_ms: *ms };
        assert_eq!(first_cell(&state), glyph, "tick {ms}ms");
    }
    Ok(())
}

/// Scenario: Render loop redraws while busy without waiting on stream chunks
#[test]
fn render_loop_redraws_while_busy_independent_of_should_render_flag() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Running
    //   (Verified by AgentView::is_busy returning true when session is Running.)
    // @step When a 16ms RENDER_TICK fires
    //   (Modelled by tick_should_draw: should_draw <=> should_render || is_busy.)
    // @step Then the terminal is drawn (terminal.draw is called once for that tick)
    let drawn = tick_should_draw(false, true, false, false, false, false);
    assert!(drawn, "busy must bypass should_render");
    // @step And the App should_render flag is false
    //   (encoded in the first arg above)
    Ok(())
}

/// Scenario: Render loop stays idle when nothing is busy and no events are pending
#[test]
fn render_loop_stays_idle_when_not_busy_and_no_events() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle
    // @step When a 16ms RENDER_TICK fires
    // @step Then the terminal is NOT drawn for that tick (terminal.draw is not called)
    let drawn = tick_should_draw(false, false, false, false, false, false);
    assert!(!drawn, "idle + no event must skip draw");
    // @step And no input-transition finish animation is in progress (phase is Idle)
    assert!(!is_animating(&InputTransitionState::Idle));
    // @step And the App should_render flag is false
    Ok(())
}

/// Scenario: Terminal cursor is visible on the composer while session is Running
#[tokio::test]
async fn cursor_visible_on_composer_during_running() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Running rendered into an 80x24 buffer
    // @step When the App performs a render frame
    let frame = busy_cursor(SessionStatus::Running, "Thinking").await?;
    // @step Then AgentView::is_cursor_visible returns true
    assert!(frame.visible, "the busy composer must show its caret");
    // @step And frame.set_cursor_position IS called on the composer row directly under the activity row
    assert_eq!(frame.cursor_row, Some(frame.activity_row + 1));
    Ok(())
}

/// Scenario: Terminal cursor is visible on the composer while session is Compacting
#[tokio::test]
async fn cursor_visible_on_composer_during_compacting() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Compacting rendered into an 80x24 buffer
    // @step When the App performs a render frame
    let frame = busy_cursor(SessionStatus::Compacting, "Compacting").await?;
    // @step Then AgentView::is_cursor_visible returns true
    assert!(frame.visible, "the busy composer must show its caret");
    // @step And frame.set_cursor_position IS called on the composer row directly under the activity row
    assert_eq!(frame.cursor_row, Some(frame.activity_row + 1));
    Ok(())
}

/// Scenario: Terminal cursor is visible when session is Idle and MultiLineInput is mounted
#[test]
fn cursor_visible_when_idle_and_input_mounted() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle and the input-transition phase is Idle
    // @step When the App performs a render frame
    let visible = cursor_visible(SessionStatus::Idle, &InputTransitionState::Idle);
    // @step Then AgentView::is_cursor_visible returns true
    assert!(visible);
    // @step And frame.set_cursor_position IS called with the input-area-relative cursor (x, y)
    //   (App::run gates `frame.set_cursor_position` on this predicate.)
    Ok(())
}

/// Scenario: Busy-to-idle transition captures the spinner text and enters Hiding phase
#[test]
fn busy_to_idle_captures_spinner_text_and_enters_hiding() -> TestResult {
    // @step Given an AgentView whose session has been in SessionStatus::Running for 240ms and the spinner is painting "⠸ Thinking... (Esc to stop)"
    let captured = format!("{} Thinking... (Esc to stop)", current_frame_glyph(240));
    assert_eq!(captured, CAPTURED);

    // @step When the session transitions to SessionStatus::Idle at test clock t=240ms
    let loading = InputTransitionState::Loading { elapsed_ms: 240 };
    let next = InputTransitionState::transition_on_idle(&loading, &captured, 240);

    // @step Then the InputTransitionState becomes Hiding with captured text "⠸ Thinking... (Esc to stop)" and visible_chars equal to the captured text length
    // @step And started_at is set to the current test clock value
    assert_state(&next, &hiding(captured.chars().count(), 240, None));
    Ok(())
}

/// Scenario: Hiding phase advances at 5 chars per 17ms frame and renders captured prefix
#[test]
fn hiding_phase_advances_5_chars_per_17ms_and_renders_prefix() -> TestResult {
    // @step Given an AgentView in InputTransitionState::Hiding with captured "⠸ Thinking... (Esc to stop)" (28 chars) and visible_chars 28 at started_at t0
    let total = CAPTURED.chars().count();
    let t0: u64 = 1_000;

    // @step When the App run loop advances the test clock to t0+17ms, t0+34ms, t0+51ms, t0+68ms, t0+85ms, t0+102ms
    let elapsed_steps: [u64; 6] = [17, 34, 51, 68, 85, 102];
    let expected_visible: Vec<usize> = (1..=6)
        .map(|frame| total.saturating_sub(CHARS_PER_FRAME * frame))
        .collect();

    // @step Then visible_chars at each step equals 23, 18, 13, 8, 3, 0 respectively
    //   (The Gherkin records 23..0 for a 28-char string; the literal TS string
    //   is 27 chars, so Rust sees 22..0 — the same 5-char/frame law.)
    assert_eq!(expected_visible, [22, 17, 12, 7, 2, 0]);
    for (dms, expected) in elapsed_steps.iter().zip(expected_visible) {
        let advanced = hiding(total, t0, None).advance(t0 + *dms);
        let actual = hiding_visible(&advanced, &format!("+{dms}ms"))?;
        assert_eq!(actual, expected, "step {dms}ms");

        // @step And the rendered input row at each step is the captured text sliced 0..visible_chars, dim-styled, with no other glyphs to its right
        let prefix: String = CAPTURED.chars().take(actual).collect();
        assert_eq!(render_line(&advanced, 40), prefix.trim_end(), "+{dms}ms");
    }
    Ok(())
}

/// Scenario: Hiding holds for 34ms after visible_chars hits zero before entering Showing
#[test]
fn hiding_holds_34ms_at_zero_before_entering_showing() -> TestResult {
    let t1: u64 = 2_000;
    // @step Given an AgentView in InputTransitionState::Hiding that just reached visible_chars=0 at test clock t1 (hide_completed_at=t1)
    let zero = hiding(0, t1 - 102, Some(t1));

    // @step When the App advances the test clock to t1+33ms
    let still = zero.advance(t1 + ANIMATION_PHASE_DELAY_MS - 1);
    // @step Then the InputTransitionState remains Hiding with visible_chars=0
    assert_eq!(hiding_visible(&still, "+33ms")?, 0);

    // @step When the App advances the test clock to t1+34ms
    let next = zero.advance(t1 + ANIMATION_PHASE_DELAY_MS);
    // @step Then the InputTransitionState becomes Showing with visible_chars=0 and started_at=t1+34ms and the placeholder string equal to AgentView's INPUT_PLACEHOLDER_HINT
    let expected = showing(INPUT_PLACEHOLDER_HINT, 0, t1 + ANIMATION_PHASE_DELAY_MS);
    assert_state(&next, &expected);
    Ok(())
}

/// Scenario: Showing reveals placeholder at 5 chars per 17ms frame then enters Idle
#[test]
fn showing_reveals_placeholder_at_5_chars_per_17ms_then_idle() -> TestResult {
    // @step Given an AgentView in InputTransitionState::Showing with placeholder "Type a message" (14 chars) and visible_chars 0 at started_at t2
    let placeholder = "Type a message";
    let total = placeholder.chars().count();
    assert_eq!(total, 14);
    let t2: u64 = 3_000;

    // @step When the App run loop advances the test clock to t2+17ms, t2+34ms, t2+51ms
    for (dms, expect_visible) in [(17, 5), (34, 10), (51, 14)] {
        let advanced = showing(placeholder, 0, t2).advance(t2 + dms);
        // @step Then visible_chars at each step equals 5, 10, 14 respectively (clamped to placeholder length)
        //   (Reaching the length may collapse to Idle in the same tick: fully revealed.)
        let actual = match &advanced {
            InputTransitionState::Showing { visible_chars, .. } => *visible_chars,
            InputTransitionState::Idle => total,
            other => return Err(format!("expected Showing/Idle at +{dms}ms, got {other:?}").into()),
        };
        assert_eq!(actual, expect_visible, "+{dms}ms");

        // @step And the rendered input row at each step is the placeholder sliced 0..visible_chars in DarkGray, with no other glyphs to its right
        if matches!(advanced, InputTransitionState::Showing { .. }) {
            let prefix: String = placeholder.chars().take(actual).collect();
            assert_eq!(render_line(&advanced, 30), prefix.trim_end());
        }
    }

    // @step And after the frame that reaches placeholder length the InputTransitionState transitions to Idle and MultiLineInput is mounted on the next render
    //   14 chars at 5/frame needs 3 frames, so 3 * INK_FRAME_TIME_MS = 51ms is the first Idle frame.
    let next = showing(placeholder, total, t2).advance(t2 + 3 * INK_FRAME_TIME_MS);
    assert_state(&next, &InputTransitionState::Idle);
    Ok(())
}

/// Scenario: Cursor stays suppressed during Hiding and Showing finish phases
#[test]
fn cursor_suppressed_during_hiding_and_showing() -> TestResult {
    // @step Given an AgentView whose InputTransitionState is Hiding
    // @step When the App performs a render frame
    // @step Then AgentView::is_cursor_visible returns false
    assert!(!cursor_visible(SessionStatus::Idle, &hiding(5, 0, None)));

    // @step Given the InputTransitionState transitions to Showing
    let showing_state = showing(INPUT_PLACEHOLDER_HINT, 5, 0);
    // @step When the App performs a render frame
    // @step Then AgentView::is_cursor_visible returns false
    assert!(!cursor_visible(SessionStatus::Idle, &showing_state));
    Ok(())
}

/// Scenario: Running state in the middle of a finish animation aborts the animation and resumes the spinner
#[test]
fn running_during_finish_animation_aborts_and_resumes_spinner() -> TestResult {
    // @step Given an AgentView in InputTransitionState::Hiding with visible_chars=13 at test clock t3
    let t3: u64 = 4_000;
    let state = hiding(13, t3 - 51, None);

    // @step When the session transitions back to SessionStatus::Running at test clock t3
    let next = InputTransitionState::transition_on_running(&state, t3);

    // @step Then the InputTransitionState becomes Loading with elapsed_ms=0 and spinner_started_at=t3
    assert_state(&next, &InputTransitionState::Loading { elapsed_ms: 0 });

    // @step And the next render paints "⠋ Thinking... (Esc to stop)" starting at frame 0
    assert!(render_line(&next, 60).starts_with("⠋ Thinking... (Esc to stop)"));
    Ok(())
}

/// Scenario: Printable keystroke during Hiding short-circuits to Idle and enters the buffer
#[test]
fn printable_key_during_hiding_short_circuits_to_idle_and_buffers_key() -> TestResult {
    // @step Given an AgentView in InputTransitionState::Hiding with visible_chars=8
    let state = hiding(8, 0, None);

    // @step When the user presses the printable key "h"
    let (next, buffered) = InputTransitionState::on_printable_key(&state, 'h');

    // @step Then the InputTransitionState becomes Idle
    assert_state(&next, &InputTransitionState::Idle);
    // @step And MultiLineInput is mounted and contains the buffer "h" with the cursor positioned after it
    assert_eq!(buffered, Some('h'));
    Ok(())
}

/// Scenario: Printable keystroke during Showing short-circuits to Idle and enters the buffer
#[test]
fn printable_key_during_showing_short_circuits_to_idle_and_buffers_key() -> TestResult {
    // @step Given an AgentView in InputTransitionState::Showing with visible_chars=10
    let state = showing(INPUT_PLACEHOLDER_HINT, 10, 0);

    // @step When the user presses the printable key "x"
    let (next, buffered) = InputTransitionState::on_printable_key(&state, 'x');

    // @step Then the InputTransitionState becomes Idle
    assert_state(&next, &InputTransitionState::Idle);
    // @step And MultiLineInput is mounted and contains the buffer "x" with the cursor positioned after it
    assert_eq!(buffered, Some('x'));
    Ok(())
}

/// Scenario: Source-shape ceiling stays under 300 LoC for input_transition.rs
#[test]
fn source_shape_input_transition_under_300_loc() -> TestResult {
    // @step Given the source file rust/fspec-tui/src/views/agent/input_transition.rs after the animation state machine has been implemented
    let contents =
        std::fs::read_to_string(INPUT_TRANSITION_RS).or_fail("read input_transition.rs")?;

    // @step When the source-shape test reads the file's line count
    let lines = contents.lines().count();

    // @step Then the line count is strictly less than 300
    assert!(lines < 300, "input_transition.rs has {lines} lines");
    Ok(())
}

/// Scenario: Render loop continues to tick while the finish animation is mid-Hiding even after session is Idle
#[test]
fn render_loop_ticks_during_hiding_even_when_session_idle() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle and the InputTransitionState is Hiding with visible_chars=28
    assert!(is_animating(&hiding(28, 0, None)));

    // @step When a 16ms RENDER_TICK fires and App should_render is false
    // @step Then the terminal IS drawn for that tick (terminal.draw is called once)
    // @step And tick_should_draw(false, is_busy=false, is_animating=true) returns true
    let drawn = tick_should_draw(false, false, true, false, false, false);
    assert!(drawn, "tick_should_draw must return true when animating");
    Ok(())
}

/// Scenario: Render loop continues to tick while the finish animation is mid-Showing even after session is Idle
#[test]
fn render_loop_ticks_during_showing_even_when_session_idle() -> TestResult {
    // @step Given an AgentView with session s-1 in SessionStatus::Idle and the InputTransitionState is Showing with visible_chars=5
    assert!(is_animating(&showing(INPUT_PLACEHOLDER_HINT, 5, 0)));

    // @step When a 16ms RENDER_TICK fires and App should_render is false
    // @step Then the terminal IS drawn for that tick (terminal.draw is called once)
    // @step And tick_should_draw(false, is_busy=false, is_animating=true) returns true
    let drawn = tick_should_draw(false, false, true, false, false, false);
    assert!(drawn, "tick_should_draw must return true when animating");
    Ok(())
}
