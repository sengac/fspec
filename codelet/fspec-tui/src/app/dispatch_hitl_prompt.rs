//! RPC-411 — App::dispatch reducers for the inline HITL prompt.
//!
//! Feature: spec/features/inline-hitl-prompt.feature
//!
//! Store-authoritative transitions over the per-session HITL slot
//! (`store/agent_view/hitl_state.rs`), routed from the catch-all arm
//! of `dispatch_pause_hitl.rs::try_dispatch_pause_hitl`. The cancel
//! path ALWAYS sends `{cancelled:true, answers:[]}` through
//! `handle_hitl_submitted` before the slot clears — no code path may
//! dismiss the HITL UI without submitting or cancelling (the RPC-053
//! modal's silent Esc-pop stranded the backend Paused forever).

use codelet_rpc_types::{HitlAnswer, HitlResponse, SessionId};

use crate::components::Action;
use crate::store::agent_view::hitl_state::HitlAdvance;

use super::state::App;

impl App {
    /// RPC-411: fold a fetched HitlRequest into the per-session store
    /// slot (fresh machine state). The AgentView paints the inline
    /// prompt from this slot on the next frame.
    pub(crate) fn handle_hitl_prompt_fetched(
        &mut self,
        session_id: SessionId,
        request: codelet_rpc_types::HitlRequest,
    ) {
        self.agent_view_store.set_hitl_prompt(session_id, request);
        self.should_render = true;
    }

    /// RPC-411: ↑/↓ wrap-around over options + the virtual "Other...".
    pub(crate) fn handle_hitl_prompt_nav(&mut self, session_id: &SessionId, delta: i32) {
        if let Some(state) = self.agent_view_store.hitl_prompt_for_mut(session_id) {
            state.cycle_selection(delta);
            self.should_render = true;
        }
    }

    /// RPC-411: Enter on an options question. Reads the authoritative
    /// slot: Other... selected → enter Other mode; else capture
    /// `{id, selected:[label]}` and advance-or-submit.
    pub(crate) fn handle_hitl_prompt_enter(&mut self, session_id: SessionId) {
        let Some(state) = self.agent_view_store.hitl_prompt_for_mut(&session_id) else {
            return;
        };
        if state.other_selected() {
            state.other_active = true;
            state.show_empty_hint = false;
            self.should_render = true;
            return;
        }
        let answer = match state.current_question() {
            Some(q) => {
                let label = q
                    .options
                    .get(state.selected_option)
                    .map(|o| o.label.clone());
                match label {
                    Some(label) => HitlAnswer {
                        id: q.id.clone(),
                        selected: vec![label],
                        other: None,
                    },
                    None => return,
                }
            }
            None => return,
        };
        self.hitl_advance_or_submit(session_id, answer);
    }

    /// RPC-411: freeform/Other Enter with non-empty text — the key
    /// handler already read + cleared the SHARED composer input.
    /// Captures `{id, selected:[], other:text}` and advance-or-submits.
    pub(crate) fn handle_hitl_answer_captured(&mut self, session_id: SessionId, text: String) {
        let Some(state) = self.agent_view_store.hitl_prompt_for_mut(&session_id) else {
            return;
        };
        let answer = match state.current_question() {
            Some(q) => HitlAnswer {
                id: q.id.clone(),
                selected: vec![],
                other: Some(text),
            },
            None => return,
        };
        self.hitl_advance_or_submit(session_id, answer);
    }

    /// Shared advance-or-submit tail: on the last question send ONE
    /// `{cancelled:false, answers}` response (fire-and-forget) — the
    /// slot clears inside `handle_hitl_submitted`.
    fn hitl_advance_or_submit(&mut self, session_id: SessionId, answer: HitlAnswer) {
        let Some(state) = self.agent_view_store.hitl_prompt_for_mut(&session_id) else {
            return;
        };
        match state.advance_or_submit(answer) {
            HitlAdvance::Advanced => {
                self.should_render = true;
            }
            HitlAdvance::Submit(answers) => {
                self.handle_hitl_submitted(
                    session_id,
                    HitlResponse {
                        cancelled: false,
                        answers,
                    },
                );
            }
        }
    }

    /// RPC-411: Esc in Other mode — LOCAL only. Back to the options
    /// list, hint cleared. The key handler already cleared the shared
    /// input value. NOTHING is sent.
    pub(crate) fn handle_hitl_other_exit(&mut self, session_id: &SessionId) {
        if let Some(state) = self.agent_view_store.hitl_prompt_for_mut(session_id) {
            state.other_active = false;
            state.show_empty_hint = false;
            self.should_render = true;
        }
    }

    /// RPC-411: empty/whitespace freeform Enter — set the yellow hint.
    pub(crate) fn handle_hitl_empty_submit(&mut self, session_id: &SessionId) {
        if let Some(state) = self.agent_view_store.hitl_prompt_for_mut(session_id) {
            state.show_empty_hint = true;
            self.should_render = true;
        }
    }

    /// RPC-411: typing while the hint shows — clear it (TS
    /// useHitlInput.ts:201-208, 255-258).
    pub(crate) fn handle_hitl_hint_cleared(&mut self, session_id: &SessionId) {
        if let Some(state) = self.agent_view_store.hitl_prompt_for_mut(session_id) {
            if state.show_empty_hint {
                state.show_empty_hint = false;
                self.should_render = true;
            }
        }
    }

    /// RPC-411: Esc outside Other mode — cancel the WHOLE request:
    /// `{cancelled:true, answers:[]}` is SENT through
    /// `handle_hitl_submitted` (which clears the slot). The composer
    /// draft is left untouched.
    pub(crate) fn handle_hitl_cancelled(&mut self, session_id: SessionId) {
        if self.agent_view_store.hitl_prompt_for(&session_id).is_none() {
            return;
        }
        self.handle_hitl_submitted(
            session_id,
            HitlResponse {
                cancelled: true,
                answers: vec![],
            },
        );
    }

    /// Route the RPC-411 Action variants through their reducers.
    /// Called from the tail of `try_dispatch_pause_hitl`.
    pub(crate) fn try_dispatch_hitl_prompt(&mut self, action: &Action) -> bool {
        match action {
            Action::HitlPromptFetched {
                session_id,
                request,
            } => {
                self.handle_hitl_prompt_fetched(session_id.clone(), request.clone());
            }
            Action::HitlPromptNav { session_id, delta } => {
                self.handle_hitl_prompt_nav(session_id, *delta);
            }
            Action::HitlPromptEnter { session_id } => {
                self.handle_hitl_prompt_enter(session_id.clone());
            }
            Action::HitlAnswerCaptured { session_id, text } => {
                self.handle_hitl_answer_captured(session_id.clone(), text.clone());
            }
            Action::HitlOtherExit { session_id } => {
                self.handle_hitl_other_exit(session_id);
            }
            Action::HitlEmptySubmit { session_id } => {
                self.handle_hitl_empty_submit(session_id);
            }
            Action::HitlHintCleared { session_id } => {
                self.handle_hitl_hint_cleared(session_id);
            }
            Action::HitlCancelled { session_id } => {
                self.handle_hitl_cancelled(session_id.clone());
            }
            _ => return false,
        }
        true
    }
}
