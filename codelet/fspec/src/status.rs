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

pub async fn run(connect: Option<String>) -> Result<()> {
    // Pre-resolve the URL (so we can distinguish "no daemon.json" from
    // "stale daemon.json" in stderr text per the feature scenarios).
    let url = match resolve_url(connect) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let backend = match WebSocketFspecBackend::connect(url.clone()).await {
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
    Ok(())
}

fn resolve_url(explicit: Option<String>) -> Result<Url> {
    if let Some(s) = explicit {
        return Url::parse(&s).with_context(|| format!("--connect URL parse: {s}"));
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
            return Err(anyhow::anyhow!(
                "fspec daemon: not running ({e})"
            ));
        }
    };
    Url::parse(&format!("ws://127.0.0.1:{}", handshake.port)).context("synthesize ws url")
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
