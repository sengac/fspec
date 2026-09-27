//! BUG-194 — Mux: thinking indicator disappears when focus moves off a
//! running agent pane.
//!
//! Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.
//!
//! Root cause under test: the thinking indicator's phase machine
//! (`InputTransitionState`) was a single VIEW-LEVEL state on `AgentView`
//! ticked only by the FOCUSED mux agent pane. BUG-194 makes it a
//! per-session slot on `SessionContext` (`input_transition`) so every
//! rendered agent pane ticks its OWN session's state.
//!
//! Red phase: the buffer-content assertions fail against the old
//! focus-driven machine (unfocused running panes paint a ghost draft
//! instead of the spinner), and the per-session assertions fail to
//! compile because `SessionContext::input_transition` / the new
//! store scan methods do not exist yet.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, dead_code)]

use std::time::Duration;

use codelet_fspec_tui::store::{AgentViewStore, BoardStore, SessionContext};
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::agent::input_transition::InputTransitionState;
use codelet_fspec_tui::views::agent::spinner::DOTS_FRAMES;
use codelet_fspec_tui::views::agent::INPUT_PLACEHOLDER_HINT;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::{Navigator, ViewMode};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::Terminal;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

// ── harness helpers ────────────────────────────────────────────────────

fn fresh() -> (
    Navigator,
    tokio::sync::mpsc::UnboundedReceiver<codelet_fspec_tui::components::Action>,
) {
    let (tx, rx) = unbounded_channel();
    (Navigator::new(Arc::new(Theme::default()), tx), rx)
}

fn sid(n: usize) -> SessionId {
    SessionId::new(format!("s-{n}"))
}

fn seed_sessions(agent: &mut AgentViewStore, n: usize) {
    for i in 1..=n {
        agent.append_session(SessionContext::new(sid(i)));
    }
}

/// Enable mux with the given pane list + active view Mux.
fn enable_mux(nav: &mut Navigator, panes: &[MuxPaneKind]) {
    nav.mux.set_pane_list(panes.to_vec(), None);
    nav.active_view = ViewMode::Mux;
}

fn set_status(agent: &mut AgentViewStore, id: &SessionId, status: SessionStatus) {
    agent.set_session_status(id.clone(), status);
}

/// Render one mux frame into a 120x24 TestBackend; return the buffer.
fn render(
    nav: &mut Navigator,
    board: &BoardStore,
    agent: &mut AgentViewStore,
) -> ratatui::buffer::Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| {
        nav.render_with_stores(frame.area(), frame.buffer_mut(), board, agent);
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Flatten `rect` into one string per row.
fn pane_lines(buf: &ratatui::buffer::Buffer, rect: Rect) -> Vec<String> {
    (0..rect.height)
        .map(|dy| {
            let y = rect.y + dy;
            (0..rect.width)
                .map(|dx| buf[(rect.x + dx, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

fn pane_contains(lines: &[String], needle: &str) -> bool {
    lines.iter().any(|l| l.contains(needle))
}

/// The braille glyph painted at the start of the spinner row, if any
/// row of the pane contains the full spinner text. The row is
/// `<pad space>⠋ Thinking... (Esc to stop)` — the input area pads its
/// left edge by 1 col (RPC-029 paddingX=1), so the glyph is the
/// non-space character immediately BEFORE the message, not necessarily
/// the whole row prefix.
fn spinner_glyph(lines: &[String], message: &str) -> Option<char> {
    for line in lines {
        if let Some(pos) = line.find(message) {
            let prefix: Vec<char> = line[..pos].chars().collect();
            // Walk back over the pad space to the last non-space char.
            if let Some(ch) = prefix.iter().rev().find(|c| !c.is_whitespace()) {
                return Some(*ch);
            }
        }
    }
    None
}

/// Loop-render until `predicate` holds (bounded — the sweep is ~30
/// frames; 300 is generous).
fn render_until(
    nav: &mut Navigator,
    board: &BoardStore,
    agent: &mut AgentViewStore,
    mut predicate: impl FnMut(&AgentViewStore) -> bool,
) {
    for _ in 0..300 {
        if predicate(agent) {
            return;
        }
        render(nav, board, agent);
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a running agent pane keeps its thinking spinner after focus
// moves to another agent pane
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn a_running_agent_pane_keeps_its_thinking_spinner_after_focus_moves_to_another_agent_pane() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(
        &mut nav,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    // @step And two agent sessions are open
    seed_sessions(&mut agent, 2);
    // @step And the agent 1 session is running
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    // @step And the agent 2 session is idle
    set_status(&mut agent, &sid(2), SessionStatus::Idle);
    // @step And the agent 1 pane is focused
    nav.mux.set_focus(1);
    // @step When the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent1 = pane_lines(&buf, rects[1]);
    // @step Then the agent 1 pane input shows the live thinking spinner
    assert!(
        pane_contains(&agent1, "Thinking... (Esc to stop)"),
        "focused running pane must paint its spinner: {agent1:?}"
    );
    // @step When focus moves to the agent 2 pane
    nav.mux.set_focus(2);
    // @step And the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent1 = pane_lines(&buf, rects[1]);
    let agent2 = pane_lines(&buf, rects[2]);
    // @step Then the agent 1 pane input still shows the live thinking spinner
    assert!(
        pane_contains(&agent1, "Thinking... (Esc to stop)"),
        "BUG-194: the unfocused RUNNING pane must keep its spinner: {agent1:?}"
    );
    // @step And the agent 2 pane input shows the live composer
    assert!(
        pane_contains(&agent2, "Type a message"),
        "the focused idle pane paints the live composer: {agent2:?}"
    );
    // @step And keyboard input still reaches the agent 2 pane only
    let key = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    let _ = nav.handle_event(&key, &board);
    assert_eq!(
        nav.agent.input.value(),
        "a",
        "the focused pane's composer receives the keystroke"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a running agent pane keeps its thinking spinner when the
// board pane is focused
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn a_running_agent_pane_keeps_its_thinking_spinner_when_the_board_pane_is_focused() {
    // @step Given mux mode is active with the pane list board and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(&mut nav, &[MuxPaneKind::Board, MuxPaneKind::Agent]);
    // @step And one agent session is open
    seed_sessions(&mut agent, 1);
    // @step And the agent session is running
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    // @step And the agent pane is focused
    nav.mux.set_focus(1);
    // @step When the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    assert!(
        pane_contains(&pane_lines(&buf, rects[1]), "Thinking... (Esc to stop)"),
        "focused running pane paints its spinner"
    );
    // @step When focus moves to the board pane
    nav.mux.set_focus(0);
    // @step And the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent_pane = pane_lines(&buf, rects[1]);
    // @step Then the agent pane input still shows the live thinking spinner
    assert!(
        pane_contains(&agent_pane, "Thinking... (Esc to stop)"),
        "BUG-194: the spinner must survive focus moving to the board pane: {agent_pane:?}"
    );
    // @step And the board pane receives keyboard input
    let key = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    let result = nav.handle_event(&key, &board);
    assert!(
        result.is_consumed(),
        "the focused board pane consumes its keys"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a running agent pane plays its finish sweep after going
// idle while unfocused
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn a_running_agent_pane_plays_its_finish_sweep_after_going_idle_while_unfocused() {
    // @step Given mux mode is active with the pane list agent and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(&mut nav, &[MuxPaneKind::Agent, MuxPaneKind::Agent]);
    // @step And two agent sessions are open
    seed_sessions(&mut agent, 2);
    // @step And the agent 1 session is running
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    // @step And the agent 2 session is idle
    set_status(&mut agent, &sid(2), SessionStatus::Idle);
    // @step And the agent 2 pane is focused
    nav.mux.set_focus(1);
    // prime: one render so agent 1's spinner paints frame 0 + caches its line
    let _ = render(&mut nav, &board, &mut agent);
    // @step When the agent 1 session becomes idle
    set_status(&mut agent, &sid(1), SessionStatus::Idle);
    // @step And the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent1 = pane_lines(&buf, rects[0]);
    // @step Then the agent 1 pane input shows the captured spinner line shrinking
    assert!(
        pane_contains(&agent1, "(Esc to stop)"),
        "BUG-194: the unfocused pane must start its per-session finish sweep (full captured line on frame 0): {agent1:?}"
    );
    // @step And the agent 2 pane input shows the live composer
    let agent2 = pane_lines(&buf, rects[1]);
    assert!(
        pane_contains(&agent2, "Type a message"),
        "the focused idle pane keeps its composer: {agent2:?}"
    );
    // Two more renders: the sweep quantizes on 17ms frames over a
    // 16ms/frame clock (frame_count = (clock - started_at) / 17), so
    // the first 5-char consumption lands on the SECOND frame —
    // byte-identical to the old single-view quantization (the global
    // clock also advanced 16ms/frame). After 2 frames the captured
    // line has shrunk by 5+ chars: the "(Esc to stop)" tail is cut
    // off.
    let _ = render(&mut nav, &board, &mut agent);
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent1 = pane_lines(&buf, rects[0]);
    assert!(
        pane_contains(&agent1, "Thinking"),
        "the sweep shows the captured line's prefix: {agent1:?}"
    );
    assert!(
        !pane_contains(&agent1, "(Esc to stop)"),
        "the captured line must be shrinking (5 chars/frame): {agent1:?}"
    );
    // @step When the agent 1 finish sweep completes
    render_until(&mut nav, &board, &mut agent, |store| {
        matches!(
            store
                .session_context_for(&sid(1))
                .unwrap()
                .input_transition
                .state,
            InputTransitionState::Idle
        )
    });
    // @step And the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    // @step Then the agent 1 pane input shows its ghost draft
    let agent1 = pane_lines(&buf, rects[0]);
    assert!(
        pane_contains(&agent1, "Type a message"),
        "after the sweep the pane returns to its ghost draft row: {agent1:?}"
    );
    // @step And the agent 2 pane input still shows the live composer
    let agent2 = pane_lines(&buf, rects[1]);
    assert!(
        pane_contains(&agent2, "Type a message"),
        "the focused pane is untouched by the other session's sweep: {agent2:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a compacting agent pane keeps its compacting spinner while
// another pane is focused
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn a_compacting_agent_pane_keeps_its_compacting_spinner_while_another_pane_is_focused() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(
        &mut nav,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    );
    // @step And two agent sessions are open
    seed_sessions(&mut agent, 2);
    // @step And the agent 1 session is idle
    set_status(&mut agent, &sid(1), SessionStatus::Idle);
    // @step And the agent 2 session is compacting
    set_status(&mut agent, &sid(2), SessionStatus::Compacting);
    // @step And the agent 1 pane is focused
    nav.mux.set_focus(1);
    // @step When the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent2 = pane_lines(&buf, rects[2]);
    // @step Then the agent 2 pane input shows the live compacting spinner
    assert!(
        pane_contains(&agent2, "Compacting... (Esc to stop)"),
        "BUG-194: the unfocused COMPACTING pane must keep its spinner: {agent2:?}"
    );
    // @step And the agent 1 pane input shows the live composer
    let agent1 = pane_lines(&buf, rects[1]);
    assert!(
        pane_contains(&agent1, "Type a message"),
        "the focused idle pane paints its composer: {agent1:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: an unfocused running pane paints the identical spinner row as
// a focused one
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn an_unfocused_running_pane_paints_the_identical_spinner_row_as_a_focused_one() {
    // @step Given mux mode is active with the pane list agent and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(&mut nav, &[MuxPaneKind::Agent, MuxPaneKind::Agent]);
    // @step And two agent sessions are open
    seed_sessions(&mut agent, 2);
    // @step And both agent sessions are running
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    set_status(&mut agent, &sid(2), SessionStatus::Running);
    // @step And the agent 1 pane is focused
    nav.mux.set_focus(0);
    // @step When the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent1 = pane_lines(&buf, rects[0]);
    let agent2 = pane_lines(&buf, rects[1]);
    // @step Then the agent 1 pane spinner row contains "Thinking... (Esc to stop)" with a braille glyph
    assert!(pane_contains(&agent1, "Thinking... (Esc to stop)"));
    let g1 = spinner_glyph(&agent1, "Thinking... (Esc to stop)")
        .expect("focused pane paints a braille glyph");
    assert!(
        DOTS_FRAMES.iter().any(|f| f.starts_with(g1)),
        "focused pane glyph must be a braille dot, got {g1:?}"
    );
    // @step And the agent 2 pane spinner row contains "Thinking... (Esc to stop)" with the same braille glyph
    assert!(
        pane_contains(&agent2, "Thinking... (Esc to stop)"),
        "BUG-194: the unfocused running pane paints the SAME spinner row: {agent2:?}"
    );
    let g2 = spinner_glyph(&agent2, "Thinking... (Esc to stop)")
        .expect("unfocused pane paints a braille glyph");
    assert_eq!(
        g1, g2,
        "both panes arm their spinner in the same frame → same frame-0 glyph"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: moving focus away and back does not restart a running
// session's spinner clock
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn moving_focus_away_and_back_does_not_restart_a_running_sessions_spinner_clock() {
    // @step Given mux mode is active with the pane list agent and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(&mut nav, &[MuxPaneKind::Agent, MuxPaneKind::Agent]);
    // @step And two agent sessions are open
    seed_sessions(&mut agent, 2);
    // @step And the agent 1 session is running
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    // @step And the agent 2 session is idle
    set_status(&mut agent, &sid(2), SessionStatus::Idle);
    // @step And the agent 1 pane is focused
    nav.mux.set_focus(0);
    // @step When the grid is rendered
    for _ in 0..3 {
        render(&mut nav, &board, &mut agent);
    }
    // @step Then the agent 1 pane spinner has a recorded elapsed time since start
    let ctx = agent.session_context_for(&sid(1)).expect("session 1 open");
    assert!(
        matches!(
            ctx.input_transition.state,
            InputTransitionState::Loading { .. }
        ),
        "the per-session slot must be Loading while running"
    );
    let start_before = ctx
        .input_transition
        .spinner_started_at
        .expect("spinner start recorded per session");
    // @step When focus moves to the agent 2 pane and back to the agent 1 pane
    nav.mux.set_focus(1);
    render(&mut nav, &board, &mut agent);
    nav.mux.set_focus(0);
    // @step And the grid is rendered
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    // @step Then the agent 1 pane spinner elapsed time is strictly greater than before
    let ctx = agent.session_context_for(&sid(1)).expect("session 1 open");
    assert!(
        matches!(
            ctx.input_transition.state,
            InputTransitionState::Loading { .. }
        ),
        "BUG-194: focus movement must NOT drive the per-session state off Loading (the idle focused pane must not kill it)"
    );
    let start_after = ctx
        .input_transition
        .spinner_started_at
        .expect("spinner start still recorded");
    assert_eq!(
        start_after - start_before,
        Duration::ZERO,
        "BUG-194: the per-session spinner clock must not restart on a focus round-trip"
    );
    // @step And the spinner has not reset to zero
    let lines = pane_lines(&buf, rects[0]);
    assert!(
        pane_contains(&lines, "Thinking... (Esc to stop)"),
        "the spinner row is still live: {lines:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the run loop keeps redrawing while an unfocused pane's finish
// sweep is in flight
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn the_run_loop_keeps_redrawing_while_an_unfocused_panes_finish_sweep_is_in_flight() {
    // @step Given mux mode is active with the pane list board and agent
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    enable_mux(&mut nav, &[MuxPaneKind::Board, MuxPaneKind::Agent]);
    // @step And one agent session is open
    seed_sessions(&mut agent, 1);
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    nav.mux.set_focus(1);
    render(&mut nav, &board, &mut agent); // spinner active (Loading)
                                          // @step And the agent session just became idle
    set_status(&mut agent, &sid(1), SessionStatus::Idle);
    // @step And the board pane is focused
    nav.mux.set_focus(0);
    // one render: the per-session sweep enters Hiding
    let _ = render(&mut nav, &board, &mut agent);
    // @step When the redraw gate is evaluated while the agent session's finish sweep is in progress
    let any_animating = agent.any_session_transition_animating();
    // @step Then the redraw gate stays open even though no session is busy
    assert!(
        !agent.any_session_busy(),
        "no session is Running/Compacting anymore"
    );
    assert!(
        any_animating,
        "BUG-194: the redraw gate input must see the UNFOCUSED pane's in-flight sweep"
    );
    assert!(
        codelet_fspec_tui::app::tick_should_draw(false, false, true, false, false),
        "the gate fn stays open while any transition is mid-sweep"
    );
    // @step When the grid is rendered repeatedly
    render_until(&mut nav, &board, &mut agent, |store| {
        matches!(
            store
                .session_context_for(&sid(1))
                .unwrap()
                .input_transition
                .state,
            InputTransitionState::Idle
        )
    });
    // @step Then the agent pane finish sweep advances and ends with the agent pane showing its ghost draft
    let buf = render(&mut nav, &board, &mut agent);
    let rects = nav.mux.pane_rects().to_vec();
    let agent_pane = pane_lines(&buf, rects[1]);
    assert!(
        pane_contains(&agent_pane, "Type a message"),
        "the sweep must complete down to the ghost draft row: {agent_pane:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: single-view mode keeps its existing spinner and finish-sweep
// behavior
// ─────────────────────────────────────────────────────────────────────────

/// Feature: spec/features/mux-thinking-indicator-disappears-when-focus-moves-off-a-running-agent-pane.feature
#[test]
fn single_view_mode_keeps_its_existing_spinner_and_finish_sweep_behavior() {
    // @step Given mux mode is inactive
    let (mut nav, _rx) = fresh();
    let board = BoardStore::default();
    let mut agent = AgentViewStore::default();
    nav.active_view = ViewMode::Agent;
    // @step And one agent session is open and running
    seed_sessions(&mut agent, 1);
    set_status(&mut agent, &sid(1), SessionStatus::Running);
    // @step When the agent view is rendered
    let buf = render(&mut nav, &board, &mut agent);
    // @step Then the agent input row shows the live thinking spinner exactly as before
    assert!(
        pane_contains(
            &pane_lines(&buf, Rect::new(0, 0, 120, 24)),
            "Thinking... (Esc to stop)"
        ),
        "single-view spinner unchanged"
    );
    // @step When the agent session becomes idle
    set_status(&mut agent, &sid(1), SessionStatus::Idle);
    // @step And the agent view is rendered
    let buf = render(&mut nav, &board, &mut agent);
    // @step Then the agent input row plays the finish sweep exactly as before
    assert!(
        pane_contains(&pane_lines(&buf, Rect::new(0, 0, 120, 24)), "(Esc to stop)"),
        "single-view finish sweep unchanged (full captured line on frame 0)"
    );
    render_until(&mut nav, &board, &mut agent, |store| {
        matches!(
            store
                .session_context_for(&sid(1))
                .unwrap()
                .input_transition
                .state,
            InputTransitionState::Idle
        )
    });
    let buf = render(&mut nav, &board, &mut agent);
    assert!(
        pane_contains(
            &pane_lines(&buf, Rect::new(0, 0, 120, 24)),
            INPUT_PLACEHOLDER_HINT,
        ),
        "the sweep ends at the live composer"
    );
}
