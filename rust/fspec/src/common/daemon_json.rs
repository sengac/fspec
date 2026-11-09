//! daemon.json autodiscovery (RPC-010/RPC-011): atomic write, idempotent
//! remove, and verified reads with a stale-pid check. Re-exported from
//! `common`; split out of `common.rs` to keep it under its size cap (BUG-292).

use super::daemon_listener::{daemon_url, host_from_entry, DaemonListener, DaemonScheme};
use super::home_fspec_dir;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// RPC-011 rule [21] / scenario "daemon.json schema upgrade": the
/// authoritative client-side view of daemon.json. `read_and_verify_daemon_json`
/// returns this on success.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct DaemonHandshake {
    pub port: u16,
    pub pid: u32,
    pub started_at: Option<String>,
    pub version: Option<String>,
    /// BUG-595: the scheme the daemon serves.
    pub scheme: DaemonScheme,
    /// BUG-595: the host a local client dials.
    pub host: String,
}

impl DaemonHandshake {
    /// BUG-595: the URL a discovery-based client dials.
    pub fn url(&self) -> String {
        daemon_url(self.scheme, &self.host, self.port)
    }
}

/// Resolve `daemon.json` path: `$XDG_RUNTIME_DIR/fspec/daemon.json` if
/// set, else `~/.fspec/daemon.json`.
pub fn daemon_json_path() -> Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            return Ok(p.join("fspec").join("daemon.json"));
        }
    }
    Ok(home_fspec_dir()?.join("daemon.json"))
}

/// Atomic write daemon.json (temp+rename). RPC-011 schema: includes
/// pid + started_at (ISO 8601) + version (CARGO_PKG_VERSION).
pub fn write_daemon_json(
    path: &Path,
    port: u16,
    pid: u32,
    workspace: &Path,
    started_at: std::time::SystemTime,
    listener: &DaemonListener,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let started_at_iso = system_time_iso8601(started_at);
    let body = serde_json::json!({
        "port": port,
        "pid": pid,
        "workspace": workspace.canonicalize().unwrap_or_else(|_| workspace.to_path_buf()),
        "started_at": started_at_iso,
        "version": env!("CARGO_PKG_VERSION"),
        // BUG-595: appended; readers take an absent field as ws / 127.0.0.1.
        "scheme": listener.scheme.as_str(),
        "host": listener.host,
    });
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    f.write_all(serde_json::to_string_pretty(&body)?.as_bytes())?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Render a `SystemTime` as an ISO 8601 / RFC 3339 UTC string with
/// second precision (e.g. `2026-05-11T12:34:56Z`). Avoids a
/// chrono dep by going through humantime's RFC3339 formatter.
fn system_time_iso8601(t: std::time::SystemTime) -> String {
    let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs() as i64;
    // Minimal RFC 3339 formatter — pure stdlib so no chrono dep.
    let days_from_epoch = secs / 86_400;
    let mut secs_in_day = (secs % 86_400 + 86_400) % 86_400;
    let hour = secs_in_day / 3_600;
    secs_in_day %= 3_600;
    let minute = secs_in_day / 60;
    let second = secs_in_day % 60;
    let (year, month, day) = civil_from_days(days_from_epoch);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's days_from_civil inverse, in pure stdlib.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Idempotent remove daemon.json.
pub fn remove_daemon_json(path: &Path) {
    let _ = fs::remove_file(path);
}

/// Read daemon.json and return the `port` field.
///
/// RPC-011 rule [20] keeps this function as a legacy accessor that
/// delegates to `read_and_verify_daemon_json`. Both production
/// callers (`client.rs` and `status.rs`) use `read_and_verify_daemon_json`
/// directly so they can capture the full `DaemonHandshake` struct — but
/// the legacy port-only accessor is preserved so out-of-tree callers
/// (and future scripts) can still extract just the port without pulling
/// in serde_json.
#[allow(dead_code)]
pub fn read_daemon_json_port(path: &Path) -> Result<u16> {
    let handshake = read_and_verify_daemon_json(path)?;
    Ok(handshake.port)
}

/// RPC-011 rule [21]: read daemon.json, parse, then probe `kill(pid, None)`
/// (unix) / `GetExitCodeProcess` (windows) BEFORE trusting any URL.
/// On stale (dead pid) delete the file and return Err with the stable
/// text "no daemon.json found" so callers (client.rs, status.rs) can
/// match it.
pub fn read_and_verify_daemon_json(path: &Path) -> Result<DaemonHandshake> {
    let body = match fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(anyhow!(
                "no daemon.json found at {} — start `fspec daemon` first",
                path.display()
            ));
        }
        Err(e) => {
            return Err(
                anyhow::Error::from(e).context(format!("read daemon.json at {}", path.display()))
            );
        }
    };
    let parsed: serde_json::Value = serde_json::from_str(&body).context("parse daemon.json")?;
    let port = parsed
        .get("port")
        .and_then(|p| p.as_u64())
        .ok_or_else(|| anyhow!("daemon.json missing or invalid `port` field"))?
        as u16;
    let pid = parsed
        .get("pid")
        .and_then(|p| p.as_u64())
        .ok_or_else(|| anyhow!("daemon.json missing or invalid `pid` field"))? as u32;
    let started_at = parsed
        .get("started_at")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let version = parsed
        .get("version")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let scheme = DaemonScheme::from_entry(parsed.get("scheme"))
        .with_context(|| format!("daemon.json at {}", path.display()))?;
    let host = host_from_entry(parsed.get("host"))
        .with_context(|| format!("daemon.json at {}", path.display()))?;

    if !pid_is_alive(pid) {
        // Stale — delete the file and tell callers no daemon is running.
        let _ = fs::remove_file(path);
        return Err(anyhow!(
            "no daemon.json found (stale daemon.json removed at {}) — start `fspec daemon` first",
            path.display()
        ));
    }

    Ok(DaemonHandshake {
        port,
        pid,
        started_at,
        version,
        scheme,
        host,
    })
}

/// Probe whether `pid` is alive without sending a real signal.
#[cfg(unix)]
pub(super) fn pid_is_alive(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    match kill(Pid::from_raw(pid as i32), None) {
        Ok(()) => true,
        Err(nix::errno::Errno::EPERM) => true, // owned by another user but exists
        Err(_) => false,
    }
}

#[cfg(not(unix))]
pub(super) fn pid_is_alive(_pid: u32) -> bool {
    // RPC-011 windows support is best-effort — we conservatively
    // accept the daemon.json. A future card adds GetExitCodeProcess.
    true
}
