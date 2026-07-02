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

use super::input_transition::paint_input_or_spinner;
use super::pause_prompt;
use super::{multiline_input_render, transition_driver, AgentView};

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
}
