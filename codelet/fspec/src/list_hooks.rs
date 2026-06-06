//! `list-hooks` shell-facing CLI bridge (RPC-247).
//!
//! Feature: spec/features/list-hooks-cli-subcommand.feature
//!
//! Per RPC-003 §7/§11 the standalone fspec Rust binary uses clap v4
//! derive as the Commander.js equivalent. This module is the thin
//! façade that parses argv (the `Mode::ListHooks` clap variant in
//! [`crate::main`]) and delegates to the single source-of-truth in
//! [`codelet_fspec_core::commands::list_hooks::run`] — the SAME function
//! the LLM-facing agent_loop dispatcher invokes.
//!
//! Two-front-doors pattern (architecture note [7] on RPC-253, reused
//! here for RPC-247):
//!   - Shell argv         → clap → this module → fspec_core::commands::list_hooks::run
//!   - LLM tool call JSON → fspec_core::dispatch::dispatch_command → fspec_core::commands::list_hooks::run
//!
//! Both call sites pass a JSON-encoded args shape and a
//! `project_root: &Path`. The CLI surface resolves project_root from
//! CWD (parity with the TypeScript `process.cwd()` default at
//! `src/commands/list-hooks.ts:25`). The clap subcommand carries NO
//! flags — matching the flag-less TS Commander.js registration at
//! `src/commands/list-hooks.ts:47-54`. No event-aggregation or
//! rendering logic is duplicated here.
//!
//! Byte-parity contract with the TS canonical CLI:
//!   - The TypeScript Commander.js action at
//!     `src/commands/list-hooks.ts:51-53` is
//!     `.action(async (options) => { await listHooks(options); })`
//!     and DISCARDS the returned `ListHooksResult` without ever
//!     calling `console.log`. The TS CLI writes **zero bytes** to
//!     stdout on every input (missing file, empty hooks, populated
//!     config, invalid JSON, etc.).
//!   - This bridge mirrors that exactly: we invoke
//!     `list_hooks::run` (to preserve any future side-effects and to
//!     surface real errors) but we DO NOT print its rendered text.
//!     The dispatcher path retains the rendered text for the
//!     structured-output contract used by the LLM tool-call protocol;
//!     only the shell-CLI surface stays silent.
//!
//! Exit-code contract:
//!   - 0 on success; stdout is empty (byte-parity with TS).
//!   - 1 on any [`codelet_fspec_core::FspecCoreError`]; the message is
//!     written to stderr prefixed with `Error:`. In practice the
//!     broad-swallow semantics of the underlying command mean this
//!     error path is rarely hit — missing files and malformed JSON
//!     both succeed silently with the empty sentinel.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_hooks;
use serde_json::json;

/// Strongly-typed args mirrored from the TypeScript Commander.js flag
/// set for `list-hooks` (`src/commands/list-hooks.ts:47-54`).
///
/// The TS registration declares NO `.option(...)` calls, so this struct
/// currently has no public fields — the JSON shape handed to
/// `fspec_core::commands::list_hooks::run` always serialises to `{}`.
/// The struct is kept (mirroring the `list_prefixes::CliArgs` shape)
/// so future flag additions land as field additions rather than an
/// API break, and so the bridge's `run` signature stays symmetric
/// with `list_prefixes::run` for the cross-command parity expected
/// by RPC-003 §7/§11.
#[derive(Debug, Default)]
pub struct CliArgs {}

/// Entry point invoked from `main.rs` for the `list-hooks` clap
/// subcommand. Returns the process exit code so `main` can propagate
/// it verbatim via `std::process::ExitCode::from(...)`.
pub async fn run(_args: CliArgs) -> Result<u8> {
    // Resolve project root from CWD. The TS implementation uses
    // `process.cwd()` for the default; we mirror that here so
    // script-driven invocations behave identically.
    let project_root: PathBuf = env::current_dir().context("resolve current working directory")?;

    // Reconstruct the JSON args shape that
    // fspec_core::commands::list_hooks::run validates with serde. With
    // no flags currently exposed on `CliArgs`, the shape is the empty
    // object — matching both the TS Commander.js behaviour and
    // `fspec_core`'s `#[serde(default)]` arms. The marshalling lives
    // here (rather than a hard-coded `"{}"`) so adding a field to
    // `CliArgs` automatically threads through to `args_json`.
    let obj = serde_json::Map::new();
    let args_json = json!(obj).to_string();

    match list_hooks::run(&args_json, &project_root).await {
        Ok(_rendered) => {
            // BYTE-PARITY WITH TS COMMANDER.JS: the canonical TS
            // action at `src/commands/list-hooks.ts:51-53` discards the
            // returned `ListHooksResult` without printing. We MUST NOT
            // write to stdout here. The dispatcher path retains the
            // rendered text for the structured-output contract;
            // this shell-CLI surface stays silent on success.
            Ok(0)
        }
        Err(err) => {
            // Mirror the TS `output.error('Error:', ...)` path: stderr,
            // prefixed, no ANSI required. The broad-swallow semantics
            // of `list_hooks::run` mean this branch is unreachable for
            // the empty / malformed-file inputs — both fold into the
            // Ok(_) arm above with the empty-sentinel payload.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
