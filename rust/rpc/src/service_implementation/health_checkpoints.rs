macro_rules! compose_rpc_health_checkpoints {
    ($($collected:tt)*) => {
        compose_rpc_workspace_history! {
            $($collected)*
    async fn health(self, _ctx: Context) -> HealthInfo {
        // RPC-011 question [12]: HealthInfo fields are typed `i64` (not
        // `u64`) so the cfg-gated `napi(object)` compiles under
        // napi-derive v3 + `napi4` feature. ServerStats keeps its
        // natural `u64` accessors; we cast at the RPC boundary.
        let uptime_secs = self.inner.started_at.elapsed().as_secs() as i64;
        let stats = self.inner.stats.lock().await.clone();
        let (connected_clients, last_watcher_event_secs_ago, lag_chunks, lag_logs, lag_work_units) =
            match stats.as_ref() {
                Some(s) => (
                    s.connected_clients() as i64,
                    s.last_watcher_event_secs_ago().map(|n| n as i64),
                    s.lag_chunks() as i64,
                    s.lag_logs() as i64,
                    s.lag_work_units() as i64,
                ),
                None => (0, None, 0, 0, 0),
            };
        HealthInfo {
            uptime_secs,
            connected_clients,
            last_watcher_event_secs_ago,
            lag_chunks,
            lag_logs,
            lag_work_units,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    async fn checkpoint_counts(self, _ctx: Context) -> Result<CheckpointCounts, String> {
        // RPC-015 / RPC-453: no workspace or a git failure is an error, not
        // zero counts.
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        codelet_git::ghost_commit::count_checkpoints(cwd).map_err(|error| error.to_string())
    }

    async fn changed_files(self, _ctx: Context) -> Result<Vec<ChangedFile>, String> {
        // RPC-355: gated on the attached cwd exactly like checkpoint_counts.
        // BUG-465: no workspace or a git failure is returned, not [].
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        collect_changed_files(cwd).map_err(|error| error.to_string())
    }

    async fn file_diff(self, _ctx: Context, path: String) -> Result<Option<String>, String> {
        // RPC-355: `None` when the file has no diff. RPC-453: no workspace or
        // a git failure is an error, not "no diff".
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        codelet_git::get_file_diff(cwd, &path).map_err(|error| error.to_string())
    }

    async fn list_checkpoints(self, _ctx: Context) -> Result<Vec<CheckpointInfo>, String> {
        // RPC-362: gated on the attached cwd exactly like checkpoint_counts.
        // TUI-109: drive the streaming variant with a callback that
        // publishes a progress frame on the shared broadcast; the final
        // Vec still returns through the same RPC. BUG-465: no workspace or
        // a git failure is returned, not [].
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        let tx = self.inner.checkpoints_progress_tx();
        checkpoints::collect_checkpoints_stream(cwd, &mut |frame| {
            // Best-effort: no live subscribers (or a lagging subscriber) is
            // a no-op, never an error.
            let _ = tx.send(frame);
        })
        .map_err(|error| error.to_string())
    }

    async fn checkpoint_diff_files(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>, String> {
        // BUG-465: no workspace or a git failure is returned, not [].
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        checkpoints::collect_checkpoint_diff_files(cwd, &work_unit_id, &name)
            .map_err(|error| error.to_string())
    }

    async fn checkpoint_file_diff(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>, String> {
        // RPC-453: no workspace or a git failure is an error, not "no diff".
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        checkpoints::checkpoint_file_diff(cwd, &work_unit_id, &name, &path)
            .map_err(|error| error.to_string())
    }

    async fn restore_checkpoint_file(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => checkpoints::restore_file(cwd, &work_unit_id, &name, &path)
                .map_err(|e| format!("{e}")),
            None => Err(
                "restore_checkpoint_file requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn restore_checkpoint_all(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => {
                checkpoints::restore_all(cwd, &work_unit_id, &name).map_err(|e| format!("{e}"))
            }
            None => Err(
                "restore_checkpoint_all requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn delete_checkpoint(
        self,
        _ctx: Context,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => {
                checkpoints::delete_one(cwd, &work_unit_id, &name).map_err(|e| format!("{e}"))
            }
            None => Err(
                "delete_checkpoint requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn delete_all_checkpoints(self, _ctx: Context) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => checkpoints::delete_all(cwd).map_err(|e| format!("{e}")),
            None => Err(
                "delete_all_checkpoints requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }
        }
    };
}
