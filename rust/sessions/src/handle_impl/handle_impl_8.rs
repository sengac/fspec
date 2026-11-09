macro_rules! compose_handle_impl_8 {
    ($($collected:tt)*) => {
        compose_handle_impl_9! {
            $($collected)*
    fn refresh_models_cache(&self, provider_id: &str) -> Result<Vec<ModelEntry>, String> {
        // BUG-207: this surface used to re-list STATIC provider info, so
        // against a months-old models.dev snapshot it was a no-op: it never
        // touched the network and returned an empty list for every built-in.
        // It now FORCES a catalog re-fetch (bypassing the cache TTL and the
        // revalidation cooldown), persists the new snapshot to disk, and
        // sources the returned entries from that fresh registry — the same
        // `cloud_models::cloud_model_entries` path `list_providers` uses, so
        // there is one sourcing rule for built-in models.
        //
        // An empty `provider_id` means "refresh the shared catalog only":
        // the model selector's refresh spans every provider, so there is no
        // single provider's list to return.
        let registry = force_refresh_cloud_registry().or_else(|| {
            // An unreachable catalog must never empty the selector: fall
            // back to the on-disk snapshot (which `build_cloud_registry`
            // serves while revalidating) rather than erroring out.
            tracing::warn!(
                "refresh_models_cache: models.dev force-refresh failed; falling back to the on-disk snapshot"
            );
            build_cloud_registry()
        });
        let registry = registry.ok_or_else(|| {
            "refresh_models_cache: models.dev catalog unavailable (no network and no usable snapshot)"
                .to_string()
        })?;

        if provider_id.is_empty() {
            return Ok(Vec::new());
        }

        // Custom providers declare their models in their own config file,
        // not in models.dev — the force-refresh cannot add to them, so keep
        // the statically-declared list as their fallback.
        let custom = codelet_providers::custom::list_providers_info()
            .map_err(|e| format!("list_providers_info failed: {e}"))?
            .into_iter()
            .find(|p| p.name == provider_id && p.is_custom);
        if let Some(p) = custom {
            return Ok(p
                .models
                .into_iter()
                .map(|m| ModelEntry {
                    id: m.id,
                    display_name: String::new(),
                    // PROV-170: an undeclared window stays unavailable;
                    // PROV-172: list_providers_info already rejected a value the u32
                    // wire cannot carry.
                    context_window: m.context_window,
                    supports_reasoning: m.supports_thinking,
                    supports_vision: m.supports_vision,
                    is_custom: true,
                })
                .collect());
        }

        let has_credentials = crate::cloud_models::provider_has_credentials(provider_id);
        Ok(crate::cloud_models::cloud_model_entries(
            &registry,
            provider_id,
            has_credentials,
        ))
    }
    // ========================================================================
    // RPC-056: Blocklist surface.
    //
    // Re-loads the system + project blocklist configs SEPARATELY (rather
    // than via `load_blocklist_config`, which merges and loses the
    // per-rule provenance) so the wire snapshot can stamp each rule with
    // its `source` tag ("system" | "project"). The project root is
    // derived from the start workspace (RPC-444) — matching the
    // `create_session` pattern in this file.
    //
    // System and project configs each fall back to an empty
    // `BlocklistConfig` when the file is absent or fails to parse, so
    // missing configs surface as an empty list rather than an error.
    // The TS frontend takes the same fall-back path via
    // `blocklistLoad(cwd)`.
    // ========================================================================

    /// BUG-445: no start workspace (BUG-437) or a blocklist config that
    /// cannot be loaded is the error, not a shorter list.
    fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>, String> {
        let mut out: Vec<BlocklistRuleInfo> = Vec::new();
        let project_root = self.default_project()?;
        let system = codelet_tools::blocklist::system_config_path()?;
        let project = codelet_tools::blocklist::project_config_path(&project_root)?;
        for (path, scope) in [(system, "system"), (project, "project")] {
            if !path.exists() {
                continue;
            }
            let config = codelet_tools::blocklist::BlocklistConfig::load_from_file(&path)
                .map_err(|error| {
                    format!("cannot load the {scope} blocklist {}: {error}", path.display())
                })?;
            out.extend(
                config
                    .rules
                    .into_iter()
                    .map(|rule| blocklist_rule_to_wire(rule, scope)),
            );
        }
        Ok(out)
    }
    /// BUG-601: see `blocklist_session_rules.rs`.
    fn blocklist_disabled_rules(&self, session_id: &SessionId) -> Result<Vec<String>, String> {
        self.session_blocklist_disabled_rules(session_id)
    }
    fn blocklist_set_rule_enabled(
        &self,
        session_id: &SessionId,
        rule_id: &str,
        enabled: bool,
    ) -> Result<Vec<String>, String> {
        self.set_session_blocklist_rule_enabled(session_id, rule_id, enabled)
    }
    // ========================================================================
    // RPC-057: Merge/worktree surface — delegates to `codelet-git`.
    //
    // WT-003: repo_path resolution. Session-scoped ops (merge/discard/
    // inspect) resolve the repository from the session's own recorded
    // project root (in-memory `BackgroundSession.project`, falling back
    // to the git-session manifest's `project_root`, then the process
    // cwd) — see `SessionManager::resolve_worktree_repo_path`. The
    // no-session-id ops (list/prune) enumerate the distinct project
    // roots of all in-memory sessions and run per root, aggregating
    // results — never driven by the process cwd. MergeStrategy is
    // accepted on the trait surface for future evolution but the
    // underlying codelet-git layer uses a single fast-forward-style
    // algorithm (parity with the current TS sessionMergeChanges).
    // ========================================================================

    fn merge_session_worktree(
        &self,
        session_id: &SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        // WT-008: the strategy is enforced in codelet-git — only
        // FastForward is supported; anything else errors up front.
        // WT-003: resolve the repo from the session's own project root,
        // NOT the process cwd (the cwd only matters when neither an
        // in-memory session nor a git-session manifest exists).
        let repo_path = self.resolve_worktree_repo_path(&session_id.value)?;
        // WT-009: resolve the worktree path for the outcome so the TUI's
        // "Effective worktree: ..." conflict footer prints a real path the
        // LLM can act on (instead of the session UUID).
        let worktree_path = Some(
            repo_path
                .join(codelet_git::FSPEC_WORKTREES_DIR)
                .join(&session_id.value)
                .to_string_lossy()
                .to_string(),
        );
        match codelet_git::merge_session_with_strategy(&repo_path, &session_id.value, &strategy) {
            Ok(result) => {
                let total_changed = result.files_modified.len()
                    + result.files_added.len()
                    + result.files_deleted.len();
                if total_changed == 0 {
                    // WT-009: NoChanges carries no worktree path — there is
                    // nothing to act on in the worktree.
                    return Ok(MergeOutcome {
                        status: MergeStatus::NoChanges,
                        conflicts: Vec::new(),
                        merge_commit: result.merge_commit,
                        worktree_path: None,
                    });
                }
                Ok(MergeOutcome {
                    status: MergeStatus::Success,
                    conflicts: Vec::new(),
                    merge_commit: result.merge_commit,
                    worktree_path,
                })
            }
            Err(codelet_git::GitError::ConflictError { files }) => Ok(MergeOutcome {
                status: MergeStatus::Conflict,
                conflicts: files,
                merge_commit: None,
                worktree_path,
            }),
            Err(e) => Err(format!("{e}")),
        }
    }
    fn discard_session_worktree(&self, session_id: &SessionId) -> Result<(), String> {
        // WT-003: resolve the repo from the session's own project root.
        let repo_path = self.resolve_worktree_repo_path(&session_id.value)?;
        codelet_git::discard_session(&repo_path, &session_id.value)
            .map(|_| ())
            .map_err(|e| format!("{e}"))
    }
    fn session_card_changes(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>, String> {
        // WT-018: the board of the session's main checkout (WT-003 repo path).
        let repo_path = self.resolve_worktree_repo_path(&session_id.value)?;
        codelet_fspec_core::session_cards::session_card_changes(&repo_path, &session_id.value)
            .map_err(|e| e.to_string())
    }
    fn move_session_cards_back(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>, String> {
        // WT-018: moved back from the main checkout, after the discard.
        let repo_path = self.resolve_worktree_repo_path(&session_id.value)?;
        codelet_fspec_core::session_cards::move_session_cards_back(&repo_path, &session_id.value)
            .map_err(|e| e.to_string())
    }
    fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String> {
        // WT-003: no session id on the wire — enumerate the distinct
        // project roots of ALL in-memory sessions plus every git-session
        // manifest (closed sessions leave their manifest behind, and it
        // records the project root), and prune each root, aggregating the
        // pruned ids. The active-session set is every in-memory session
        // id (no project filter, no persisted-only sessions) so a live
        // session is never pruned regardless of the process cwd the TUI
        // was launched from.
        let active = self.all_in_memory_session_ids();
        let active_set: std::collections::HashSet<String> = active.into_iter().collect();
        let roots = session_worktree_repo_roots(self);
        let root_count = roots.len();
        let mut pruned = Vec::new();
        let mut last_error: Option<String> = None;
        for root in &roots {
            match codelet_git::prune_orphaned(root, &active_set) {
                Ok(result) => pruned.extend(result.pruned),
                Err(e) => {
                    // A non-repository project root is expected (sessions
                    // don't require a git repo) — warn and continue.
                    tracing::warn!(
                        project_root = %root,
                        error = %e,
                        "prune_orphaned_worktrees: skipping project root that is not a git repository"
                    );
                    last_error = Some(format!("{e}"));
                }
            }
        }
        // WT-003: only surface an error when NO root could be pruned at
        // all (no in-memory sessions, no manifests, or the single known
        // root is not a repository) — a mixed aggregate (some roots
        // pruned, one root not a repo) must still return the pruned ids.
        match (root_count, pruned.is_empty(), last_error) {
            (0, _, _) => codelet_git::prune_orphaned(self.default_project()?, &active_set)
                .map(|r| r.pruned)
                .map_err(|e| format!("{e}")),
            (1, true, Some(err)) => Err(err),
            _ => Ok(pruned),
        }
    }
        }
    };
}
