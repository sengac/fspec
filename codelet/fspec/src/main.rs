//! `fspec` binary entry point (RPC-010).
//!
//! Feature files:
//!   - spec/features/fspec-binary-combined-mode-rpc010.feature
//!   - spec/features/fspec-binary-daemon-mode-rpc010.feature
//!   - spec/features/fspec-binary-client-mode-rpc010.feature
//!   - spec/features/fspec-binary-cargo-shape-rpc010.feature
//!
//! Three modes selected by clap subcommand:
//!   `fspec`           — combined (TUI + always-on WS server)
//!   `fspec daemon`    — headless (WS server only)
//!   `fspec client`    — frontend (WebSocket backend, no service)
//!
//! Per architecture note [0]: `#[tokio::main]` drives the runtime; every
//! downstream module sources its handle from `tokio::runtime::Handle::current()`.
//! `codelet/fspec/src/` contains NO `tokio::runtime::Builder` /
//! `Runtime::new` calls (source-shape regression locked by RPC-005 Q9 and
//! widened to scan `fspec/src/` by RPC-010).

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod client;
mod combined;
mod common;
mod daemon;
mod status;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// `fspec` — combined frontend+server, daemon, or client.
#[derive(Parser, Debug)]
#[command(
    name = "fspec",
    version,
    about = "fspec — combined TUI + WS server (default), `daemon` (headless server), or `client` (frontend-only)",
    long_about = "The fspec binary runs in one of three modes selected by the subcommand:\n\n\
                  - (no subcommand)  combined mode: ratatui TUI + always-on WS server in one process\n\
                  - `daemon`         headless WS server only (suitable for systemd / launchd)\n\
                  - `client`         frontend-only; connects to a running daemon via WebSocket"
)]
struct Cli {
    /// Workspace root to observe via WorkUnitsWatcher.
    /// Defaults to the current working directory in combined and daemon modes;
    /// ignored by client mode (the daemon owns the watcher).
    #[arg(long, value_name = "PATH", global = true)]
    workspace: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Option<Mode>,
}

#[derive(Subcommand, Debug)]
enum Mode {
    /// Run as a headless WS daemon (no TUI).
    Daemon {
        /// TCP bind address (must be a loopback host — REJECTED otherwise).
        #[arg(long, value_name = "ADDR", default_value = "127.0.0.1:0")]
        bind: String,
        /// Write `pid=<u32>\nport=<u16>` to this path on bootstrap; remove on shutdown.
        #[arg(long, value_name = "PATH")]
        pidfile: Option<PathBuf>,
    },
    /// Run as a frontend client attached to a running daemon.
    Client {
        /// Explicit WS URL (e.g. `ws://127.0.0.1:12345`). When omitted, the
        /// client resolves the daemon via `$XDG_RUNTIME_DIR/fspec/daemon.json`
        /// (or `~/.fspec/daemon.json` if XDG_RUNTIME_DIR is unset).
        #[arg(long, value_name = "URL")]
        connect: Option<String>,
    },
    /// RPC-011: print live daemon health and exit.
    ///
    /// Resolves the daemon via `read_and_verify_daemon_json` (or
    /// `--connect` if supplied), opens a one-shot WebSocketFspecBackend
    /// (no supervisor), calls `health()`, pretty-prints multi-line
    /// output, exits 0 on success / 1 on any failure.
    Status {
        /// Explicit WS URL — bypasses daemon.json autodiscovery.
        #[arg(long, value_name = "URL")]
        connect: Option<String>,
    },
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let res = match cli.cmd {
        None => combined::run(cli.workspace).await,
        Some(Mode::Daemon { bind, pidfile }) => {
            daemon::run(cli.workspace, bind, pidfile).await
        }
        Some(Mode::Client { connect }) => client::run(connect).await,
        Some(Mode::Status { connect }) => status::run(connect).await,
    };
    match res {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            std::process::ExitCode::from(1)
        }
    }
}
