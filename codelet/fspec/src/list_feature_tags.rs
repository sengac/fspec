//! `list-feature-tags` shell-facing CLI bridge (RPC-244).
//!
//! Feature: spec/features/list-feature-tags-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::ListFeatureTags` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_feature_tags::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused
//! here for RPC-244):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_feature_tags::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_feature_tags::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/list-feature-tags.ts:26`). The clap subcommand
//! exposes a single REQUIRED positional `<FILE>` plus one optional
//! `--show-categories` flag — mirroring the TS Commander.js
//! registration at `src/commands/list-feature-tags.ts:159-167` which
//! declares
//! `.command('list-feature-tags').argument('<file>', ...).option('--show-categories', ...)`.
//! No `--format`, no `--workspace`, no `--cwd`.
//!
//! No parsing / categorisation / rendering logic is duplicated here —
//! the bridge's only computation is JSON arg marshalling and CWD
//! resolution. The
//! `scenario_cli_bridge_module_embeds_no_duplicated_business_logic`
//! test scans this file for forbidden TAG-DOMAIN substrings that would
//! betray re-implementation of the dispatcher's behaviour.
//!
//! Exit-code contract (parity with RPC-253 rule [14] / RPC-251):
//!   - 0 on success; the rendered text (no ANSI) is written to stdout.
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message
//!     is written to stderr prefixed with `Error:` (parity with the
//!     TS chalk-red `output.error('Error:', ...)` path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_feature_tags;
use serde_json::{json, Map, Value};

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-feature-tags`
/// (`src/commands/list-feature-tags.ts:159-167`).
///
/// The TS registration declares exactly one positional
/// (`<file>`, required) and one `.option(...)` call
/// (`--show-categories`, boolean), so this struct carries two fields
/// only. Future flag additions land as field additions only,
/// preserving the bridge's `run` signature.
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Required feature file path (project-root-relative). The TS
    /// Commander.js positional is declared
    /// `.argument('<file>', 'Feature file path (e.g., spec/features/login.feature)')`
    /// — `String` here covers the same surface.
    pub file: String,
    /// `--show-categories` flag passed through to
    /// `fspec_core::commands::list_feature_tags::run` as the
    /// `showCategories` camelCase JSON key. `false` ⇔ omit categories.
    pub show_categories: bool,
}

/// Entry point invoked from `main.rs` for the `list-feature-tags`
/// clap subcommand. Returns the process exit code so `main` can
/// propagate it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD (parity with TS `process.cwd()`).
    let project_root: PathBuf =
        env::current_dir().context("resolve current working directory")?;

    // Marshal CliArgs → JSON object expected by
    // `fspec_core::commands::list_feature_tags::run`. The args struct
    // there uses `#[serde(rename_all = "camelCase")]`, so the
    // optional flag is keyed as `showCategories` and elided when
    // unset (mirroring the TS Commander.js `options` object where
    // omitted flags are `undefined`).
    let mut obj = Map::new();
    obj.insert("file".to_string(), Value::String(args.file));
    if args.show_categories {
        obj.insert("showCategories".to_string(), Value::Bool(true));
    }
    let args_json = json!(obj).to_string();

    match list_feature_tags::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own newline structure; print
            // as-is and append a trailing newline only when the
            // dispatcher's rendered output does not already end with
            // one. The empty-tags sentinel and error branches return
            // a single line with no trailing `\n`, so this guard is
            // load-bearing for shell-pipeline friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', error.message)`
            // path: stderr, prefixed, no ANSI required for parity
            // with RPC-253 rule [14]. The canonical error substrings
            // are carried in the Display impl of FspecCoreError
            // verbatim by fspec_core itself.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
