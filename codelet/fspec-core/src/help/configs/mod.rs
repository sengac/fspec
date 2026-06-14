//! Per-command help configurations — Rust ports of `src/commands/*-help.ts`.
//!
//! Each module exposes a `pub const CONFIG: CommandHelpConfig` describing
//! the documented `--help` output for a single fspec subcommand. The
//! `format_command_help` function consumes these to produce byte-for-byte
//! parity with the TS reference output.

pub mod list_attachments;
pub mod list_checkpoints;
pub mod list_epics;
pub mod list_feature_tags;
pub mod list_features;
pub mod list_hooks;
pub mod list_prefixes;
pub mod list_scenario_tags;
pub mod list_schedules;
pub mod list_tags;
pub mod list_virtual_hooks;
pub mod list_work_units;
pub mod query_bottlenecks;
pub mod query_dependency_stats;
pub mod query_estimate_accuracy;
pub mod query_estimation_guide;
pub mod query_example_mapping_stats;
pub mod query_metrics;
pub mod query_orphans;
pub mod query_work_units;
pub mod show_acceptance_criteria;
pub mod show_coverage;
pub mod show_deleted;
pub mod show_epic;
pub mod show_event_storm;
pub mod show_feature;
pub mod show_foundation;
pub mod show_foundation_event_storm;
pub mod show_test_patterns;
pub mod show_work_unit;
pub mod tag_stats;
// Batch 7 (2026-06-10) — mutation commands
pub mod add_dependencies;
pub mod clear_dependencies;
pub mod create_epic;
pub mod create_prefix;
pub mod delete_epic;
pub mod delete_tag;
// `register-tag` deliberately has no `CommandHelpConfig` — the TS source
// (`src/commands/register-tag.ts`) ships without a custom `-help.ts`, so
// `node dist/index.js register-tag --help` falls through to bare
// Commander.js. The Rust binary special-cases the help intercept in
// `codelet/fspec/src/main.rs` and emits the byte-exact static string
// `REGISTER_TAG_HELP` instead (mirrors the `list-foundation-sections`
// pattern documented in `command-port.md` §4).
pub mod remove_dependency;
pub mod update_prefix;
pub mod update_tag;
// Batch 8 (2026-06-11) — Example Mapping mutation commands
pub mod add_architecture_note;
pub mod add_assumption;
pub mod add_example;
pub mod add_question;
pub mod add_rule;
pub mod remove_architecture_note;
pub mod remove_example;
pub mod remove_question;
pub mod remove_rule;
pub mod set_user_story;
// Batch 9 (2026-06-11) — dependency, q&a, tag-feature, tag-scenario, restore-*
pub mod add_dependency;
pub mod add_tag_to_feature;
pub mod add_tag_to_scenario;
pub mod answer_question;
pub mod remove_tag_from_feature;
pub mod remove_tag_from_scenario;
pub mod restore_architecture_note;
pub mod restore_example;
pub mod restore_question;
pub mod restore_rule;
// Batch 10 (2026-06-11) — attachments, virtual hooks, hooks, diagrams
pub mod add_attachment;
pub mod add_diagram;
pub mod add_hook;
pub mod add_virtual_hook;
pub mod clear_virtual_hooks;
pub mod copy_virtual_hooks;
pub mod delete_diagram;
pub mod remove_attachment;
pub mod remove_hook;
pub mod remove_virtual_hook;
// Batch 11 (2026-06-12) — Event Storm item-add + create-* commands
pub mod add_aggregate;
pub mod add_bounded_context;
pub mod add_command;
pub mod add_domain_event;
pub mod add_external_system;
pub mod add_hotspot;
pub mod add_policy;
pub mod create_bug;
pub mod create_story;
pub mod create_task;
// Batch 12 (2026-06-12) — work-units.json mutation + export commands
pub mod compact_work_unit;
pub mod delete_work_unit;
pub mod export_dependencies;
pub mod export_example_map;
pub mod export_work_units;
pub mod prioritize_work_unit;
pub mod record_iteration;
pub mod repair_work_units;
pub mod update_work_unit;
pub mod update_work_unit_estimate;
// Batch 13 (2026-06-12) — foundation mutation commands
pub mod add_aggregate_to_foundation;
pub mod add_capability;
pub mod add_command_to_foundation;
pub mod add_foundation_bounded_context;
pub mod add_persona;
pub mod remove_aggregate_from_foundation;
pub mod remove_capability;
pub mod remove_command_from_foundation;
pub mod remove_foundation_bounded_context;
pub mod remove_persona;
// RPC-233 — generate-foundation-md
pub mod generate_foundation_md;

// Batch 14 (2026-06-13)
pub mod add_domain_event_to_foundation;
pub mod add_schedule;
pub mod configure_tools;
pub mod dependencies;
pub mod get_scenarios;
pub mod pause_schedule;
pub mod remove_domain_event_from_foundation;
pub mod remove_schedule;
pub mod resume_schedule;
pub mod update_foundation;

// Batch 15 (2026-06-14) — feature-file (.feature) mutation commands.
// delete-features has NO custom -help.ts in TS → bare Commander.js help is
// hard-coded in main.rs (DELETE_FEATURES_HELP), so no config module here.
pub mod add_architecture;
pub mod add_background;
pub mod add_scenario;
pub mod add_step;
pub mod create_feature;
pub mod delete_scenario;
pub mod delete_step;
pub mod update_scenario;
pub mod update_step;

// Batch 16 (2026-06-14) — validation + search + coverage + generator/retag.
// All ten have rich `-help.ts` in TS → normal CommandHelpConfig modules.
pub mod generate_tags_md;
pub mod retag;
pub mod search_implementation;
pub mod search_scenarios;
pub mod unlink_coverage;
pub mod validate;
pub mod validate_foundation_schema;
pub mod validate_hooks;
pub mod validate_tags;
pub mod validate_work_units;
