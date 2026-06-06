//! `fspec` binary entry point (RPC-010).
//!
//! Feature files:
//!   - spec/features/fspec-binary-combined-mode-rpc010.feature
//!   - spec/features/fspec-binary-daemon-mode-rpc010.feature
//!   - spec/features/fspec-binary-client-mode-rpc010.feature
//!   - spec/features/fspec-binary-cargo-shape-rpc010.feature
//!   - spec/features/list-work-units-cli-subcommand.feature  (RPC-253)
//!
//! Modes selected by clap subcommand. Per architecture note [0]:
//! `#[tokio::main]` drives the runtime; every downstream module sources
//! its handle from `tokio::runtime::Handle::current()`.
//! `codelet/fspec/src/` contains NO `tokio::runtime::Builder` /
//! `Runtime::new` calls (source-shape regression locked by RPC-005 Q9
//! and widened to scan `fspec/src/` by RPC-010).

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod client;
mod combined;
mod common;
mod daemon;
mod list_attachments;
mod list_epics;
mod list_feature_tags;
mod list_features;
mod list_foundation_sections;
mod list_hooks;
mod list_prefixes;
mod list_scenario_tags;
mod list_schedules;
mod list_tags;
mod list_virtual_hooks;
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
                  - `list-*`            shell-facing ports of the TypeScript Commander.js commands\n\
                                        (RPC-241, 243-253 — see `--help` on each subcommand)"
)]
struct Cli {
    /// Workspace root to observe via WorkUnitsWatcher.
    /// Defaults to CWD in combined and daemon modes; ignored by client mode.
    /// NOT clap `global = true` — list-* subcommands resolve project_root
    /// from CWD and must not advertise `--workspace`.
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
        /// Explicit WS URL (e.g. `ws://127.0.0.1:12345`). When omitted,
        /// client resolves daemon via `$XDG_RUNTIME_DIR/fspec/daemon.json`.
        #[arg(long, value_name = "URL")]
        connect: Option<String>,
    },
    /// RPC-011: print live daemon health and exit.
    Status {
        /// Explicit WS URL — bypasses daemon.json autodiscovery.
        #[arg(long, value_name = "URL")]
        connect: Option<String>,
    },
    /// RPC-253: list work units from `spec/work-units.json`. Delegates to
    /// `fspec_core::commands::list_work_units::run` for two-front-doors parity.
    #[command(name = "list-work-units")]
    ListWorkUnits {
        /// Filter by status (e.g. `backlog`, `implementing`).
        #[arg(short, long, value_name = "STATUS")]
        status: Option<String>,
        /// Filter by work-unit ID prefix (e.g. `AUTH`); dispatcher appends `-`.
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
    /// RPC-248: list registered prefixes from `spec/prefixes.json`.
    #[command(name = "list-prefixes", about = "List all prefixes")]
    ListPrefixes,
    /// RPC-243: list registered epics from `spec/epics.json`.
    #[command(name = "list-epics", about = "List all epics")]
    ListEpics,
    /// RPC-251: list registered tags from `spec/tags.json`.
    #[command(name = "list-tags", about = "List all registered tags")]
    ListTags {
        /// Filter tags by category (exact-match against category name).
        #[arg(long, value_name = "CATEGORY")]
        category: Option<String>,
    },
    /// RPC-245: list Gherkin feature files under `spec/features/`.
    #[command(name = "list-features", about = "List all feature files")]
    ListFeatures {
        /// Filter features whose top-level tag set includes this tag (e.g. `@critical`).
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,
    },
    /// RPC-241: list attachments for a work unit from `spec/work-units.json`.
    #[command(name = "list-attachments", about = "List attachments for a work unit")]
    ListAttachments {
        /// Required work-unit identifier (e.g. `AUTH-001`).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
    },
    /// RPC-247: list configured lifecycle hooks from `spec/fspec-hooks.json`.
    #[command(name = "list-hooks", about = "List all configured lifecycle hooks")]
    ListHooks,
    /// RPC-250: list scheduled jobs from `spec/schedules.json`.
    #[command(name = "list-schedules", about = "List all configured scheduled jobs")]
    ListSchedules {
        /// Emit the JSON payload instead of the tab-separated text
        /// table. Maps to the dispatcher `format: "json"` key.
        #[arg(long)]
        json: bool,
    },
    /// RPC-246: list canonical foundation sections.
    #[command(
        name = "list-foundation-sections",
        about = "List canonical foundation sections with their JSON paths and constraints"
    )]
    ListFoundationSections {
        /// Output format: `text` (default) or `json`.
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-244: list all tags applied to scenarios in a feature file.
    #[command(
        name = "list-feature-tags",
        about = "List feature/scenario tags from a single feature file"
    )]
    ListFeatureTags {
        /// Required path to the .feature file (positional).
        #[arg(value_name = "FILE")]
        file: String,
        /// Group tags by category instead of flat alphabetical list.
        #[arg(long)]
        show_categories: bool,
    },
    /// RPC-249: list tags applied to a specific Scenario in a feature file.
    #[command(
        name = "list-scenario-tags",
        about = "List tags applied to a specific Scenario in a feature file"
    )]
    ListScenarioTags {
        /// Required path to the .feature file (positional).
        #[arg(value_name = "FILE")]
        file: String,
        /// Required Scenario name (positional).
        #[arg(value_name = "SCENARIO")]
        scenario: String,
        /// Group tags by category instead of flat alphabetical list.
        #[arg(long)]
        show_categories: bool,
    },
    /// RPC-252: list virtual (work-unit-scoped) hooks for a work unit.
    #[command(
        name = "list-virtual-hooks",
        about = "List virtual hooks for a work unit"
    )]
    ListVirtualHooks {
        /// Required work-unit identifier (e.g. `AUTH-001`).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
    },
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    // All list-* bridge arms share the same exit-code contract: delegate
    // to the bridge module's `run`, propagate the returned code verbatim,
    // on `Err` print the anyhow chain to stderr and exit 1.
    macro_rules! forward {
        ($bridge:path, $args:expr) => {
            match $bridge($args).await {
                Ok(code) => return std::process::ExitCode::from(code),
                Err(err) => {
                    eprintln!("{err:#}");
                    return std::process::ExitCode::from(1);
                }
            }
        };
    }

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
        }) => forward!(
            list_work_units::run,
            list_work_units::CliArgs {
                status,
                prefix,
                epic,
                r#type,
                format: Some(format),
            }
        ),
        Some(Mode::ListPrefixes) => {
            forward!(list_prefixes::run, list_prefixes::CliArgs::default())
        }
        Some(Mode::ListEpics) => forward!(list_epics::run, list_epics::CliArgs::default()),
        Some(Mode::ListTags { category }) => {
            forward!(list_tags::run, list_tags::CliArgs { category })
        }
        Some(Mode::ListFeatures { tag }) => {
            forward!(list_features::run, list_features::CliArgs { tag })
        }
        Some(Mode::ListAttachments { work_unit_id }) => forward!(
            list_attachments::run,
            list_attachments::CliArgs { work_unit_id }
        ),
        Some(Mode::ListHooks) => forward!(list_hooks::run, list_hooks::CliArgs::default()),
        Some(Mode::ListSchedules { json }) => {
            forward!(list_schedules::run, list_schedules::CliArgs { json })
        }
        Some(Mode::ListFoundationSections { format }) => forward!(
            list_foundation_sections::run,
            list_foundation_sections::CliArgs { format }
        ),
        Some(Mode::ListFeatureTags {
            file,
            show_categories,
        }) => forward!(
            list_feature_tags::run,
            list_feature_tags::CliArgs {
                file,
                show_categories,
            }
        ),
        Some(Mode::ListScenarioTags {
            file,
            scenario,
            show_categories,
        }) => forward!(
            list_scenario_tags::run,
            list_scenario_tags::CliArgs {
                file,
                scenario,
                show_categories,
            }
        ),
        Some(Mode::ListVirtualHooks { work_unit_id }) => forward!(
            list_virtual_hooks::run,
            list_virtual_hooks::CliArgs { work_unit_id }
        ),
    };
    match res {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            std::process::ExitCode::from(1)
        }
    }
}
