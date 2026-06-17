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
mod create_epic;
mod create_prefix;
mod clear_dependencies;
mod daemon;
mod delete_epic;
mod delete_tag;
mod add_dependencies;
mod add_architecture_note;
mod add_assumption;
mod add_example;
mod add_question;
mod add_rule;
mod remove_architecture_note;
mod remove_dependency;
mod remove_example;
mod remove_question;
mod remove_rule;
mod set_user_story;
// Batch 9 (2026-06-11) — dependency, q&a, tag-feature, tag-scenario, restore-*
mod add_dependency;
mod add_tag_to_feature;
mod add_tag_to_scenario;
mod answer_question;
mod remove_tag_from_feature;
mod remove_tag_from_scenario;
mod restore_architecture_note;
mod restore_example;
mod restore_question;
mod restore_rule;
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
mod register_tag;
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
mod update_prefix;
mod update_tag;
// Batch 10 (2026-06-11) — attachments, virtual hooks, hooks, diagrams
mod add_attachment;
mod add_diagram;
mod add_hook;
mod add_virtual_hook;
mod clear_virtual_hooks;
mod copy_virtual_hooks;
mod delete_diagram;
mod remove_attachment;
mod remove_hook;
mod remove_virtual_hook;
// Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
mod add_aggregate;
mod add_bounded_context;
mod add_command;
mod add_domain_event;
mod add_external_system;
mod add_hotspot;
mod add_policy;
mod create_bug;
mod create_story;
mod create_task;
// Batch 12 (2026-06-12) — work-units.json mutation + export commands
mod compact_work_unit;
mod delete_work_unit;
mod export_dependencies;
mod export_example_map;
mod export_work_units;
mod prioritize_work_unit;
mod record_iteration;
mod repair_work_units;
mod update_work_unit;
mod update_work_unit_estimate;
// Batch 13 (2026-06-12) — foundation mutation commands
mod add_capability;
mod remove_capability;
mod add_persona;
mod remove_persona;
mod add_foundation_bounded_context;
mod remove_foundation_bounded_context;
mod add_aggregate_to_foundation;
mod remove_aggregate_from_foundation;
mod add_command_to_foundation;
mod remove_command_from_foundation;
mod generate_foundation_md;

// Batch 14 (2026-06-13)
mod add_schedule;
mod remove_schedule;
mod pause_schedule;
mod resume_schedule;
mod add_domain_event_to_foundation;
mod remove_domain_event_from_foundation;
mod dependencies;
mod get_scenarios;
mod update_foundation;
mod configure_tools;
// Batch 15 (2026-06-14) — feature-file (.feature) mutation command bridges.
mod add_architecture;
mod add_background;
mod add_scenario;
mod add_step;
mod create_feature;
mod delete_features;
mod delete_scenario;
mod delete_step;
mod update_scenario;
mod update_step;

// Batch 16 (2026-06-14) — validation + search + coverage + generator/retag bridges.
mod generate_tags_md;
mod retag;
mod search_implementation;
mod search_scenarios;
mod unlink_coverage;
mod validate;
mod validate_foundation_schema;
mod validate_hooks;
mod validate_tags;
mod validate_work_units;

// Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
mod audit_coverage;
mod board;
mod check;
mod compare_implementations;
mod delete_scenarios;
mod format;
mod generate_coverage;
mod generate_summary_report;
mod import_example_map;
mod link_coverage;

// Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
mod auto_advance;
mod discover_event_storm;
mod generate_example_mapping_from_event_storm;
mod remove_init_files;
mod suggest_dependencies;
mod validate_spec_alignment;
mod workflow_automation;
mod checkpoint;
mod cleanup_checkpoints;
mod restore_checkpoint;

use std::path::PathBuf;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{CommandFactory, Parser, Subcommand};

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
    /// RPC-211: create a new epic in spec/epics.json.
    #[command(name = "create-epic", about = "Create a new epic")]
    CreateEpic {
        /// Required epic identifier (positional).
        #[arg(value_name = "EPIC_ID")]
        epic_id: String,
        /// Required epic title (positional).
        #[arg(value_name = "TITLE")]
        title: String,
        /// Optional epic description.
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
    },
    /// RPC-217: delete an epic from spec/epics.json.
    #[command(name = "delete-epic", about = "Delete an epic")]
    DeleteEpic {
        /// Required epic identifier (positional).
        #[arg(value_name = "EPIC_ID")]
        epic_id: String,
        /// Force delete even when work-units reference this epic.
        /// NOTE: long-form `--force` only — TS Commander.js does NOT
        /// expose a `-f` short alias (`src/commands/delete-epic.ts:97`)
        /// and accepting one here would diverge from the byte-parity
        /// help fixture and from any shell script that relies on `-f`
        /// being an "unknown option" error.
        #[arg(long)]
        force: bool,
    },
    /// RPC-213: register a new work-unit prefix in spec/prefixes.json.
    #[command(name = "create-prefix", about = "Register a new work unit prefix")]
    CreatePrefix {
        /// Required prefix code, 2-6 uppercase letters (positional).
        #[arg(value_name = "PREFIX")]
        prefix: String,
        /// Required prefix description (positional).
        #[arg(value_name = "DESCRIPTION")]
        description: String,
    },
    /// RPC-265: register a new tag in spec/tags.json.
    #[command(name = "register-tag", about = "Register a new tag")]
    RegisterTag {
        /// Tag name (positional; e.g. `@critical`).
        #[arg(value_name = "TAG")]
        tag: String,
        /// Tag category (positional).
        #[arg(value_name = "CATEGORY")]
        category: String,
        /// Tag description (positional).
        #[arg(value_name = "DESCRIPTION")]
        description: String,
    },
    /// RPC-313: update an existing prefix in spec/prefixes.json.
    #[command(name = "update-prefix", about = "Update an existing prefix")]
    UpdatePrefix {
        /// Required prefix code (positional).
        #[arg(value_name = "PREFIX")]
        prefix: String,
        /// New description for the prefix.
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
    },
    /// RPC-316: update an existing tag in spec/tags.json.
    #[command(name = "update-tag", about = "Update an existing tag")]
    UpdateTag {
        /// Tag name (positional).
        #[arg(value_name = "TAG")]
        tag: String,
        /// New category for the tag.
        #[arg(short = 'c', long, value_name = "CATEGORY")]
        category: Option<String>,
        /// New description for the tag.
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
    },
    /// RPC-176: add multiple dependencies to a work unit at once.
    #[command(name = "add-dependencies", about = "Add multiple dependencies to a work unit")]
    AddDependencies {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Work-unit IDs this unit blocks (variadic per flag).
        #[arg(long, num_args = 1.., value_name = "IDS")]
        blocks: Option<Vec<String>>,
        /// Work-unit IDs blocking this unit (variadic per flag).
        #[arg(long = "blocked-by", num_args = 1.., value_name = "IDS")]
        blocked_by: Option<Vec<String>>,
        /// Work-unit IDs this unit depends on (variadic per flag).
        #[arg(long = "depends-on", num_args = 1.., value_name = "IDS")]
        depends_on: Option<Vec<String>>,
        /// Related work-unit IDs (variadic per flag).
        #[arg(long = "relates-to", num_args = 1.., value_name = "IDS")]
        relates_to: Option<Vec<String>>,
    },
    /// RPC-222: delete a tag from spec/tags.json.
    #[command(name = "delete-tag", about = "Delete a tag")]
    DeleteTag {
        /// Tag name (positional).
        #[arg(value_name = "TAG")]
        tag: String,
        /// Force delete even when the tag is in use.
        #[arg(short = 'f', long)]
        force: bool,
        /// Dry-run: report intended deletion without writing.
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// RPC-271: remove a dependency relationship between two work units.
    #[command(name = "remove-dependency", about = "Remove a dependency relationship")]
    RemoveDependency {
        /// Source work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Optional positional shorthand: removes a depends-on link.
        #[arg(value_name = "DEPENDS_ON_ID")]
        depends_on_positional: Option<String>,
        /// Remove a blocks edge.
        #[arg(long, value_name = "TARGET_ID")]
        blocks: Option<String>,
        /// Remove a blocked-by edge.
        #[arg(long = "blocked-by", value_name = "TARGET_ID")]
        blocked_by: Option<String>,
        /// Remove a depends-on edge.
        #[arg(long = "depends-on", value_name = "TARGET_ID")]
        depends_on: Option<String>,
        /// Remove a relates-to edge.
        #[arg(long = "relates-to", value_name = "TARGET_ID")]
        relates_to: Option<String>,
    },
    /// RPC-204: clear all dependencies from a work unit.
    #[command(name = "clear-dependencies", about = "Clear all dependencies from a work unit")]
    ClearDependencies {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Confirm the destructive operation.
        #[arg(long)]
        confirm: bool,
    },
    /// RPC-189: add a business rule to a work unit (Blue card in Example Mapping).
    #[command(name = "add-rule", about = "Add a business rule to a work unit (Blue card in Example Mapping)")]
    AddRule {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Business rule description (positional).
        #[arg(value_name = "RULE")]
        rule: String,
    },
    /// RPC-279: remove a business rule from a work unit by index.
    #[command(name = "remove-rule", about = "Remove a business rule from a work unit by index")]
    RemoveRule {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Rule index (0-based, positional). Accepted as a raw string so TS
        /// `parseInt('abc', 10) → NaN` semantics are preserved when the
        /// caller passes a non-numeric value — the core then surfaces
        /// `"Rule with ID NaN not found"` instead of clap exiting with code 2.
        #[arg(value_name = "INDEX")]
        index: String,
    },
    /// RPC-169: add an assumption to a work unit during specification.
    #[command(name = "add-assumption", about = "Add assumption to work unit during specification")]
    AddAssumption {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Assumption text (positional).
        #[arg(value_name = "ASSUMPTION")]
        assumption: String,
    },
    /// RPC-181: add an example to a work unit (Green card in Example Mapping).
    #[command(name = "add-example", about = "Add an example to a work unit during specification phase")]
    AddExample {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Example description (positional).
        #[arg(value_name = "EXAMPLE")]
        example: String,
    },
    /// RPC-273: remove an example from a work unit by index.
    #[command(name = "remove-example", about = "Remove an example from a work unit by index")]
    RemoveExample {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Example index (0-based, positional). Accepted as a raw string so
        /// TS `parseInt('abc', 10) → NaN` semantics are preserved when the
        /// caller passes a non-numeric value — the core then surfaces
        /// `"Example with ID NaN not found"` instead of clap exiting with
        /// code 2. `allow_hyphen_values` lets clap accept negative integers
        /// such as `-1` so TS-style `parseInt('-1', 10) → -1` parity holds.
        #[arg(value_name = "INDEX", allow_hyphen_values = true)]
        index: String,
    },
    /// RPC-188: add a question to a work unit (Red card in Example Mapping).
    #[command(name = "add-question", about = "Add a question to a work unit during specification phase")]
    AddQuestion {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Question text (positional).
        #[arg(value_name = "QUESTION")]
        question: String,
    },
    /// RPC-278: remove a question from a work unit by index.
    #[command(name = "remove-question", about = "Remove a question from a work unit by index")]
    RemoveQuestion {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Question index (0-based, positional). Accepted as a raw String
        /// so the bridge can preserve TS `parseInt(_, 10)` semantics
        /// (non-numeric input → "NaN" → canonical not-found error).
        #[arg(value_name = "INDEX")]
        index: String,
    },
    /// RPC-168: add an architecture note to a work unit during Example Mapping.
    #[command(name = "add-architecture-note", about = "Add architecture note to work unit during Example Mapping")]
    AddArchitectureNote {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Architecture note text (positional).
        #[arg(value_name = "NOTE")]
        note: String,
    },
    /// RPC-267: remove an architecture note from a work unit by index.
    #[command(name = "remove-architecture-note", about = "Remove architecture note from work unit by index")]
    RemoveArchitectureNote {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Index of note to remove (0-based, positional). Accepted as a
        /// raw String so the bridge can preserve TS `parseInt(_, 10)`
        /// semantics (non-numeric input → "NaN" → canonical not-found
        /// error).
        #[arg(value_name = "INDEX")]
        index: String,
    },
    /// RPC-298: set user-story fields (role/action/benefit) for a work unit.
    #[command(name = "set-user-story", about = "Set user story fields for work unit")]
    SetUserStory {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// User role (As a...).
        #[arg(long, value_name = "ROLE")]
        role: String,
        /// User action (I want to...).
        #[arg(long, value_name = "ACTION")]
        action: String,
        /// User benefit (So that...).
        #[arg(long, value_name = "BENEFIT")]
        benefit: String,
    },
    /// RPC-177: add a dependency relationship between two work units.
    #[command(name = "add-dependency", about = "Add dependency relationship between two work units")]
    AddDependency {
        /// Required source work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Optional positional shorthand: adds a depends-on link.
        #[arg(value_name = "DEPENDS_ON_ID")]
        depends_on_positional: Option<String>,
        /// Add a blocks edge.
        #[arg(long, value_name = "TARGET_ID")]
        blocks: Option<String>,
        /// Add a blocked-by edge.
        #[arg(long = "blocked-by", value_name = "TARGET_ID")]
        blocked_by: Option<String>,
        /// Add a depends-on edge.
        #[arg(long = "depends-on", value_name = "TARGET_ID")]
        depends_on: Option<String>,
        /// Add a relates-to edge.
        #[arg(long = "relates-to", value_name = "TARGET_ID")]
        relates_to: Option<String>,
    },
    /// RPC-196: answer an Example Mapping question and optionally promote it to a rule/assumption.
    #[command(name = "answer-question", about = "Answer a question and optionally promote it")]
    AnswerQuestion {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Question index (0-based, positional).
        #[arg(value_name = "INDEX", allow_hyphen_values = true)]
        index: i64,
        /// Optional answer text.
        #[arg(long, value_name = "ANSWER")]
        answer: Option<String>,
        /// Where to promote the answer: rule|rules|assumption|assumptions|none.
        #[arg(long = "add-to", value_name = "TARGET")]
        add_to: Option<String>,
    },
    /// RPC-289: restore a soft-deleted example on a work unit.
    #[command(name = "restore-example", about = "Restore a soft-deleted example")]
    RestoreExample {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Example index/id (raw — TS `parseInt('abc', 10)` semantics).
        #[arg(value_name = "INDEX")]
        index: String,
    },
    /// RPC-291: restore a soft-deleted rule on a work unit.
    #[command(name = "restore-rule", about = "Restore a soft-deleted rule")]
    RestoreRule {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Rule index/id (raw — TS `parseInt('abc', 10)` semantics).
        #[arg(value_name = "INDEX")]
        index: String,
    },
    /// RPC-290: restore a soft-deleted question on a work unit.
    #[command(name = "restore-question", about = "Restore a soft-deleted question")]
    RestoreQuestion {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Question index (0-based, positional).
        #[arg(value_name = "INDEX")]
        index: u64,
    },
    /// RPC-287: restore a soft-deleted architecture note on a work unit.
    #[command(name = "restore-architecture-note", about = "Restore a soft-deleted architecture note")]
    RestoreArchitectureNote {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Architecture note index (0-based, positional).
        #[arg(value_name = "INDEX")]
        index: u64,
    },
    /// RPC-193: add one or more tags to a feature file.
    #[command(name = "add-tag-to-feature", about = "Add tags to a feature file")]
    AddTagToFeature {
        /// Feature file path (positional).
        #[arg(value_name = "FILE")]
        file: String,
        /// One or more tags to add (variadic positional).
        #[arg(value_name = "TAGS", num_args = 1..)]
        tags: Vec<String>,
        /// Validate added tags against `spec/tags.json` registry.
        #[arg(long = "validate-registry")]
        validate_registry: bool,
    },
    /// RPC-281: remove one or more tags from a feature file.
    #[command(name = "remove-tag-from-feature", about = "Remove tags from a feature file")]
    RemoveTagFromFeature {
        /// Feature file path (positional).
        #[arg(value_name = "FILE")]
        file: String,
        /// One or more tags to remove (variadic positional).
        #[arg(value_name = "TAGS", num_args = 1..)]
        tags: Vec<String>,
    },
    /// RPC-194: add one or more tags to a specific scenario in a feature file.
    #[command(name = "add-tag-to-scenario", about = "Add tags to a scenario in a feature file")]
    AddTagToScenario {
        /// Feature file path (positional).
        #[arg(value_name = "FILE")]
        file: String,
        /// Scenario name (positional).
        #[arg(value_name = "SCENARIO")]
        scenario_name: String,
        /// One or more tags to add (variadic positional).
        #[arg(value_name = "TAGS", num_args = 1..)]
        tags: Vec<String>,
        /// Validate added tags against `spec/tags.json` registry.
        #[arg(long = "validate-registry")]
        validate_registry: bool,
    },
    /// RPC-282: remove one or more tags from a specific scenario in a feature file.
    #[command(name = "remove-tag-from-scenario", about = "Remove tags from a scenario in a feature file")]
    RemoveTagFromScenario {
        /// Feature file path (positional).
        #[arg(value_name = "FILE")]
        file: String,
        /// Scenario name (positional).
        #[arg(value_name = "SCENARIO")]
        scenario_name: String,
        /// One or more tags to remove (variadic positional).
        #[arg(value_name = "TAGS", num_args = 1..)]
        tags: Vec<String>,
    },
    /// RPC-170: add an attachment to a work unit.
    #[command(name = "add-attachment", about = "Add an attachment to a work unit")]
    AddAttachment {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Source file path (positional).
        #[arg(value_name = "FILE_PATH")]
        file_path: String,
        /// Optional description of the attachment.
        #[arg(short = 'd', long = "description", value_name = "TEXT")]
        description: Option<String>,
    },
    /// RPC-268: remove an attachment from a work unit.
    #[command(name = "remove-attachment", about = "Remove an attachment from a work unit")]
    RemoveAttachment {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Attachment file name (positional).
        #[arg(value_name = "FILE_NAME")]
        file_name: String,
        /// Keep the file on disk; only remove the JSON reference.
        #[arg(long = "keep-file")]
        keep_file: bool,
    },
    /// RPC-195: add a virtual hook to a work unit.
    #[command(name = "add-virtual-hook", about = "Add a virtual hook to a work unit")]
    AddVirtualHook {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Event name (positional).
        #[arg(value_name = "EVENT")]
        event: String,
        /// Hook command (positional).
        #[arg(value_name = "COMMAND")]
        command: String,
        /// Blocking hook (default false).
        #[arg(long = "blocking")]
        blocking: bool,
        /// Pass git context to the hook.
        #[arg(long = "git-context")]
        git_context: bool,
    },
    /// RPC-283: remove a virtual hook from a work unit by name.
    #[command(name = "remove-virtual-hook", about = "Remove a virtual hook from a work unit by name")]
    RemoveVirtualHook {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        /// Hook name (positional).
        #[arg(value_name = "HOOK_NAME")]
        hook_name: String,
    },
    /// RPC-205: remove all virtual hooks from a work unit.
    #[command(name = "clear-virtual-hooks", about = "Clear all virtual hooks from a work unit")]
    ClearVirtualHooks {
        /// Required work-unit ID (positional).
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
    },
    /// RPC-209: copy virtual hooks from one work unit to another.
    #[command(name = "copy-virtual-hooks", about = "Copy virtual hooks from one work unit to another")]
    CopyVirtualHooks {
        /// Source work-unit ID.
        #[arg(long = "from", value_name = "FROM_ID")]
        from: Option<String>,
        /// Target work-unit ID.
        #[arg(long = "to", value_name = "TO_ID")]
        to: Option<String>,
        /// Copy only the named hook.
        #[arg(long = "hook-name", value_name = "HOOK_NAME")]
        hook_name: Option<String>,
    },
    /// RPC-184: add a project-level hook.
    #[command(name = "add-hook", about = "Add a project-level lifecycle hook")]
    AddHook {
        /// Event name (positional).
        #[arg(value_name = "EVENT")]
        event: String,
        /// Hook name (positional).
        #[arg(value_name = "NAME")]
        name: String,
        /// Hook command.
        #[arg(long = "command", value_name = "COMMAND")]
        command: String,
        /// Blocking hook (default false).
        #[arg(long = "blocking")]
        blocking: bool,
        /// Timeout in seconds.
        #[arg(long = "timeout", value_name = "SECONDS")]
        timeout: Option<u64>,
    },
    /// RPC-275: remove a project-level hook by event + name.
    #[command(name = "remove-hook", about = "Remove a project-level lifecycle hook")]
    RemoveHook {
        /// Event name (positional).
        #[arg(value_name = "EVENT")]
        event: String,
        /// Hook name (positional).
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// RPC-178: add a mermaid diagram to a foundation section.
    #[command(name = "add-diagram", about = "Add a mermaid diagram to a foundation section")]
    AddDiagram {
        /// Foundation section (positional).
        #[arg(value_name = "SECTION")]
        section: String,
        /// Diagram title (positional).
        #[arg(value_name = "TITLE")]
        title: String,
        /// Mermaid code (positional).
        #[arg(value_name = "CODE")]
        code: String,
    },
    /// RPC-216: delete a mermaid diagram from a foundation section.
    #[command(name = "delete-diagram", about = "Delete a mermaid diagram from a foundation section")]
    DeleteDiagram {
        /// Foundation section (positional).
        #[arg(value_name = "SECTION")]
        section: String,
        /// Diagram title (positional).
        #[arg(value_name = "TITLE")]
        title: String,
    },
    // Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
    /// RPC-165: add an aggregate to a work unit's Event Storm section.
    #[command(name = "add-aggregate", about = "Add aggregate to Event Storm section of work unit")]
    AddAggregate {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, value_name = "LIST")]
        responsibilities: Option<String>,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "CONTEXT")]
        bounded_context: Option<String>,
    },
    /// RPC-174: add a command to a work unit's Event Storm section.
    #[command(name = "add-command", about = "Add command to Event Storm section of work unit")]
    AddCommand {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, value_name = "ACTOR")]
        actor: Option<String>,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "CONTEXT")]
        bounded_context: Option<String>,
    },
    /// RPC-179: add a domain event to a work unit's Event Storm section.
    #[command(name = "add-domain-event", about = "Add domain event to Event Storm section of work unit")]
    AddDomainEvent {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "CONTEXT")]
        bounded_context: Option<String>,
    },
    /// RPC-185: add a hotspot to a work unit's Event Storm section.
    #[command(name = "add-hotspot", about = "Add hotspot to Event Storm section of work unit")]
    AddHotspot {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, value_name = "CONCERN")]
        concern: Option<String>,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "CONTEXT")]
        bounded_context: Option<String>,
    },
    /// RPC-172: add a bounded context to a work unit's Event Storm section.
    #[command(name = "add-bounded-context", about = "Add bounded context to Event Storm section of work unit")]
    AddBoundedContext {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, value_name = "DESCRIPTION")]
        description: Option<String>,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "CONTEXT")]
        context: Option<String>,
    },
    /// RPC-182: add an external system to a work unit's Event Storm section.
    #[command(name = "add-external-system", about = "Add external system to Event Storm section of work unit")]
    AddExternalSystem {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long = "type", value_name = "TYPE")]
        system_type: Option<String>,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "CONTEXT")]
        context: Option<String>,
    },
    /// RPC-187: add a policy item to a work unit's Event Storm section.
    #[command(name = "add-policy", about = "Add policy to Event Storm section for reactive business logic (WHEN event THEN command)")]
    AddPolicy {
        #[arg(value_name = "WORK_UNIT_ID")]
        work_unit_id: String,
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, value_name = "EVENT")]
        when: Option<String>,
        #[arg(long, value_name = "COMMAND")]
        then: Option<String>,
        #[arg(long, value_name = "MS")]
        timestamp: Option<String>,
        #[arg(long = "bounded-context", value_name = "NAME")]
        bounded_context: Option<String>,
    },
    /// RPC-214: create a new story work unit.
    #[command(name = "create-story", about = "Create a new story with Example Mapping guidance for defining acceptance criteria")]
    CreateStory {
        #[arg(value_name = "PREFIX")]
        prefix: String,
        #[arg(value_name = "TITLE")]
        title: String,
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
        #[arg(short = 'e', long, value_name = "EPIC")]
        epic: Option<String>,
        #[arg(short = 'p', long, value_name = "PARENT")]
        parent: Option<String>,
    },
    /// RPC-210: create a new bug work unit.
    #[command(name = "create-bug", about = "Create a new bug with research guidance")]
    CreateBug {
        #[arg(value_name = "PREFIX")]
        prefix: String,
        #[arg(value_name = "TITLE")]
        title: String,
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
        #[arg(short = 'e', long, value_name = "EPIC")]
        epic: Option<String>,
        #[arg(short = 'p', long, value_name = "PARENT")]
        parent: Option<String>,
    },
    /// RPC-215: create a new task work unit.
    #[command(name = "create-task", about = "Create a new task with minimal requirements")]
    CreateTask {
        #[arg(value_name = "PREFIX")]
        prefix: String,
        #[arg(value_name = "TITLE")]
        title: String,
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
        #[arg(short = 'e', long, value_name = "EPIC")]
        epic: Option<String>,
        #[arg(short = 'p', long, value_name = "PARENT")]
        parent: Option<String>,
    },
    /// RPC-317: update work unit metadata.
    #[command(name = "update-work-unit", about = "Update work unit fields (title, description, epic, parent)")]
    UpdateWorkUnit {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(short = 't', long, value_name = "TITLE")]
        title: Option<String>,
        #[arg(short = 'd', long, value_name = "DESCRIPTION")]
        description: Option<String>,
        #[arg(short = 'e', long, value_name = "EPIC")]
        epic: Option<String>,
        #[arg(short = 'p', long, value_name = "PARENT")]
        parent: Option<String>,
    },
    /// RPC-318: set a Fibonacci story-point estimate.
    #[command(name = "update-work-unit-estimate", about = "Set a Fibonacci story-point estimate on a work unit")]
    UpdateWorkUnitEstimate {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(value_name = "estimate")]
        estimate: String,
    },
    /// RPC-223: delete a work unit.
    #[command(name = "delete-work-unit", about = "Delete a work unit")]
    DeleteWorkUnit {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(long)]
        force: bool,
        #[arg(long = "skip-confirmation")]
        skip_confirmation: bool,
        #[arg(long = "cascade-dependencies")]
        cascade_dependencies: bool,
    },
    /// RPC-206: permanently remove soft-deleted items from a work unit.
    #[command(name = "compact-work-unit", about = "Permanently remove all soft-deleted items from a work unit")]
    CompactWorkUnit {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
    },
    /// RPC-255: reorder a work unit within its status column.
    #[command(name = "prioritize-work-unit", about = "Reorder a work unit within its status column")]
    PrioritizeWorkUnit {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(long, value_name = "POSITION", allow_hyphen_values = true)]
        position: Option<String>,
        #[arg(long, value_name = "workUnitId")]
        before: Option<String>,
        #[arg(long, value_name = "workUnitId")]
        after: Option<String>,
    },
    /// RPC-284: rebuild work-unit state arrays and bidirectional dependency links.
    #[command(name = "repair-work-units", about = "Repair work unit state arrays and bidirectional dependency links")]
    RepairWorkUnits {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// RPC-264: record an iteration increment on a work unit.
    #[command(name = "record-iteration", about = "Record an iteration increment on a work unit")]
    RecordIteration {
        #[arg(value_name = "name")]
        name: String,
        #[arg(long, value_name = "DATE")]
        start: Option<String>,
        #[arg(long, value_name = "DATE")]
        end: Option<String>,
    },
    /// RPC-229: export all work units to a file.
    #[command(name = "export-work-units", about = "Export all work units to a JSON file")]
    ExportWorkUnits {
        #[arg(value_name = "format")]
        format: String,
        #[arg(value_name = "output")]
        output: String,
        #[arg(long, value_name = "STATUS")]
        status: Option<String>,
    },
    /// RPC-228: export a work unit's Example Map to a JSON file.
    #[command(name = "export-example-map", about = "Export a work unit's Example Map to a JSON file")]
    ExportExampleMap {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(value_name = "file")]
        file: String,
    },
    /// RPC-227: export the dependency graph to mermaid or JSON.
    #[command(name = "export-dependencies", about = "Export the work-unit dependency graph to mermaid or JSON")]
    ExportDependencies {
        #[arg(value_name = "format")]
        format: String,
        #[arg(value_name = "output")]
        output: String,
    },
    // Batch 13 (2026-06-12) — foundation mutation commands
    /// RPC-173: add a capability to foundation.json / .draft.
    #[command(name = "add-capability", about = "Add a capability to foundation.json or foundation.json.draft")]
    AddCapability {
        #[arg(value_name = "name")]
        name: String,
        #[arg(value_name = "description")]
        description: String,
    },
    /// RPC-269: remove a capability from foundation.json / .draft.
    #[command(name = "remove-capability", about = "Remove a capability from foundation.json or foundation.json.draft")]
    RemoveCapability {
        #[arg(value_name = "name")]
        name: String,
    },
    /// RPC-186: add a user persona to foundation.json / .draft.
    #[command(name = "add-persona", about = "Add a user persona to foundation.json or foundation.json.draft")]
    AddPersona {
        #[arg(value_name = "name")]
        name: String,
        #[arg(value_name = "description")]
        description: String,
        #[arg(long = "goal", value_name = "GOAL")]
        goal: Vec<String>,
    },
    /// RPC-277: remove a persona from foundation.json / .draft.
    #[command(name = "remove-persona", about = "Remove a persona from foundation.json or foundation.json.draft")]
    RemovePersona {
        #[arg(value_name = "name")]
        name: String,
    },
    /// RPC-183: add a bounded context to the foundation Big Picture Event Storm.
    #[command(name = "add-foundation-bounded-context", about = "Add a bounded context to foundation Big Picture Event Storm")]
    AddFoundationBoundedContext {
        #[arg(value_name = "text")]
        text: String,
    },
    /// RPC-274: remove a bounded context from the foundation Big Picture Event Storm (soft-delete).
    #[command(name = "remove-foundation-bounded-context", about = "Remove a bounded context from foundation Big Picture Event Storm (soft-delete)")]
    RemoveFoundationBoundedContext {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(long)]
        cascade: bool,
    },
    /// RPC-166: add an aggregate to a foundation bounded context.
    #[command(name = "add-aggregate-to-foundation", about = "Add an aggregate to a foundation bounded context in Big Picture Event Storm")]
    AddAggregateToFoundation {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(value_name = "aggregate-name")]
        aggregate_name: String,
        #[arg(short = 'd', long = "description", value_name = "text")]
        description: Option<String>,
    },
    /// RPC-266: remove an aggregate from a foundation bounded context (soft-delete).
    #[command(name = "remove-aggregate-from-foundation", about = "Remove an aggregate from a foundation bounded context (soft-delete)")]
    RemoveAggregateFromFoundation {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(value_name = "aggregate-name")]
        aggregate_name: String,
    },
    /// RPC-175: add a command to a foundation bounded context.
    #[command(name = "add-command-to-foundation", about = "Add a command to a foundation bounded context in Big Picture Event Storm")]
    AddCommandToFoundation {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(value_name = "command-name")]
        command_name: String,
        #[arg(short = 'd', long = "description", value_name = "text")]
        description: Option<String>,
    },
    /// RPC-270: remove a command from a foundation bounded context (soft-delete).
    #[command(name = "remove-command-from-foundation", about = "Remove a command from a foundation bounded context (soft-delete)")]
    RemoveCommandFromFoundation {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(value_name = "command-name")]
        command_name: String,
    },
    /// RPC-233: generate FOUNDATION.md from foundation.json.
    #[command(name = "generate-foundation-md", about = "Generate FOUNDATION.md from foundation.json")]
    GenerateFoundationMd {
        #[arg(long = "output", value_name = "path")]
        output: Option<String>,
    },
    /// RPC-191: add a scheduled job (agent or shell) to schedules.json.
    #[command(name = "add-schedule", about = "Add a scheduled workflow automation job")]
    AddSchedule {
        #[arg(short = 'n', long = "name", value_name = "name", allow_hyphen_values = true)]
        name: String,
        #[arg(short = 'c', long = "cron", value_name = "expression", allow_hyphen_values = true)]
        cron: String,
        #[arg(short = 'z', long = "timezone", value_name = "tz", allow_hyphen_values = true)]
        timezone: String,
        #[arg(short = 't', long = "type", value_name = "type", allow_hyphen_values = true)]
        r#type: String,
        #[arg(short = 'r', long = "role", value_name = "role", allow_hyphen_values = true)]
        role: Option<String>,
        #[arg(short = 'p', long = "prompt", value_name = "prompt", allow_hyphen_values = true)]
        prompt: Option<String>,
        #[arg(long = "command", value_name = "command", allow_hyphen_values = true)]
        command: Option<String>,
        #[arg(short = 'o', long = "overlap", value_name = "policy", default_value = "skip", allow_hyphen_values = true)]
        overlap: String,
    },
    /// RPC-280: remove a scheduled job from schedules.json.
    #[command(name = "remove-schedule", about = "Remove a scheduled job")]
    RemoveSchedule {
        #[arg(value_name = "name")]
        name: String,
    },
    /// RPC-254: pause an active scheduled job.
    #[command(name = "pause-schedule", about = "Pause an active scheduled job")]
    PauseSchedule {
        #[arg(value_name = "name")]
        name: String,
    },
    /// RPC-292: resume a paused scheduled job.
    #[command(name = "resume-schedule", about = "Resume a paused scheduled job")]
    ResumeSchedule {
        #[arg(value_name = "name")]
        name: String,
    },
    /// RPC-180: add a domain event to a foundation bounded context.
    #[command(name = "add-domain-event-to-foundation", about = "Add a domain event to a foundation bounded context in Big Picture Event Storm")]
    AddDomainEventToFoundation {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(value_name = "event-name")]
        event_name: String,
        #[arg(short = 'd', long = "description", value_name = "text")]
        description: Option<String>,
    },
    /// RPC-272: remove a domain event from a foundation bounded context (soft-delete).
    #[command(name = "remove-domain-event-from-foundation", about = "Remove a domain event from a foundation bounded context (soft-delete)")]
    RemoveDomainEventFromFoundation {
        #[arg(value_name = "context-name")]
        context_name: String,
        #[arg(value_name = "event-name")]
        event_name: String,
    },
    /// RPC-224: show dependency relationships for a work unit.
    #[command(name = "dependencies", about = "Show all dependency relationships for a work unit")]
    Dependencies {
        #[arg(value_name = "work-unit-id")]
        work_unit_id: String,
        #[arg(long = "graph")]
        graph: bool,
    },
    /// RPC-237: extract scenarios from feature files.
    #[command(name = "get-scenarios", about = "Extract scenarios from feature files with optional tag filtering")]
    GetScenarios {
        #[arg(long = "tag", value_name = "tag")]
        tag: Vec<String>,
        #[arg(long = "format", value_name = "format", default_value = "text")]
        format: String,
    },
    /// RPC-312: update a foundation field.
    #[command(name = "update-foundation", about = "Update a foundation field")]
    UpdateFoundation {
        #[arg(value_name = "section")]
        section: String,
        #[arg(value_name = "content")]
        content: String,
    },
    /// RPC-208: configure test and quality-check tool commands.
    #[command(name = "configure-tools", about = "Configure test and quality check commands")]
    ConfigureTools {
        #[arg(long = "test-command", value_name = "command")]
        test_command: Option<String>,
        #[arg(long = "quality-commands", value_name = "commands", num_args = 1..)]
        quality_commands: Option<Vec<String>>,
        #[arg(long = "reconfigure")]
        reconfigure: bool,
    },
    /// RPC-212: create a new feature file from a template.
    #[command(name = "create-feature", about = "Create a new feature file with proper Gherkin structure template")]
    CreateFeature {
        #[arg(value_name = "name")]
        name: String,
    },
    /// RPC-190: add a scenario to an existing feature file.
    #[command(name = "add-scenario", about = "Add a new scenario to an existing feature file")]
    AddScenario {
        #[arg(value_name = "file")]
        file: String,
        #[arg(value_name = "scenario-name")]
        scenario_name: String,
    },
    /// RPC-192: add a step to a scenario in a feature file.
    #[command(name = "add-step", about = "Add a step to a scenario in a feature file")]
    AddStep {
        #[arg(value_name = "file")]
        file: String,
        #[arg(value_name = "scenario")]
        scenario: String,
        #[arg(value_name = "type")]
        r#type: String,
        #[arg(value_name = "text")]
        text: String,
    },
    /// RPC-171: add or update the Background (user story) section.
    #[command(name = "add-background", about = "Add or update Background (user story) section in a feature file")]
    AddBackground {
        #[arg(value_name = "feature")]
        feature: String,
        #[arg(value_name = "text")]
        text: String,
    },
    /// RPC-167: add architecture notes to a feature file via doc strings.
    #[command(name = "add-architecture", about = "Add architecture notes to a feature file using doc strings")]
    AddArchitecture {
        #[arg(value_name = "feature")]
        file: String,
        #[arg(value_name = "text")]
        notes: String,
    },
    /// RPC-219: delete a scenario from a feature file.
    #[command(name = "delete-scenario", about = "Delete a scenario from a feature file")]
    DeleteScenario {
        #[arg(value_name = "feature")]
        file: String,
        #[arg(value_name = "scenario")]
        scenario: String,
    },
    /// RPC-221: delete a step from a scenario.
    #[command(name = "delete-step", about = "Delete a step from a scenario")]
    DeleteStep {
        #[arg(value_name = "feature")]
        file: String,
        #[arg(value_name = "scenario")]
        scenario: String,
        #[arg(value_name = "step")]
        step: String,
    },
    /// RPC-218: bulk delete feature files by tag.
    #[command(name = "delete-features", about = "Bulk delete feature files by tag")]
    DeleteFeatures {
        #[arg(long = "tag", value_name = "tag")]
        tag: Vec<String>,
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// RPC-314: update a scenario name in a feature file.
    #[command(name = "update-scenario", about = "Update a scenario name in a feature file")]
    UpdateScenario {
        #[arg(value_name = "feature")]
        file: String,
        #[arg(value_name = "old-name")]
        old_name: String,
        #[arg(value_name = "new-name")]
        new_name: String,
    },
    /// RPC-315: update step text or keyword in a scenario.
    #[command(name = "update-step", about = "Update step text or keyword in a scenario by finding and replacing the current step")]
    UpdateStep {
        #[arg(value_name = "feature")]
        feature: String,
        #[arg(value_name = "scenario")]
        scenario: String,
        #[arg(value_name = "current-step")]
        current_step: String,
        #[arg(long = "text", value_name = "text", overrides_with = "text")]
        text: Option<String>,
        #[arg(long = "keyword", value_name = "keyword", overrides_with = "keyword")]
        keyword: Option<String>,
    },
    // Batch 16 (2026-06-14) — validation + search + coverage + generator/retag.
    /// RPC-324: validate feature-file tags against spec/tags.json.
    #[command(name = "validate-tags", about = "Validate that all tags used in feature files are registered")]
    ValidateTags {
        #[arg(value_name = "file")]
        file: Option<String>,
        #[arg(long = "verbose")]
        verbose: bool,
        #[arg(long = "summary")]
        summary: bool,
    },
    /// RPC-325: validate work-units.json data integrity.
    ///
    /// NOTE: the `--fix` option appears in `--help` (rich help, byte-parity
    /// with the TS reference) but is DOCUMENTED-ONLY. The TS Commander
    /// registration declares no functional flags, so `--fix` is rejected at
    /// runtime as an unknown option. The clap variant therefore declares NO
    /// fields — passing `--fix` raises `UnknownArgument` →
    /// `error: unknown option '--fix'`, exit 1 (parity, RPC-325 rule [9]).
    #[command(name = "validate-work-units", about = "Validate work units data integrity")]
    ValidateWorkUnits {},
    /// RPC-322: validate hook configuration and verify scripts exist.
    #[command(name = "validate-hooks", about = "Validate hook configuration and verify that all hook scripts exist")]
    ValidateHooks {},
    /// RPC-321: validate foundation.json against its JSON schema.
    #[command(name = "validate-foundation-schema", about = "Validate foundation.json against its JSON schema using Ajv")]
    ValidateFoundationSchema {},
    /// RPC-320: validate Gherkin syntax in feature files.
    #[command(name = "validate", about = "Validate Gherkin syntax in feature files using @cucumber/gherkin parser")]
    Validate {
        #[arg(value_name = "file")]
        file: Option<String>,
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// RPC-297: search scenarios across all feature files.
    #[command(name = "search-scenarios", about = "Search for scenarios across all feature files")]
    SearchScenarios {
        #[arg(long = "query", value_name = "pattern")]
        query: String,
        #[arg(long = "regex")]
        regex: bool,
        #[arg(long = "json")]
        json: bool,
    },
    /// RPC-296: search implementation files linked via coverage data.
    #[command(name = "search-implementation", about = "Search implementation files for a specific function")]
    SearchImplementation {
        #[arg(long = "function", value_name = "name")]
        function: String,
        #[arg(long = "show-work-units")]
        show_work_units: bool,
        #[arg(long = "json")]
        json: bool,
    },
    /// RPC-311: remove test/impl mappings from a scenario's coverage sidecar.
    #[command(name = "unlink-coverage", about = "Remove test or implementation mappings from a scenario")]
    UnlinkCoverage {
        #[arg(value_name = "feature-name")]
        feature_name: String,
        #[arg(long = "scenario", value_name = "name")]
        scenario: String,
        #[arg(long = "test-file", value_name = "path")]
        test_file: Option<String>,
        #[arg(long = "impl-file", value_name = "path")]
        impl_file: Option<String>,
        #[arg(long = "all")]
        all: bool,
    },
    /// RPC-236: render spec/TAGS.md from spec/tags.json.
    #[command(name = "generate-tags-md", about = "Generate TAGS.md from spec/tags.json")]
    GenerateTagsMd {
        #[arg(long = "output", value_name = "output")]
        output: Option<String>,
    },
    /// RPC-293: bulk-rename a tag across all feature files.
    #[command(name = "retag", about = "Rename a tag across all feature files")]
    Retag {
        #[arg(long = "from", value_name = "tag")]
        from: Option<String>,
        #[arg(long = "to", value_name = "tag")]
        to: Option<String>,
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    // Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
    /// RPC-197: verify coverage-mapped test/impl files exist.
    #[command(name = "audit-coverage", about = "Verify that test files and implementation files referenced in coverage mappings actually exist")]
    AuditCoverage {
        #[arg(value_name = "feature-name")]
        feature_name: String,
    },
    /// RPC-199: display Kanban board of work units grouped by status.
    #[command(name = "board", about = "Display Kanban board of all work units grouped by status")]
    Board {
        #[arg(long = "format", value_name = "format")]
        format: Option<String>,
        #[arg(long = "limit", value_name = "limit")]
        limit: Option<usize>,
    },
    /// RPC-201: run all validation checks (Gherkin, tags, formatting).
    #[command(name = "check", about = "Run all validation checks: Gherkin syntax, tag compliance, and formatting")]
    Check {
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// RPC-207: compare implementation approaches across work units.
    #[command(name = "compare-implementations", about = "Compare implementation approaches across work units to identify patterns and inconsistencies")]
    CompareImplementations {
        #[arg(long = "tag", value_name = "tag")]
        tag: String,
        #[arg(long = "show-coverage")]
        show_coverage: bool,
        #[arg(long = "json")]
        json: bool,
    },
    /// RPC-220: bulk delete scenarios by tag across multiple files.
    #[command(name = "delete-scenarios", about = "Bulk delete scenarios by tag across multiple files")]
    DeleteScenarios {
        #[arg(long = "tag", value_name = "tag")]
        tags: Vec<String>,
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// RPC-230: format feature files with the AST-based Gherkin formatter.
    #[command(name = "format", about = "Format feature files with custom AST-based Gherkin formatter")]
    Format {
        #[arg(value_name = "file")]
        file: Option<String>,
    },
    /// RPC-231: generate/update .feature.coverage files.
    #[command(name = "generate-coverage", about = "Generate or update .feature.coverage files for existing .feature files")]
    GenerateCoverage {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// RPC-240: link scenarios to test/impl files for traceability.
    #[command(name = "link-coverage", about = "Link Gherkin scenarios to test files and implementation code for full traceability")]
    LinkCoverage {
        #[arg(value_name = "feature-name")]
        feature_name: String,
        #[arg(long = "scenario", value_name = "name")]
        scenario: String,
        #[arg(long = "test-file", value_name = "path")]
        test_file: Option<String>,
        #[arg(long = "test-lines", value_name = "range")]
        test_lines: Option<String>,
        #[arg(long = "impl-file", value_name = "path")]
        impl_file: Option<String>,
        #[arg(long = "impl-lines", value_name = "lines")]
        impl_lines: Option<String>,
        #[arg(long = "skip-validation")]
        skip_validation: bool,
        #[arg(long = "skip-step-validation")]
        skip_step_validation: bool,
    },
    /// RPC-235: generate a comprehensive project summary report.
    #[command(name = "generate-summary-report", about = "Generate a comprehensive project summary report")]
    GenerateSummaryReport {
        #[arg(long = "format", value_name = "format")]
        format: Option<String>,
        #[arg(long = "output", value_name = "file")]
        output: Option<String>,
    },
    /// RPC-238: import Example Mapping data from JSON into a work unit.
    #[command(name = "import-example-map", about = "Import Example Mapping data from JSON file to work unit")]
    ImportExampleMap {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(value_name = "file")]
        file: String,
    },
    // Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
    /// RPC-225: scaffold an Event Storm session on a work unit.
    #[command(name = "discover-event-storm", about = "Start Event Storming discovery for a work unit")]
    DiscoverEventStorm {
        #[arg(value_name = "work-unit-id")]
        work_unit_id: String,
    },
    /// RPC-232: transform an Event Storm into Example Mapping entries.
    #[command(name = "generate-example-mapping-from-event-storm", about = "Transform Event Storm artifacts into Example Mapping rules, examples, and questions")]
    GenerateExampleMappingFromEventStorm {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
    },
    /// RPC-309: suggest dependencies between work units.
    #[command(name = "suggest-dependencies", about = "Suggest dependencies between work units based on heuristics")]
    SuggestDependencies {
        #[arg(long = "output", value_name = "format")]
        output: Option<String>,
    },
    /// RPC-323: validate spec/test/impl alignment for a work unit.
    #[command(name = "validate-spec-alignment", about = "Validate that specifications align with tests and implementation")]
    ValidateSpecAlignment {
        #[arg(value_name = "workUnitId")]
        work_unit_id: String,
        #[arg(long = "fix")]
        fix: bool,
    },
    /// RPC-276: remove fspec init files for the detected agent.
    #[command(name = "remove-init-files", about = "Remove fspec initialization files created by init")]
    RemoveInitFiles {
        #[arg(long = "keep-config")]
        keep_config: bool,
        #[arg(long = "no-keep-config")]
        no_keep_config: bool,
    },
    /// RPC-198: auto-advance a work unit through its lifecycle.
    #[command(name = "auto-advance", about = "Automatically advance work units through their lifecycle")]
    AutoAdvance {
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// RPC-326: drive workflow automation actions for a work unit.
    #[command(name = "workflow-automation", about = "Run workflow automation actions for a work unit")]
    WorkflowAutomation {
        #[arg(value_name = "action")]
        action: String,
        #[arg(value_name = "work-unit-id")]
        work_unit_id: String,
        #[arg(long = "event", value_name = "event")]
        event: Option<String>,
        #[arg(long = "from-state", value_name = "state")]
        from_state: Option<String>,
    },
    /// RPC-202: create a manual git checkpoint for a work unit.
    #[command(name = "checkpoint", about = "Create a manual checkpoint for safe experimentation")]
    Checkpoint {
        #[arg(value_name = "work-unit-id")]
        work_unit_id: String,
        #[arg(value_name = "checkpoint-name")]
        checkpoint_name: String,
    },
    /// RPC-203: prune old checkpoints for a work unit.
    #[command(name = "cleanup-checkpoints", about = "Clean up old checkpoints for a work unit, keeping the most recent N")]
    CleanupCheckpoints {
        #[arg(value_name = "work-unit-id")]
        work_unit_id: String,
        // Parity with TS Commander.js `requiredOption('--keep-last <number>')`
        // (`src/commands/cleanup-checkpoints.ts:113`): the option is REQUIRED,
        // and Commander reports its absence (`required option '--keep-last
        // <number>' not specified`) even when the positional is also missing.
        // `allow_hyphen_values` lets clap accept a negative value (e.g. `-3`)
        // as the option's argument so it reaches our domain validation (which
        // emits the `--keep-last must be a positive number` message) instead
        // of clap treating `-3` as an unknown flag.
        #[arg(
            long = "keep-last",
            value_name = "number",
            required = true,
            allow_hyphen_values = true
        )]
        keep_last: String,
    },
    /// RPC-288: restore a checkpoint for a work unit.
    #[command(name = "restore-checkpoint", about = "Restore a checkpoint for a work unit")]
    RestoreCheckpoint {
        #[arg(value_name = "work-unit-id")]
        work_unit_id: String,
        #[arg(value_name = "checkpoint-name")]
        checkpoint_name: String,
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

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => return render_clap_error(err),
    };

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
        Some(Mode::CreateEpic { epic_id, title, description }) => forward!(
            create_epic::run,
            create_epic::CliArgs { epic_id, title, description }
        ),
        Some(Mode::DeleteEpic { epic_id, force }) => forward!(
            delete_epic::run,
            delete_epic::CliArgs { epic_id, force }
        ),
        Some(Mode::CreatePrefix { prefix, description }) => forward!(
            create_prefix::run,
            create_prefix::CliArgs { prefix, description }
        ),
        Some(Mode::RegisterTag { tag, category, description }) => forward!(
            register_tag::run,
            register_tag::CliArgs { tag, category, description }
        ),
        Some(Mode::UpdatePrefix { prefix, description }) => forward!(
            update_prefix::run,
            update_prefix::CliArgs { prefix, description }
        ),
        Some(Mode::UpdateTag { tag, category, description }) => forward!(
            update_tag::run,
            update_tag::CliArgs { tag, category, description }
        ),
        Some(Mode::AddDependencies { work_unit_id, blocks, blocked_by, depends_on, relates_to }) => forward!(
            add_dependencies::run,
            add_dependencies::CliArgs { work_unit_id, blocks, blocked_by, depends_on, relates_to }
        ),
        Some(Mode::DeleteTag { tag, force, dry_run }) => forward!(
            delete_tag::run,
            delete_tag::CliArgs { tag, force, dry_run }
        ),
        Some(Mode::RemoveDependency { work_unit_id, depends_on_positional, blocks, blocked_by, depends_on, relates_to }) => forward!(
            remove_dependency::run,
            remove_dependency::CliArgs {
                work_unit_id,
                depends_on_positional,
                blocks,
                blocked_by,
                depends_on,
                relates_to,
            }
        ),
        Some(Mode::ClearDependencies { work_unit_id, confirm }) => forward!(
            clear_dependencies::run,
            clear_dependencies::CliArgs { work_unit_id, confirm }
        ),
        Some(Mode::AddRule { work_unit_id, rule }) => forward!(
            add_rule::run,
            add_rule::CliArgs { work_unit_id, rule }
        ),
        Some(Mode::RemoveRule { work_unit_id, index }) => forward!(
            remove_rule::run,
            remove_rule::CliArgs { work_unit_id, index }
        ),
        Some(Mode::AddAssumption { work_unit_id, assumption }) => forward!(
            add_assumption::run,
            add_assumption::CliArgs { work_unit_id, assumption }
        ),
        Some(Mode::AddExample { work_unit_id, example }) => forward!(
            add_example::run,
            add_example::CliArgs { work_unit_id, example }
        ),
        Some(Mode::RemoveExample { work_unit_id, index }) => forward!(
            remove_example::run,
            remove_example::CliArgs { work_unit_id, index }
        ),
        Some(Mode::AddQuestion { work_unit_id, question }) => forward!(
            add_question::run,
            add_question::CliArgs { work_unit_id, question }
        ),
        Some(Mode::RemoveQuestion { work_unit_id, index }) => forward!(
            remove_question::run,
            remove_question::CliArgs { work_unit_id, index }
        ),
        Some(Mode::AddArchitectureNote { work_unit_id, note }) => forward!(
            add_architecture_note::run,
            add_architecture_note::CliArgs { work_unit_id, note }
        ),
        Some(Mode::RemoveArchitectureNote { work_unit_id, index }) => forward!(
            remove_architecture_note::run,
            remove_architecture_note::CliArgs { work_unit_id, index }
        ),
        Some(Mode::SetUserStory { work_unit_id, role, action, benefit }) => forward!(
            set_user_story::run,
            set_user_story::CliArgs { work_unit_id, role, action, benefit }
        ),
        // Batch 9 (2026-06-11) — dependency, q&a, tag-feature, tag-scenario, restore-*
        Some(Mode::AddDependency {
            work_unit_id,
            depends_on_positional,
            blocks,
            blocked_by,
            depends_on,
            relates_to,
        }) => forward!(
            add_dependency::run,
            add_dependency::CliArgs {
                work_unit_id,
                depends_on_positional,
                blocks,
                blocked_by,
                depends_on,
                relates_to,
            }
        ),
        Some(Mode::AnswerQuestion { work_unit_id, index, answer, add_to }) => forward!(
            answer_question::run,
            answer_question::CliArgs { work_unit_id, index, answer, add_to }
        ),
        Some(Mode::RestoreExample { work_unit_id, index }) => forward!(
            restore_example::run,
            restore_example::CliArgs { work_unit_id, index }
        ),
        Some(Mode::RestoreRule { work_unit_id, index }) => forward!(
            restore_rule::run,
            restore_rule::CliArgs { work_unit_id, index }
        ),
        Some(Mode::RestoreQuestion { work_unit_id, index }) => forward!(
            restore_question::run,
            restore_question::CliArgs { work_unit_id, index }
        ),
        Some(Mode::RestoreArchitectureNote { work_unit_id, index }) => forward!(
            restore_architecture_note::run,
            restore_architecture_note::CliArgs { work_unit_id, index }
        ),
        Some(Mode::AddTagToFeature { file, tags, validate_registry }) => forward!(
            add_tag_to_feature::run,
            add_tag_to_feature::CliArgs { file, tags, validate_registry }
        ),
        Some(Mode::RemoveTagFromFeature { file, tags }) => forward!(
            remove_tag_from_feature::run,
            remove_tag_from_feature::CliArgs { file, tags }
        ),
        Some(Mode::AddTagToScenario { file, scenario_name, tags, validate_registry }) => forward!(
            add_tag_to_scenario::run,
            add_tag_to_scenario::CliArgs { file, scenario_name, tags, validate_registry }
        ),
        Some(Mode::RemoveTagFromScenario { file, scenario_name, tags }) => forward!(
            remove_tag_from_scenario::run,
            remove_tag_from_scenario::CliArgs { file, scenario_name, tags }
        ),
        Some(Mode::AddAttachment { work_unit_id, file_path, description }) => forward!(
            add_attachment::run,
            add_attachment::CliArgs { work_unit_id, file_path, description }
        ),
        Some(Mode::RemoveAttachment { work_unit_id, file_name, keep_file }) => forward!(
            remove_attachment::run,
            remove_attachment::CliArgs { work_unit_id, file_name, keep_file }
        ),
        Some(Mode::AddVirtualHook { work_unit_id, event, command, blocking, git_context }) => forward!(
            add_virtual_hook::run,
            add_virtual_hook::CliArgs { work_unit_id, event, command, blocking, git_context }
        ),
        Some(Mode::RemoveVirtualHook { work_unit_id, hook_name }) => forward!(
            remove_virtual_hook::run,
            remove_virtual_hook::CliArgs { work_unit_id, hook_name }
        ),
        Some(Mode::ClearVirtualHooks { work_unit_id }) => forward!(
            clear_virtual_hooks::run,
            clear_virtual_hooks::CliArgs { work_unit_id }
        ),
        Some(Mode::CopyVirtualHooks { from, to, hook_name }) => forward!(
            copy_virtual_hooks::run,
            copy_virtual_hooks::CliArgs { from, to, hook_name }
        ),
        Some(Mode::AddHook { event, name, command, blocking, timeout }) => forward!(
            add_hook::run,
            add_hook::CliArgs { event, name, command, blocking, timeout }
        ),
        Some(Mode::RemoveHook { event, name }) => forward!(
            remove_hook::run,
            remove_hook::CliArgs { event, name }
        ),
        Some(Mode::AddDiagram { section, title, code }) => forward!(
            add_diagram::run,
            add_diagram::CliArgs { section, title, code }
        ),
        Some(Mode::DeleteDiagram { section, title }) => forward!(
            delete_diagram::run,
            delete_diagram::CliArgs { section, title }
        ),
        // Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
        Some(Mode::AddAggregate { work_unit_id, text, responsibilities, timestamp, bounded_context }) => forward!(
            add_aggregate::run,
            add_aggregate::CliArgs { work_unit_id, text, responsibilities, timestamp, bounded_context }
        ),
        Some(Mode::AddCommand { work_unit_id, text, actor, timestamp, bounded_context }) => forward!(
            add_command::run,
            add_command::CliArgs { work_unit_id, text, actor, timestamp, bounded_context }
        ),
        Some(Mode::AddDomainEvent { work_unit_id, text, timestamp, bounded_context }) => forward!(
            add_domain_event::run,
            add_domain_event::CliArgs { work_unit_id, text, timestamp, bounded_context }
        ),
        Some(Mode::AddHotspot { work_unit_id, text, concern, timestamp, bounded_context }) => forward!(
            add_hotspot::run,
            add_hotspot::CliArgs { work_unit_id, text, concern, timestamp, bounded_context }
        ),
        Some(Mode::AddBoundedContext { work_unit_id, text, description, timestamp, context }) => forward!(
            add_bounded_context::run,
            add_bounded_context::CliArgs { work_unit_id, text, description, timestamp, context }
        ),
        Some(Mode::AddExternalSystem { work_unit_id, text, system_type, timestamp, context }) => forward!(
            add_external_system::run,
            add_external_system::CliArgs { work_unit_id, text, system_type, timestamp, context }
        ),
        Some(Mode::AddPolicy { work_unit_id, text, when, then, timestamp, bounded_context }) => forward!(
            add_policy::run,
            add_policy::CliArgs { work_unit_id, text, when, then, timestamp, bounded_context }
        ),
        Some(Mode::CreateStory { prefix, title, description, epic, parent }) => forward!(
            create_story::run,
            create_story::CliArgs { prefix, title, description, epic, parent }
        ),
        Some(Mode::CreateBug { prefix, title, description, epic, parent }) => forward!(
            create_bug::run,
            create_bug::CliArgs { prefix, title, description, epic, parent }
        ),
        Some(Mode::CreateTask { prefix, title, description, epic, parent }) => forward!(
            create_task::run,
            create_task::CliArgs { prefix, title, description, epic, parent }
        ),
        Some(Mode::UpdateWorkUnit { work_unit_id, title, description, epic, parent }) => forward!(
            update_work_unit::run,
            update_work_unit::CliArgs { work_unit_id, title, description, epic, parent }
        ),
        Some(Mode::UpdateWorkUnitEstimate { work_unit_id, estimate }) => forward!(
            update_work_unit_estimate::run,
            update_work_unit_estimate::CliArgs { work_unit_id, points: estimate }
        ),
        Some(Mode::DeleteWorkUnit { work_unit_id, force, skip_confirmation, cascade_dependencies }) => forward!(
            delete_work_unit::run,
            delete_work_unit::CliArgs { work_unit_id, force, skip_confirmation, cascade_dependencies }
        ),
        Some(Mode::CompactWorkUnit { work_unit_id }) => forward!(
            compact_work_unit::run,
            compact_work_unit::CliArgs { work_unit_id }
        ),
        Some(Mode::PrioritizeWorkUnit { work_unit_id, position, before, after }) => forward!(
            prioritize_work_unit::run,
            prioritize_work_unit::CliArgs { work_unit_id, position, before, after }
        ),
        Some(Mode::RepairWorkUnits { dry_run }) => forward!(
            repair_work_units::run,
            repair_work_units::CliArgs { dry_run }
        ),
        Some(Mode::RecordIteration { name, start, end }) => forward!(
            record_iteration::run,
            record_iteration::CliArgs { name, start, end }
        ),
        Some(Mode::ExportWorkUnits { format, output, status }) => forward!(
            export_work_units::run,
            export_work_units::CliArgs { format, output, status }
        ),
        Some(Mode::ExportExampleMap { work_unit_id, file }) => forward!(
            export_example_map::run,
            export_example_map::CliArgs { work_unit_id, file }
        ),
        Some(Mode::ExportDependencies { format, output }) => forward!(
            export_dependencies::run,
            export_dependencies::CliArgs { format, output }
        ),
        // Batch 13 (2026-06-12) — foundation mutation commands
        Some(Mode::AddCapability { name, description }) => forward!(
            add_capability::run,
            add_capability::CliArgs { name, description }
        ),
        Some(Mode::RemoveCapability { name }) => forward!(
            remove_capability::run,
            remove_capability::CliArgs { name }
        ),
        Some(Mode::AddPersona { name, description, goal }) => forward!(
            add_persona::run,
            add_persona::CliArgs { name, description, goals: goal }
        ),
        Some(Mode::RemovePersona { name }) => forward!(
            remove_persona::run,
            remove_persona::CliArgs { name }
        ),
        Some(Mode::AddFoundationBoundedContext { text }) => forward!(
            add_foundation_bounded_context::run,
            add_foundation_bounded_context::CliArgs { text }
        ),
        Some(Mode::RemoveFoundationBoundedContext { context_name, cascade }) => forward!(
            remove_foundation_bounded_context::run,
            remove_foundation_bounded_context::CliArgs { context_name, cascade }
        ),
        Some(Mode::AddAggregateToFoundation { context_name, aggregate_name, description }) => forward!(
            add_aggregate_to_foundation::run,
            add_aggregate_to_foundation::CliArgs { context_name, aggregate_name, description }
        ),
        Some(Mode::RemoveAggregateFromFoundation { context_name, aggregate_name }) => forward!(
            remove_aggregate_from_foundation::run,
            remove_aggregate_from_foundation::CliArgs { context_name, aggregate_name }
        ),
        Some(Mode::AddCommandToFoundation { context_name, command_name, description }) => forward!(
            add_command_to_foundation::run,
            add_command_to_foundation::CliArgs { context_name, command_name, description }
        ),
        Some(Mode::RemoveCommandFromFoundation { context_name, command_name }) => forward!(
            remove_command_from_foundation::run,
            remove_command_from_foundation::CliArgs { context_name, command_name }
        ),
        Some(Mode::GenerateFoundationMd { output }) => forward!(
            generate_foundation_md::run,
            generate_foundation_md::CliArgs { output }
        ),
        Some(Mode::AddSchedule {
            name,
            cron,
            timezone,
            r#type,
            role,
            prompt,
            command,
            overlap,
        }) => forward!(
            add_schedule::run,
            add_schedule::CliArgs {
                name,
                cron,
                timezone,
                job_type: r#type,
                role,
                prompt,
                command,
                overlap,
            }
        ),
        Some(Mode::RemoveSchedule { name }) => {
            forward!(remove_schedule::run, remove_schedule::CliArgs { name })
        }
        Some(Mode::PauseSchedule { name }) => {
            forward!(pause_schedule::run, pause_schedule::CliArgs { name })
        }
        Some(Mode::ResumeSchedule { name }) => {
            forward!(resume_schedule::run, resume_schedule::CliArgs { name })
        }
        Some(Mode::AddDomainEventToFoundation { context_name, event_name, description }) => forward!(
            add_domain_event_to_foundation::run,
            add_domain_event_to_foundation::CliArgs { context_name, event_name, description }
        ),
        Some(Mode::RemoveDomainEventFromFoundation { context_name, event_name }) => forward!(
            remove_domain_event_from_foundation::run,
            remove_domain_event_from_foundation::CliArgs { context_name, event_name }
        ),
        Some(Mode::Dependencies { work_unit_id, graph }) => forward!(
            dependencies::run,
            dependencies::CliArgs { work_unit_id, graph }
        ),
        Some(Mode::GetScenarios { tag, format }) => forward!(
            get_scenarios::run,
            get_scenarios::CliArgs { tags: tag, format }
        ),
        Some(Mode::UpdateFoundation { section, content }) => forward!(
            update_foundation::run,
            update_foundation::CliArgs { section, content }
        ),
        Some(Mode::ConfigureTools { test_command, quality_commands, reconfigure }) => forward!(
            configure_tools::run,
            configure_tools::CliArgs { test_command, quality_commands, reconfigure }
        ),
        // Batch 15 (2026-06-14) — feature-file (.feature) mutation commands
        Some(Mode::CreateFeature { name }) => forward!(
            create_feature::run,
            create_feature::CliArgs { name }
        ),
        Some(Mode::AddScenario { file, scenario_name }) => forward!(
            add_scenario::run,
            add_scenario::CliArgs { feature: file, scenario: scenario_name }
        ),
        Some(Mode::AddStep { file, scenario, r#type, text }) => forward!(
            add_step::run,
            add_step::CliArgs { feature: file, scenario, step_type: r#type, text }
        ),
        Some(Mode::AddBackground { feature, text }) => forward!(
            add_background::run,
            add_background::CliArgs { feature, text }
        ),
        Some(Mode::AddArchitecture { file, notes }) => forward!(
            add_architecture::run,
            add_architecture::CliArgs { feature: file, text: notes }
        ),
        Some(Mode::DeleteScenario { file, scenario }) => forward!(
            delete_scenario::run,
            delete_scenario::CliArgs { feature: file, scenario }
        ),
        Some(Mode::DeleteStep { file, scenario, step }) => forward!(
            delete_step::run,
            delete_step::CliArgs { feature: file, scenario, step }
        ),
        Some(Mode::DeleteFeatures { tag, dry_run }) => forward!(
            delete_features::run,
            delete_features::CliArgs { tags: tag, dry_run }
        ),
        Some(Mode::UpdateScenario { file, old_name, new_name }) => forward!(
            update_scenario::run,
            update_scenario::CliArgs { file, old_name, new_name }
        ),
        Some(Mode::UpdateStep { feature, scenario, current_step, text, keyword }) => forward!(
            update_step::run,
            update_step::CliArgs { feature, scenario, current_step, text, keyword }
        ),
        // Batch 16 (2026-06-14) — validation + search + coverage + generator/retag
        Some(Mode::ValidateTags { file, verbose, summary }) => forward!(
            validate_tags::run,
            validate_tags::CliArgs { file, verbose, summary }
        ),
        Some(Mode::ValidateWorkUnits {}) => forward!(
            validate_work_units::run,
            validate_work_units::CliArgs {}
        ),
        Some(Mode::ValidateHooks {}) => forward!(
            validate_hooks::run,
            validate_hooks::CliArgs {}
        ),
        Some(Mode::ValidateFoundationSchema {}) => forward!(
            validate_foundation_schema::run,
            validate_foundation_schema::CliArgs
        ),
        Some(Mode::Validate { file, verbose }) => forward!(
            validate::run,
            validate::CliArgs { file, verbose }
        ),
        Some(Mode::SearchScenarios { query, regex, json }) => forward!(
            search_scenarios::run,
            search_scenarios::CliArgs { query, regex, json }
        ),
        Some(Mode::SearchImplementation { function, show_work_units, json }) => forward!(
            search_implementation::run,
            search_implementation::CliArgs { function, show_work_units, json }
        ),
        Some(Mode::UnlinkCoverage { feature_name, scenario, test_file, impl_file, all }) => forward!(
            unlink_coverage::run,
            unlink_coverage::CliArgs { feature_name, scenario, test_file, impl_file, all }
        ),
        Some(Mode::GenerateTagsMd { output }) => forward!(
            generate_tags_md::run,
            generate_tags_md::CliArgs { output }
        ),
        Some(Mode::Retag { from, to, dry_run }) => forward!(
            retag::run,
            retag::CliArgs { from, to, dry_run }
        ),
        // Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
        Some(Mode::AuditCoverage { feature_name }) => forward!(
            audit_coverage::run,
            audit_coverage::CliArgs { feature_name }
        ),
        Some(Mode::Board { format, limit }) => forward!(
            board::run,
            board::CliArgs { format, limit }
        ),
        Some(Mode::Check { verbose }) => forward!(
            check::run,
            check::CliArgs { verbose }
        ),
        Some(Mode::CompareImplementations { tag, show_coverage, json }) => forward!(
            compare_implementations::run,
            compare_implementations::CliArgs { tag, show_coverage, json }
        ),
        Some(Mode::DeleteScenarios { tags, dry_run }) => forward!(
            delete_scenarios::run,
            delete_scenarios::CliArgs { tags, dry_run }
        ),
        Some(Mode::Format { file }) => forward!(
            format::run,
            format::CliArgs { file }
        ),
        Some(Mode::GenerateCoverage { dry_run }) => forward!(
            generate_coverage::run,
            generate_coverage::CliArgs { dry_run }
        ),
        Some(Mode::LinkCoverage {
            feature_name,
            scenario,
            test_file,
            test_lines,
            impl_file,
            impl_lines,
            skip_validation,
            skip_step_validation,
        }) => forward!(
            link_coverage::run,
            link_coverage::CliArgs {
                feature_name,
                scenario,
                test_file,
                test_lines,
                impl_file,
                impl_lines,
                skip_validation,
                skip_step_validation,
            }
        ),
        Some(Mode::GenerateSummaryReport { format, output }) => forward!(
            generate_summary_report::run,
            generate_summary_report::CliArgs { format, output }
        ),
        Some(Mode::ImportExampleMap { work_unit_id, file }) => forward!(
            import_example_map::run,
            import_example_map::CliArgs { work_unit_id, file }
        ),
        // Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
        Some(Mode::DiscoverEventStorm { work_unit_id }) => forward!(
            discover_event_storm::run,
            discover_event_storm::CliArgs { work_unit_id }
        ),
        Some(Mode::GenerateExampleMappingFromEventStorm { work_unit_id }) => forward!(
            generate_example_mapping_from_event_storm::run,
            generate_example_mapping_from_event_storm::CliArgs { work_unit_id }
        ),
        Some(Mode::SuggestDependencies { output }) => forward!(
            suggest_dependencies::run,
            suggest_dependencies::CliArgs { output }
        ),
        Some(Mode::ValidateSpecAlignment { work_unit_id, fix }) => forward!(
            validate_spec_alignment::run,
            validate_spec_alignment::CliArgs { work_unit_id, fix }
        ),
        Some(Mode::RemoveInitFiles { keep_config, no_keep_config }) => forward!(
            remove_init_files::run,
            remove_init_files::CliArgs {
                keep_config: if no_keep_config {
                    Some(false)
                } else if keep_config {
                    Some(true)
                } else {
                    None
                }
            }
        ),
        Some(Mode::AutoAdvance { dry_run }) => forward!(
            auto_advance::run,
            auto_advance::CliArgs { dry_run }
        ),
        Some(Mode::WorkflowAutomation { action, work_unit_id, event, from_state }) => forward!(
            workflow_automation::run,
            workflow_automation::CliArgs { action, work_unit_id, event, from_state }
        ),
        Some(Mode::Checkpoint { work_unit_id, checkpoint_name }) => forward!(
            checkpoint::run,
            checkpoint::CliArgs { work_unit_id, checkpoint_name }
        ),
        Some(Mode::CleanupCheckpoints { work_unit_id, keep_last }) => forward!(
            cleanup_checkpoints::run,
            cleanup_checkpoints::CliArgs { work_unit_id, keep_last }
        ),
        Some(Mode::RestoreCheckpoint { work_unit_id, checkpoint_name }) => forward!(
            restore_checkpoint::run,
            restore_checkpoint::CliArgs { work_unit_id, checkpoint_name }
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

/// Render a clap parse error in the byte-exact Commander.js format used by
/// the TypeScript `fspec` reference, then exit with the matching code.
///
/// The TS CLI uses Commander.js, whose argument/usage errors differ from
/// clap's default output in BOTH wording and exit code:
///
/// | situation                | Commander.js (TS)                                        | exit |
/// |--------------------------|----------------------------------------------------------|------|
/// | help / version requested | clap's own help/version text                             | 0    |
/// | missing required arg     | `error: missing required argument '<name>'`              | 1    |
/// | extra positional         | `error: too many arguments for '<cmd>'. Expected N ...`  | 1    |
/// | unknown option           | `error: unknown option '<flag>'`                         | 1    |
/// | option needs a value     | `error: option '<spec>' argument missing`                | 1    |
/// | unknown subcommand       | `error: unknown command '<name>'`                        | 1    |
///
/// clap reports usage errors on stderr with exit code 2 and the multi-line
/// "the following required arguments were not provided" block. This helper
/// re-renders the FIRST offending item in Commander's single-line style.
///
/// Help (`--help`/`-h`) and version (`--version`/`-V`) are NOT errors in the
/// Commander sense — clap surfaces them as `DisplayHelp` / `DisplayVersion`
/// "errors" carrying the rendered text. We print that text verbatim on stdout
/// and exit 0, matching the TS behaviour.
fn render_clap_error(err: clap::Error) -> std::process::ExitCode {
    match err.kind() {
        // clap routes help/version text through Error::print(); reuse it so
        // the rendered block matches clap's own formatting and exits 0.
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion => {
            let _ = err.print();
            std::process::ExitCode::from(0)
        }
        ErrorKind::MissingRequiredArgument => {
            // clap stores the missing items as usage tokens. Commander
            // reports only the FIRST, and distinguishes positional arguments
            // from required options:
            //   - positional `<TITLE>`        → `missing required argument 'title'`
            //   - required option `--role ...` → `required option '--role <role>' not specified`
            match first_invalid_arg(&err) {
                Some(tok) if tok.starts_with('-') => {
                    eprintln!(
                        "error: required option '{}' not specified",
                        commander_option_spec(&tok)
                    );
                }
                Some(tok) => {
                    eprintln!(
                        "error: missing required argument '{}'",
                        commander_arg_name(&tok)
                    );
                }
                None => {
                    eprintln!("error: missing required argument ''");
                }
            }
            std::process::ExitCode::from(1)
        }
        ErrorKind::UnknownArgument => {
            // clap stores the typed token (e.g. "--zzz" or "-z"). For an
            // extra positional value clap ALSO raises UnknownArgument, but
            // Commander reports that as a "too many arguments" error — we
            // distinguish by whether the token starts with '-'.
            match first_invalid_arg(&err) {
                Some(tok) if tok.starts_with('-') => {
                    eprintln!("error: unknown option '{tok}'");
                }
                _ => {
                    eprintln!("{}", too_many_arguments_message());
                }
            }
            std::process::ExitCode::from(1)
        }
        ErrorKind::TooManyValues => {
            eprintln!("{}", too_many_arguments_message());
            std::process::ExitCode::from(1)
        }
        ErrorKind::InvalidValue if option_needs_value(&err) => {
            // clap: "a value is required for '...' but none was supplied".
            // Commander: "option '<spec>' argument missing".
            eprintln!(
                "error: option '{}' argument missing",
                invalid_arg_spec(&err).unwrap_or_default()
            );
            std::process::ExitCode::from(1)
        }
        ErrorKind::NoEquals => {
            eprintln!(
                "error: option '{}' argument missing",
                invalid_arg_spec(&err).unwrap_or_default()
            );
            std::process::ExitCode::from(1)
        }
        ErrorKind::InvalidSubcommand => {
            // clap stores the rejected subcommand under InvalidSubcommand;
            // fall back to argv[1] if absent.
            let name = match err.get(ContextKind::InvalidSubcommand) {
                Some(ContextValue::String(s)) => s.clone(),
                _ => subcommand_token().unwrap_or_default(),
            };
            eprintln!("error: unknown command '{name}'");
            std::process::ExitCode::from(1)
        }
        // Any other clap error: fall back to clap's own rendering + exit 1
        // (Commander always exits 1 on argument errors).
        _ => {
            let _ = err.print();
            std::process::ExitCode::from(1)
        }
    }
}

/// Extract the first `InvalidArg` context string from a clap error.
fn first_invalid_arg(err: &clap::Error) -> Option<String> {
    match err.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(s)) => Some(s.clone()),
        Some(ContextValue::Strings(v)) => v.first().cloned(),
        _ => None,
    }
}

/// Render the full `-d, --description <description>` style flag spec for an
/// option error (Commander includes BOTH short and long forms).
fn invalid_arg_spec(err: &clap::Error) -> Option<String> {
    match err.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(s)) => Some(commander_option_spec(s)),
        Some(ContextValue::Strings(v)) => v.first().map(|s| commander_option_spec(s)),
        _ => None,
    }
}

/// Whether an `InvalidValue` error is the "option requires a value" variant.
/// clap's `empty_value` constructor stores an EMPTY `InvalidValue` string —
/// distinguishing the "needs a value" case from a genuine bad-value rejection
/// (which carries the offending value).
fn option_needs_value(err: &clap::Error) -> bool {
    matches!(
        err.get(ContextKind::InvalidValue),
        Some(ContextValue::String(s)) if s.is_empty()
    )
}

/// Convert a clap usage token / value_name to the Commander argument name.
///
/// clap surfaces required-argument tokens from the declared `value_name`,
/// which in this codebase is either `UPPER_SNAKE` (e.g. `WORK_UNIT_ID`, auto
/// or explicit) or already the verbatim Commander name in camelCase
/// (e.g. `workUnitId`). Commander uses the `.argument('<name>')` name verbatim:
/// a single lowercase word (`title`) or camelCase for multi-word names
/// (`workUnitId`). We strip the angle brackets and:
///   - `UPPER_SNAKE` / `snake_case` (contains `_`) → `camelCase`;
///   - already mixed-case (e.g. `workUnitId`) → returned verbatim;
///   - a single all-upper or all-lower word (e.g. `TITLE`, `file`) → lowercased.
fn commander_arg_name(token: &str) -> String {
    let trimmed = token.trim_start_matches('<').trim_end_matches('>');
    if trimmed.contains('_') {
        // UPPER_SNAKE / snake_case → camelCase.
        let words: Vec<&str> = trimmed.split('_').filter(|w| !w.is_empty()).collect();
        let mut out = String::new();
        for (i, w) in words.iter().enumerate() {
            let lower = w.to_lowercase();
            if i == 0 {
                out.push_str(&lower);
            } else {
                let mut chars = lower.chars();
                if let Some(first) = chars.next() {
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                }
            }
        }
        out
    } else if trimmed.chars().any(|c| c.is_ascii_uppercase())
        && trimmed.chars().any(|c| c.is_ascii_lowercase())
    {
        // Already camelCase / mixed case (e.g. `workUnitId`) — verbatim.
        trimmed.to_string()
    } else {
        // Single all-upper or all-lower word (e.g. `TITLE`, `file`).
        trimmed.to_lowercase()
    }
}

/// Render the `-s, --long <value>` Commander option spec from clap's
/// `InvalidArg` token (which is the typed flag, e.g. `--description` or `-d`).
/// We look up the matching clap argument by long/short to reconstruct both
/// forms and the value placeholder.
fn commander_option_spec(typed: &str) -> String {
    let cmd = Cli::command();
    let sub_name = subcommand_token();
    let sub = sub_name
        .as_deref()
        .and_then(|n| cmd.get_subcommands().find(|c| c.get_name() == n));
    let search = sub.unwrap_or(&cmd);
    // clap's InvalidArg for an option error is `arg.to_string()`, e.g.
    // "--description <DESCRIPTION>" or "-d, --description <DESCRIPTION>".
    // Reduce it to the bare flag token (first whitespace/comma-delimited
    // piece) before matching against the declared arguments.
    let flag_token = typed
        .split([' ', ','])
        .next()
        .unwrap_or(typed)
        .trim();
    let flag = flag_token.trim_start_matches('-');
    for arg in search.get_arguments() {
        let matches_long = arg.get_long().map(|l| l == flag).unwrap_or(false);
        let matches_short = arg
            .get_short()
            .map(|s| s.to_string() == flag)
            .unwrap_or(false);
        if matches_long || matches_short {
            let short = arg.get_short().map(|s| format!("-{s}, ")).unwrap_or_default();
            let long = arg.get_long().map(|l| format!("--{l}")).unwrap_or_default();
            let value = arg
                .get_value_names()
                .and_then(|v| v.first())
                .map(|v| format!(" <{}>", v.as_str().to_lowercase()))
                .unwrap_or_default();
            return format!("{short}{long}{value}");
        }
    }
    typed.to_string()
}

/// Build the Commander "too many arguments" message for the current
/// subcommand: `too many arguments for '<cmd>'. Expected N arguments but
/// got M.`. N = declared positional count, M = supplied positional count.
fn too_many_arguments_message() -> String {
    let cmd = Cli::command();
    let argv: Vec<String> = std::env::args().collect();
    let sub_name = subcommand_token().unwrap_or_default();
    let sub = cmd.get_subcommands().find(|c| c.get_name() == sub_name);
    let expected = sub
        .map(|c| c.get_positionals().count())
        .unwrap_or(0);
    // Supplied positionals: argv after the subcommand, excluding flags and
    // their values. This is a best-effort count that matches Commander's
    // "got M" for the common all-positional commands.
    let got = count_supplied_positionals(&argv, sub);
    // Commander pluralises the EXPECTED noun: `argument` for 1, else
    // `arguments` (`expected === 1 ? '' : 's'`). The `got` count is never
    // pluralised in Commander's template.
    let noun = if expected == 1 { "argument" } else { "arguments" };
    format!(
        "error: too many arguments for '{sub_name}'. Expected {expected} {noun} but got {got}."
    )
}

/// Count positional tokens supplied after the subcommand, skipping options
/// and any value they consume.
fn count_supplied_positionals(argv: &[String], sub: Option<&clap::Command>) -> usize {
    let mut count = 0usize;
    let mut iter = argv.iter().skip(2); // program + subcommand
    while let Some(tok) = iter.next() {
        if tok == "--" {
            count += iter.count();
            break;
        }
        if tok.starts_with('-') && tok.len() > 1 {
            // Option: if it takes a value and has no '=', skip the next token.
            if !tok.contains('=') && option_takes_value(sub, tok) {
                let _ = iter.next();
            }
            continue;
        }
        count += 1;
    }
    count
}

/// Whether the option token (`--epic` / `-e`) for the given subcommand
/// consumes a following value.
fn option_takes_value(sub: Option<&clap::Command>, tok: &str) -> bool {
    let Some(sub) = sub else { return false };
    let flag = tok.trim_start_matches('-');
    for arg in sub.get_arguments() {
        let matches_long = arg.get_long().map(|l| l == flag).unwrap_or(false);
        let matches_short = arg
            .get_short()
            .map(|s| s.to_string() == flag)
            .unwrap_or(false);
        if matches_long || matches_short {
            // Prefer the explicit num_args when clap has finalised it; otherwise
            // fall back to the ArgAction, which the derive layer always sets
            // (Set/Append consume a value; SetTrue/SetFalse/Count/Help/Version
            // are valueless flags). Relying on num_args alone is unsafe because
            // `Command::command()` does not finalise num_args for derive-defined
            // options, leaving it `None` and mis-counting option values as
            // excess positionals (TS parity: Commander counts only true
            // positionals).
            if let Some(n) = arg.get_num_args() {
                return n.takes_values();
            }
            return matches!(
                arg.get_action(),
                clap::ArgAction::Set | clap::ArgAction::Append
            );
        }
    }
    false
}

/// The subcommand name from argv (first non-flag token after the program),
/// or `None` if absent.
fn subcommand_token() -> Option<String> {
    std::env::args().nth(1).filter(|a| !a.starts_with('-'))
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
        // Batch 7 (2026-06-10) — mutation commands
        "create-epic" => format_command_help(&configs::create_epic::CONFIG),
        "delete-epic" => format_command_help(&configs::delete_epic::CONFIG),
        "create-prefix" => format_command_help(&configs::create_prefix::CONFIG),
        "update-prefix" => format_command_help(&configs::update_prefix::CONFIG),
        "update-tag" => format_command_help(&configs::update_tag::CONFIG),
        "add-dependencies" => format_command_help(&configs::add_dependencies::CONFIG),
        "delete-tag" => format_command_help(&configs::delete_tag::CONFIG),
        "remove-dependency" => format_command_help(&configs::remove_dependency::CONFIG),
        "clear-dependencies" => format_command_help(&configs::clear_dependencies::CONFIG),
        // Batch 8 (2026-06-11) — Example Mapping mutation commands
        "add-rule" => format_command_help(&configs::add_rule::CONFIG),
        "remove-rule" => format_command_help(&configs::remove_rule::CONFIG),
        "add-assumption" => format_command_help(&configs::add_assumption::CONFIG),
        "add-example" => format_command_help(&configs::add_example::CONFIG),
        "remove-example" => format_command_help(&configs::remove_example::CONFIG),
        "add-question" => format_command_help(&configs::add_question::CONFIG),
        "remove-question" => format_command_help(&configs::remove_question::CONFIG),
        "add-architecture-note" => format_command_help(&configs::add_architecture_note::CONFIG),
        "remove-architecture-note" => format_command_help(&configs::remove_architecture_note::CONFIG),
        "set-user-story" => format_command_help(&configs::set_user_story::CONFIG),
        // Batch 9 (2026-06-11) — dependency, q&a, tag-feature, tag-scenario, restore-*
        "add-dependency" => format_command_help(&configs::add_dependency::CONFIG),
        "answer-question" => format_command_help(&configs::answer_question::CONFIG),
        "restore-example" => format_command_help(&configs::restore_example::CONFIG),
        "restore-rule" => format_command_help(&configs::restore_rule::CONFIG),
        "restore-question" => format_command_help(&configs::restore_question::CONFIG),
        "restore-architecture-note" => {
            format_command_help(&configs::restore_architecture_note::CONFIG)
        }
        "add-tag-to-feature" => format_command_help(&configs::add_tag_to_feature::CONFIG),
        "remove-tag-from-feature" => {
            format_command_help(&configs::remove_tag_from_feature::CONFIG)
        }
        "add-tag-to-scenario" => format_command_help(&configs::add_tag_to_scenario::CONFIG),
        "remove-tag-from-scenario" => {
            format_command_help(&configs::remove_tag_from_scenario::CONFIG)
        }
        // Batch 10 (2026-06-11) — attachments, virtual hooks, hooks, diagrams
        "add-attachment" => format_command_help(&configs::add_attachment::CONFIG),
        "remove-attachment" => format_command_help(&configs::remove_attachment::CONFIG),
        "add-virtual-hook" => format_command_help(&configs::add_virtual_hook::CONFIG),
        "remove-virtual-hook" => format_command_help(&configs::remove_virtual_hook::CONFIG),
        "clear-virtual-hooks" => format_command_help(&configs::clear_virtual_hooks::CONFIG),
        "copy-virtual-hooks" => format_command_help(&configs::copy_virtual_hooks::CONFIG),
        "add-hook" => format_command_help(&configs::add_hook::CONFIG),
        "remove-hook" => format_command_help(&configs::remove_hook::CONFIG),
        "add-diagram" => format_command_help(&configs::add_diagram::CONFIG),
        "delete-diagram" => format_command_help(&configs::delete_diagram::CONFIG),
        // Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
        "add-aggregate" => format_command_help(&configs::add_aggregate::CONFIG),
        "add-command" => format_command_help(&configs::add_command::CONFIG),
        "add-domain-event" => format_command_help(&configs::add_domain_event::CONFIG),
        "add-hotspot" => format_command_help(&configs::add_hotspot::CONFIG),
        "add-bounded-context" => format_command_help(&configs::add_bounded_context::CONFIG),
        "add-external-system" => format_command_help(&configs::add_external_system::CONFIG),
        "add-policy" => format_command_help(&configs::add_policy::CONFIG),
        "create-story" => format_command_help(&configs::create_story::CONFIG),
        "create-bug" => format_command_help(&configs::create_bug::CONFIG),
        "create-task" => format_command_help(&configs::create_task::CONFIG),
        // Batch 12 (2026-06-12) — work-units.json mutation + export commands
        "update-work-unit" => format_command_help(&configs::update_work_unit::CONFIG),
        "update-work-unit-estimate" => {
            format_command_help(&configs::update_work_unit_estimate::CONFIG)
        }
        "delete-work-unit" => format_command_help(&configs::delete_work_unit::CONFIG),
        "compact-work-unit" => format_command_help(&configs::compact_work_unit::CONFIG),
        "prioritize-work-unit" => format_command_help(&configs::prioritize_work_unit::CONFIG),
        "repair-work-units" => format_command_help(&configs::repair_work_units::CONFIG),
        "record-iteration" => format_command_help(&configs::record_iteration::CONFIG),
        "export-work-units" => format_command_help(&configs::export_work_units::CONFIG),
        "export-example-map" => format_command_help(&configs::export_example_map::CONFIG),
        "export-dependencies" => format_command_help(&configs::export_dependencies::CONFIG),
        // Batch 13 (2026-06-12) — foundation mutation commands
        "add-capability" => format_command_help(&configs::add_capability::CONFIG),
        "remove-capability" => format_command_help(&configs::remove_capability::CONFIG),
        "add-persona" => format_command_help(&configs::add_persona::CONFIG),
        "remove-persona" => format_command_help(&configs::remove_persona::CONFIG),
        "add-foundation-bounded-context" => {
            format_command_help(&configs::add_foundation_bounded_context::CONFIG)
        }
        "remove-foundation-bounded-context" => {
            format_command_help(&configs::remove_foundation_bounded_context::CONFIG)
        }
        "add-aggregate-to-foundation" => {
            format_command_help(&configs::add_aggregate_to_foundation::CONFIG)
        }
        "remove-aggregate-from-foundation" => {
            format_command_help(&configs::remove_aggregate_from_foundation::CONFIG)
        }
        "add-command-to-foundation" => {
            format_command_help(&configs::add_command_to_foundation::CONFIG)
        }
        "remove-command-from-foundation" => {
            format_command_help(&configs::remove_command_from_foundation::CONFIG)
        }
        // RPC-233 — generate-foundation-md
        "generate-foundation-md" => {
            format_command_help(&configs::generate_foundation_md::CONFIG)
        }
        // Batch 14 (2026-06-13)
        "add-schedule" => format_command_help(&configs::add_schedule::CONFIG),
        "remove-schedule" => format_command_help(&configs::remove_schedule::CONFIG),
        "pause-schedule" => format_command_help(&configs::pause_schedule::CONFIG),
        "resume-schedule" => format_command_help(&configs::resume_schedule::CONFIG),
        "add-domain-event-to-foundation" => {
            format_command_help(&configs::add_domain_event_to_foundation::CONFIG)
        }
        "remove-domain-event-from-foundation" => {
            format_command_help(&configs::remove_domain_event_from_foundation::CONFIG)
        }
        "dependencies" => format_command_help(&configs::dependencies::CONFIG),
        "get-scenarios" => format_command_help(&configs::get_scenarios::CONFIG),
        "update-foundation" => format_command_help(&configs::update_foundation::CONFIG),
        "configure-tools" => format_command_help(&configs::configure_tools::CONFIG),
        // Batch 15 (2026-06-14) — feature-file (.feature) mutation commands
        "create-feature" => format_command_help(&configs::create_feature::CONFIG),
        "add-scenario" => format_command_help(&configs::add_scenario::CONFIG),
        "add-step" => format_command_help(&configs::add_step::CONFIG),
        "add-background" => format_command_help(&configs::add_background::CONFIG),
        "add-architecture" => format_command_help(&configs::add_architecture::CONFIG),
        "delete-scenario" => format_command_help(&configs::delete_scenario::CONFIG),
        "delete-step" => format_command_help(&configs::delete_step::CONFIG),
        "update-scenario" => format_command_help(&configs::update_scenario::CONFIG),
        "update-step" => format_command_help(&configs::update_step::CONFIG),
        // Batch 16 (2026-06-14) — validation + search + coverage + generator/retag
        "validate-tags" => format_command_help(&configs::validate_tags::CONFIG),
        "validate-work-units" => format_command_help(&configs::validate_work_units::CONFIG),
        "validate-hooks" => format_command_help(&configs::validate_hooks::CONFIG),
        "validate-foundation-schema" => {
            format_command_help(&configs::validate_foundation_schema::CONFIG)
        }
        "validate" => format_command_help(&configs::validate::CONFIG),
        "search-scenarios" => format_command_help(&configs::search_scenarios::CONFIG),
        "search-implementation" => format_command_help(&configs::search_implementation::CONFIG),
        "unlink-coverage" => format_command_help(&configs::unlink_coverage::CONFIG),
        "generate-tags-md" => format_command_help(&configs::generate_tags_md::CONFIG),
        "retag" => format_command_help(&configs::retag::CONFIG),
        // Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
        "audit-coverage" => format_command_help(&configs::audit_coverage::CONFIG),
        // RPC-199 parity fix: `board` has NO custom `-help.ts` in TS;
        // `fspec board --help` falls through to bare Commander.js output (the
        // earlier rich CommandHelpConfig diverged ~60 lines from the TS
        // reference). Emit the byte-exact static string, mirroring the
        // `delete-scenarios` / `register-tag` special-cases below.
        "board" => {
            print!("{}", BOARD_HELP);
            return Some(0);
        }
        "check" => format_command_help(&configs::check::CONFIG),
        "compare-implementations" => {
            format_command_help(&configs::compare_implementations::CONFIG)
        }
        "format" => format_command_help(&configs::format::CONFIG),
        "generate-coverage" => format_command_help(&configs::generate_coverage::CONFIG),
        "link-coverage" => format_command_help(&configs::link_coverage::CONFIG),
        "generate-summary-report" => {
            format_command_help(&configs::generate_summary_report::CONFIG)
        }
        "import-example-map" => format_command_help(&configs::import_example_map::CONFIG),
        // Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
        "discover-event-storm" => format_command_help(&configs::discover_event_storm::CONFIG),
        "generate-example-mapping-from-event-storm" => {
            format_command_help(&configs::generate_example_mapping_from_event_storm::CONFIG)
        }
        "suggest-dependencies" => format_command_help(&configs::suggest_dependencies::CONFIG),
        "validate-spec-alignment" => {
            format_command_help(&configs::validate_spec_alignment::CONFIG)
        }
        "remove-init-files" => format_command_help(&configs::remove_init_files::CONFIG),
        "auto-advance" => format_command_help(&configs::auto_advance::CONFIG),
        "workflow-automation" => format_command_help(&configs::workflow_automation::CONFIG),
        "checkpoint" => format_command_help(&configs::checkpoint::CONFIG),
        "cleanup-checkpoints" => format_command_help(&configs::cleanup_checkpoints::CONFIG),
        "restore-checkpoint" => format_command_help(&configs::restore_checkpoint::CONFIG),
        // RPC-220: delete-scenarios has no custom -help.ts in TS; the reference
        // is bare Commander.js output (mirrors the delete-features special-case).
        "delete-scenarios" => {
            print!("{}", DELETE_SCENARIOS_HELP);
            return Some(0);
        }
        // RPC-218: delete-features has no custom -help.ts in TS; the reference
        // is bare Commander.js output. Emit the byte-exact static string,
        // mirroring the `list-foundation-sections` / `register-tag` special-cases.
        "delete-features" => {
            print!("{}", DELETE_FEATURES_HELP);
            return Some(0);
        }
        // RPC-265 follow-up: register-tag has no custom `-help.ts` in TS;
        // `node dist/index.js register-tag --help` falls through to bare
        // Commander.js. The earlier Rust port introduced a rich
        // CommandHelpConfig that diverges 18 lines from the TS reference —
        // emit the bare TS string verbatim instead, mirroring the
        // `list-foundation-sections` special-case directly below.
        "register-tag" => {
            print!("{}", REGISTER_TAG_HELP);
            return Some(0);
        }
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
/// `node dist/index.js register-tag --help` piped to non-TTY.
///
/// The TS reference has NO custom `-help.ts` for register-tag — Commander.js
/// emits its default Usage/Arguments/Options block, which the earlier Rust
/// port's `configs::register_tag::CONFIG` mismatched. Mirroring the bare
/// Commander output here keeps the help fixture byte-stable against the
/// upstream TS binary.
/// Captured fixture: `codelet/fspec/tests/fixtures/help/register-tag.txt`.
const REGISTER_TAG_HELP: &str = "\
Usage: fspec register-tag [options] <tag> <category> <description>

Register a new tag in TAGS.md registry

Arguments:
  tag          Tag name (e.g., \"@my-tag\")
  category     Category name (e.g., \"Technical Tags\")
  description  Tag description

Options:
  -h, --help   Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js delete-features --help` piped to non-TTY.
///
/// The TS reference (`src/commands/delete-features-by-tag.ts`) has NO custom
/// `-help.ts`; Commander.js emits its default Usage/Description/Options block.
/// Captured fixture: `codelet/fspec/tests/fixtures/help/delete-features.txt`.
const DELETE_FEATURES_HELP: &str = "\
Usage: fspec delete-features [options]

Bulk delete feature files by tag

Options:
  --tag <tag>  Filter by tag (can specify multiple times for AND logic)
  --dry-run    Preview deletions without making changes
  -h, --help   Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js delete-scenarios --help` piped to non-TTY.
///
/// The TS reference (`src/commands/delete-scenarios-by-tag.ts`) has NO custom
/// `-help.ts`; Commander.js emits its default Usage/Description/Options block.
/// Captured fixture: `codelet/fspec/tests/fixtures/help/delete-scenarios.txt`.
const DELETE_SCENARIOS_HELP: &str = "\
Usage: fspec delete-scenarios [options]

Bulk delete scenarios by tag across multiple files

Options:
  --tag <tag>  Filter by tag (can specify multiple times for AND logic)
  --dry-run    Preview deletions without making changes
  -h, --help   Display help for command
";

/// Byte-exact TS reference output of
/// `node dist/index.js board --help` piped to non-TTY.
///
/// The TS reference (`src/commands/display-board.ts:90-96`) registers `board`
/// with plain Commander.js and has NO custom `-help.ts`, so `fspec board
/// --help` emits Commander's default Usage/Description/Options block. The
/// earlier Rust port wrongly introduced a rich `CommandHelpConfig`; this
/// reproduces the real TS bytes (RPC-199 parity fix).
const BOARD_HELP: &str = "\
Usage: fspec board [options]

Display Kanban board of work units

Options:
  --format <format>  Output format: text or json (default: \"text\")
  --limit <limit>    Max items per column (default: \"25\")
  -h, --help         Display help for command
";

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
