//! RPC-406 — input-area painting for the AgentView, extracted from
//! `views/agent.rs` so the orchestrator stays under its 300-LoC
//! source-shape ceiling.
//!
//! Feature: spec/features/inline-tool-approval-pause-prompt.feature
//!
//! `paint_input_area` consults the FOCUSED session's pause slot BEFORE
//! `paint_input_or_spinner`: when the slot is `Some`, the inline
//! tool-approval prompt replaces the MultiLineInput / spinner rendering
//! entirely (TS parity — `InputTransition.tsx:467-533` early-returns the
//! pause UI). The `MultiLineInput`'s TextArea state is NEVER touched
//! while swapped out, so the user's draft text and cursor survive the
//! pause round-trip untouched (tui-textarea keeps state fully separate
//! from its render pass).
//!
//! `input_area_height` feeds the RPC-405 auto-grow layout seam: the
//! input row is `prompt_height(state, width)` rows while paused (the
//! wrapped prompt header plus its options / details / Y-N rows, at the
//! padded body width the prompt paints into) and the wrap-aware draft
//! height otherwise.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use codelet_rpc_types::SessionId;

use crate::store::AgentViewStore;

use super::busy_composer::BusyKind;
use super::hitl_prompt;
use super::input_transition::{paint_input_or_spinner_with_waiting, InputTransitionState};
use super::pane_composer::{paint_missing_session, pane_composer, PaneComposer};
use super::pause_prompt;
use super::stdin_answer_box::{below_notice, notice_rows, offset_below, paint_notice};
use super::{multiline_input_render, transition_driver, AgentView, INPUT_PLACEHOLDER_HINT};

mod relayed_prompt; // AMGR-022

/// RPC-406 — the padded body width the pause prompt renders into:
/// the input-area width minus the 1-col pad on each side (mirrors
/// `paint_input_area`'s `padded` rect).
fn pause_body_width(area_width: u16) -> u16 {
    let pad = area_width.min(1);
    area_width.saturating_sub(pad * 2)
}

/// BUG-194: cache the spinner line in the session's OWN slot (the Hiding
/// transition captures this pane's last painted line, not another pane's).
fn cache_spinner_line(store: &mut AgentViewStore, sid: Option<&SessionId>) {
    let Some(ctx) = sid.and_then(|s| store.session_context_mut_for(s)) else {
        return;
    };
    if let Some(line) = transition_driver::cached_spinner_line(&ctx.input_transition.state) {
        ctx.input_transition.last_spinner_line = Some(line);
    }
}

impl AgentView {
    /// RPC-405/RPC-406 — input-row height for the layout split:
    /// the pause prompt's wrapped height when the focused session is
    /// paused, otherwise the wrap-aware draft height. `area_width` is
    /// the FULL input-area width; each consumer derives its own body
    /// width from it (pause prompt: minus side pads; draft: minus side
    /// pads and the "> " prompt).
    pub(super) fn input_area_height(
        &self,
        store: &AgentViewStore,
        sid: Option<&SessionId>,
        area_width: u16,
    ) -> u16 {
        // RPC-411: the HITL slot wins over everything. TUI-116: the
        // focused stdin answer box wins over the pause slot; the stdin
        // notice adds its rows on top of the composer or busy stack
        // (TUI-131: the notice wraps, so it may take more than one).
        if let Some(state) = sid.and_then(|s| store.hitl_prompt_for(s)) {
            return hitl_prompt::prompt_height(state, pause_body_width(area_width), &self.input);
        }
        let stdin = sid.and_then(|s| store.exec_stdin_for(s).map(|request| (s, request)));
        if let Some(height) = stdin
            .and_then(|(s, request)| self.focused_stdin_box_height(s, &request.command, area_width))
        {
            return height;
        }
        if let Some(state) = sid.and_then(|s| store.pause_state_for(s)) {
            return pause_prompt::prompt_height(state, pause_body_width(area_width));
        }
        let width = pause_body_width(area_width);
        if let Some(height) = relayed_prompt::relayed_height(store, sid, width, &self.input) {
            return height;
        }
        let notice_rows = stdin.map_or(0, |(s, request)| {
            let probe_error = store.exec_stdin_probe_error(s);
            notice_rows(&request.command, probe_error, width)
        }) + relayed_prompt::waiting_rows(store, sid, width);
        // TUI-115: a busy session stacks activity + queued + composer + hint.
        let rows = if BusyKind::of(store, sid).is_some() {
            self.busy_input_height(store, sid, area_width)
        } else {
            self.input
                .visible_rows_for_width(multiline_input_render::input_body_width(area_width))
        };
        rows.saturating_add(notice_rows)
    }

    /// Paint the input row: inline pause prompt (RPC-406) OR the
    /// spinner / transition slice / MultiLineInput (RPC-093/095).
    /// Caches `last_pause` for the key router and the cursor gate.
    /// BUG-194: the spinner/transition row is the FOCUSED session's
    /// OWN per-session slot (store-owned) — `store` is `&mut` so the
    /// slot's `last_spinner_line` cache updates in place.
    pub(super) fn paint_input_area(
        &mut self,
        input_area: Rect,
        buf: &mut Buffer,
        store: &mut AgentViewStore,
        sid: Option<&SessionId>,
    ) {
        // RPC-029: input has no border; paddingX=1.
        let pad = input_area.width.min(1);
        let padded = Rect {
            x: input_area.x + pad,
            y: input_area.y,
            width: input_area.width.saturating_sub(pad * 2),
            height: input_area.height,
        };
        // RPC-412: clear any stale freeform cursor offset — only the
        // freeform HITL branch below re-sets it. Normal composer,
        // options-mode HITL and pause prompts all leave it None.
        self.last_hitl_input_offset = None;
        // BUG-377: the focused session's OWN transition state (BUG-194);
        // a pane naming a session that is not open says so instead.
        let state = match pane_composer(store, sid) {
            PaneComposer::NoSession => InputTransitionState::Idle,
            PaneComposer::Open(context) => context.input_transition.state.clone(),
            PaneComposer::Missing(id) => {
                paint_missing_session(padded, buf, id);
                self.clear_prompt_caches();
                return;
            }
        };
        // RPC-411: the focused session's HITL slot wins over the pause
        // slot, the MultiLineInput AND the spinner (TS early-return
        // order — InputTransition.tsx:385-388).
        if let Some((session, state)) = sid.and_then(|s| store.hitl_prompt_for(s).map(|h| (s, h))) {
            if state.freeform_active() {
                // The shared input paints below the header — keep its
                // viewport in sync (prompt "> " geometry).
                let input_body_width = multiline_input_render::input_body_width(input_area.width);
                self.input
                    .sync_viewport(input_body_width, input_area.height);
            }
            // RPC-412: capture the freeform header offset (row where the
            // "> " input line is painted); `None` in options mode.
            self.last_hitl_input_offset =
                hitl_prompt::render_hitl_prompt(padded, buf, state, &self.input);
            self.last_hitl = Some((
                session.clone(),
                if state.freeform_active() {
                    super::hitl_keys::HitlKeyMode::Freeform {
                        other: state.other_active,
                        hint: state.show_empty_hint,
                    }
                } else {
                    super::hitl_keys::HitlKeyMode::Options
                },
            ));
            self.last_pause = None;
            return;
        }
        self.last_hitl = None;
        // TUI-116: the focused session's exec-stdin request paints the
        // focused answer box (over the pause slot), or a notice row on
        // top of the composer. Pure overlay — the session status is NOT
        // flipped, so the agent keeps streaming.
        let notice = match sid.and_then(|s| store.exec_stdin_for(s).map(|r| (s, r))) {
            Some((session, request)) => {
                self.last_exec_stdin = Some((session.clone(), request.exec_session_id.clone()));
                if self.stdin_box.is_focused_for(session) {
                    self.paint_stdin_box(input_area, padded, buf, session, request);
                    self.last_pause = None;
                    return;
                }
                let probe_error = store.exec_stdin_probe_error(session).map(str::to_string);
                Some((request.command.clone(), probe_error))
            }
            None => {
                self.last_exec_stdin = None;
                None
            }
        };
        // RPC-406: the focused session's pause slot wins over the
        // MultiLineInput AND the spinner (TS early-return order).
        if let Some((session, state)) = sid.and_then(|s| store.pause_state_for(s).map(|p| (s, p))) {
            let selection = store.triple_pause_selection_for(session);
            pause_prompt::render_pause_prompt(padded, buf, state, selection);
            self.last_pause = Some((session.clone(), state.kind));
            return;
        }
        self.last_pause = None;
        if self.paint_relayed_prompt(input_area, padded, buf, store, sid) {
            return;
        }
        let notice_rows = notice.map_or(0, |(command, probe_error)| {
            paint_notice(padded, buf, &command, probe_error.as_deref())
        });
        let notice_rows = notice_rows
            + relayed_prompt::paint_waiting(below_notice(padded, notice_rows), buf, store, sid);
        let (input_area, padded) = (
            below_notice(input_area, notice_rows),
            below_notice(padded, notice_rows),
        );
        // TUI-115: a busy session keeps the composer under the activity row.
        if let Some(kind) = BusyKind::of(store, sid) {
            self.paint_busy_input_area(input_area, padded, buf, store, sid, kind);
            self.last_hitl_input_offset = offset_below(self.last_hitl_input_offset, notice_rows);
            cache_spinner_line(store, sid);
            return;
        }
        // RPC-405: cursor-follow + clamp before the immutable paint.
        let input_body_width = multiline_input_render::input_body_width(input_area.width);
        self.input
            .sync_viewport(input_body_width, input_area.height);
        let waiting = sid
            .and_then(|session| store.session_context_for(session))
            .and_then(|context| context.provider_waiting.as_deref());
        paint_input_or_spinner_with_waiting(padded, buf, &self.input, &state, waiting);
        self.last_hitl_input_offset = offset_below(self.last_hitl_input_offset, notice_rows);
        cache_spinner_line(store, sid);
    }

    /// BUG-163 + BUG-194 — ghost input row for UNfocused mux agent
    /// panes. BUG-194: the pane's OWN session transition slot drives
    /// the row — a non-Idle slot (Loading/Compacting/Hiding/Showing)
    /// paints the SAME spinner/finish-sweep row the focused pane
    /// paints; a slot at Idle falls back to the read-only ghost draft
    /// (BUG-163: the ghost never touches the shared `MultiLineInput`).
    pub(super) fn paint_ghost_input_row(
        &mut self,
        input_area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        store: &mut AgentViewStore,
        sid: Option<&SessionId>,
    ) {
        // Mirror `paint_input_area`'s padding (RPC-029 paddingX=1).
        let pad = input_area.width.min(1);
        let padded = Rect {
            x: input_area.x + pad,
            y: input_area.y,
            width: input_area.width.saturating_sub(pad * 2),
            height: input_area.height.max(1),
        };
        // BUG-194: this pane's session's OWN transition state (ticked
        // earlier in `render_session_pane` from its own status). BUG-377:
        // a pane naming a session that is not open says so instead.
        let (state, draft) = match pane_composer(store, sid) {
            PaneComposer::NoSession => (InputTransitionState::Idle, String::new()),
            PaneComposer::Open(context) => (
                context.input_transition.state.clone(),
                context.input_draft.clone(),
            ),
            PaneComposer::Missing(id) => {
                paint_missing_session(padded, buf, id);
                self.clear_prompt_caches();
                return;
            }
        };
        if !matches!(state, InputTransitionState::Idle) {
            // The running/compacting/finishing pane paints the
            // identical indicator row as its focused twin.
            let waiting = sid
                .and_then(|session| store.session_context_for(session))
                .and_then(|context| context.provider_waiting.as_deref());
            paint_input_or_spinner_with_waiting(padded, buf, &self.input, &state, waiting);
            // Cache the spinner line in this pane's session slot (the
            // Hiding transition captures the line THIS pane painted
            // last).
            if let Some(line) = transition_driver::cached_spinner_line(&state) {
                if let Some(ctx) = sid.and_then(|s| store.session_context_mut_for(s)) {
                    ctx.input_transition.last_spinner_line = Some(line);
                }
            }
        } else {
            self.input
                .render_ghost_draft(padded, buf, &draft, INPUT_PLACEHOLDER_HINT);
        }
        // BUG-163: no pause/HITL prompt is painted in a ghost pane — the
        // focused pane's `paint_input_area` owns those caches. TOOL-022 P2:
        // the exec-stdin overlay is likewise focused-pane-only.
        self.clear_prompt_caches();
    }

    /// The pause, HITL and exec-stdin prompt caches, cleared when no
    /// prompt of the pane's session is painted.
    fn clear_prompt_caches(&mut self) {
        self.last_pause = None;
        self.last_hitl = None;
        self.last_exec_stdin = None;
    }
}
