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
