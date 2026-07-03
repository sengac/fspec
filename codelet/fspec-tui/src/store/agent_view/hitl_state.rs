//! RPC-411 — per-session inline HITL prompt slot held by
//! `AgentViewStore`.
//!
//! Feature file: spec/features/inline-hitl-prompt.feature
//!
//! Replaces the RPC-053 Critical modal state: when the chunk-driven
//! fetcher (`app/dispatch_pause_hitl.rs::handle_pause_chunk`) resolves
//! a `Some(HitlRequest)` it dispatches `Action::HitlPromptFetched`,
//! whose reducer writes the state here. The AgentView paints the
//! inline prompt from this slot only when the session is FOCUSED, and
//! the HITL slot wins over the pause slot (TS parity —
//! `InputTransition.tsx:385-388`).
//!
//! `HitlPromptState` is a faithful port of the TS `useHitlInput`
//! machine (`src/tui/hooks/useHitlInput.ts:134-262`): one question at
//! a time, wrap-around selection over `options.len() + 1` (the virtual
//! "Other..." entry), advance-or-submit answer accumulation, an
//! `other_active` freeform sub-mode, and a `show_empty_hint` flag for
//! rejected empty submissions. All state resets when the slot clears.

use std::collections::HashMap;

use codelet_rpc_types::{HitlAnswer, HitlQuestion, HitlRequest, SessionId};

use super::AgentViewStore;

/// What `advance_or_submit` decided: keep prompting or submit all
/// accumulated answers (the caller sends ONE `HitlResponse`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitlAdvance {
    /// More questions remain — index advanced, selection reset.
    Advanced,
    /// The last question was answered — submit these answers.
    Submit(Vec<HitlAnswer>),
}

/// Per-session inline HITL prompt state: the wire request plus the
/// `useHitlInput` machine state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitlPromptState {
    /// The full wire request (RPC-410: 1–3 ordered questions).
    pub request: HitlRequest,
    /// Index of the question currently prompted (0-based).
    pub question_index: usize,
    /// Options-list selection; `options.len()` = virtual "Other...".
    pub selected_option: usize,
    /// Answers accumulated so far (one per answered question).
    pub answers: Vec<HitlAnswer>,
    /// True while the virtual "Other..." freeform sub-mode is active
    /// on an options question.
    pub other_active: bool,
    /// True after an empty/whitespace freeform submit was rejected;
    /// cleared by typing or leaving the mode.
    pub show_empty_hint: bool,
}

impl HitlPromptState {
    pub fn new(request: HitlRequest) -> Self {
        Self {
            request,
            question_index: 0,
            selected_option: 0,
            answers: Vec::new(),
            other_active: false,
            show_empty_hint: false,
        }
    }

    /// The question currently prompted (`None` past the end — should
    /// not happen while the slot is live).
    pub fn current_question(&self) -> Option<&HitlQuestion> {
        self.request.questions.get(self.question_index)
    }

    /// True when the CURRENT question has options (radio list mode
    /// unless `other_active`); false = pure freeform question.
    pub fn has_options(&self) -> bool {
        self.current_question()
            .map(|q| !q.options.is_empty())
            .unwrap_or(false)
    }

    /// True while the SHARED composer input is live: pure freeform
    /// question, or the "Other..." sub-mode of an options question.
    pub fn freeform_active(&self) -> bool {
        !self.has_options() || self.other_active
    }

    /// True when the selection sits on the virtual "Other..." entry.
    pub fn other_selected(&self) -> bool {
        self.current_question()
            .map(|q| self.selected_option == q.options.len())
            .unwrap_or(false)
    }

    /// ↑/↓ wrap-around over `options.len() + 1` items (the virtual
    /// "Other..." is always appended — useHitlInput.ts:210-222).
    pub fn cycle_selection(&mut self, delta: i32) {
        let total = self
            .current_question()
            .map(|q| q.options.len() + 1)
            .unwrap_or(1) as i32;
        let cur = self.selected_option as i32;
        self.selected_option = (cur + delta).rem_euclid(total.max(1)) as usize;
    }

    /// Append `answer` and either advance to the next question
    /// (selection reset, Other mode exited, hint cleared) or return
    /// the full answer set for submission (useHitlInput.ts:134-151).
    pub fn advance_or_submit(&mut self, answer: HitlAnswer) -> HitlAdvance {
        self.answers.push(answer);
        if self.question_index + 1 < self.request.questions.len() {
            self.question_index += 1;
            self.selected_option = 0;
            self.other_active = false;
            self.show_empty_hint = false;
            HitlAdvance::Advanced
        } else {
            HitlAdvance::Submit(std::mem::take(&mut self.answers))
        }
    }
}

/// Slot map type held by [`AgentViewStore`].
pub type HitlPromptBySession = HashMap<SessionId, HitlPromptState>;

impl AgentViewStore {
    /// Read the active HITL prompt state for `session`. `None` when no
    /// request is pending (or it was answered / cancelled / cleared).
    pub fn hitl_prompt_for(&self, session: &SessionId) -> Option<&HitlPromptState> {
        self.hitl_prompt_by_session.get(session)
    }

    /// Mutable access for the App reducers (store-authoritative
    /// machine transitions).
    pub fn hitl_prompt_for_mut(&mut self, session: &SessionId) -> Option<&mut HitlPromptState> {
        self.hitl_prompt_by_session.get_mut(session)
    }

    /// Persist a fetched [`HitlRequest`] for `session`, resetting the
    /// machine state (fresh request = fresh flow).
    pub fn set_hitl_prompt(&mut self, session: SessionId, request: HitlRequest) {
        self.hitl_prompt_by_session
            .insert(session, HitlPromptState::new(request));
    }

    /// Drop the HITL slot for `session` — called after submit/cancel
    /// (the response has ALREADY been sent) and when a Running/Idle
    /// chunk clears the pause server-side. All machine state resets
    /// with the slot.
    pub fn clear_hitl_prompt(&mut self, session: &SessionId) {
        self.hitl_prompt_by_session.remove(session);
    }
}
