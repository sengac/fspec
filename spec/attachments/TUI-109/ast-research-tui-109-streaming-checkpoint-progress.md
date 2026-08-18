# AST Research — TUI-109 streaming checkpoint progress

Discovery-phase AST analysis (AstGrep) of the four integration points for
streaming per-item checkpoint-enumeration progress.

## 1. `codelet_git::ghost_commit::list_all_ghost_checkpoints` (rust/git/src/ghost_commit.rs:637)

```rust
pub fn list_all_ghost_checkpoints(dir: &Path) -> Result<Vec<(String, String)>> {
    let repo = match open_repo(dir) { Ok(r) => r, Err(_) => return Ok(Vec::new()) };
    let prefix = format!("{CHECKPOINT_REF_PREFIX}/");
    let mut out: Vec<(String, String)> = Vec::new();
    let refs = repo.references().map_err(...)?;
    for reference in refs.all().map_err(...)? {
        let reference = reference.map_err(...)?;
        let name = reference.name().as_bstr().to_string();
        if let Some(suffix) = name.strip_prefix(&prefix) {
            if let Some((work_unit_id, checkpoint_name)) = suffix.split_once('/') {
                out.push((work_unit_id.to_string(), checkpoint_name.to_string()));
            }
        }
    }
    Ok(out)
}
```

**TUI-109 plan:** add `list_all_ghost_checkpoints_stream(dir, on_item: &mut dyn FnMut(&(String, String)))`
that ticks the callback per matched ref; the existing non-streaming fn delegates
to it with a no-op callback (one source of truth).

## 2. `codelet_rpc::checkpoints::collect_checkpoints` (rust/rpc/src/checkpoints.rs:40)

```rust
pub fn collect_checkpoints(cwd: impl AsRef<Path>) -> codelet_git::Result<Vec<CheckpointInfo>> {
    let cwd = cwd.as_ref();
    let pairs = list_all_ghost_checkpoints(cwd)?;
    let mut out: Vec<CheckpointInfo> = pairs.into_iter().map(|(work_unit_id, name)| {
        let is_automatic = name.contains(AUTO_CHECKPOINT_PATTERN);
        let timestamp = read_index(cwd, &work_unit_id).as_ref()
            .and_then(|idx| lookup_timestamp(idx, &name))
            .unwrap_or_else(fallback_timestamp);
        CheckpointInfo { work_unit_id, name, timestamp, is_automatic }
    }).collect();
    out.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    out.truncate(MAX_CHECKPOINTS);   // MAX_CHECKPOINTS = 200
    Ok(out)
}
```

**TUI-109 plan:** add `collect_checkpoints_stream(cwd, on_progress: &mut dyn FnMut(CheckpointsProgress))`
driving the callback per collected item (loaded = min(count, cap), total = full
enumeration count, done at the end). `collect_checkpoints` delegates with a no-op
callback — CLI `fspec list-checkpoints` stays byte-identical.

## 3. `FspecBackend` transport trait (rust/fspec-tui/src/transport/mod.rs:84)

```rust
fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;
fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>;
fn logs_rx(&self) -> broadcast::Receiver<LogRecord>;
```

**TUI-109 plan:** add `checkpoints_progress_rx()` with a closed-receiver default
(mirrors `status_changes_rx` / `session_created_rx` at lines 933/947). Embedded
overrides to forward `SharedFspecService::checkpoints_progress_rx()`; websocket
returns the closed default (degrades to spinner-only, no new WS message kind).

## 4. `SharedFspecService` (rust/rpc/src/lib.rs:684) + `FspecServiceImpl::list_checkpoints` (rust/rpc/src/lib.rs:1038)

```rust
pub struct SharedFspecService {
    watcher: ArcSwap<WorkUnitsWatcher>,
    session_manager: Option<Arc<dyn SessionManagerHandle>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
    ...
    cwd: Option<PathBuf>,
}

async fn list_checkpoints(self, _ctx: Context) -> Vec<CheckpointInfo> {
    match self.inner.cwd() {
        Some(cwd) => checkpoints::collect_checkpoints(cwd).unwrap_or_default(),
        None => Vec::new(),
    }
}
```

**TUI-109 plan:** add `checkpoints_progress_tx: broadcast::Sender<CheckpointsProgress>`
to `SharedFspecService` (+ `checkpoints_progress_rx()` accessor);
`list_checkpoints` drives `collect_checkpoints_stream` with a callback that
publishes on the tx.

## 5. App fold points (rust/fspec-tui)

- `App::spawn_subscriber_tasks` (app/bootstrap.rs:140) — 5 subscriber tasks;
  add a 6th on `checkpoints_progress_rx()` → `Action::CheckpointsProgress`.
- `App::handle_checkpoints_loaded` (app/dispatch_checkpoints.rs:52) — the
  `CheckpointsLoaded` fold; progress fold must be stale-dropped once
  `load.is_loaded()`.
- `CheckpointsView { loading: LoadingDialog, load: LoadTracker }`
  (views/checkpoints/mod.rs:105-108) — `loading.set_progress(idx, total)`
  slot exists from TUI-106; `LoadingDialog::progress: Option<(usize, usize)>`
  renders `(idx/total)`; extend to render `(loaded/…)` when total is unknown.
