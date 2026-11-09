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
//! `rust/fspec/src/` contains NO `tokio::runtime::Builder` /
//! `Runtime::new` calls (source-shape regression locked by RPC-005 Q9
//! and widened to scan `fspec/src/` by RPC-010).

#![allow(clippy::expect_used, clippy::unwrap_used)]

mod add_architecture_note;
mod add_assumption;
mod add_dependencies;
mod add_example;
mod add_question;
mod add_rule;
mod astgrep;
mod clear_dependencies;
mod client;
mod client_mcp_full;
mod client_mcp_results;
mod client_mcp_service;
mod combined;
mod common;
mod daemon_directory;
mod daemons_cmd;
mod workspaces_cmd; // RPC-457
                    // RPC-407: in-crate regression tests proving build_service initializes
                    // the process-global project blocklist root. Lives in src/ (not tests/)
                    // because it needs the binary-crate-private `common::build_service`.
#[cfg(test)]
mod blocklist_init_tests;
mod create_epic;
mod create_prefix;
mod daemon;
mod delete_epic;
mod delete_tag;
// CLI-016: guidance and help mentions parse with the real CLI (needs the
// binary-private `Cli`).
#[cfg(test)]
mod guidance_contract_tests;
// BUG-590: every add-hook mention names an event add-hook accepts.
#[cfg(test)]
mod bug590_add_hook_event_guidance_tests;
// BUG-566: help configs and feature steps carry no TypeScript-era leftovers.
#[cfg(test)]
mod help_and_step_leftovers_tests;
mod remove_architecture_note;
mod remove_assumption;
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
mod list_attachments;
mod list_checkpoints;
mod list_epics;
mod list_feature_tags;
mod list_features;
mod list_foundation_sections;
// DISC-003 — Rust-only extension command
mod foundation_status;
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
mod remove_tag_from_feature;
mod remove_tag_from_scenario;
mod restore_architecture_note;
mod restore_assumption;
mod restore_example;
mod restore_question;
mod restore_rule;
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
mod update_cmd;
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
mod add_aggregate_to_foundation;
mod add_capability;
mod add_command_to_foundation;
mod add_foundation_bounded_context;
mod add_persona;
mod generate_foundation_md;
mod remove_aggregate_from_foundation;
mod remove_capability;
mod remove_command_from_foundation;
mod remove_foundation_bounded_context;
mod remove_persona;

// Batch 14 (2026-06-13)
mod add_domain_event_to_foundation;
mod add_schedule;
mod configure_tools;
mod dependencies;
mod pause_schedule;
mod remove_domain_event_from_foundation;
mod remove_schedule;
mod resume_schedule;
mod update_foundation;
// Batch 15 (2026-06-14) — feature-file (.feature) mutation command bridges.
mod add_architecture;
mod add_background;
mod add_scenario;
mod add_step;
mod create_feature;
mod delete_features;
mod delete_scenario;
mod delete_step;
mod update_feature_text;
mod update_scenario;
mod update_step;

// Batch 16 (2026-06-14) — validation + search + coverage + generator/retag bridges.
mod generate_tags_md;
mod retag;
mod search_implementation;
mod unlink_coverage;
mod validate;
mod validate_config; // CONFIG-009 — Rust-only extension command
mod validate_foundation_schema;
mod validate_hooks;
mod validate_tags;
mod validate_work_units;

// Batch 17 (2026-06-15) — coverage/board/check/format/compare/import/report
mod audit; // AUDIT-003
mod board;
mod compare_implementations;
mod context; // CLI-019
mod delete_scenarios;
mod format;
mod generate_coverage;
mod generate_summary_report;
mod import_example_map;
mod link_coverage;

// Batch 18 (2026-06-16) — event-storm/analysis/work-unit-status
mod auto_advance;
mod bootstrap;
mod checkpoint;
mod cleanup_checkpoints;
mod discover_event_storm;
mod discover_foundation;
mod generate_example_mapping_from_event_storm;
mod generate_scenarios;
mod init;
mod init_selector;
mod remove_init_files;
mod report_bug_to_github;
mod research;
mod restore_checkpoint;
mod reverse;
mod review;
mod suggest_dependencies;
// BUG-285: hidden top-level `--sync-version` flag bridge.
mod sync_version;
mod update_work_unit_status;
mod validate_spec_alignment;
mod workflow_automation;
// PROV-097: dotenv-at-startup seam (load_startup_env / load_dotenv_from).
mod startup_env;

use std::path::PathBuf;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{CommandFactory, Parser, Subcommand};

// REFAC-020: whole-item composition keeps every maintained source below
// 300 lines while the resolved crate root stays token-identical.
include!("cli_registration/cli.rs");

include!("cli_registration/mode_01.rs");
include!("cli_registration/mode_02.rs");
include!("cli_registration/mode_03.rs");
include!("cli_registration/mode_04.rs");
include!("cli_registration/mode_05.rs");
include!("cli_registration/mode_06.rs");
include!("cli_registration/mode_07.rs");
include!("cli_registration/mode_08.rs");
include!("cli_registration/mode_09.rs");
include!("cli_registration/mode_finish.rs");
compose_mode_01! {}

include!("cli_registration/route_01.rs");
include!("cli_registration/route_02.rs");
include!("cli_registration/route_03.rs");
include!("cli_registration/route_04.rs");
include!("cli_registration/route_05.rs");
include!("cli_registration/route_06.rs");
include!("cli_registration/route_finish.rs");
include!("cli_registration/entry.rs");
include!("cli_registration/support_01.rs");
include!("cli_registration/support_02.rs");
include!("cli_registration/support_03.rs");
include!("cli_registration/support_04.rs");
