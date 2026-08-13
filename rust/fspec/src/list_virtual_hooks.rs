//! `list-virtual-hooks` shell-facing CLI bridge (RPC-252).
//!
//! Feature: spec/features/list-virtual-hooks-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4 derive
//! as the Commander.js equivalent. This module is the thin façade that parses
//! argv (the `Mode::ListVirtualHooks` clap variant in [`crate::main`]) and
//! delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_virtual_hooks::run`] — the SAME
//! function the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused here
//! for RPC-252):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_virtual_hooks::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_virtual_hooks::run
//!
//! Both call sites pass a JSON-encoded args shape and a `project_root: &Path`.
//! The CLI surface resolves project_root from CWD (parity with the TS
//! `process.cwd()` default at `src/commands/list-virtual-hooks.ts:21`). The
//! clap subcommand has a single REQUIRED positional `<WORK_UNIT_ID>` and no
//! flags — matching the TS Commander.js registration at
//! `src/commands/list-virtual-hooks.ts:49-53`.
//!
//! No grouping / lookup / rendering logic is duplicated here — the bridge's
//! only computation is JSON arg marshalling. The CLI-delegation test
//! `scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher`
//! scans this file for forbidden substrings that would indicate inlined
//! business logic.
//!
//! Exit-code contract:
//!   - 0 on success — including the empty-hooks sentinel emitted by
//!     fspec_core for work units with no virtual hooks (TS exits 0 on
//!     the empty case).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`] — message written to
//!     stderr prefixed with `Error:` (parity with TS chalk-red error path).
//!   - 2 (clap's own usage error) when the required positional is omitted —
//!     clap validates before this module is reached, so we never see that
//!     case here.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_virtual_hooks;
use codelet_fspec_core::FspecCoreError;
use serde_json::json;

/// Strongly-typed args mirrored from the TS Commander.js positional surface
/// for `list-virtual-hooks`. Today the TS registration declares ONLY the
/// `<workUnitId>` positional (no `.option(...)` calls), so `CliArgs` carries
/// exactly one field — kept as a `pub struct` (rather than a tuple/newtype)
/// so future flag additions (e.g. a `--format json` parity surface) land as
/// field additions only, mirroring the `list_attachments::CliArgs` shape.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub work_unit_id: String,
}

/// Entry point invoked from `main.rs` for the `list-virtual-hooks` clap
/// subcommand. Returns the process exit code so `main` can propagate it
/// verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so script-driven
    // invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that fspec_core validates with serde.
    // The dispatcher arg key is `workUnitId` (camelCase) — verified by
    // reading `ListVirtualHooksArgs` in
    // `rust/fspec-core/src/commands/list_virtual_hooks.rs` which uses
    // `#[serde(rename_all = "camelCase")]`.
    let args_json = json!({
        "workUnitId": args.work_unit_id,
    })
    .to_string();

    match list_virtual_hooks::run(&args_json, &project_root).await {
        Ok(rendered) => {
            // text format embeds its own leading + trailing newline structure
            // (see render_text in fspec-core); print as-is. The empty-result
            // sentinel has no trailing newline, so we append one for
            // shell-pipeline friendliness.
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('✗ Failed to list virtual hooks:', ...)`
            // path: stderr with the canonical prefix. The dispatcher wraps
            // domain errors (e.g. "Work unit 'X' does not exist") in
            // `FspecCoreError::InvalidArgs { reason }` — strip that wrapper
            // so the printed message matches the bare TS Error.message
            // (parity with the show_deleted bridge pattern).
            match &err {
                FspecCoreError::InvalidArgs { reason, .. } => {
                    eprintln!("✗ Failed to list virtual hooks: {reason}");
                }
                _ => {
                    eprintln!("✗ Failed to list virtual hooks: {err}");
                }
            }
            Ok(1)
        }
    }
}
