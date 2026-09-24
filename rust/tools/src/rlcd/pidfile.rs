//! RLCD-001 — pidfile + process-liveness helpers for local auto-spawn.
//!
//! Feature: spec/features/rlcd-service-supervisor.feature
//!
//! Split from `spawn.rs` to keep every module under the 300-line ceiling
//! (RLCD-001 architecture note). Houses the pidfile path/IO, the pid
//! liveness probe, the stale-pidfile cleanup, and the small PATH/port URL
//! helpers the spawn flow needs.

use std::path::Path;

use tracing::info;

use crate::rlcd::config::user_dir;

/// The pidfile path: `<user_dir>/rlcd/rlcd.pid` (user dir = `FSPEC_USER_DIR`
/// override, else `~/.fspec`; falls back to a CWD-relative path when no home
/// can be resolved).
#[must_use]
pub fn pidfile_path() -> std::path::PathBuf {
    user_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".fspec"))
        .join("rlcd")
        .join("rlcd.pid")
}

/// Read the pid recorded in `pidfile`. `None` when the file is absent,
/// unreadable, or does not contain a u32.
#[must_use]
pub fn read_pidfile(pidfile: &Path) -> Option<u32> {
    let content = std::fs::read_to_string(pidfile).ok()?;
    content.trim().parse::<u32>().ok()
}

/// Whether a process with this pid currently exists. Probes with the `kill`
/// binary and signal 0 (no signal is delivered — existence/permission check
/// only): exit success or "operation not permitted" means the process
/// exists; "no such process" (or the probe itself failing) means it is
/// dead. A brief blocking probe; callers run it off hot paths.
#[must_use]
pub fn is_pid_alive(pid: u32) -> bool {
    let output = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok();
    match output {
        Some(out) if out.status.success() => true,
        Some(out) => {
            // Distinguish EPERM (alive, other user) from ESRCH (dead) via
            // the platform error text in stderr.
            let stderr = String::from_utf8_lossy(&out.stderr);
            stderr.to_lowercase().contains("not permitted")
                || stderr.to_lowercase().contains("permission denied")
        }
        None => {
            // `kill` binary unavailable (minimal container): fall back to a
            // /proc probe on Linux; on other platforms assume dead (the
            // spawn path degrades to connect-only there anyway).
            #[cfg(target_os = "linux")]
            {
                std::path::Path::new(&format!("/proc/{pid}")).exists()
            }
            #[cfg(not(target_os = "linux"))]
            {
                false
            }
        }
    }
}

/// Remove a pidfile whose recorded pid no longer exists (stale cleanup).
/// A live pid is left in place (the server is still loading — never
/// double-spawn).
pub fn remove_stale_pidfile(pidfile: &Path) {
    match read_pidfile(pidfile) {
        Some(pid) if is_pid_alive(pid) => {} // live: keep it
        Some(_) => {
            let _ = std::fs::remove_file(pidfile);
            info!(path = %pidfile.display(), "rlcd: removed stale pidfile");
        }
        None => {}
    }
}

/// Whether `url` points at a local host (spawn-eligible). Only
/// 127.0.0.1 / localhost qualify; anything else is connect-only.
#[must_use]
pub fn is_local_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    matches!(
        parsed.host_str(),
        Some("127.0.0.1" | "localhost" | "[::1]")
    )
}

/// The port encoded in `url` (falls back to 8000 when it does not parse).
pub fn url_port(url: &str) -> u16 {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.port())
        .unwrap_or(8000)
}

/// The local server URL for a spawn port on the configured bind address.
/// Used to persist `rlcd.url` after a spawn/reuse lands on a port other
/// than the configured one (the 2026-09-24 re-anchor regression).
#[must_use]
pub fn spawn_url_of(bind_address: &str, port: u16) -> String {
    format!("http://{bind_address}:{port}")
}

/// Resolve a binary name against `PATH`.
pub fn resolve_binary(bin: &str) -> Option<std::path::PathBuf> {
    let var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&var) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
