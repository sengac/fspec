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
//!   - 1 when the dispatcher returns a structured `success=false`
//!     result (missing file, malformed Gherkin, missing Feature header,
//!     etc.); the bridge inspects the JSON payload and writes
//!     `Error: <message>` to stderr BEFORE writing anything to stdout
//!     — parity with the TS CLI's
//!     `if (!result.success) { output.error('Error:', result.error); process.exit(1); }`
//!     branch at `src/commands/list-feature-tags.ts:120-123`.
//!   - 1 on any escalated [`codelet_fspec_core::FspecCoreError`]; the
//!     message is written to stderr prefixed with `Error:` (parity
//!     with the TS chalk-red `output.error('Error:', ...)` path).

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use codelet_fspec_core::commands::list_feature_tags;
use serde::Deserialize;
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

/// Subset of the canonical `ListFeatureTagsResult` JSON payload that the
/// CLI bridge needs to decide between the success-rendering path and the
/// `Error: <message>` stderr path. Only the discriminator (`success`) and
/// the error text are deserialized; the rest of the payload (tags,
/// categorizedTags, message) is intentionally ignored here — the rendered
/// text form returned in the second `format: "text"` call carries them.
///
/// This exists ONLY in the bridge — not the dispatcher core — because
/// the TS CLI action discriminates on `result.success` before invoking
/// `output.error(...)` / `process.exit(1)`. Mirroring that decision in
/// the bridge keeps `fspec_core` free of CLI-specific stream-routing.
#[derive(Debug, Deserialize)]
struct StructuredOutcome {
    success: bool,
    #[serde(default)]
    error: Option<String>,
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
    obj.insert("file".to_string(), Value::String(args.file.clone()));
    if args.show_categories {
        obj.insert("showCategories".to_string(), Value::Bool(true));
    }

    // First call — ask the dispatcher for the structured JSON payload
    // so we can inspect `success` / `error` and route errors to stderr
    // exactly like the TS CLI's
    // `if (!result.success) { output.error('Error:', result.error); process.exit(1); }`
    // branch at `src/commands/list-feature-tags.ts:120-123`. We could
    // alternatively expose a typed `pub fn list_feature_tags()` on
    // `fspec_core`, but going through the JSON contract preserves the
    // single source-of-truth boundary at the dispatcher surface.
    let mut json_args = obj.clone();
    json_args.insert("format".to_string(), Value::String("json".to_string()));
    let json_args_str = json!(json_args).to_string();
    let json_payload = list_feature_tags::run(&json_args_str, &project_root).await?;

    let outcome: StructuredOutcome =
        serde_json::from_str(&json_payload).context("parse list-feature-tags JSON payload")?;

    if !outcome.success {
        // Parity with TS `output.error('Error:', result.error)` →
        // stderr, "Error:" prefix, then `process.exit(1)`. Stdout
        // intentionally stays empty so shell pipelines that pipe
        // stdout through `jq` etc. see nothing instead of a stray
        // diagnostic line.
        let msg = outcome
            .error
            .unwrap_or_else(|| "unknown list-feature-tags error".to_string());
        eprintln!("Error: {msg}");
        return Ok(1);
    }

    // Success path — fetch the text rendering and print it. We do NOT
    // hand-render here because that would re-implement the bullet-list
    // / sentinel / categorized layout that `fspec_core` already owns
    // (RPC-244 architecture note [4]).
    let text_args_str = json!(obj).to_string();
    match list_feature_tags::run(&text_args_str, &project_root).await {
        Ok(rendered) => {
            print!("{rendered}");
            if !rendered.ends_with('\n') {
                println!();
            }
            Ok(0)
        }
        Err(err) => {
            // Defence-in-depth — the JSON call already succeeded above,
            // so the only realistic failure here is a serde/format
            // glitch. Surface it with the generic prefix and exit 1.
            eprintln!("Error: {err}");
            Ok(1)
        }
    }
}
