macro_rules! compose_handle_trait_4 {
    ($($collected:tt)*) => {
        compose_handle_trait_finish! {
            $($collected)*
    /// RPC-054: clear credentials for a provider. Idempotent on
    /// already-cleared rows.
    fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String> {
        let _ = provider_id;
        Ok(())
    }
    /// RPC-054: perform a network round-trip to the provider's base URL
    /// and return latency + success metadata. Used by the `t` key on a
    /// focused provider row.
    fn test_provider_connection(&self, provider_id: &str) -> Result<TestConnectionResult, String> {
        let _ = provider_id;
        Ok(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 0,
        })
    }
    /// RPC-054: refresh the provider's cached model list (no-op when
    /// the provider does not publish a dynamic catalog) and return the
    /// fresh `ModelEntry` list. Used by the `r` key on a focused
    /// provider row.
    fn refresh_models_cache(
        &self,
        provider_id: &str,
    ) -> Result<Vec<codelet_rpc_types::ModelEntry>, String> {
        let _ = provider_id;
        Ok(Vec::new())
    }
    // ========================================================================
    // RPC-056: Blocklist surface. Default impl returns an empty list so
    // existing handles compile unchanged. The production `codelet-sessions`
    // SessionManager overrides this by re-loading the system + project
    // blocklist configs separately via `codelet_tools::blocklist`.
    // `StubSessionManagerHandle` overrides this with a deterministic
    // in-memory snapshot + per-call counter for cross-transport parity
    // tests.
    // ========================================================================

    /// RPC-056: list every blocklist rule with its `source` provenance
    /// ("system" | "project"). Drives the left pane of
    /// `BlocklistView` in the Rust ratatui frontend. The TS frontend
    /// reaches the same data via the legacy `blocklistLoad(cwd)` NAPI
    /// call.
    fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>, String> {
        Err("blocklist_list is not implemented by this session manager".to_string())
    }
    /// BUG-601: the rule ids turned off for the live session `session_id`
    /// (sorted). They are off for that session only.
    fn blocklist_disabled_rules(&self, session_id: &SessionId) -> Result<Vec<String>, String> {
        let _ = session_id;
        Err("blocklist_disabled_rules is not implemented by this session manager".to_string())
    }
    /// BUG-601: turn the blocklist rule `rule_id` off (`enabled` false) or
    /// on for the live session `session_id`, saved with the session; answers
    /// the session's turned-off rule ids (sorted).
    fn blocklist_set_rule_enabled(
        &self,
        session_id: &SessionId,
        rule_id: &str,
        enabled: bool,
    ) -> Result<Vec<String>, String> {
        let _ = (session_id, rule_id, enabled);
        Err("blocklist_set_rule_enabled is not implemented by this session manager".to_string())
    }
    // ========================================================================
    // RPC-057: Merge/worktree surface. Default impls return safe values so
    // existing handles compile unchanged. The production `codelet-sessions`
    // SessionManager overrides each by delegating to `codelet-git`.
    // `StubSessionManagerHandle` overrides them with per-call counters +
    // deterministic seeded payloads for cross-transport parity tests.
    // ========================================================================

    /// RPC-057: merge a session's worktree changes back to the base
    /// branch. The strategy is reserved for future evolution — the
    /// underlying codelet-git layer ignores it today.
    fn merge_session_worktree(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        let _ = (session_id, strategy);
        Ok(MergeOutcome {
            status: MergeStatus::NoChanges,
            conflicts: Vec::new(),
            merge_commit: None,
            worktree_path: None,
        })
    }
    /// RPC-057: discard a session's worktree changes.
    fn discard_session_worktree(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Ok(())
    }
    /// RPC-057: prune any orphaned worktrees that no longer belong to a
    /// known session. Returns the session ids that were pruned.
    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    /// RPC-057: list every known session worktree with its base/head
    /// commits and a dirty heuristic.
    fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>, String> {
        Err("list_session_worktrees is not implemented by this session manager".to_string())
    }
    /// RPC-057: inspect a session's pending changes summary
    /// (files_changed / insertions / deletions / commits).
    fn inspect_session_changes(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionChangesSummary, String> {
        let _ = session_id;
        Ok(SessionChangesSummary {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
            commits: Vec::new(),
            files_ignored: 0,
        })
    }
    /// WT-018: the cards session `session_id` created, moved or edited on
    /// its main checkout's board.
    fn session_card_changes(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>, String> {
        let _ = session_id;
        Err("session_card_changes is not implemented by this session manager".to_string())
    }
    /// WT-018: move every card session `session_id` moved back to its
    /// status before the session, on its main checkout's board.
    fn move_session_cards_back(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>, String> {
        let _ = session_id;
        Err("move_session_cards_back is not implemented by this session manager".to_string())
    }
    /// WT-009: detach a session from its isolation worktree — the flip
    /// side of session creation. Clears the session's `worktree_path`
    /// and `base_commit`, deletes the git-session manifest so the
    /// worktree becomes prunable, and emits
    /// `IsolationStateChange(false, None)`. The session itself keeps
    /// running with an effective cwd of the project root. The default
    /// impl returns a safe "not isolated" error so existing handles
    /// compile unchanged.
    fn detach_session_worktree(&self, session_id: &SessionId) -> Result<(), String> {
        let _ = session_id;
        Err("session is not isolated".to_string())
    }
    // ========================================================================
    // RPC-058: /schedule surface. Default impls return safe values so existing
    // handles compile unchanged. The production `codelet-sessions`
    // SessionManager overrides each by delegating to
    // `codelet_core::scheduler::crud`. `StubSessionManagerHandle` overrides
    // them with per-call counters + deterministic seeded payloads for
    // cross-transport parity tests.
    // ========================================================================

    /// RPC-058: persist a new scheduled job to `spec/schedules.json`.
    fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob, String> {
        Ok(job)
    }
    /// RPC-058: list every persisted scheduled job.
    fn schedule_list(&self) -> Result<Vec<ScheduledJob>, String> {
        Err("schedule_list is not implemented by this session manager".to_string())
    }
    /// RPC-058: flip the `status` of a job to `paused`.
    fn schedule_pause(&self, name: &str) -> Result<ScheduledJob, String> {
        let _ = name;
        Ok(ScheduledJob::default())
    }
    /// RPC-058: flip the `status` of a job to `active`.
    fn schedule_resume(&self, name: &str) -> Result<ScheduledJob, String> {
        let _ = name;
        Ok(ScheduledJob::default())
    }
    /// RPC-058: remove a job from `spec/schedules.json`.
    fn schedule_remove(&self, name: &str) -> Result<(), String> {
        let _ = name;
        Ok(())
    }
    // ========================================================================
    // RPC-059 — /loop subcommand surface. Each method has a safe default
    // (returns a blank `RegisteredLoop`, `false`, or an empty `Vec`) so
    // existing handle implementations compile unchanged. The production
    // impl lives in `rust/sessions/src/handle_impl.rs` and routes
    // through the shared `codelet_core::loops::LoopStore` singleton.
    // `StubSessionManagerHandle` overrides them with per-call counters
    // + deterministic seeded payloads for cross-transport parity tests.
    // ========================================================================

    /// RPC-059: register a session-scoped recurring prompt on the
    /// shared `LoopStore` singleton.
    fn loop_add(
        &self,
        session_id: &SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        let _ = session_id;
        let _ = interval_seconds;
        let _ = prompt;
        Ok(RegisteredLoop::default())
    }
    /// RPC-059: cancel a registered loop by id. Returns `true` when a
    /// matching loop was removed.
    fn loop_cancel(&self, id: &str) -> Result<bool, String> {
        let _ = id;
        Ok(false)
    }
    /// RPC-059: list every loop registered against a session.
    fn loop_list(&self, session_id: &SessionId) -> Result<Vec<RegisteredLoop>, String> {
        let _ = session_id;
        Err("loop_list is not implemented by this session manager".to_string())
    }
    /// MCP-003: set a daemon client's MCP servers as one live, idle
    /// session's whole MCP set (MCP-008), or leave it unchanged. The default refuses: a handle without live
    /// sessions (including the stub) has no MCP registry to configure.
    fn configure_session_mcp_servers(
        &self,
        session_id: &SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<codelet_rpc_types::SessionMcpServerTools>, String>,
                > + Send,
        >,
    > {
        let _ = servers;
        let error = format!(
            "session {} cannot configure MCP servers on this handle",
            session_id.value
        );
        Box::pin(async move { Err(error) })
    }
    /// TOOL-027: send an interrupt or end of input to a live exec
    /// session's command. The default refuses: a handle without live
    /// sessions (including the stub) has no exec sessions to control.
    fn control_exec_stdin(
        &self,
        session_id: &SessionId,
        exec_session_id: &str,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<(), String> {
        Err(format!(
            "session {} cannot send {control:?} to exec session {exec_session_id} on this handle",
            session_id.value
        ))
    }
        }
    };
}
