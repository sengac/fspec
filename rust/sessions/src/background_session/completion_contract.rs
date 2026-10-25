use super::*;

impl BackgroundSession {
    pub fn get_continue_state(&self) -> (bool, u32) {
        (
            self.continue_enabled.load(Ordering::Acquire),
            self.continue_budget.load(Ordering::Acquire),
        )
    }

    pub fn set_continue_state(&self, enabled: bool, budget: u32) {
        self.continue_explicit.store(true, Ordering::Release);
        self.continue_enabled.store(enabled, Ordering::Release);
        self.continue_budget.store(budget.max(1), Ordering::Release);
    }

    pub fn inherit_continue_state(&self, enabled: bool, budget: u32) {
        if !self.continue_explicit.load(Ordering::Acquire) {
            self.continue_enabled.store(enabled, Ordering::Release);
            self.continue_budget.store(budget.max(1), Ordering::Release);
        }
    }

    pub fn get_goal_state(&self) -> Option<(String, Option<String>)> {
        self.goal_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn set_goal_state(&self, goal: Option<(String, Option<String>)>) {
        let mut guard = self
            .goal_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.goal_generation.fetch_add(1, Ordering::AcqRel);
        *guard = goal;
    }

    /// CONT-008: engine-side goal-satisfied write-back, called by BOTH
    /// BackgroundOutput twins when they map a
    /// `ContinueStateReason::GoalSatisfied` snapshot (the shared done()
    /// teardown cleared an active goal on the inner session).
    ///
    /// Clears the chrome goal state ONLY when the goal generation is
    /// unchanged since the last dispatch sync — i.e. the chrome copy is
    /// exactly the goal the engine just satisfied. A mid-turn `/goal`
    /// replacement (generation bumped) WINS and survives untouched; the
    /// next dispatch sync applies it to the inner session.
    ///
    /// The clear intentionally does NOT bump the generation: the chrome
    /// and inner copies are both `None` afterwards, so the next dispatch
    /// sync correctly skips the goal apply.
    ///
    /// TIMING CONTRACT (delayed-write-back race): correctness relies on
    /// the `GoalSatisfied` emission being SYNCHRONOUS while the twin holds
    /// the inner-session lock — i.e. this write-back runs serialized
    /// against `sync_completion_contract_for_user_turn(&mut inner)`, so a
    /// `/goal` replacement is either (a) applied before the turn (generation
    /// consumed → generations equal → the engine satisfies THAT goal and the
    /// clear is correct) or (b) still pending (generation bumped → the check
    /// fails → the replacement survives). If the emission ever becomes
    /// async-queued (processed after the twin releases the lock), a LATE
    /// clear could observe an already-re-synced replacement goal with equal
    /// generations and wrongly wipe it — do not decouple the emission from
    /// the locked turn without revisiting this guard.
    pub fn clear_goal_state_if_unchanged_since_sync(&self) {
        let mut guard = self
            .goal_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let generation = self.goal_generation.load(Ordering::Acquire);
        let synced = self.goal_synced_generation.load(Ordering::Acquire);
        if generation == synced {
            *guard = None;
        }
    }

    pub fn sync_completion_contract_for_user_turn(
        &self,
        inner_session: &mut codelet_cli::session::Session,
    ) {
        self.sync_completion_contract(inner_session, true);
    }

    pub fn sync_completion_contract_for_recovery(
        &self,
        inner_session: &mut codelet_cli::session::Session,
    ) {
        self.sync_completion_contract(inner_session, false);
    }

    fn sync_completion_contract(
        &self,
        inner_session: &mut codelet_cli::session::Session,
        genuine_user_input: bool,
    ) {
        let (continue_enabled, continue_budget) = self.get_continue_state();
        inner_session.continue_enabled = continue_enabled;
        inner_session.continue_budget = continue_budget;
        if genuine_user_input {
            codelet_cli::interactive::auto_continue::reset_for_new_user_turn(inner_session);
        }
        let generation = self.goal_generation.load(Ordering::Acquire);
        let generation_changed = generation != self.goal_synced_generation.load(Ordering::Acquire);
        let chrome_goal = self.get_goal_state();
        let inner_goal_pair = inner_session
            .goal
            .as_ref()
            .map(|g| (g.text.clone(), g.verify.clone()));
        if generation_changed && chrome_goal != inner_goal_pair {
            match &chrome_goal {
                Some((text, verify)) => inner_session.set_goal(text, verify.as_deref()),
                None => inner_session.clear_goal(),
            }
        }
        self.goal_synced_generation
            .store(generation, Ordering::Release);
        let goal_spec = inner_session
            .goal
            .as_ref()
            .map(|g| codelet_tools::GoalSpec {
                text: g.text.clone(),
                verify: g.verify.clone(),
            });
        codelet_tools::set_continue_armed(self.id, continue_enabled || goal_spec.is_some());
        if codelet_tools::get_session_goal(self.id) != goal_spec {
            codelet_tools::set_session_goal(self.id, goal_spec);
        }
    }
}
