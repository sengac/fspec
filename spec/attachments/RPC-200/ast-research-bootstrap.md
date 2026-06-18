# AST / Source Research — `bootstrap` (RPC-200)

Port of the TypeScript `bootstrap` command (`src/commands/bootstrap.ts`) to the
Rust `fspec-core` crate. This document captures the source-level findings that
drive the Phase A specification and the Phase C porting strategy.

## 1. What `bootstrap` does (TS reference)

`export async function bootstrap(options: { cwd?: string }): Promise<string>`
returns one large documentation string. Composition order:

1. `getSlashCommandTemplate()` — returns ONLY `getHeaderSection()` (title +
   "IMMEDIATELY" block + `fspec --sync-version <version>` + `fspec bootstrap` +
   "YOU MUST RUN THOSE COMMANDS..."). Embeds the package.json version.
2. `"\n\n"` + `getCompleteWorkflowDocumentation()` — joins 17 section functions
   from `src/utils/slashCommandSections/*` with `'\n'`:
   personaIntro, acddConcept, loadContext, bootstrapFoundation,
   bigPictureEventStorm, eventStorm, exampleMapping, estimation,
   kanbanWorkflow, toolConfiguration, criticalRules, acddWorkflowExample,
   parallelization, coverageTracking, monitoringProgress, acddPrinciples,
   readyToStart.
3. A hard-coded `## Step 12: Complete Command Reference` explainer block
   (template literal in bootstrap.ts lines 125-144).
4. `"\n\n" +` each of the 6 help-topic strings, in order:
   `getSpecsHelpContent()`, `getWorkHelpContent()`, `getDiscoveryHelpContent()`,
   `getMetricsHelpContent()`, `getSetupHelpContent()`, `getHooksHelpContent()`
   (each `"\n\n"`-separated). These live in `src/help.ts` and are produced by
   `captureConsoleOutput(() => display*Help())` then `stripAnsi(...)` — i.e.
   plain text, chalk colours removed.

## 2. Dynamic behaviour (the ONLY non-static parts)

* **Config placeholder replacement.** Reads `spec/fspec-config.json` (relative
  to `cwd`). If `config.tools.test.command` exists → global replace
  `/<test-command>/g`. If `config.tools.qualityCheck.commands` exists → join
  with `" && "` and global replace `/<quality-check-commands>/g`. On any read/
  parse error: silently leave placeholders intact (bootstrap still succeeds).
* **Big Picture Event Storm reminder** (`shouldPromptEventStorm(cwd)`):
  - If `spec/foundation.json` is absent → no reminder.
  - Read foundation.json. If `foundation.eventStorm.items.length > 0` → no
    reminder (already populated).
  - Else, if `spec/work-units.json` exists, find a work unit whose `id` starts
    with `FOUND-`, whose `title` (lowercased) contains `"event storm"`, and
    whose `status !== "done"`. If found → append the **work-unit variant**
    reminder (mentions the work unit id). Else → append the **no-work-unit
    variant** reminder.
  - Any error reading files → no reminder.
  - Both reminders are wrapped with `wrapInSystemReminder(...)` and prefixed
    with `"\n\n"`. Verbatim text is in bootstrap.ts lines 186-247.

## 3. Volume / byte-exactness

* Static documentation source: `src/utils/slashCommandSections/*` ≈ **2086 LOC**
  of embedded string literals, plus the 6 `display*Help()` bodies inside the
  1931-line `src/help.ts`. The combined `bootstrap` output is > 10 000 chars
  (asserted by the TS test).
* **Recommendation:** do NOT re-port 4 000+ lines of Rust string-building.
  Instead capture the byte-exact static output of `node dist/index.js bootstrap`
  run in an EMPTY directory (no config, no foundation → placeholders intact, no
  reminder) and embed it via `include_str!` of an owned asset file adjacent to
  the command (e.g. `codelet/fspec-core/src/commands/bootstrap_doc.txt`). `run()`
  then applies ONLY the two string replacements + the event-storm reminder.
  *Capturing requires a node build run — this must be done by the supervisor /
  cargo-runner (workers cannot run binaries). FLAGGED as a Phase-C dependency.*

## 4. Rust infrastructure already available for reuse

* `spec/fspec-config.json` reading: precedent in
  `codelet/fspec-core/src/commands/configure_tools.rs` (tools.test.command,
  tools.qualityCheck.commands shape).
* `foundation.json` reading: precedent in `board.rs`, `generate_foundation_md.rs`
  (eventStorm.items access).
* `work-units.json` reading: standard `io` store helpers (WorkUnit.id/title/
  status), used across the codebase.
* System-reminder wrapping: inline helpers exist in `discover_event_storm.rs`,
  `discover_foundation.rs`, `show_work_unit.rs` — mirror the exact
  `wrapInSystemReminder` byte format.
* Version constant: `init.rs` pins `FSPEC_VERSION = "0.9.3"` (parity with
  `getVersion()` reading package.json). Reuse the same literal in the embedded
  header (already baked into the captured asset if we embed).

## 5. CLI / dispatch surface

* TS registration: `bootstrap` subcommand, description "Load complete fspec
  documentation (required before using fspec commands)", **no options/flags**,
  prints result to stdout, exit 0; on error prints `Error running bootstrap:
  <msg>` and exits 1.
* Rust core stub today: `codelet/fspec-core/src/commands/bootstrap.rs`
  `pub async fn run(_args_json: &str)` → `NotYetPorted`. The port must adopt the
  2-arg signature `run(args_json: &str, project_root: &Path)` because behaviour
  depends on `cwd` (config + foundation + work-units reads).
* Both front doors (LLM dispatcher JSON + clap CLI) converge on
  `fspec_core::commands::bootstrap::run(args_json, project_root)`.

## 6. SHARED-FILE CHANGES required (supervisor, Phase C)

1. `dispatch.rs` — bootstrap arm → `commands::bootstrap::run(args_json,
   project_root).await`; remove the stub arm. Signature change from 1-arg to
   2-arg.
2. `canonical.rs` — add `"bootstrap"` to `PORTED_COMMANDS`.
3. `help/configs/mod.rs` — register `pub mod bootstrap`.
4. `main.rs` — add `mod bootstrap`, a `Mode::Bootstrap` clap variant (NO
   positional args, NO flags), the `forward!` arm, and the `--help` intercept
   arm.
5. `commands/mod.rs` — stub already registered; verify only.
6. **Capture asset** (`bootstrap_doc.txt` byte-exact from `node dist/index.js
   bootstrap` in an empty dir) — needs a node build; supervisor/cargo-runner.

## 7. Async assessment

NONE. Pure blocking `std::fs` reads (config, foundation, work-units) + string
replacement + in-memory concatenation. No network, no child process, no real
tokio `.await` — fully compatible with `poll_sync_future`. The only TS I/O is
`readFile`, all mirrored with blocking `std::fs::read_to_string`.
