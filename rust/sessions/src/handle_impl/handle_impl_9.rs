macro_rules! compose_handle_impl_9 {
    ($($collected:tt)*) => {
        compose_handle_impl_10! {
            $($collected)*
    fn list_session_worktrees(&self) -> Result<Vec<SessionWorktreeInfo>, String> {
        // WT-003: no session id on the wire — enumerate the distinct
        // project roots of all in-memory sessions plus every git-session
        // manifest and aggregate the per-root worktree listings
        // (deduplicated by session id).
        let roots = session_worktree_repo_roots(self);
        let mut out: Vec<SessionWorktreeInfo> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for root in &roots {
            // BUG-465: only a project root that is not a repository is
            // skipped (expected); every other git failure is returned.
            let worktrees = match codelet_git::list_worktrees(root) {
                Ok(w) => w,
                Err(codelet_git::GitError::NotARepository { .. }) => continue,
                Err(error) => {
                    return Err(format!(
                        "cannot list the worktrees of {}: {error}",
                        root
                    ))
                }
            };
            for w in worktrees {
                if !seen.insert(w.session_id.clone()) {
                    continue;
                }
                // Dirty heuristic: a non-empty session diff means uncommitted
                // changes are present in the worktree. BUG-466: a diff that
                // cannot be read is the error, never "clean at HEAD".
                let diff = codelet_git::get_session_diff(root, &w.session_id).map_err(|error| {
                    format!("cannot read the changes of worktree {}: {error}", w.session_id)
                })?;
                let dirty = !diff.files_changed.is_empty()
                    || !diff.files_added.is_empty()
                    || !diff.files_deleted.is_empty();
                let base_commit = diff.base_commit;
                out.push(SessionWorktreeInfo {
                    session_id: SessionId::new(w.session_id),
                    worktree_path: w.path.to_string_lossy().to_string(),
                    base_commit,
                    head_commit: w.head_commit,
                    dirty,
                });
            }
        }
        Ok(out)
    }
    fn inspect_session_changes(
        &self,
        session_id: &SessionId,
    ) -> Result<SessionChangesSummary, String> {
        // WT-003: resolve the repo from the session's own project root.
        let repo_path = self.resolve_worktree_repo_path(&session_id.value)?;
        let result = codelet_git::inspect_session(&repo_path, &session_id.value)
            .map_err(|e| format!("{e}"))?;
        let files_changed = (result.files_changed.len()
            + result.files_added.len()
            + result.files_deleted.len()) as u32;
        // Naively derive insertions/deletions from the unified diff —
        // counts `^+`/`^-` lines while skipping the `+++`/`---` headers.
        let mut insertions: u32 = 0;
        let mut deletions: u32 = 0;
        for line in result.diff.lines() {
            if line.starts_with("+++") || line.starts_with("---") {
                continue;
            }
            if line.starts_with('+') {
                insertions = insertions.saturating_add(1);
            } else if line.starts_with('-') {
                deletions = deletions.saturating_add(1);
            }
        }
        Ok(SessionChangesSummary {
            files_changed,
            insertions,
            deletions,
            // codelet-git does not yet surface a session commit log;
            // leave empty until it does.
            commits: Vec::new(),
            // WT-006: gitignored untracked files are honest, not changes.
            files_ignored: result.files_ignored.len() as u32,
        })
    }
    /// WT-009: detach a session from its isolation worktree — the flip
    /// side of session creation. Clears the session's `worktree_path`
    /// and `base_commit` (interior-mutable on `BackgroundSession`),
    /// deletes the git-session manifest so the worktree becomes
    /// prunable, re-registers the footer poller against the project
    /// root, and emits `IsolationStateChange(false, None)`. The
    /// session itself keeps running.
    fn detach_session_worktree(&self, session_id: &SessionId) -> Result<(), String> {
        let session = self
            .get_session(&session_id.value)?;

        // Only isolated sessions can be detached.
        let worktree = session
            .worktree_path()
            .ok_or_else(|| format!("session {} is not isolated", session_id.value))?;

        // 1. Clear the in-memory isolation state — effective cwd falls
        //    back to the project root from here on.
        session.clear_isolation();

        // 2. Delete the git-session manifest so
        //    `codelet_git::is_orphaned` reports the worktree as prunable
        //    (mirrors the WT-005 terminate-on-close cleanup; here the
        //    session stays alive, so deletion is the ONLY signal).
        //    Best-effort: a missing manifest (already merged/discarded)
        //    is a silent no-op; a failure is logged and swallowed so the
        //    detach never fails on bookkeeping.
        match codelet_git::delete_manifest(&session_id.value) {
            Ok(()) => tracing::info!(
                session_id = %session_id.value,
                "detach_session_worktree: git-session manifest deleted (worktree now prunable)"
            ),
            Err(e) => tracing::warn!(
                session_id = %session_id.value,
                error = %e,
                "detach_session_worktree: failed to delete git-session manifest (best-effort)"
            ),
        }

        // 3. Re-register the footer cwd registry against the project
        //    root and re-spawn the footer poller so the footer stops
        //    polling the (now untracked) worktree.
        let project = session.project.clone();
        let sid = session_id.value.clone();
        self.hooks().stop_footer_poller(&sid);
        codelet_tools::unregister_footer_cwd(parse_session_uuid(session_id)?);
        self.hooks().spawn_footer_poller(sid, project, None);

        // 4. Re-inject the environment context WITHOUT isolation context
        //    so the LLM's next reminder reflects the non-isolated state.
        //    `session.inner` is a tokio mutex — bridge sync→async the
        //    same way the other RPC-070 bridges do (multi-thread
        //    runtime precondition).
        let inner_mutex = session.inner.clone();
        let project = std::path::PathBuf::from(&session.project);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut inner = inner_mutex.lock().await;
                inner.inject_context_reminders(&project);
            })
        });

        // 5. Emit IsolationStateChange(false, None) on the manager-owned
        //    chunks_tx so the TUI store flips the isolation badge off.
        let _ = SessionManager::chunks_tx(self).send((
            codelet_rpc_types::SessionId::from(session_id.value.clone()),
            codelet_rpc_types::StreamChunk::isolation_state_change(false, None),
        ));

        tracing::info!(
            session_id = %session_id.value,
            former_worktree = %worktree.to_string_lossy(),
            "detach_session_worktree: session detached from worktree"
        );
        Ok(())
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule. Each method resolves the repo_path at call
    // time via the start workspace (RPC-444; matching the blocklist_list
    // pattern in RPC-056) and delegates to
    // `codelet_core::scheduler::crud` — the lifted, NAPI-free
    // CRUD helpers introduced by RPC-058.
    // ─────────────────────────────────────────────────────────────────

    fn schedule_add(&self, job: ScheduledJob) -> Result<ScheduledJob, String> {
        let repo_path = self.default_project()?;
        codelet_core::scheduler::crud::schedule_add(&repo_path.to_string_lossy(), job)
    }
    fn schedule_list(&self) -> Result<Vec<ScheduledJob>, String> {
        // BUG-445: no start workspace (BUG-437) or an unreadable schedule is
        // the error, not an empty list.
        let repo_path = self.default_project()?;
        codelet_core::scheduler::crud::schedule_list(&repo_path.to_string_lossy())
    }
    fn schedule_pause(&self, name: &str) -> Result<ScheduledJob, String> {
        let repo_path = self.default_project()?;
        codelet_core::scheduler::crud::schedule_pause(&repo_path.to_string_lossy(), name)
    }
    fn schedule_resume(&self, name: &str) -> Result<ScheduledJob, String> {
        let repo_path = self.default_project()?;
        codelet_core::scheduler::crud::schedule_resume(&repo_path.to_string_lossy(), name)
    }
    fn schedule_remove(&self, name: &str) -> Result<(), String> {
        let repo_path = self.default_project()?;
        codelet_core::scheduler::crud::schedule_remove(&repo_path.to_string_lossy(), name)
    }
    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop. Each method delegates to the shared
    // `codelet_core::loops::LoopStore` singleton via a sync→async
    // bridge so callers MUST be on a thread with an active tokio
    // runtime.
    // ─────────────────────────────────────────────────────────────────

    fn loop_add(
        &self,
        session_id: &SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        let session = self
            .get_session(session_id.value.as_str())?;
        let session_uuid = session.id;

        // Construct the entry with a fresh 8-char hex id derived from a
        // UUID v4 (avoids pulling in `rand` since `uuid` is already in
        // the workspace dependency tree).
        let id = generate_loop_id();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::days(3);
        let entry = codelet_core::loops::LoopEntry {
            id,
            session_id: session_uuid,
            prompt,
            interval_seconds,
            created_at,
            expires_at,
            last_run_at: None,
        };

        // Capture the BackgroundSession by Arc for both callbacks.
        let session_for_fire = Arc::clone(&session);
        let on_fire: Arc<dyn Fn(String) + Send + Sync + 'static> =
            Arc::new(move |prompt_text: String| {
                let _ = session_for_fire.send_input(prompt_text, None);
            });

        let session_for_idle = Arc::clone(&session);
        let idle_check: codelet_core::loops::IdleCheckFn = Arc::new(move |_session_id: Uuid| {
            let session = Arc::clone(&session_for_idle);
            Box::pin(async move { session.get_status() == SessionStatus::Idle })
        });

        let entry_for_store = entry.clone();
        loop_block_on(async move {
            codelet_core::loops::LoopStore::instance()
                .try_register_with_task_and_idle_check(entry_for_store, on_fire, idle_check)
                .await
        })?;

        Ok(entry_to_wire(&entry))
    }
        }
    };
}
