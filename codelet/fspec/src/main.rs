//! `fspec` binary entry point (RPC-010).
//!
//! Feature files:
//!   - spec/features/fspec-binary-combined-mode-rpc010.feature
//!   - spec/features/fspec-binary-daemon-mode-rpc010.feature
//!   - spec/features/fspec-binary-client-mode-rpc010.feature
//!   - spec/features/fspec-binary-cargo-shape-rpc010.feature
//!   - spec/features/list-work-units-cli-subcommand.feature  (RPC-253)
//!
//! Modes selected by clap subcommand:
//!   `fspec`              — combined (TUI + always-on WS server)
//!   `fspec daemon`       — headless (WS server only)
//!   `fspec client`       — frontend (WebSocket backend, no service)
//!   `fspec list-work-units` — RPC-253: shell-facing port of the TS
//!     Commander.js `list-work-units` command. Delegates to
//!     `fspec_core::commands::list_work_units::run` so the agent-loop
//!     dispatcher and the shell CLI share a single source of truth.
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
mod list_work_units;
mod status;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// `fspec` — combined frontend+server, daemon, or client.
#[derive(Parser, Debug)]
#[command(
    name = "fspec",
    version,
    about = "fspec — combined TUI + WS server (default), `daemon` (headless server), or `client` (frontend-only)",
    long_about = "The fspec binary runs in one of several modes selected by the subcommand:\n\n\
                  - (no subcommand)     combined mode: ratatui TUI + always-on WS server in one process\n\
                  - `daemon`            headless WS server only (suitable for systemd / launchd)\n\
                  - `client`            frontend-only; connects to a running daemon via WebSocket\n\
                  - `status`            one-shot health probe against the running daemon\n\
                  - `list-work-units`   shell-facing port of the TypeScript Commander.js command (RPC-253)"
)]
struct Cli {
    /// Workspace root to observe via WorkUnitsWatcher.
    /// Defaults to the current working directory in combined and daemon modes;
    /// ignored by client mode (the daemon owns the watcher).
    ///
    /// NOT a clap-`global = true` flag: subcommands that resolve the
    /// project root from CWD (e.g. `list-work-units` per rule [15] on
    /// RPC-253) MUST NOT advertise `--workspace` in their `--help` output,
    /// because they neither read nor honour it. Only the default
    /// (combined) mode and the `daemon` subcommand consume `cli.workspace`.
    #[arg(long, value_name = "PATH")]
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
    /// RPC-253: list work units from `spec/work-units.json`.
    ///
    /// Shell-facing port of the TypeScript Commander.js
    /// `list-work-units` command (`src/commands/list-work-units.ts`).
    /// Flags mirror that surface 1:1 — see rule [11] on RPC-253. The
    /// action arm delegates to
    /// `fspec_core::commands::list_work_units::run` so the LLM-facing
    /// dispatcher and the shell CLI share a single source of truth
    /// (architecture note [7] on RPC-253).
    #[command(name = "list-work-units")]
    ListWorkUnits {
        /// Filter by status (e.g. `backlog`, `implementing`).
        #[arg(short, long, value_name = "STATUS")]
        status: Option<String>,
        /// Filter by work-unit ID prefix (e.g. `AUTH`); the dispatcher
        /// appends `-` before the startsWith comparison.
        #[arg(short, long, value_name = "PREFIX")]
        prefix: Option<String>,
        /// Filter by epic slug (exact match).
        #[arg(short, long, value_name = "EPIC")]
        epic: Option<String>,
        /// Filter by work unit type: story, task, or bug.
        #[arg(short = 't', long, value_name = "TYPE")]
        r#type: Option<String>,
        /// Output format: `text` (default) or `json`.
        #[arg(long, value_name = "FORMAT", default_value = "text")]
        format: String,
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
        Some(Mode::ListWorkUnits {
            status,
            prefix,
            epic,
            r#type,
            format,
        }) => {
            // The list-work-units subcommand owns its own exit-code
            // contract (0 on success, 1 on FspecCoreError per rule [14]
            // on RPC-253). Return the resolved code via the early
            // `process::exit` path so the success-Ok arm below does not
            // re-map it to ExitCode::SUCCESS.
            let args = list_work_units::CliArgs {
                status,
                prefix,
                epic,
                r#type,
                format: Some(format),
            };
            return match list_work_units::run(args).await {
                Ok(code) => std::process::ExitCode::from(code),
                Err(err) => {
                    eprintln!("{err:#}");
                    std::process::ExitCode::from(1)
                }
            };
        }
    };
    match res {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            std::process::ExitCode::from(1)
        }
    }
}
