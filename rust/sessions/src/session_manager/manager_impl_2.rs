//! SessionManager inherent methods (part 2).

use super::*;

impl SessionManager {
    /// List sessions, filtering persisted sessions by `project_path`.
    ///
    /// RPC-427: Added `project_path` parameter so `/resume` only shows sessions
    /// belonging to the current project. In-memory sessions are inherently
    /// project-scoped (created with `std::env::current_dir()`), so only
    /// persisted sessions require explicit filtering.
    ///
    /// An EMPTY `project_path` means "no project filter" — persisted
    /// sessions from ALL projects are merged (the pre-RPC-427 semantics),
    /// so a caller without a known project root still sees its own
    /// sessions rather than an empty list.
    /// BUG-465: a persisted-session read failure is returned, not an
    /// in-memory-only list.
    pub fn list_sessions(&self, project_path: &str) -> Result<Vec<SessionInfo>, String> {
        let in_memory: Vec<SessionInfo> = self
            .sessions
            .read()
            .expect("sessions lock poisoned")
            .values()
            .map(|s| s.get_info())
            .collect();

        tracing::debug!(
            in_memory_count = in_memory.len(),
            "list_sessions: collected in-memory sessions"
        );

        // RPC-427: Filter persisted sessions by project path instead of loading all.
        // An empty project path means "no filter" (all projects) — callers that
        // don't know the project root still get the full merged list.
        let manifests: Vec<codelet_core::persistence::SessionManifest> = if project_path.is_empty()
        {
            codelet_core::persistence::list_all_sessions()?
        } else {
            codelet_core::persistence::list_sessions_for_project(std::path::Path::new(
                project_path,
            ))?
        };
        tracing::info!(
            persisted_on_disk = manifests.len(),
            "list_sessions: loaded persisted session manifests from disk"
        );
        let persisted = Self::persisted_manifests_to_infos(manifests, &in_memory);

        let mut all = in_memory;
        all.extend(persisted);

        // TUI-099: Sort by updated_at_ms descending (most recent first),
        // with session ID as alphabetical tiebreaker. Sessions without
        // a timestamp (None) appear at the end.
        all.sort_by(|a, b| match (a.updated_at_ms, b.updated_at_ms) {
            (Some(ts_a), Some(ts_b)) => ts_b.cmp(&ts_a).then_with(|| a.id.cmp(&b.id)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.id.cmp(&b.id),
        });

        tracing::debug!(
            total_count = all.len(),
            "list_sessions: returning merged session list"
        );

        // DEBUG: Log the IDs of all sessions returned so callers can trace
        // which sessions are included in the merge result.
        let ids: Vec<String> = all.iter().map(|s| s.id.clone()).collect();
        tracing::debug!(
            session_ids = ?ids,
            "list_sessions: session IDs in merged result"
        );

        Ok(all)
    }
    /// RPC-427: convert persisted manifests into `SessionInfo`s, dropping
    /// any session that still lives in memory (it is already reported via
    /// `in_memory`). Shared by both the empty-path (all projects) and
    /// filtered (single project) list paths.
    pub(super) fn persisted_manifests_to_infos(
        manifests: Vec<codelet_core::persistence::SessionManifest>,
        in_memory: &[SessionInfo],
    ) -> Vec<SessionInfo> {
        let in_memory_ids: std::collections::HashSet<String> =
            in_memory.iter().map(|s| s.id.clone()).collect();
        let result: Vec<SessionInfo> = manifests
            .into_iter()
            .filter(|m| !in_memory_ids.contains(&m.id.to_string()))
            .map(|m| SessionInfo {
                id: m.id.to_string(),
                name: m.name,
                status: "idle".to_string(),
                project: m.project.to_string_lossy().to_string(),
                message_count: m.messages.len() as u32,
                provider_id: if m.provider.is_empty() {
                    None
                } else {
                    Some(
                        m.provider
                            .split('/')
                            .next()
                            .unwrap_or(&m.provider)
                            .to_string(),
                    )
                },
                model_id: if m.provider.is_empty() {
                    None
                } else {
                    m.provider.split('/').nth(1).map(|s| s.to_string())
                },
                is_isolated: false,
                worktree_path: None,
                role: None,
                updated_at_ms: Some(m.updated_at.timestamp_millis()),
                work_unit_focus: m.work_unit_focus,
            })
            .collect();
        tracing::info!(
            persisted_new = result.len(),
            "list_sessions: merged persisted sessions not in memory"
        );
        result
    }
    /// WT-003: IDs of every in-memory session (no project filter, no
    /// persisted-only sessions). Used by the RPC-057 worktree surface to
    /// build the active-session set for `prune_orphaned_worktrees` — a
    /// session that is alive in this process must never be pruned,
    /// regardless of which process directory the TUI was launched from.
    pub fn all_in_memory_session_ids(&self) -> Vec<String> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .keys()
            .map(|id| id.to_string())
            .collect()
    }
    /// WT-003: the distinct project roots of all in-memory sessions
    /// (deduplicated, deterministic order). The RPC-057 worktree surface
    /// (list/prune) has no session id on the wire, so it enumerates these
    /// roots instead of trusting the process cwd.
    pub fn all_in_memory_project_roots(&self) -> Vec<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let mut roots: Vec<String> = sessions
            .values()
            .map(|s| s.project.clone())
            .filter(|p| !p.is_empty())
            .collect();
        drop(sessions);
        roots.sort();
        roots.dedup();
        roots
    }
    /// WT-003: resolve the repository path a session-worktree RPC op must
    /// target for `session_id`. Resolution order:
    /// 1. the in-memory session's recorded project root
    ///    (`BackgroundSession.project` — captured at session creation),
    /// 2. the git-session manifest's `project_root`
    ///    (`~/.fspec/git-sessions/<id>.json` — survives a TUI restart),
    ///
    /// BUG-437: a session known to neither is an error (no cwd or start
    /// workspace stand-in), and a manifest that cannot be read is reported.
    pub fn resolve_worktree_repo_path(&self, session_id: &str) -> Result<PathBuf, String> {
        if let Ok(session) = self.get_session(session_id) {
            let project = session.project.clone();
            if !project.is_empty() {
                return Ok(PathBuf::from(project));
            }
        }
        let manifest = codelet_git::read_manifest(session_id).map_err(|error| {
            format!("cannot read the git-session manifest of {session_id}: {error}")
        })?;
        match manifest {
            Some(manifest) if !manifest.project_root.as_os_str().is_empty() => Ok(manifest.project_root),
            _ => Err(format!(
                "session {session_id} has no recorded project root (no live session or git-session manifest)"
            )),
        }
    }
    /// VIEWNV-001: Set the active (currently viewed) session
    pub fn set_active_session(&self, id: Uuid) {
        *self
            .active_session_id
            .write()
            .expect("active_session lock poisoned") = Some(id);
    }
    /// VIEWNV-001: Clear the active session (when returning to board)
    pub fn clear_active_session(&self) {
        *self
            .active_session_id
            .write()
            .expect("active_session lock poisoned") = None;
    }
    /// VIEWNV-001: Get the active session ID
    pub fn get_active_session(&self) -> Option<Uuid> {
        *self
            .active_session_id
            .read()
            .expect("active_session lock poisoned")
    }
    /// SCHED-003: Start the scheduler if not already running.
    pub(super) fn maybe_start_scheduler(&self, project: &str) {
        let mut handle = self
            .scheduler_handle
            .write()
            .expect("scheduler lock poisoned");
        if handle.is_some() {
            return;
        }
        let schedules_path =
            codelet_common::spec_paths::ProjectRoots::resolve(std::path::Path::new(project))
                .map(|roots| roots.schedules())
                .map_err(|error| error.to_string());
        // WT-015: a broken state-root pointer is reported, never read as
        // "no schedules".
        let schedules_path = match schedules_path {
            Ok(path) => path,
            Err(error) => {
                tracing::warn!("Not starting the scheduler for {project}: {error}");
                return;
            }
        };
        if schedules_path.exists() {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                tracing::info!("Starting scheduler for project: {}", project);
                self.hooks().spawn_scheduler(project.to_string(), rt);
                *handle = Some(tokio::spawn(async {}));
            } else {
                tracing::warn!("No Tokio runtime available for scheduler");
            }
        }
    }
    /// SCHED-011: Ensure the scheduler is running (for /loop support).
    pub fn ensure_scheduler_running(&self, project: &str, rt: &tokio::runtime::Handle) {
        let mut handle = self
            .scheduler_handle
            .write()
            .expect("scheduler lock poisoned");
        if handle.is_some() {
            return;
        }
        tracing::info!("Starting scheduler for /loop support: {}", project);
        self.hooks()
            .ensure_scheduler_running_for_loop(project.to_string(), rt.clone());
        *handle = Some(tokio::spawn(async {}));
    }
    /// Navigation: next session
    pub fn get_next_session(&self) -> Option<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let active = self
            .active_session_id
            .read()
            .expect("active_session lock poisoned");
        let nav_list = crate::navigation::build_navigation_list(&sessions, &self.chain_of_command);
        match crate::navigation::get_next_target(&nav_list, *active) {
            crate::navigation::NavigationTarget::Session(id) => Some(id.to_string()),
            _ => None,
        }
    }
}
