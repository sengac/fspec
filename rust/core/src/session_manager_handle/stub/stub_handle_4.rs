macro_rules! compose_stub_handle_4 {
    ($($collected:tt)*) => {
        compose_stub_handle_finish! {
            $($collected)*
    fn merge_session_worktree(
        &self,
        _session_id: &SessionId,
        _strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        self.merge_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.merge_outcome, "merge_outcome");
            Ok(g.clone())
        }
    }
    fn discard_session_worktree(&self, _session_id: &SessionId) -> Result<(), String> {
        self.discard_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String> {
        self.prune_orphaned_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.pruned_sessions, "pruned_sessions");
            Ok(g.clone())
        }
    }
    fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>, String> {
        Ok({
        self.list_session_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.session_worktrees, "session_worktrees");
            g.clone()
        }
    })
    }
    fn inspect_session_changes(
        &self,
        _session_id: &SessionId,
    ) -> Result<SessionChangesSummary, String> {
        self.inspect_session_changes_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.session_changes_summary, "session_changes_summary");
            Ok(g.clone())
        }
    }
    fn detach_session_worktree(&self, _session_id: &SessionId) -> Result<(), String> {
        self.detach_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule stub impls.
    // ─────────────────────────────────────────────────────────────────

    fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob, String> {
        self.schedule_add_calls.fetch_add(1, Ordering::SeqCst);
        let g = stub_lock(&self.scheduled_jobs, "scheduled_jobs");
            if let Some(first) = g.first() {
                Ok(first.clone())
            } else {
                Ok(job)
            }
    }
    fn schedule_list(&self) -> Result<Vec<ScheduledJob>, String> {
        Ok({
        self.schedule_list_calls.fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.scheduled_jobs, "scheduled_jobs");
            g.clone()
        }
    })
    }
    fn schedule_pause(&self, name: &str) -> Result<ScheduledJob, String> {
        self.schedule_pause_calls.fetch_add(1, Ordering::SeqCst);
        let g = stub_lock(&self.scheduled_jobs, "scheduled_jobs");
            if let Some(first) = g.first() {
                Ok(first.clone())
            } else {
                Ok(ScheduledJob {
                    name: name.to_string(),
                    status: "paused".to_string(),
                    ..ScheduledJob::default()
                })
            }
    }
    fn schedule_resume(&self, name: &str) -> Result<ScheduledJob, String> {
        self.schedule_resume_calls.fetch_add(1, Ordering::SeqCst);
        let g = stub_lock(&self.scheduled_jobs, "scheduled_jobs");
            if let Some(first) = g.first() {
                Ok(first.clone())
            } else {
                Ok(ScheduledJob {
                    name: name.to_string(),
                    status: "active".to_string(),
                    ..ScheduledJob::default()
                })
            }
    }
    fn schedule_remove(&self, _name: &str) -> Result<(), String> {
        self.schedule_remove_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop stub impls.
    // ─────────────────────────────────────────────────────────────────

    fn loop_add(
        &self,
        session_id: &SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        self.loop_add_calls.fetch_add(1, Ordering::SeqCst);
        let g = stub_lock(&self.registered_loops, "registered_loops");
            if let Some(first) = g.first() {
                Ok(first.clone())
            } else {
                Ok(RegisteredLoop {
                    id: "stub-loop".to_string(),
                    session_id: session_id.clone(),
                    prompt,
                    interval_seconds,
                    created_at: String::new(),
                    expires_at: String::new(),
                    last_run_at: None,
                })
            }
    }
    fn loop_cancel(&self, _id: &str) -> Result<bool, String> {
        self.loop_cancel_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.loop_cancel_result.load(Ordering::SeqCst))
    }
    fn loop_list(&self, _session_id: &SessionId) -> Result<Vec<RegisteredLoop>, String> {
        Ok({
        self.loop_list_calls.fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.registered_loops, "registered_loops");
            g.clone()
        }
    })
    }
    fn set_default_model(&self, model: &str) -> Result<(), String> {
        if model.is_empty() {
            return Err("an empty model string is not a default model".to_string());
        }
        *stub_lock(&self.default_model, "default_model") = Some(model.to_string());
        Ok(())
    }
    fn get_default_model(&self) -> Option<String> {
        stub_lock(&self.default_model, "default_model").clone()
    }
    // BUG-601: a session's turned-off rules; only seeded rules can be turned
    // off or on.
    fn blocklist_disabled_rules(&self, session_id: &SessionId) -> Result<Vec<String>, String> {
        Ok(stub_lock(&self.blocklist_disabled, "blocklist_disabled")
            .get(session_id)
            .map(|rules| rules.iter().cloned().collect())
            .unwrap_or_default())
    }
    fn blocklist_set_rule_enabled(
        &self,
        session_id: &SessionId,
        rule_id: &str,
        enabled: bool,
    ) -> Result<Vec<String>, String> {
        let known = stub_lock(&self.blocklist_rules, "blocklist_rules")
            .iter()
            .any(|rule| rule.id == rule_id);
        if !known {
            return Err(format!("the blocklist has no rule \"{rule_id}\""));
        }
        let mut disabled = stub_lock(&self.blocklist_disabled, "blocklist_disabled");
        let rules = disabled.entry(session_id.clone()).or_default();
        if enabled {
            rules.remove(rule_id);
        } else {
            rules.insert(rule_id.to_string());
        }
        Ok(rules.iter().cloned().collect())
    }
        }
    };
}
