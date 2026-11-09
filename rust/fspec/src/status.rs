//! RPC-011 rule [24]: `fspec status` subcommand — one-shot health
//! against the autodiscovered (or `--connect`'d) daemon.
//!
//! Sequence:
//!   1. resolve URL: explicit `--connect` OR
//!      `common::read_and_verify_daemon_json` (deletes stale daemon.json).
//!   2. open `WebSocketFspecBackend::connect` (no supervisor — one-shot).
//!   3. call `backend.health()`.
//!   4. pretty-print multi-line output to stdout.
//!   5. exit 0 on success; exit 1 on any failure path (no daemon, stale,
//!      connect failure).

use anyhow::{Context, Result};
use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use codelet_rpc_types::HealthInfo;
use url::Url;

use crate::common;

pub async fn run(connect: Option<String>, token: Option<String>) -> Result<()> {
    // Pre-resolve the URL (so we can distinguish "no daemon.json" from
    // "stale daemon.json" in stderr text per the feature scenarios).
    let (url, bearer, ca) = match resolve_url(connect, token) {
        Ok(resolved) => resolved,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // RPC-458: a saved daemon's CA is trusted beside the roots and
    // FSPEC_DAEMON_CA.
    let backend = match WebSocketFspecBackend::connect_trusting(url.clone(), bearer, ca).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("fspec daemon: not running (connect failed: {e:#})");
            std::process::exit(1);
        }
    };

    let health = match backend.health().await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("fspec daemon: not running (health RPC failed: {e:#})");
            std::process::exit(1);
        }
    };

    print_health(&health);
    print_local_daemons();
    Ok(())
}

/// RPC-446: every live local daemon advertised in its discovery file
/// (stale entries are pruned by the listing).
fn print_local_daemons() {
    let daemons = match common::daemons_dir().and_then(|dir| common::list_live_daemons(&dir)) {
        Ok(daemons) => daemons,
        Err(e) => {
            tracing::warn!(error = %e, "failed to list local daemons");
            return;
        }
    };
    println!("local daemons: {}", daemons.len());
    for daemon in &daemons {
        println!(
            "  pid={} port={} workspace={}",
            daemon.pid,
            daemon.port,
            daemon.workspace.as_deref().unwrap_or("?")
        );
    }
}

fn resolve_url(
    explicit: Option<String>,
    token: Option<String>,
) -> Result<crate::daemons_cmd::ConnectTarget> {
    if let Some(s) = explicit {
        // RPC-447: a URL, or the name of a saved daemon (with its token).
        return crate::daemons_cmd::resolve_connect(&s, token);
    }
    let djson = common::daemon_json_path()?;
    if !djson.exists() {
        return Err(anyhow::anyhow!(
            "fspec daemon: not running (no daemon.json at {})",
            djson.display()
        ));
    }
    let handshake = match common::read_and_verify_daemon_json(&djson) {
        Ok(h) => h,
        Err(e) => {
            // The error text already contains "stale daemon.json removed"
            // when the file was deleted in the verify step.
            return Err(anyhow::anyhow!("fspec daemon: not running ({e})"));
        }
    };
    // BUG-595: the scheme and host the daemon's entry names.
    let url = Url::parse(&handshake.url()).context("synthesize the daemon url")?;
    // BUG-429: an autodiscovered local daemon takes --token, else
    // FSPEC_DAEMON_TOKEN.
    let bearer = match token {
        Some(token) => Some(token),
        None => crate::daemon_directory::local_daemon_token()?,
    };
    Ok((url, bearer, None))
}

fn print_health(h: &HealthInfo) {
    println!("fspec daemon: alive");
    println!("uptime: {}", format_uptime(h.uptime_secs));
    println!("connected_clients: {}", h.connected_clients);
    match h.last_watcher_event_secs_ago {
        Some(s) => println!("last_watcher_event: {s}s ago"),
        None => println!("last_watcher_event: never"),
    }
    println!(
        "broadcast_lag: chunks={} logs={} work_units={}",
        h.lag_chunks, h.lag_logs, h.lag_work_units
    );
    println!("version: {}", h.version);
}

fn format_uptime(secs: i64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}
