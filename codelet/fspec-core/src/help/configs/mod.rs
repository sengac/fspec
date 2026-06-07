//! Per-command help configurations — Rust ports of `src/commands/*-help.ts`.
//!
//! Each module exposes a `pub const CONFIG: CommandHelpConfig` describing
//! the documented `--help` output for a single fspec subcommand. The
//! `format_command_help` function consumes these to produce byte-for-byte
//! parity with the TS reference output.

pub mod list_attachments;
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
