// RPC-446: per-daemon local discovery files. Every running local daemon
// writes `<daemons_dir>/<pid>.json` (the daemon.json schema) beside the
// default-daemon pointer and removes it on shutdown; listing keeps only the
// entries whose process is alive and deletes the stale ones. Included into
// `common` (not a child module) so test binaries that `#[path]`-include
// common.rs see the same items.

/// One local daemon advertised in its discovery file.
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
#[serde(try_from = "RawDaemonEntry")]
pub struct DaemonEntry {
    pub port: u16,
    pub pid: u32,
    pub workspace: Option<String>,
    pub started_at: Option<String>,
    pub version: Option<String>,
    /// BUG-595: the scheme the daemon serves.
    pub scheme: DaemonScheme,
    /// BUG-595: the host a local client dials.
    pub host: String,
}

impl DaemonEntry {
    /// BUG-595: the URL a discovery-based client dials.
    pub fn url(&self) -> String {
        DaemonListener {
            scheme: self.scheme,
            host: self.host.clone(),
        }
        .url(self.port)
    }
}

/// A discovery file as written, before the BUG-595 fields are resolved.
#[derive(serde::Deserialize)]
struct RawDaemonEntry {
    port: u16,
    pid: u32,
    workspace: Option<String>,
    started_at: Option<String>,
    version: Option<String>,
    scheme: Option<serde_json::Value>,
    host: Option<serde_json::Value>,
}

impl TryFrom<RawDaemonEntry> for DaemonEntry {
    type Error = String;

    fn try_from(raw: RawDaemonEntry) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            port: raw.port,
            pid: raw.pid,
            workspace: raw.workspace,
            started_at: raw.started_at,
            version: raw.version,
            scheme: DaemonScheme::from_entry(raw.scheme.as_ref()).map_err(|e| e.to_string())?,
            host: host_from_entry(raw.host.as_ref()).map_err(|e| e.to_string())?,
        })
    }
}

/// The `daemons/` directory beside daemon.json
/// (`$XDG_RUNTIME_DIR/fspec/daemons` or `~/.fspec/daemons`).
pub fn daemons_dir() -> Result<PathBuf> {
    let pointer = daemon_json_path()?;
    let parent = pointer
        .parent()
        .ok_or_else(|| anyhow!("daemon.json path has no parent: {}", pointer.display()))?;
    Ok(parent.join("daemons"))
}

fn daemon_entry_path(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("{pid}.json"))
}

/// Write this daemon's discovery file (atomic, daemon.json schema).
pub fn write_daemon_entry(
    dir: &Path,
    port: u16,
    pid: u32,
    workspace: &Path,
    started_at: std::time::SystemTime,
    listener: &DaemonListener,
) -> Result<()> {
    write_daemon_json(
        &daemon_entry_path(dir, pid),
        port,
        pid,
        workspace,
        started_at,
        listener,
    )
}

/// Remove this daemon's discovery file (idempotent).
pub fn remove_daemon_entry(dir: &Path, pid: u32) {
    remove_daemon_json(&daemon_entry_path(dir, pid));
}

/// Every local daemon whose process is alive, ordered by pid. Entries for
/// dead processes and unreadable entries are deleted.
pub fn list_live_daemons(dir: &Path) -> Result<Vec<DaemonEntry>> {
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("read {}", dir.display())),
    };
    let mut live = Vec::new();
    for entry in read_dir {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let parsed = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<DaemonEntry>(&text).ok());
        match parsed {
            Some(daemon) if daemon_json::pid_is_alive(daemon.pid) => live.push(daemon),
            _ => {
                let _ = fs::remove_file(&path);
            }
        }
    }
    live.sort_by_key(|daemon| daemon.pid);
    Ok(live)
}

/// The pid recorded in a daemon.json-schema file, if it can be read.
fn pointer_pid(pointer: &Path) -> Option<u32> {
    let body: serde_json::Value = serde_json::from_str(&fs::read_to_string(pointer).ok()?).ok()?;
    body.get("pid")?.as_u64()?.try_into().ok()
}

/// BUG-289: write daemon.json for this daemon unless it already names
/// another live daemon. Returns whether this daemon took the default; when
/// it did not, the daemon stays discoverable through its `daemons/` entry.
pub fn claim_default_pointer(
    pointer: &Path,
    port: u16,
    pid: u32,
    workspace: &Path,
    started_at: std::time::SystemTime,
    listener: &DaemonListener,
) -> Result<bool> {
    if let Some(owner) = pointer_pid(pointer) {
        if owner != pid && daemon_json::pid_is_alive(owner) {
            tracing::warn!(
                owner,
                pointer = %pointer.display(),
                "daemon.json already names a live daemon; keeping it (this daemon is listed in daemons/)"
            );
            return Ok(false);
        }
    }
    write_daemon_json(pointer, port, pid, workspace, started_at, listener)?;
    Ok(true)
}

/// BUG-289: on shutdown, remove daemon.json only when it names this daemon,
/// then point it at another live daemon from `daemons_dir`, if any, so the
/// remaining daemons stay discoverable by default.
pub fn release_default_pointer(pointer: &Path, daemons_dir: &Path, pid: u32) -> Result<()> {
    if pointer_pid(pointer) != Some(pid) {
        return Ok(());
    }
    remove_daemon_json(pointer);
    let Some(next) = list_live_daemons(daemons_dir)?
        .into_iter()
        .find(|daemon| daemon.pid != pid)
    else {
        return Ok(());
    };
    let tmp = pointer.with_extension("json.tmp");
    fs::copy(daemon_entry_path(daemons_dir, next.pid), &tmp)
        .with_context(|| format!("copy daemon entry for {}", next.pid))?;
    fs::rename(&tmp, pointer)
        .with_context(|| format!("rename {} -> {}", tmp.display(), pointer.display()))?;
    tracing::info!(pid = next.pid, port = next.port, "daemon.json handed to another live daemon");
    Ok(())
}
