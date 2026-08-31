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

use super::hitl_prompt;
use super::input_transition::paint_input_or_spinner;
use super::pause_prompt;
use super::{multiline_input_render, transition_driver, AgentView, INPUT_PLACEHOLDER_HINT};

/// RPC-406 — the padded body width the pause prompt renders into:
/// the input-area width minus the 1-col pad on each side (mirrors
/// `paint_input_area`'s `padded` rect).
fn pause_body_width(area_width: u16) -> u16 {
    let pad = area_width.min(1);
    area_width.saturating_sub(pad * 2)
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
        // RPC-411: the HITL slot wins over the pause slot (TS
        // InputTransition.tsx:385-388 priority).
        if let Some(state) = sid.and_then(|s| store.hitl_prompt_for(s)) {
            return hitl_prompt::prompt_height(state, pause_body_width(area_width), &self.input);
        }
        if let Some(state) = sid.and_then(|s| store.pause_state_for(s)) {
            return pause_prompt::prompt_height(state, pause_body_width(area_width));
        }
        self.input
            .visible_rows_for_width(multiline_input_render::input_body_width(area_width))
    }

    /// Paint the input row: inline pause prompt (RPC-406) OR the
    /// spinner / transition slice / MultiLineInput (RPC-093/095).
    /// Caches `last_pause` for the key router and the cursor gate.
    pub(super) fn paint_input_area(
        &mut self,
        input_area: Rect,
        buf: &mut Buffer,
        store: &AgentViewStore,
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
        // RPC-406: the focused session's pause slot wins over the
        // MultiLineInput AND the spinner (TS early-return order).
        if let Some((session, state)) = sid.and_then(|s| store.pause_state_for(s).map(|p| (s, p))) {
            let selection = store.triple_pause_selection_for(session);
            pause_prompt::render_pause_prompt(padded, buf, state, selection);
            self.last_pause = Some((session.clone(), state.kind));
            return;
        }
        self.last_pause = None;
        // RPC-405: cursor-follow + clamp before the immutable paint.
        let input_body_width = multiline_input_render::input_body_width(input_area.width);
        self.input
            .sync_viewport(input_body_width, input_area.height);
        paint_input_or_spinner(padded, buf, &self.input, &self.input_transition_state);
        if let Some(line) = transition_driver::cached_spinner_line(&self.input_transition_state) {
            self.last_spinner_line = Some(line);
        }
    }

    /// BUG-163 — read-only ghost input row for UNfocused mux agent
    /// panes: paints the session's persisted `input_draft` (or the dim
    /// placeholder hint when empty) into the same padded input-area
    /// geometry the live composer uses, WITHOUT touching the shared
    /// `MultiLineInput` / viewport state. The focused pane is the only
    /// one that paints the live composer.
    pub(super) fn paint_ghost_input_row(
        &mut self,
        input_area: Rect,
        buf: &mut ratatui::buffer::Buffer,
        store: &AgentViewStore,
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
        let draft = sid
            .and_then(|s| store.session_context_for(s))
            .map(|c| c.input_draft.clone())
            .unwrap_or_default();
        self.input
            .render_ghost_draft(padded, buf, &draft, INPUT_PLACEHOLDER_HINT);
        // BUG-163: no pause/HITL prompt is painted in a ghost pane — the
        // focused pane's `paint_input_area` owns those caches.
        self.last_pause = None;
        self.last_hitl = None;
    }
}
