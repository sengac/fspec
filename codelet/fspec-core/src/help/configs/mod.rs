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
