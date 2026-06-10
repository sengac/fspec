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
mod list_checkpoints;
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
mod query_bottlenecks;
mod query_dependency_stats;
mod query_estimate_accuracy;
mod query_estimation_guide;
mod query_example_mapping_stats;
mod query_metrics;
mod query_orphans;
mod query_work_units;
mod show_acceptance_criteria;
mod show_coverage;
mod show_deleted;
mod show_epic;
mod show_event_storm;
mod show_feature;
mod show_foundation;
mod show_foundation_event_storm;
mod show_test_patterns;
mod show_work_unit;
mod status;
mod tag_stats;

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
                  - `status`            one-shot health probe against the running daemon\n\n\
                  All remaining subcommands are the main fspec CLI commands. For details on any\n\
                  individual command, run `fspec <command> --help`."
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
    #[command(about = "One-shot health probe against the running daemon")]
    Status {
        /// Explicit WS URL — bypasses daemon.json autodiscovery.
        #[arg(long, value_name = "URL")]
        connect: Option<String>,
    },
    /// RPC-253: list work units from `spec/work-units.json`. Delegates to
    /// `fspec_core::commands::list_work_units::run` for two-front-doors parity.
    #[command(
        name = "list-work-units",
        about = "List work units (filter by status, prefix, epic, or type)"
    )]
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
    /// RPC-242: list checkpoints (git stashes) saved for a work unit.
    #[command(
        name = "list-checkpoints",
        about = "List all checkpoints for a work unit"
    )]
    ListCheckpoints {
        /// Required work-unit identifier (e.g. `AUTH-001`).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
    },
    /// RPC-301: show soft-deleted items for a work unit.
    #[command(name = "show-deleted", about = "Show all soft-deleted items for a work unit")]
    ShowDeleted {
        /// Required work-unit identifier (e.g. `AUTH-001`).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
    },
    /// RPC-302: show details and progress for an epic.
    #[command(name = "show-epic", about = "Show epic details and progress")]
    ShowEpic {
        /// Required epic identifier.
        #[arg(value_name = "EPIC_ID")]
        epic_id: String,
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-304: show the contents of a feature file with work-unit annotations.
    #[command(name = "show-feature", about = "Show feature file contents")]
    ShowFeature {
        /// Required feature file path or basename.
        #[arg(value_name = "FEATURE")]
        feature: String,
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
        /// Optional output file path.
        #[arg(short = 'o', long, value_name = "PATH")]
        output: Option<String>,
    },
    /// RPC-310: print tag usage statistics across the project.
    #[command(name = "tag-stats", about = "Show tag usage statistics across feature files")]
    TagStats,
    /// RPC-308: show full details of a single work unit.
    #[command(name = "show-work-unit", about = "Show full details of a single work unit")]
    ShowWorkUnit {
        /// Required work-unit identifier (e.g. `AUTH-001`).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-257: print dependency relationship statistics.
    #[command(
        name = "query-dependency-stats",
        about = "Query dependency relationship statistics"
    )]
    QueryDependencyStats {
        /// Output format: `text` (default — silent) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-258: print estimate accuracy aggregated across completed work units.
    #[command(
        name = "query-estimate-accuracy",
        about = "Query estimate accuracy across completed work units"
    )]
    QueryEstimateAccuracy {
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-261: print velocity / iteration / cycle-time metrics.
    #[command(
        name = "query-metrics",
        about = "Query velocity, iteration, and cycle-time metrics"
    )]
    QueryMetrics {
        /// `--work-unit-id <id>` — query metrics for a single unit.
        #[arg(long = "work-unit-id", value_name = "WORK_UNIT_ID")]
        work_unit_id: Option<String>,
        /// `--type <type>` — filter aggregate metrics to `story`, `task`, `bug`.
        #[arg(long = "type", value_name = "TYPE")]
        r#type: Option<String>,
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-263: query work units with rich filtering.
    #[command(
        name = "query-work-units",
        about = "Query work units with filters (status, prefix, epic, type, tag)"
    )]
    QueryWorkUnits {
        /// Filter by status.
        #[arg(short = 's', long, value_name = "STATUS")]
        status: Option<String>,
        /// Filter by prefix.
        #[arg(short = 'p', long, value_name = "PREFIX")]
        prefix: Option<String>,
        /// Filter by epic.
        #[arg(short = 'e', long, value_name = "EPIC")]
        epic: Option<String>,
        /// Filter by type.
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        r#type: Option<String>,
        /// Filter by tag.
        #[arg(long, value_name = "TAG")]
        tag: Option<String>,
        /// Output format: `text` (default), `json`, or `csv`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-256: print bottleneck analysis for work-unit dependency graph.
    #[command(name = "query-bottlenecks", about = "Identify dependency bottlenecks")]
    QueryBottlenecks {
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'o', long, value_name = "FORMAT")]
        output: Option<String>,
    },
    /// RPC-262: print orphan work units (no dependencies in either direction).
    #[command(name = "query-orphans", about = "Show orphan work units")]
    QueryOrphans {
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'o', long, value_name = "FORMAT")]
        output: Option<String>,
        /// Filter out work units in done status.
        #[arg(long = "exclude-done")]
        exclude_done: bool,
    },
    /// RPC-259: print estimation guide for a single work unit.
    #[command(
        name = "query-estimation-guide",
        about = "Show estimation guide for a work unit"
    )]
    QueryEstimationGuide {
        /// Work unit ID.
        work_unit_id: String,
        /// Output format: `text` (default — silent) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-260: print Example Mapping coverage statistics.
    #[command(
        name = "query-example-mapping-stats",
        about = "Show Example Mapping coverage statistics"
    )]
    QueryExampleMappingStats {
        /// Output format: `text` (default — silent) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
    },
    /// RPC-303: show Event Storm artifacts for a work unit.
    #[command(
        name = "show-event-storm",
        about = "Show Event Storm artifacts for a work unit"
    )]
    ShowEventStorm {
        /// Work unit ID.
        work_unit_id: String,
    },
    /// RPC-305: show contents of foundation.json (or a single section).
    #[command(name = "show-foundation", about = "Show foundation.json contents")]
    ShowFoundation {
        /// Optional positional section name (e.g. `whatWeAreBuilding`).
        section: Option<String>,
        /// Output format: `text` (default), `json`, or `markdown`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
        /// Write output to file instead of stdout.
        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<String>,
        /// Read from `foundation.json.draft` instead of `foundation.json`.
        #[arg(long)]
        draft: bool,
        /// Accepted for TS parity; lists all valid sections.
        #[arg(long = "list-sections")]
        list_sections: bool,
        /// Accepted for TS parity; no behaviour change.
        #[arg(long = "line-numbers")]
        line_numbers: bool,
    },
    /// RPC-306: show foundation-level Event Storm.
    #[command(
        name = "show-foundation-event-storm",
        about = "Show foundation-level Event Storm"
    )]
    ShowFoundationEventStorm {
        /// Filter by type: `event`, `command`, `aggregate`, etc.
        #[arg(long = "type", value_name = "TYPE")]
        r#type: Option<String>,
        /// Filter by bounded context name.
        #[arg(long, value_name = "CONTEXT")]
        context: Option<String>,
    },
    /// RPC-307: show test patterns across coverage files for a tag.
    #[command(
        name = "show-test-patterns",
        about = "Show test patterns for a tag"
    )]
    ShowTestPatterns {
        /// Tag to filter on (required).
        #[arg(long, value_name = "TAG")]
        tag: String,
        /// Include coverage data in output.
        #[arg(long = "include-coverage")]
        include_coverage: bool,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// RPC-299: show acceptance criteria filtered by tags.
    #[command(
        name = "show-acceptance-criteria",
        about = "Show acceptance criteria filtered by tags"
    )]
    ShowAcceptanceCriteria {
        /// Tag filter (repeatable; AND semantics).
        #[arg(long, value_name = "TAG")]
        tag: Vec<String>,
        /// Output format: `text` (default), `markdown`, or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT", default_value = "text")]
        format: String,
        /// Write to file instead of stdout.
        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<String>,
    },
    /// RPC-300: show coverage report for a feature or project-wide.
    #[command(
        name = "show-coverage",
        about = "Show coverage report for a feature (or project-wide)"
    )]
    ShowCoverage {
        /// Optional feature name (with or without `.feature` extension).
        feature_name: Option<String>,
        /// Output format: `text` (default) or `json`.
        #[arg(short = 'f', long, value_name = "FORMAT")]
        format: Option<String>,
        /// Write output to file instead of stdout.
        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // RPC-247 / strict byte-parity: intercept `fspec <list-*> --help` before
    // clap parses, so we can emit the TS-formatted help block instead of
    // clap's auto-generated one. Returns Some(exit_code) when handled.
    if let Some(code) = intercept_ts_help() {
        return std::process::ExitCode::from(code);
    }

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
        Some(Mode::ListCheckpoints { work_unit_id }) => forward!(
            list_checkpoints::run,
            list_checkpoints::CliArgs { work_unit_id }
        ),
        Some(Mode::ShowDeleted { work_unit_id }) => forward!(
            show_deleted::run,
            show_deleted::CliArgs { work_unit_id }
        ),
        Some(Mode::ShowEpic { epic_id, format }) => forward!(
            show_epic::run,
            show_epic::CliArgs { epic_id, format }
        ),
        Some(Mode::ShowFeature {
            feature,
            format,
            output,
        }) => forward!(
            show_feature::run,
            show_feature::CliArgs {
                feature,
                format,
                output,
            }
        ),
        Some(Mode::TagStats) => forward!(tag_stats::run, tag_stats::CliArgs::default()),
        Some(Mode::ShowWorkUnit { work_unit_id, format }) => forward!(
            show_work_unit::run,
            show_work_unit::CliArgs { work_unit_id, format }
        ),
        Some(Mode::QueryDependencyStats { format }) => forward!(
            query_dependency_stats::run,
            query_dependency_stats::CliArgs { format }
        ),
        Some(Mode::QueryEstimateAccuracy { format }) => forward!(
            query_estimate_accuracy::run,
            query_estimate_accuracy::CliArgs { format }
        ),
        Some(Mode::QueryMetrics {
            work_unit_id,
            r#type,
            format,
        }) => forward!(
            query_metrics::run,
            query_metrics::CliArgs {
                work_unit_id,
                r#type,
                format,
            }
        ),
        Some(Mode::QueryWorkUnits {
            status,
            prefix,
            epic,
            r#type,
            tag,
            format,
        }) => forward!(
            query_work_units::run,
            query_work_units::CliArgs {
                status,
                prefix,
                epic,
                r#type,
                tag,
                format,
            }
        ),
        Some(Mode::QueryBottlenecks { output }) => forward!(
            query_bottlenecks::run,
            query_bottlenecks::CliArgs { output }
        ),
        Some(Mode::QueryOrphans { output, exclude_done }) => forward!(
            query_orphans::run,
            query_orphans::CliArgs { output, exclude_done }
        ),
        Some(Mode::QueryEstimationGuide { work_unit_id, format }) => forward!(
            query_estimation_guide::run,
            query_estimation_guide::CliArgs { work_unit_id, format }
        ),
        Some(Mode::QueryExampleMappingStats { format }) => forward!(
            query_example_mapping_stats::run,
            query_example_mapping_stats::CliArgs { format }
        ),
        Some(Mode::ShowEventStorm { work_unit_id }) => forward!(
            show_event_storm::run,
            show_event_storm::CliArgs { work_unit_id }
        ),
        Some(Mode::ShowFoundation {
            section,
            format,
            output,
            draft,
            list_sections,
            line_numbers,
        }) => forward!(
            show_foundation::run,
            show_foundation::CliArgs {
                section,
                format,
                output,
                draft,
                list_sections,
                line_numbers,
            }
        ),
        Some(Mode::ShowFoundationEventStorm { r#type, context }) => forward!(
            show_foundation_event_storm::run,
            show_foundation_event_storm::CliArgs { r#type, context }
        ),
        Some(Mode::ShowTestPatterns { tag, include_coverage, json }) => forward!(
            show_test_patterns::run,
            show_test_patterns::CliArgs { tag, include_coverage, json }
        ),
        Some(Mode::ShowAcceptanceCriteria { tag, format, output }) => forward!(
            show_acceptance_criteria::run,
            show_acceptance_criteria::CliArgs { tags: tag, format: Some(format), output }
        ),
        Some(Mode::ShowCoverage { feature_name, format, output }) => forward!(
            show_coverage::run,
            show_coverage::CliArgs { feature_name, format, output }
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

/// Pre-clap inspection of argv to handle `fspec <list-*> --help` / `-h`
/// without going through clap's auto-generated help block. Returns
/// `Some(exit_code)` when the request was handled (caller must exit
/// immediately); returns `None` to let clap take over.
///
/// Per RPC-247 strict byte-parity: the printed text is whatever
/// `codelet_fspec_core::help::format_command_help(&CONFIG)` produces for
/// the matched subcommand. The TS reference `node dist/index.js <cmd> --help`
/// piped to non-TTY is the contract.
fn intercept_ts_help() -> Option<u8> {
    use codelet_fspec_core::help::{configs, format_command_help};

    let args: Vec<String> = std::env::args().collect();
    // Need at least: program, subcommand, --help/-h
    if args.len() < 3 {
        return None;
    }
    let sub = args[1].as_str();
    let wants_help = args[2..].iter().any(|a| a == "--help" || a == "-h");
    if !wants_help {
        return None;
    }
    let rendered = match sub {
        "list-attachments" => format_command_help(&configs::list_attachments::CONFIG),
        "list-checkpoints" => format_command_help(&configs::list_checkpoints::CONFIG),
        "list-epics" => format_command_help(&configs::list_epics::CONFIG),
        "list-feature-tags" => format_command_help(&configs::list_feature_tags::CONFIG),
        "list-features" => format_command_help(&configs::list_features::CONFIG),
        "list-hooks" => format_command_help(&configs::list_hooks::CONFIG),
        "list-prefixes" => format_command_help(&configs::list_prefixes::CONFIG),
        "list-scenario-tags" => format_command_help(&configs::list_scenario_tags::CONFIG),
        "list-schedules" => format_command_help(&configs::list_schedules::CONFIG),
        "list-tags" => format_command_help(&configs::list_tags::CONFIG),
        "list-virtual-hooks" => format_command_help(&configs::list_virtual_hooks::CONFIG),
        "list-work-units" => format_command_help(&configs::list_work_units::CONFIG),
        "show-deleted" => format_command_help(&configs::show_deleted::CONFIG),
        "show-epic" => format_command_help(&configs::show_epic::CONFIG),
        "show-feature" => format_command_help(&configs::show_feature::CONFIG),
        "show-work-unit" => format_command_help(&configs::show_work_unit::CONFIG),
        "tag-stats" => format_command_help(&configs::tag_stats::CONFIG),
        "query-dependency-stats" => format_command_help(&configs::query_dependency_stats::CONFIG),
        "query-estimate-accuracy" => format_command_help(&configs::query_estimate_accuracy::CONFIG),
        "query-metrics" => format_command_help(&configs::query_metrics::CONFIG),
        "query-work-units" => format_command_help(&configs::query_work_units::CONFIG),
        "query-bottlenecks" => format_command_help(&configs::query_bottlenecks::CONFIG),
        "query-orphans" => format_command_help(&configs::query_orphans::CONFIG),
        "query-estimation-guide" => format_command_help(&configs::query_estimation_guide::CONFIG),
        "query-example-mapping-stats" => {
            format_command_help(&configs::query_example_mapping_stats::CONFIG)
        }
        "show-event-storm" => format_command_help(&configs::show_event_storm::CONFIG),
        "show-foundation" => format_command_help(&configs::show_foundation::CONFIG),
        "show-foundation-event-storm" => {
            format_command_help(&configs::show_foundation_event_storm::CONFIG)
        }
        "show-test-patterns" => format_command_help(&configs::show_test_patterns::CONFIG),
        "show-acceptance-criteria" => {
            format_command_help(&configs::show_acceptance_criteria::CONFIG)
        }
        "show-coverage" => format_command_help(&configs::show_coverage::CONFIG),
        // RPC-246: list-foundation-sections has no custom -help.ts in TS; the
        // reference is the bare Commander.js default output. We emit a byte-
        // for-byte static string (mirrors `node dist/index.js
        // list-foundation-sections --help` piped to non-TTY) and skip the
        // double-newline tail that the rich formatter produces.
        "list-foundation-sections" => {
            print!("{}", LIST_FOUNDATION_SECTIONS_HELP);
            return Some(0);
        }
        _ => return None,
    };
    // TS uses `console.log(formatCommandHelp(config))` which appends a
    // trailing newline. The formatter output itself already ends in `\n`
    // (final `lines.push('')` joined), so we use `println!` to mirror the
    // double-newline tail that the TS reference produces.
    println!("{rendered}");
    Some(0)
}

/// Byte-exact TS reference output of
/// `node dist/index.js list-foundation-sections --help` piped to non-TTY.
///
/// The TS reference uses bare Commander.js without a custom `-help.ts`
/// file (see `src/commands/list-foundation-sections.ts:191-202`), so we
/// reproduce Commander's default Usage/Description/Options block verbatim.
/// Captured fixture: `codelet/fspec/tests/fixtures/help/list-foundation-sections.txt`.
const LIST_FOUNDATION_SECTIONS_HELP: &str = "\
Usage: fspec list-foundation-sections [options]

List every valid foundation section with its JSON path and constraint info

Options:
  --format <format>  Output format: text (default) or json (default: \"text\")
  -h, --help         Display help for command
";
